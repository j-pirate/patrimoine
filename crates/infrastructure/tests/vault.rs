use patrimoine_application::VaultService;
use patrimoine_domain::Settings;
use patrimoine_infrastructure::Vault;
use rusqlite::Connection;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn database_path(root: &Path) -> PathBuf {
    let generation = fs::read_to_string(root.join("CURRENT")).unwrap();
    root.join("generations")
        .join(generation.trim())
        .join("patrimoine.sqlite3")
}

fn settings(name: &str, currency: &str) -> Settings {
    Settings {
        workspace_name: name.to_owned(),
        display_currency: currency.to_owned(),
    }
}

fn sorted_documents(vault: &Vault) -> Vec<(String, String, String, u64)> {
    let mut values = vault
        .list_documents()
        .unwrap()
        .into_iter()
        .map(|document| {
            (
                document.id,
                document.original_name,
                document.sha256,
                document.size_bytes,
            )
        })
        .collect::<Vec<_>>();
    values.sort();
    values
}

#[test]
fn migration_and_settings_survive_repeated_opening_without_recreating_data() {
    let temporary = TempDir::new().unwrap();
    let root = temporary.path().join("coffre");
    let initial_generation;
    {
        let mut vault = Vault::open(&root).unwrap();
        let initial = vault.status().unwrap();
        assert_eq!(initial.schema_version, 6);
        assert_eq!(initial.document_count, 0);
        assert_eq!(initial.country_count, 4);
        assert_eq!(initial.currency_count, 5);
        initial_generation = initial.generation_id;

        let saved = vault
            .save_settings(settings("Famille d'Émilie", "TND"))
            .unwrap();
        assert_eq!(saved.workspace_name, "Famille d'Émilie");
        assert_eq!(saved.display_currency, "TND");
        assert!(vault.status().unwrap().mutation_sequence > initial.mutation_sequence);
    }

    for _ in 0..2 {
        let vault = Vault::open(&root).unwrap();
        let status = vault.status().unwrap();
        assert_eq!(status.generation_id, initial_generation);
        assert_eq!(status.settings.workspace_name, "Famille d'Émilie");
        assert_eq!(status.settings.display_currency, "TND");
        assert_eq!(status.country_count, 4);
        assert_eq!(status.currency_count, 5);
    }

    let connection = Connection::open(database_path(&root)).unwrap();
    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    let migration_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(migration_count, 6);
    let mut foreign_keys = connection.prepare("PRAGMA foreign_key_check").unwrap();
    assert!(foreign_keys.query([]).unwrap().next().unwrap().is_none());
}

#[test]
fn invalid_settings_do_not_modify_persisted_preferences_or_mutation_sequence() {
    let temporary = TempDir::new().unwrap();
    let mut vault = Vault::open(temporary.path().join("coffre")).unwrap();
    vault.save_settings(settings("Maison", "CHF")).unwrap();
    let before = vault.status().unwrap();

    assert!(vault.save_settings(settings("  ", "EUR")).is_err());
    assert!(vault.save_settings(settings("Maison", "XXX")).is_err());

    let after = vault.status().unwrap();
    assert_eq!(
        after.settings.workspace_name,
        before.settings.workspace_name
    );
    assert_eq!(
        after.settings.display_currency,
        before.settings.display_currency
    );
    assert_eq!(after.mutation_sequence, before.mutation_sequence);
}

#[test]
fn originals_are_owned_by_the_vault_and_duplicate_bytes_share_one_blob() {
    let temporary = TempDir::new().unwrap();
    let mut vault = Vault::open(temporary.path().join("coffre")).unwrap();
    let content = b"%PDF-1.7\nDocument familial de test\n";
    let first_source = temporary.path().join("acte.pdf");
    let second_source = temporary.path().join("copie.pdf");
    fs::write(&first_source, content).unwrap();
    fs::write(&second_source, content).unwrap();

    let first = vault
        .import_document(&first_source, "Acte original")
        .unwrap();
    let second = vault
        .import_document(&second_source, "Copie logique")
        .unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(first.sha256, second.sha256);
    assert_eq!(first.sha256.len(), 64);
    assert_eq!(first.size_bytes, content.len() as u64);
    assert_eq!(first.original_name, "acte.pdf");
    assert_eq!(second.original_name, "copie.pdf");

    fs::remove_file(first_source).unwrap();
    fs::remove_file(second_source).unwrap();
    let destination = temporary.path().join("document-exporte.pdf");
    vault.export_document(&first.id, &destination).unwrap();
    assert_eq!(fs::read(&destination).unwrap(), content);
    assert!(vault.export_document(&first.id, &destination).is_err());
    assert_eq!(fs::read(&destination).unwrap(), content);

    let backup = temporary.path().join("deduplication.zip");
    let result = vault.create_backup(&backup).unwrap();
    assert_eq!(result.document_count, 2);
    assert_eq!(result.blob_count, 1);
    assert_eq!(vault.list_documents().unwrap().len(), 2);

    // A standard ZIP reader and SQLite connection suffice to recover the data.
    let mut archive = zip::ZipArchive::new(fs::File::open(&backup).unwrap()).unwrap();
    let blob_paths = archive
        .file_names()
        .map(|name| name.unwrap().into_owned())
        .filter(|name| name.starts_with("documents/sha256/") && !name.ends_with('/'))
        .collect::<Vec<_>>();
    assert_eq!(blob_paths.len(), 1);
    let mut original = Vec::new();
    archive
        .by_name(&blob_paths[0])
        .unwrap()
        .read_to_end(&mut original)
        .unwrap();
    assert_eq!(original, content);

    let extracted_database = temporary.path().join("independent.sqlite3");
    let mut database_bytes = Vec::new();
    archive
        .by_name("data/patrimoine.sqlite3")
        .unwrap()
        .read_to_end(&mut database_bytes)
        .unwrap();
    fs::write(&extracted_database, database_bytes).unwrap();
    let connection = Connection::open(extracted_database).unwrap();
    let logical_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
        .unwrap();
    let blob_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM document_blobs", [], |row| row.get(0))
        .unwrap();
    assert_eq!((logical_count, blob_count), (2, 1));
    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
}

#[test]
fn backup_restore_preserves_ids_bytes_settings_and_previous_generation() {
    let temporary = TempDir::new().unwrap();
    let source_root = temporary.path().join("source");
    let mut source = Vault::open(&source_root).unwrap();
    source
        .save_settings(settings("Famille restaurée", "AED"))
        .unwrap();
    let first_path = temporary.path().join("bail.txt");
    let second_path = temporary.path().join("releve.txt");
    fs::write(&first_path, "Bail français — données synthétiques").unwrap();
    fs::write(&second_path, "Relevé indépendant\n1 234,560 TND\n").unwrap();
    source.import_document(&first_path, "Bail").unwrap();
    source.import_document(&second_path, "Relevé").unwrap();
    let original_documents = sorted_documents(&source);
    let source_generation = source.status().unwrap().generation_id;
    let backup_path = temporary.path().join("famille.zip");
    let backup = source.create_backup(&backup_path).unwrap();
    assert_eq!((backup.document_count, backup.blob_count), (2, 2));

    let target_root = temporary.path().join("cible");
    let mut target = Vault::open(&target_root).unwrap();
    target
        .save_settings(settings("État avant restauration", "CHF"))
        .unwrap();
    let old_path = temporary.path().join("avant.txt");
    fs::write(&old_path, b"Document de la generation precedente").unwrap();
    let old_document = target.import_document(&old_path, "Avant").unwrap();
    let old_generation = target.status().unwrap().generation_id;

    let inspection = target.inspect_restore(&backup_path).unwrap();
    assert_eq!(inspection.source_generation_id, source_generation);
    assert_eq!(inspection.current_generation_id, old_generation);
    assert_eq!((inspection.document_count, inspection.blob_count), (2, 2));
    assert_eq!(inspection.settings.workspace_name, "Famille restaurée");
    // Inspecting has no side effect on the active generation.
    assert_eq!(target.status().unwrap().generation_id, old_generation);
    assert_eq!(target.list_documents().unwrap()[0].id, old_document.id);

    let safety_path = temporary.path().join("avant-restauration.zip");
    let restored = target
        .activate_restore(&inspection.token, true, Some(&safety_path))
        .unwrap();
    assert_ne!(restored.generation_id, old_generation);
    assert_eq!(restored.document_count, 2);
    assert_eq!(restored.settings.workspace_name, "Famille restaurée");
    assert_eq!(restored.settings.display_currency, "AED");
    assert_eq!(sorted_documents(&target), original_documents);
    assert!(target_root
        .join("generations")
        .join(&old_generation)
        .join("patrimoine.sqlite3")
        .is_file());
    assert!(safety_path.is_file());

    let safety_inspection = target.inspect_restore(&safety_path).unwrap();
    assert_eq!(safety_inspection.document_count, 1);
    assert_eq!(
        safety_inspection.settings.workspace_name,
        "État avant restauration"
    );

    for (id, original_name, _, _) in &original_documents {
        let export = temporary.path().join(format!("restaure-{original_name}"));
        target.export_document(id, &export).unwrap();
        assert_eq!(
            fs::read(export).unwrap(),
            fs::read(temporary.path().join(original_name)).unwrap()
        );
    }
    let restored_generation = restored.generation_id;
    drop(target);
    let reopened = Vault::open(&target_root).unwrap();
    assert_eq!(
        reopened.status().unwrap().generation_id,
        restored_generation
    );
    assert_eq!(sorted_documents(&reopened), original_documents);
}
