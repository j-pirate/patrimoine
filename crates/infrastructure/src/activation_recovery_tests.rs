use super::*;

fn fixture() -> (tempfile::TempDir, Vault, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let mut vault = Vault::open(temp.path().join("vault")).unwrap();
    vault
        .save_settings(Settings {
            workspace_name: "Restored data".into(),
            display_currency: "EUR".into(),
        })
        .unwrap();
    let backup = temp.path().join("backup.zip");
    vault.create_backup(&backup).unwrap();
    vault
        .save_settings(Settings {
            workspace_name: "Previous data".into(),
            display_currency: "EUR".into(),
        })
        .unwrap();
    (temp, vault, backup)
}

#[test]
fn failure_after_current_switch_returns_the_active_generation_and_retries_the_journal() {
    let (_temp, mut vault, backup) = fixture();
    let previous = vault.status().unwrap().generation_id;
    let inspection = vault.inspect_restore(&backup).unwrap();
    test_faults::fail("remove", &vault.root.join("recovery/activation.json"));
    let restored = vault
        .activate_restore(&inspection.token, true, None)
        .unwrap();
    assert_ne!(restored.generation_id, previous);
    assert_eq!(restored.settings.workspace_name, "Restored data");
    assert!(restored.recovery_required);
    assert!(restored.recovery_message.is_some());
    assert_eq!(current(&vault.root).unwrap(), restored.generation_id);
    assert!(vault.root.join("generations").join(previous).exists());
    let next = vault.inspect_restore(&backup).unwrap();
    let result = vault.activate_restore(&next.token, true, None).unwrap();
    assert!(!result.recovery_required);
    assert!(!vault.root.join("recovery/activation.json").exists());
}

#[test]
fn a_current_sync_failure_exposes_the_switched_generation_then_recovers_on_reopen() {
    let (temp, mut vault, backup) = fixture();
    let inspection = vault.inspect_restore(&backup).unwrap();
    test_faults::fail("sync", &vault.root);
    let restored = vault
        .activate_restore(&inspection.token, true, None)
        .unwrap();
    assert_eq!(restored.settings.workspace_name, "Restored data");
    assert!(restored.recovery_required);
    assert!(vault.root.join("recovery/activation.json").exists());
    drop(vault);
    let reopened = Vault::open(temp.path().join("vault")).unwrap();
    assert_eq!(
        reopened.status().unwrap().generation_id,
        restored.generation_id
    );
    assert!(!reopened.status().unwrap().recovery_required);
}

#[test]
fn failure_before_switch_keeps_previous_data_and_does_not_block_retry() {
    let (_temp, mut vault, backup) = fixture();
    let previous = vault.status().unwrap().generation_id;
    let inspection = vault.inspect_restore(&backup).unwrap();
    test_faults::fail("current_rename", &vault.root);
    assert!(vault
        .activate_restore(&inspection.token, true, None)
        .is_err());
    assert_eq!(vault.status().unwrap().generation_id, previous);
    assert_eq!(
        vault.status().unwrap().settings.workspace_name,
        "Previous data"
    );
    assert!(!vault.root.join("recovery/activation.json").exists());
    let next = vault.inspect_restore(&backup).unwrap();
    assert!(vault.activate_restore(&next.token, true, None).is_ok());
}

#[test]
fn unsynced_blob_ancestor_prevents_sqlite_reference_including_on_reuse() {
    let (temp, mut vault, _backup) = fixture();
    let source = temp.path().join("original.txt");
    fs::write(&source, b"original bytes").unwrap();
    vault.import_document(&source, "first").unwrap();
    let before = vault.status().unwrap();
    test_faults::fail("sync", &vault.generation_path().join("documents"));
    assert!(vault.import_document(&source, "second").is_err());
    let after = vault.status().unwrap();
    assert_eq!(after.document_count, before.document_count);
    assert_eq!(after.mutation_sequence, before.mutation_sequence);
}
