use super::*;
use crate::Vault;
use patrimoine_application::VaultService;
use std::{
    fs,
    time::{Duration, Instant},
};

fn fixture() -> (tempfile::TempDir, Vault) {
    let temp = tempfile::tempdir().unwrap();
    let mut vault = Vault::open(temp.path().join("vault")).unwrap();
    for name in ["one.txt", "two.txt"] {
        let path = temp.path().join(name);
        fs::write(&path, name.as_bytes()).unwrap();
        vault.import_document(&path, name).unwrap();
    }
    (temp, vault)
}

#[test]
fn rejects_malformed_technical_dates_and_command_ids_with_precise_diagnostics() {
    for (table, column) in [
        ("schema_migrations", "applied_at"),
        ("objects", "created_at"),
        ("change_sets", "recorded_at"),
        ("revisions", "recorded_at"),
        ("app_settings", "updated_at"),
        ("document_blobs", "created_at"),
    ] {
        let (_temp, vault) = fixture();
        let connection = Connection::open(vault.db_path()).unwrap();
        connection
            .execute(
                &format!("UPDATE {table} SET {column}='not-a-timestamp'"),
                [],
            )
            .unwrap();
        drop(connection);
        let error = validate(&vault.db_path()).unwrap_err();
        assert_eq!(error.code, "invalid_archive");
        assert!(
            error.message.contains(&format!("{table}.{column}")),
            "{}",
            error.message
        );
    }
    let (_temp, vault) = fixture();
    let connection = Connection::open(vault.db_path()).unwrap();
    connection
        .execute(
            "UPDATE change_sets SET command_id='invalid' WHERE sequence=1",
            [],
        )
        .unwrap();
    assert!(validate(&vault.db_path())
        .unwrap_err()
        .message
        .contains("UUID"));
    assert!(valid_timestamp("2026-11-16T12:00:00Z"));
    assert!(valid_timestamp("2026-11-16T12:00:00.123456789+00:00"));
    assert!(!valid_timestamp("2026-11-16T12:00:00+01:00"));
    assert!(!valid_timestamp("2026-02-30T12:00:00Z"));
}

#[test]
fn rejects_audit_revision_relationship_tampering_on_backup_and_reopen() {
    for column in ["object_id", "change_set_id"] {
        let (temp, mut vault) = fixture();
        let connection = Connection::open(vault.db_path()).unwrap();
        let revision: String = connection
            .query_row("SELECT id FROM revisions ORDER BY id LIMIT 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        connection.execute(&format!("UPDATE audit_events SET {column}=(SELECT {column} FROM revisions WHERE id<>?1 LIMIT 1) WHERE revision_id=?1"), [&revision]).unwrap();
        drop(connection);
        let error = vault
            .create_backup(&temp.path().join("invalid.zip"))
            .unwrap_err();
        assert_eq!(error.code, "invalid_archive");
        assert!(error.message.contains("audit"));
        assert!(!temp.path().join("invalid.zip").exists());
        drop(vault);
        let error = match Vault::open(temp.path().join("vault")) {
            Ok(_) => panic!("tampered audit accepted"),
            Err(error) => error,
        };
        assert!(error.message.contains("audit"));
    }
}

#[test]
fn snapshot_deadline_bounds_external_lock_and_removes_its_candidate() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.sqlite3");
    let target = temp.path().join("candidate.sqlite3");
    let connection = Connection::open(&source).unwrap();
    connection.execute_batch("CREATE TABLE sample(value TEXT); INSERT INTO sample VALUES('preserved'); BEGIN EXCLUSIVE;").unwrap();
    let started = Instant::now();
    let error =
        copy_snapshot_with_timeout(&source, &target, Duration::from_millis(40)).unwrap_err();
    assert_eq!(error.code, "snapshot_timeout");
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(!target.exists());
    connection.execute_batch("ROLLBACK;").unwrap();
    copy_snapshot_with_timeout(&source, &target, Duration::from_secs(1)).unwrap();
    let snapshot = Connection::open(&target).unwrap();
    assert_eq!(
        snapshot
            .query_row("SELECT value FROM sample", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "preserved"
    );
    assert_eq!(
        connection
            .query_row("SELECT value FROM sample", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "preserved"
    );
}
