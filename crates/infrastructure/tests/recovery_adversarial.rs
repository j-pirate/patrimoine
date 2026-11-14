//! Recovery rejects untrusted input before replacing the active generation.
//! Fixtures are synthetic and confined to fresh temporary directories.

use patrimoine_application::VaultService;
use patrimoine_domain::Settings;
use patrimoine_infrastructure::Vault;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[derive(Clone)]
struct Entry {
    name: String,
    bytes: Vec<u8>,
}

fn fixture() -> (TempDir, Vault, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let mut vault = Vault::open(temp.path().join("vault")).unwrap();
    vault
        .save_settings(Settings {
            workspace_name: "Coffre synthétique".into(),
            display_currency: "EUR".into(),
        })
        .unwrap();
    let document = temp.path().join("preuve.txt");
    fs::write(&document, b"Original documentaire de test\n").unwrap();
    vault.import_document(&document, "Pièce de test").unwrap();
    let backup = temp.path().join("backup.zip");
    vault.create_backup(&backup).unwrap();
    (temp, vault, backup)
}

fn read_entries(path: &Path) -> Vec<Entry> {
    let mut archive = ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut source = archive.by_index(i).unwrap();
            let name = source.name().unwrap().into_owned();
            let mut bytes = Vec::new();
            source.read_to_end(&mut bytes).unwrap();
            Entry { name, bytes }
        })
        .collect()
}

fn write_entries(path: &Path, entries: &[Entry]) {
    let mut archive = ZipWriter::new(fs::File::create(path).unwrap());
    for entry in entries {
        archive
            .start_file(
                &entry.name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        archive.write_all(&entry.bytes).unwrap();
    }
    archive.finish().unwrap();
}

fn manifest_edit(entries: &mut [Entry], edit: impl FnOnce(&mut Value)) {
    let entry = entries
        .iter_mut()
        .find(|entry| entry.name == "manifest.json")
        .unwrap();
    let mut manifest: Value = serde_json::from_slice(&entry.bytes).unwrap();
    edit(&mut manifest);
    entry.bytes = serde_json::to_vec(&manifest).unwrap();
}

fn replace_database(entries: &mut [Entry], edit: impl FnOnce(&rusqlite::Connection)) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("hostile.sqlite3");
    let entry = entries
        .iter_mut()
        .find(|entry| entry.name == "data/patrimoine.sqlite3")
        .unwrap();
    fs::write(&path, &entry.bytes).unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    edit(&conn);
    conn.close().unwrap();
    entry.bytes = fs::read(&path).unwrap();
    let name = entry.name.clone();
    let hash = format!("{:x}", Sha256::digest(&entry.bytes));
    let size = entry.bytes.len() as u64;
    manifest_edit(entries, |manifest| {
        let file = manifest["files"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|file| file["path"] == name)
            .unwrap();
        file["sha256"] = Value::String(hash);
        file["size_bytes"] = Value::from(size);
    });
}

fn reject_without_mutation(vault: &mut Vault, archive: &Path) {
    let before = vault.status().unwrap();
    assert!(vault.inspect_restore(archive).is_err());
    let after = vault.status().unwrap();
    assert_eq!(before.generation_id, after.generation_id);
    assert_eq!(before.mutation_sequence, after.mutation_sequence);
    assert_eq!(
        before.settings.workspace_name,
        after.settings.workspace_name
    );
    assert_eq!(before.document_count, after.document_count);
}

#[test]
fn future_manifest_versions_are_refused_without_replacing_the_vault() {
    let (temp, mut vault, backup) = fixture();
    for field in ["format_version", "schema_version"] {
        let mut entries = read_entries(&backup);
        manifest_edit(&mut entries, |manifest| {
            manifest[field] = Value::from(999_999_u64);
        });
        let hostile = temp.path().join(format!("future-{field}.zip"));
        write_entries(&hostile, &entries);
        reject_without_mutation(&mut vault, &hostile);
    }
}

#[test]
fn changed_sqlite_checksum_is_refused() {
    let (temp, mut vault, backup) = fixture();
    let mut entries = read_entries(&backup);
    entries
        .iter_mut()
        .find(|entry| entry.name == "data/patrimoine.sqlite3")
        .unwrap()
        .bytes[0] ^= 0x01;
    let hostile = temp.path().join("bad-hash.zip");
    write_entries(&hostile, &entries);
    reject_without_mutation(&mut vault, &hostile);
}

#[test]
fn recomputed_hash_does_not_authorize_extra_sqlite_schema() {
    let (temp, mut vault, backup) = fixture();
    let mut entries = read_entries(&backup);
    replace_database(&mut entries, |conn| {
        conn.execute_batch("CREATE VIEW archive_supplied_view AS SELECT 1 AS unexpected;")
            .unwrap();
    });
    let hostile = temp.path().join("unexpected-view.zip");
    write_entries(&hostile, &entries);
    reject_without_mutation(&mut vault, &hostile);
}

#[test]
fn recomputed_hash_does_not_authorize_a_future_sqlite_user_version() {
    let (temp, mut vault, backup) = fixture();
    let mut entries = read_entries(&backup);
    replace_database(&mut entries, |conn| {
        conn.pragma_update(None, "user_version", 999_999).unwrap();
    });
    let hostile = temp.path().join("future-sqlite.zip");
    write_entries(&hostile, &entries);
    reject_without_mutation(&mut vault, &hostile);
}

#[test]
fn recomputed_hash_does_not_authorize_sqlite_triggers() {
    let (temp, mut vault, backup) = fixture();
    let mut entries = read_entries(&backup);
    replace_database(&mut entries, |conn| {
        conn.execute_batch(
            "CREATE TRIGGER archive_supplied_trigger AFTER INSERT ON countries BEGIN SELECT 1; END;",
        )
        .unwrap();
    });
    let hostile = temp.path().join("unexpected-trigger.zip");
    write_entries(&hostile, &entries);
    reject_without_mutation(&mut vault, &hostile);
}

#[test]
fn recomputed_hash_does_not_authorize_orphan_document_references() {
    let (temp, mut vault, backup) = fixture();
    let mut entries = read_entries(&backup);
    replace_database(&mut entries, |conn| {
        conn.execute_batch("PRAGMA foreign_keys=OFF;").unwrap();
        conn.execute(
            "UPDATE document_versions SET blob_sha256 = ?1",
            ["0".repeat(64)],
        )
        .unwrap();
    });
    let hostile = temp.path().join("orphan-fk.zip");
    write_entries(&hostile, &entries);
    reject_without_mutation(&mut vault, &hostile);
}

#[test]
fn recomputed_hash_does_not_authorize_invalid_settings() {
    let (temp, mut vault, backup) = fixture();
    let mut entries = read_entries(&backup);
    replace_database(&mut entries, |conn| {
        conn.execute(
            "UPDATE app_settings SET value_text = 'ZZZ' WHERE key = 'display_currency'",
            [],
        )
        .unwrap();
    });
    let hostile = temp.path().join("invalid-settings.zip");
    write_entries(&hostile, &entries);
    reject_without_mutation(&mut vault, &hostile);
}

#[test]
fn recomputed_hash_does_not_authorize_a_missing_reference_currency() {
    let (temp, mut vault, backup) = fixture();
    let mut entries = read_entries(&backup);
    replace_database(&mut entries, |conn| {
        conn.execute("UPDATE currencies SET code = 'ZZZ' WHERE code = 'EUR'", [])
            .unwrap();
    });
    let hostile = temp.path().join("invalid-reference-currency.zip");
    write_entries(&hostile, &entries);
    reject_without_mutation(&mut vault, &hostile);
}

#[test]
fn absent_and_unexpected_archive_files_are_refused() {
    let (temp, mut vault, backup) = fixture();
    let entries = read_entries(&backup);
    let mut absent = entries.clone();
    absent.retain(|entry| entry.name != "data/patrimoine.sqlite3");
    let archive = temp.path().join("absent.zip");
    write_entries(&archive, &absent);
    reject_without_mutation(&mut vault, &archive);
    let mut extra = entries;
    extra.push(Entry {
        name: "unexpected.txt".into(),
        bytes: b"must not be imported".to_vec(),
    });
    let archive = temp.path().join("extra.zip");
    write_entries(&archive, &extra);
    reject_without_mutation(&mut vault, &archive);
}

#[test]
fn unsafe_zip_paths_do_not_escape_preparation() {
    let (temp, mut vault, backup) = fixture();
    let absolute_escape = temp.path().join("escaped-absolute.txt");
    for (index, unsafe_name) in [
        "../escaped.txt".to_owned(),
        "documents/../../escaped.txt".to_owned(),
        absolute_escape.to_string_lossy().into_owned(),
        "C:\\escaped.txt".to_owned(),
        "documents\\..\\escaped.txt".to_owned(),
    ]
    .iter()
    .enumerate()
    {
        let mut entries = read_entries(&backup);
        entries.push(Entry {
            name: unsafe_name.clone(),
            bytes: b"must never be extracted".to_vec(),
        });
        let hostile = temp.path().join(format!("unsafe-{index}.zip"));
        write_entries(&hostile, &entries);
        reject_without_mutation(&mut vault, &hostile);
        assert!(!temp.path().join("escaped.txt").exists());
        assert!(!absolute_escape.exists());
    }
}

#[test]
fn duplicate_zip_names_are_refused() {
    let (temp, mut vault, backup) = fixture();
    let mut entries = read_entries(&backup);
    let original_name = entries[0].name.clone();
    let mut alternate_name = original_name.clone().into_bytes();
    *alternate_name.last_mut().unwrap() = b'x';
    let alternate_name = String::from_utf8(alternate_name).unwrap();
    entries.push(Entry {
        name: alternate_name.clone(),
        bytes: entries[0].bytes.clone(),
    });
    let hostile = temp.path().join("duplicate.zip");
    write_entries(&hostile, &entries);
    // zip's writer itself disallows duplicates. Patch equal-length names in the
    // local and central headers to model an externally supplied hostile ZIP.
    let mut bytes = fs::read(&hostile).unwrap();
    let mut replacements = 0;
    for offset in 0..=bytes.len() - alternate_name.len() {
        if &bytes[offset..offset + alternate_name.len()] == alternate_name.as_bytes() {
            bytes[offset..offset + alternate_name.len()].copy_from_slice(original_name.as_bytes());
            replacements += 1;
        }
    }
    assert_eq!(replacements, 2);
    fs::write(&hostile, bytes).unwrap();
    reject_without_mutation(&mut vault, &hostile);
}

#[test]
fn zip_symlinks_are_refused() {
    let (temp, mut vault, backup) = fixture();
    let hostile = temp.path().join("symlink.zip");
    let mut archive = ZipWriter::new(fs::File::create(&hostile).unwrap());
    for entry in read_entries(&backup) {
        archive
            .start_file(&entry.name, SimpleFileOptions::default())
            .unwrap();
        archive.write_all(&entry.bytes).unwrap();
    }
    archive
        .add_symlink(
            "documents/link",
            "../../escaped.txt",
            SimpleFileOptions::default(),
        )
        .unwrap();
    archive.finish().unwrap();
    reject_without_mutation(&mut vault, &hostile);
    assert!(!temp.path().join("escaped.txt").exists());
}

#[test]
fn confirmation_is_required_and_does_not_mutate_on_refusal() {
    let (_temp, mut vault, backup) = fixture();
    let inspection = vault.inspect_restore(&backup).unwrap();
    let before = vault.status().unwrap();
    assert!(vault
        .activate_restore(&inspection.token, false, None)
        .is_err());
    let after = vault.status().unwrap();
    assert_eq!(before.generation_id, after.generation_id);
    assert_eq!(before.mutation_sequence, after.mutation_sequence);
}

#[test]
fn settings_mutation_makes_a_restore_token_stale() {
    let (_temp, mut vault, backup) = fixture();
    let inspection = vault.inspect_restore(&backup).unwrap();
    vault
        .save_settings(Settings {
            workspace_name: "Saisie postérieure à l'analyse".into(),
            display_currency: "CHF".into(),
        })
        .unwrap();
    let before = vault.status().unwrap();
    let error = vault
        .activate_restore(&inspection.token, true, None)
        .unwrap_err();
    assert_eq!(error.code, "conflict");
    let after = vault.status().unwrap();
    assert_eq!(before.generation_id, after.generation_id);
    assert_eq!(before.mutation_sequence, after.mutation_sequence);
    assert_eq!(after.settings.display_currency, "CHF");
}

#[test]
fn document_mutation_makes_a_restore_token_stale() {
    let (temp, mut vault, backup) = fixture();
    let inspection = vault.inspect_restore(&backup).unwrap();
    let added = temp.path().join("nouvelle-piece.txt");
    fs::write(&added, b"New document after inspection").unwrap();
    vault.import_document(&added, "Ajout ultérieur").unwrap();
    let before = vault.status().unwrap();
    let error = vault
        .activate_restore(&inspection.token, true, None)
        .unwrap_err();
    assert_eq!(error.code, "conflict");
    let after = vault.status().unwrap();
    assert_eq!(before.generation_id, after.generation_id);
    assert_eq!(before.document_count, after.document_count);
}

#[test]
fn only_one_vault_instance_can_hold_the_write_lock() {
    let (temp, vault, _backup) = fixture();
    let root = temp.path().join("vault");
    assert!(Vault::open(&root).is_err());
    drop(vault);
    let reopened = Vault::open(&root).unwrap();
    assert_eq!(reopened.status().unwrap().document_count, 1);
}

#[test]
fn current_pointer_is_an_identifier_not_a_path() {
    let (temp, vault, _backup) = fixture();
    drop(vault);
    let root = temp.path().join("vault");
    let original = fs::read(root.join("CURRENT")).unwrap();
    for hostile in ["../outside", "/tmp/outside", "generations/../../outside"] {
        fs::write(root.join("CURRENT"), hostile).unwrap();
        assert!(Vault::open(&root).is_err());
        assert_eq!(fs::read(root.join("CURRENT")).unwrap(), hostile.as_bytes());
    }
    fs::write(root.join("CURRENT"), original).unwrap();
    assert!(Vault::open(&root).is_ok());
}

#[test]
fn current_pointer_does_not_ignore_bytes_after_a_read_limit() {
    let (temp, vault, _backup) = fixture();
    let generation = vault.status().unwrap().generation_id;
    drop(vault);
    let root = temp.path().join("vault");
    let noncanonical = format!("{generation}{}unread-payload", "\n".repeat(80));
    fs::write(root.join("CURRENT"), noncanonical).unwrap();
    assert!(Vault::open(&root).is_err());
}

#[test]
fn missing_active_generation_does_not_create_an_empty_replacement() {
    let (temp, vault, _backup) = fixture();
    let status = vault.status().unwrap();
    drop(vault);
    let root = temp.path().join("vault");
    let active = root.join("generations").join(&status.generation_id);
    let preserved = root.join("preserved-generation");
    fs::rename(&active, &preserved).unwrap();
    let pointer = fs::read(root.join("CURRENT")).unwrap();
    assert!(Vault::open(&root).is_err());
    assert_eq!(fs::read(root.join("CURRENT")).unwrap(), pointer);
    assert!(!active.exists());
    assert!(preserved.join("patrimoine.sqlite3").exists());
}

#[test]
fn a_highly_compressible_original_still_produces_a_restorable_backup() {
    let (temp, mut vault, _backup) = fixture();
    let document = temp.path().join("compressible.bin");
    fs::write(&document, vec![0_u8; 2 * 1024 * 1024]).unwrap();
    vault
        .import_document(&document, "Pièce fortement compressible")
        .unwrap();
    let backup = temp.path().join("compressible-backup.zip");
    vault.create_backup(&backup).unwrap();
    let inspection = vault.inspect_restore(&backup).unwrap();
    assert_eq!(inspection.document_count, 2);
}

#[cfg(unix)]
#[test]
fn symbolic_links_cannot_be_imported_as_document_originals() {
    let (temp, mut vault, _backup) = fixture();
    let link = temp.path().join("document-link");
    std::os::unix::fs::symlink(temp.path().join("preuve.txt"), &link).unwrap();
    let before = vault.status().unwrap();
    assert!(vault.import_document(&link, "Lien symbolique").is_err());
    assert_eq!(
        vault.status().unwrap().document_count,
        before.document_count
    );
}

fn simulate_unfinished_activation(root: &Path, old: &str, new: &str) {
    fs::write(
        root.join("recovery/activation.json"),
        serde_json::to_vec(&serde_json::json!({
            "old_generation": old,
            "new_generation": new,
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn interrupted_activation_before_current_commit_keeps_the_old_generation() {
    let (temp, mut vault, backup) = fixture();
    let old = vault.status().unwrap().generation_id;
    let inspection = vault.inspect_restore(&backup).unwrap();
    let new = vault
        .activate_restore(&inspection.token, true, None)
        .unwrap()
        .generation_id;
    drop(vault);
    let root = temp.path().join("vault");
    simulate_unfinished_activation(&root, &old, &new);
    fs::write(root.join("CURRENT"), format!("{old}\n")).unwrap();
    let reopened = Vault::open(&root).unwrap();
    assert_eq!(reopened.status().unwrap().generation_id, old);
    assert_eq!(reopened.status().unwrap().document_count, 1);
}

#[test]
fn interrupted_activation_after_current_commit_keeps_a_complete_new_generation() {
    let (temp, mut vault, backup) = fixture();
    let old = vault.status().unwrap().generation_id;
    let inspection = vault.inspect_restore(&backup).unwrap();
    let new = vault
        .activate_restore(&inspection.token, true, None)
        .unwrap()
        .generation_id;
    drop(vault);
    let root = temp.path().join("vault");
    simulate_unfinished_activation(&root, &old, &new);
    let reopened = Vault::open(&root).unwrap();
    assert_eq!(reopened.status().unwrap().generation_id, new);
    assert_eq!(reopened.status().unwrap().document_count, 1);
}

#[test]
fn interrupted_activation_rolls_back_if_new_sqlite_is_corrupt() {
    let (temp, mut vault, backup) = fixture();
    let old = vault.status().unwrap().generation_id;
    let inspection = vault.inspect_restore(&backup).unwrap();
    let new = vault
        .activate_restore(&inspection.token, true, None)
        .unwrap()
        .generation_id;
    drop(vault);
    let root = temp.path().join("vault");
    simulate_unfinished_activation(&root, &old, &new);
    fs::write(
        root.join("generations")
            .join(&new)
            .join("patrimoine.sqlite3"),
        b"incomplete candidate after interruption",
    )
    .unwrap();
    let reopened = Vault::open(&root).unwrap();
    assert_eq!(reopened.status().unwrap().generation_id, old);
    assert_eq!(reopened.status().unwrap().document_count, 1);
}

#[test]
fn interrupted_activation_rolls_back_if_a_new_document_is_missing() {
    let (temp, mut vault, backup) = fixture();
    let old = vault.status().unwrap().generation_id;
    let document = vault.list_documents().unwrap().remove(0);
    let inspection = vault.inspect_restore(&backup).unwrap();
    let new = vault
        .activate_restore(&inspection.token, true, None)
        .unwrap()
        .generation_id;
    drop(vault);
    let root = temp.path().join("vault");
    simulate_unfinished_activation(&root, &old, &new);
    let blob = root.join("generations").join(&new).join(format!(
        "documents/sha256/{}/{}",
        &document.sha256[..2],
        document.sha256
    ));
    fs::remove_file(blob).unwrap();
    let reopened = Vault::open(&root).unwrap();
    assert_eq!(reopened.status().unwrap().generation_id, old);
    reopened
        .export_document(&document.id, &temp.path().join("recovered-original.txt"))
        .unwrap();
}

#[test]
fn tampering_with_a_candidate_after_analysis_is_refused_before_activation() {
    let (temp, mut vault, backup) = fixture();
    let inspection = vault.inspect_restore(&backup).unwrap();
    let before = vault.status().unwrap();
    let candidate_db = temp
        .path()
        .join("vault/staging")
        .join(&inspection.token)
        .join("data/patrimoine.sqlite3");
    fs::write(candidate_db, b"changed after analysis").unwrap();
    assert!(vault
        .activate_restore(&inspection.token, true, None)
        .is_err());
    assert_eq!(vault.status().unwrap().generation_id, before.generation_id);
    assert_eq!(
        vault.status().unwrap().mutation_sequence,
        before.mutation_sequence
    );
}

#[test]
fn an_extra_sqlite_wal_in_the_candidate_is_refused_before_activation() {
    let (temp, mut vault, backup) = fixture();
    let inspection = vault.inspect_restore(&backup).unwrap();
    let before = vault.status().unwrap();
    let candidate_wal = temp
        .path()
        .join("vault/staging")
        .join(&inspection.token)
        .join("data/patrimoine.sqlite3-wal");
    fs::write(candidate_wal, b"unlisted sidecar").unwrap();
    assert!(vault
        .activate_restore(&inspection.token, true, None)
        .is_err());
    assert_eq!(vault.status().unwrap().generation_id, before.generation_id);
}

#[test]
fn a_second_inspection_revokes_the_first_token() {
    let (_temp, mut vault, backup) = fixture();
    let first = vault.inspect_restore(&backup).unwrap();
    let second = vault.inspect_restore(&backup).unwrap();
    assert_ne!(first.token, second.token);
    let error = vault
        .activate_restore(&first.token, true, None)
        .unwrap_err();
    assert_eq!(error.code, "conflict");
    assert!(vault.activate_restore(&second.token, true, None).is_ok());
}

#[test]
fn a_validated_candidate_is_independent_of_the_original_archive_path() {
    let (_temp, mut vault, backup) = fixture();
    let inspection = vault.inspect_restore(&backup).unwrap();
    fs::remove_file(backup).unwrap();
    let status = vault
        .activate_restore(&inspection.token, true, None)
        .unwrap();
    assert_eq!(status.document_count, 1);
    assert_eq!(status.settings.workspace_name, "Coffre synthétique");
}
