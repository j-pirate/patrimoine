//! Originals, associations and revisions are independent recovery sources.
use patrimoine_application::{DocumentService, RegistryService, VaultService};
use patrimoine_domain::*;
use patrimoine_infrastructure::Vault;
use rusqlite::Connection;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use tempfile::TempDir;
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

fn person(v: &mut Vault, name: &str) -> Entity {
    v.save_entity(SaveEntityInput {
        id: None,
        expected_revision_id: None,
        reason: "Cible synthétique".into(),
        kind: "person".into(),
        display_name: name.into(),
        first_name: None,
        last_name: None,
        birth_date: None,
        residence_country: Some("FR".into()),
        preferred_currency: "EUR".into(),
        activity: String::new(),
        notes: String::new(),
        archived: false,
    })
    .unwrap()
}
fn fixture() -> (TempDir, Vault, Entity, Asset, DocumentRecord) {
    let temp = TempDir::new().unwrap();
    let mut v = Vault::open(temp.path().join("vault")).unwrap();
    let entity = person(&mut v, "Personne synthétique");
    let asset = v
        .save_asset(SaveAssetInput {
            id: None,
            expected_revision_id: None,
            reason: "Cible synthétique".into(),
            name: "Bien du test".into(),
            country: "FR".into(),
            category: "other".into(),
            notes: String::new(),
            archived: false,
        })
        .unwrap();
    let source = temp.path().join("original-écrit.txt");
    fs::write(
        &source,
        "Première version.\nCaractères accentués, € et \"guillemets\".\n",
    )
    .unwrap();
    let imported = v.import_document(&source, "Description initiale").unwrap();
    let record = v
        .list_document_library_at(None)
        .unwrap()
        .documents
        .into_iter()
        .find(|d| d.id == imported.id)
        .unwrap();
    (temp, v, entity, asset, record)
}
fn link(document: &DocumentRecord, target: &str) -> SaveDocumentLinkInput {
    SaveDocumentLinkInput {
        id: None,
        expected_revision_id: None,
        reason: "Association documentaire synthétique".into(),
        document_id: document.id.clone(),
        target_object_id: target.into(),
        role: Some("Justificatif".into()),
        valid_from: Some("2025-01-01".into()),
        valid_to: None,
    }
}
fn metadata(document: &DocumentRecord, status: &str) -> SaveDocumentMetadataInput {
    SaveDocumentMetadataInput {
        id: document.id.clone(),
        expected_revision_id: document.revision_id.clone(),
        reason: "Correction documentaire motivée".into(),
        description: "Description corrigée\n=HYPERLINK(\"texte littéral\")".into(),
        status: status.into(),
        confirmed: status == "deleted",
    }
}
fn db_path(root: &Path) -> PathBuf {
    root.join("generations")
        .join(fs::read_to_string(root.join("CURRENT")).unwrap().trim())
        .join("patrimoine.sqlite3")
}
fn state(v: &Vault) -> Value {
    serde_json::to_value(v.list_document_library_at(None).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn altered_archive(source: &Path, target: &Path, change: impl FnOnce(&Connection)) {
    let mut zip = ZipArchive::new(fs::File::open(source).unwrap()).unwrap();
    let mut files = BTreeMap::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        files.insert(file.name().unwrap().into_owned(), bytes);
    }
    let temp = TempDir::new().unwrap();
    let database = temp.path().join("altered.sqlite3");
    fs::write(&database, &files["data/patrimoine.sqlite3"]).unwrap();
    let c = Connection::open(&database).unwrap();
    c.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    change(&c);
    drop(c);
    let bytes = fs::read(database).unwrap();
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    for file in manifest["files"].as_array_mut().unwrap() {
        if file["path"] == "data/patrimoine.sqlite3" {
            file["size_bytes"] = bytes.len().into();
            file["sha256"] = hash(&bytes).into();
        }
    }
    files.insert("data/patrimoine.sqlite3".into(), bytes);
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    let mut out = ZipWriter::new(fs::File::create(target).unwrap());
    for (name, bytes) in files {
        out.start_file(name, SimpleFileOptions::default().unix_permissions(0o600))
            .unwrap();
        out.write_all(&bytes).unwrap();
    }
    out.finish().unwrap();
}

#[test]
fn shared_bytes_distinct_logical_documents_and_metadata_versions_keep_initial_date() {
    let (temp, mut v, _, _, document) = fixture();
    let other = temp.path().join("autre-nom.txt");
    fs::write(
        &other,
        fs::read(temp.path().join("original-écrit.txt")).unwrap(),
    )
    .unwrap();
    let second = v.import_document(&other, "Autre document logique").unwrap();
    assert_ne!(document.id, second.id);
    assert_eq!(document.sha256, second.sha256);
    let k = v.list_document_library_at(None).unwrap().knowledge_sequence;
    let changed = v
        .save_document_metadata(metadata(&document, "archived"))
        .unwrap();
    assert_eq!(changed.added_at, document.added_at);
    assert_ne!(changed.revision_id, document.revision_id);
    assert_eq!(changed.sha256, document.sha256);
    let old = v.list_document_library_at(Some(k)).unwrap();
    assert_eq!(
        old.documents.iter().find(|d| d.id == document.id).unwrap(),
        &document
    );
    assert_eq!(v.list_documents().unwrap().len(), 1);
    let current = v.list_document_library_at(None).unwrap();
    assert_eq!(current.documents.len(), 2);
    assert_eq!(current.versions.len(), 3);
    let c = Connection::open(db_path(&temp.path().join("vault"))).unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM document_blobs", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn import_and_identical_replacement_share_mime_and_summary_keeps_added_date() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("vault")).unwrap();
    for (extension, mime) in [
        ("CSV", "text/plain"),
        (
            "docx",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ),
        (
            "xlsx",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ),
        ("odt", "application/vnd.oasis.opendocument.text"),
        ("ods", "application/vnd.oasis.opendocument.spreadsheet"),
        ("rtf", "application/rtf"),
    ] {
        let source = temp.path().join(format!("same.{extension}"));
        fs::write(&source, b"same original bytes").unwrap();
        let imported = vault
            .import_document(&source, "Initial description")
            .unwrap();
        assert_eq!(imported.mime_type, mime);
        let library = vault.list_document_library_at(None).unwrap();
        let document = library
            .documents
            .iter()
            .find(|d| d.id == imported.id)
            .unwrap();
        let replacement = vault
            .replace_document(
                ReplaceDocumentInput {
                    id: document.id.clone(),
                    expected_revision_id: document.revision_id.clone(),
                    reason: "Même original, métadonnées cohérentes".into(),
                    description: "Description corrigée".into(),
                },
                &source,
            )
            .unwrap();
        assert_eq!(replacement.sha256, imported.sha256);
        assert_eq!(replacement.mime_type, imported.mime_type);
        assert_eq!(replacement.added_at, imported.added_at);
        let corrected = vault
            .save_document_metadata(metadata(&replacement, "active"))
            .unwrap();
        let summary = vault
            .list_documents()
            .unwrap()
            .into_iter()
            .find(|d| d.id == imported.id)
            .unwrap();
        assert_eq!(summary.added_at, imported.added_at);
        assert_eq!(summary.added_at, corrected.added_at);
    }
}

#[test]
fn one_document_can_link_two_business_objects_and_removing_one_keeps_other_and_bytes() {
    let (temp, mut v, entity, asset, document) = fixture();
    let first = v.save_document_link(link(&document, &entity.id)).unwrap();
    let second = v.save_document_link(link(&document, &asset.id)).unwrap();
    let k = v.list_document_library_at(None).unwrap().knowledge_sequence;
    v.remove_document_link(VoidRecordInput {
        id: first.id,
        expected_revision_id: first.revision_id,
        reason: "Retirer uniquement ce justificatif de personne".into(),
    })
    .unwrap();
    let current = v.list_document_library_at(None).unwrap();
    assert_eq!(current.links, vec![second]);
    assert_eq!(current.documents, vec![document.clone()]);
    assert_eq!(v.list_document_library_at(Some(k)).unwrap().links.len(), 2);
    let exported = temp.path().join("copie.txt");
    v.export_document_revision(&document.id, &document.revision_id, &exported)
        .unwrap();
    assert_eq!(
        fs::read(exported).unwrap(),
        fs::read(temp.path().join("original-écrit.txt")).unwrap()
    );
}

#[test]
fn deletion_requires_confirmation_and_preserves_every_version_original_and_association() {
    let (temp, mut v, entity, _, document) = fixture();
    let association = v.save_document_link(link(&document, &entity.id)).unwrap();
    let before = state(&v);
    let mut input = metadata(&document, "deleted");
    input.confirmed = false;
    assert_eq!(
        v.save_document_metadata(input).unwrap_err().code,
        "confirmation_required"
    );
    assert_eq!(state(&v), before);
    let deleted = v
        .save_document_metadata(metadata(&document, "deleted"))
        .unwrap();
    assert!(v.list_documents().unwrap().is_empty());
    let library = v.list_document_library_at(None).unwrap();
    assert_eq!(library.documents[0].status, "deleted");
    assert_eq!(library.links, vec![association]);
    assert_eq!(library.versions.len(), 2);
    let copy = temp.path().join("historique.txt");
    v.export_document_revision(&document.id, &document.revision_id, &copy)
        .unwrap();
    assert_eq!(hash(&fs::read(copy).unwrap()), document.sha256);
    let active = v
        .save_document_metadata(metadata(&deleted, "active"))
        .unwrap();
    assert_eq!(active.sha256, document.sha256);
    assert_eq!(v.list_documents().unwrap().len(), 1);
}

#[test]
fn replacement_retains_old_hash_and_links_after_source_files_disappear() {
    let (temp, mut v, entity, _, document) = fixture();
    let association = v.save_document_link(link(&document, &entity.id)).unwrap();
    let old_k = v.list_document_library_at(None).unwrap().knowledge_sequence;
    let source = temp.path().join("remplacement.pdf");
    fs::write(&source, b"%PDF-1.4\nsynthetic replacement bytes\n").unwrap();
    let replaced = v
        .replace_document(
            ReplaceDocumentInput {
                id: document.id.clone(),
                expected_revision_id: document.revision_id.clone(),
                reason: "Original remplacé, ancienne pièce conservée".into(),
                description: "Nouveau justificatif".into(),
            },
            &source,
        )
        .unwrap();
    assert_ne!(replaced.sha256, document.sha256);
    assert_eq!(replaced.id, document.id);
    assert_eq!(replaced.added_at, document.added_at);
    assert_eq!(replaced.mime_type, "application/pdf");
    fs::remove_file(source).unwrap();
    fs::remove_file(temp.path().join("original-écrit.txt")).unwrap();
    let old = temp.path().join("ancienne.txt");
    let new = temp.path().join("nouvelle.pdf");
    v.export_document_revision(&document.id, &document.revision_id, &old)
        .unwrap();
    v.export_document_revision(&document.id, &replaced.revision_id, &new)
        .unwrap();
    assert_eq!(hash(&fs::read(old).unwrap()), document.sha256);
    assert_eq!(hash(&fs::read(new).unwrap()), replaced.sha256);
    assert_eq!(
        v.list_document_library_at(Some(old_k)).unwrap().documents[0],
        document
    );
    assert_eq!(
        v.list_document_library_at(None).unwrap().links,
        vec![association]
    );
}

#[test]
fn stale_or_incompatible_document_mutations_leave_sequence_and_audit_unchanged() {
    let (temp, mut v, entity, asset, document) = fixture();
    let linked = v.save_document_link(link(&document, &entity.id)).unwrap();
    let current = v
        .save_document_metadata(metadata(&document, "active"))
        .unwrap();
    let before = state(&v);
    assert_eq!(
        v.save_document_metadata(metadata(&document, "archived"))
            .unwrap_err()
            .code,
        "conflict"
    );
    assert_eq!(
        v.replace_document(
            ReplaceDocumentInput {
                id: document.id.clone(),
                expected_revision_id: document.revision_id.clone(),
                reason: "Version obsolète".into(),
                description: "".into()
            },
            &temp.path().join("original-écrit.txt")
        )
        .unwrap_err()
        .code,
        "conflict"
    );
    let mut moved = link(&current, &asset.id);
    moved.id = Some(linked.id.clone());
    moved.expected_revision_id = Some(linked.revision_id.clone());
    assert!(v.save_document_link(moved).is_err());
    let mut duplicate = link(&current, &entity.id);
    duplicate.valid_from = Some("2025-02-01".into());
    duplicate.valid_to = Some("2025-01-01".into());
    assert!(v.save_document_link(duplicate).is_err());
    assert_eq!(state(&v), before);
}

#[test]
fn document_and_workspace_are_not_documentary_business_targets_and_void_targets_stay_historical() {
    let (temp, mut v, entity, _, document) = fixture();
    let before = state(&v);
    assert!(v.save_document_link(link(&document, &document.id)).is_err());
    let c = Connection::open(db_path(&temp.path().join("vault"))).unwrap();
    let workspace: String = c
        .query_row(
            "SELECT value_text FROM app_settings WHERE key='workspace_id'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(v.save_document_link(link(&document, &workspace)).is_err());
    let invalid = uuid::Uuid::new_v4().to_string();
    assert!(v.save_document_link(link(&document, &invalid)).is_err());
    assert_eq!(state(&v), before);
    let association = v.save_document_link(link(&document, &entity.id)).unwrap();
    let before_retirement = v.list_document_library_at(None).unwrap();
    v.void_registry_record(VoidRecordInput {
        id: entity.id.clone(),
        expected_revision_id: entity.revision_id,
        reason: "Cible retirée, association historique conservée".into(),
    })
    .unwrap();
    let library = v.list_document_library_at(None).unwrap();
    assert_eq!(
        library
            .targets
            .iter()
            .find(|t| t.id == entity.id)
            .unwrap()
            .label,
        entity.display_name
    );
    let historical = v
        .list_document_library_at(Some(before_retirement.knowledge_sequence))
        .unwrap();
    assert_eq!(historical.targets, before_retirement.targets);
    assert!(
        library
            .targets
            .iter()
            .find(|t| t.id == entity.id)
            .unwrap()
            .retired
    );
    assert_eq!(library.links, vec![association]);
    assert!(v.save_document_link(link(&document, &entity.id)).is_err());
}

#[test]
fn retired_target_uses_its_last_known_label_without_exposing_future_renames_at_old_k() {
    let (_temp, mut vault, entity, _, document) = fixture();
    vault
        .save_document_link(link(&document, &entity.id))
        .unwrap();
    let before_rename = vault.list_document_library_at(None).unwrap();
    let renamed = vault
        .save_entity(SaveEntityInput {
            id: Some(entity.id.clone()),
            expected_revision_id: Some(entity.revision_id.clone()),
            reason: "Nom corrigé avant retrait".into(),
            kind: entity.kind.clone(),
            display_name: "Nom historique reconnaissable".into(),
            first_name: entity.first_name.clone(),
            last_name: entity.last_name.clone(),
            birth_date: entity.birth_date.clone(),
            residence_country: entity.residence_country.clone(),
            preferred_currency: entity.preferred_currency.clone(),
            activity: entity.activity.clone(),
            notes: entity.notes.clone(),
            archived: entity.archived,
        })
        .unwrap();
    vault
        .void_registry_record(VoidRecordInput {
            id: renamed.id.clone(),
            expected_revision_id: renamed.revision_id.clone(),
            reason: "Retrait de la cible renommée".into(),
        })
        .unwrap();
    let current = vault.list_document_library_at(None).unwrap();
    let target = current.targets.iter().find(|t| t.id == entity.id).unwrap();
    assert!(target.retired);
    assert_eq!(target.label, renamed.display_name);
    let historical = vault
        .list_document_library_at(Some(before_rename.knowledge_sequence))
        .unwrap();
    let old_target = historical
        .targets
        .iter()
        .find(|t| t.id == entity.id)
        .unwrap();
    assert!(!old_target.retired);
    assert_eq!(old_target.label, entity.display_name);
}

#[test]
fn an_association_requires_a_real_start_date_and_invalid_intervals_do_not_mutate() {
    let (_temp, mut v, entity, _, document) = fixture();
    let before = state(&v);
    for (from, to) in [
        (None, None),
        (Some("2025-02-30"), None),
        (Some("2025-01-01"), Some("2025-01-01")),
    ] {
        let mut input = link(&document, &entity.id);
        input.valid_from = from.map(str::to_owned);
        input.valid_to = to.map(str::to_owned);
        assert_eq!(v.save_document_link(input).unwrap_err().code, "validation");
        assert_eq!(state(&v), before);
    }
}

#[test]
fn preparing_an_open_copy_never_exposes_original_for_editing_and_checks_its_hash() {
    let (temp, v, _, _, document) = fixture();
    let prepared = v
        .prepare_document_open(&document.id, &document.revision_id)
        .unwrap();
    assert_ne!(
        Path::new(&prepared.path),
        temp.path().join("original-écrit.txt")
    );
    assert_eq!(hash(&fs::read(&prepared.path).unwrap()), document.sha256);
    fs::write(&prepared.path, b"edited viewer copy").unwrap();
    let exported = temp.path().join("verified-original.txt");
    v.export_document_revision(&document.id, &document.revision_id, &exported)
        .unwrap();
    assert_eq!(hash(&fs::read(exported).unwrap()), document.sha256);
    let blob = temp
        .path()
        .join("vault/generations")
        .join(
            fs::read_to_string(temp.path().join("vault/CURRENT"))
                .unwrap()
                .trim(),
        )
        .join(format!(
            "documents/sha256/{}/{}",
            &document.sha256[..2],
            document.sha256
        ));
    fs::write(blob, b"corrupted original").unwrap();
    assert_eq!(
        v.prepare_document_open(&document.id, &document.revision_id)
            .unwrap_err()
            .code,
        "integrity"
    );
}

#[test]
fn replacement_rejects_symbolic_links_and_opening_unknown_formats_keeps_export_available() {
    let (temp, mut v, _, _, document) = fixture();
    let source = temp.path().join("launch.sh");
    fs::write(
        &source,
        b"#!/bin/sh\nprintf 'never execute this document automatically'\n",
    )
    .unwrap();
    let symlink = temp.path().join("shortcut.txt");
    std::os::unix::fs::symlink(&source, &symlink).unwrap();
    let input = || ReplaceDocumentInput {
        id: document.id.clone(),
        expected_revision_id: document.revision_id.clone(),
        reason: "Remplacement explicitement choisi".into(),
        description: "Fichier conservé sans ouverture automatique".into(),
    };
    let before = state(&v);
    assert!(v.replace_document(input(), &symlink).is_err());
    assert_eq!(state(&v), before);
    let replaced = v.replace_document(input(), &source).unwrap();
    assert_eq!(
        v.prepare_document_open(&replaced.id, &replaced.revision_id)
            .unwrap_err()
            .code,
        "unsupported_document_open"
    );
    let exported = temp.path().join("copie-manuel.sh");
    v.export_document_revision(&replaced.id, &replaced.revision_id, &exported)
        .unwrap();
    assert_eq!(fs::read(exported).unwrap(), fs::read(source).unwrap());
}

#[test]
fn backup_restore_keeps_deleted_documents_replacements_links_and_fixed_knowledge() {
    let (temp, mut v, entity, asset, document) = fixture();
    v.save_document_link(link(&document, &entity.id)).unwrap();
    v.save_document_link(link(&document, &asset.id)).unwrap();
    let old = v.list_document_library_at(None).unwrap();
    let source = temp.path().join("new.txt");
    fs::write(&source, b"second original").unwrap();
    let replaced = v
        .replace_document(
            ReplaceDocumentInput {
                id: document.id.clone(),
                expected_revision_id: document.revision_id.clone(),
                reason: "Nouvel original".into(),
                description: "description\navec ligne".into(),
            },
            &source,
        )
        .unwrap();
    v.save_document_metadata(metadata(&replaced, "deleted"))
        .unwrap();
    let baseline = state(&v);
    let archive = temp.path().join("documents.zip");
    v.create_backup(&archive).unwrap();
    let inspection = v.inspect_restore(&archive).unwrap();
    v.activate_restore(&inspection.token, true, None).unwrap();
    assert_eq!(state(&v), baseline);
    assert_eq!(
        serde_json::to_value(
            v.list_document_library_at(Some(old.knowledge_sequence))
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(old).unwrap()
    );
}

#[test]
fn rehashed_archive_rejects_a_link_to_a_target_unknown_at_its_recording_sequence() {
    let (temp, mut v, entity, _, document) = fixture();
    v.save_document_link(link(&document, &entity.id)).unwrap();
    let future = person(&mut v, "Créée après l’association");
    let archive = temp.path().join("source.zip");
    v.create_backup(&archive).unwrap();
    let bad = temp.path().join("future.zip");
    altered_archive(&archive, &bad, |c| {
        c.execute("UPDATE document_links SET target_object_id=?1", [future.id])
            .unwrap();
    });
    let before = state(&v);
    assert!(v.inspect_restore(&bad).is_err());
    assert_eq!(state(&v), before);
}

#[test]
fn rehashed_archive_validates_hidden_old_document_metadata_not_only_latest_active_version() {
    let (temp, mut v, _, _, document) = fixture();
    v.save_document_metadata(metadata(&document, "active"))
        .unwrap();
    let archive = temp.path().join("source.zip");
    v.create_backup(&archive).unwrap();
    let bad = temp.path().join("old-invalid.zip");
    altered_archive(&archive, &bad, |c| {
        c.execute(
            "UPDATE document_versions SET original_name='../escape.txt' WHERE revision_id=?1",
            [document.revision_id],
        )
        .unwrap();
    });
    let before = state(&v);
    assert!(v.inspect_restore(&bad).is_err());
    assert_eq!(state(&v), before);
}

#[test]
fn document_status_wire_values_remain_compatible_and_unknown_states_are_rejected() {
    for value in ["active", "archived", "deleted"] {
        let status = DocumentStatus::try_from(value).unwrap();
        assert_eq!(
            serde_json::to_string(&status).unwrap(),
            format!("\"{value}\"")
        );
        assert_eq!(
            serde_json::from_str::<DocumentStatus>(&format!("\"{value}\"")).unwrap(),
            status
        );
    }
    assert!(DocumentStatus::try_from("pending").is_err());
    assert!(serde_json::from_str::<DocumentStatus>("\"pending\"").is_err());
}
