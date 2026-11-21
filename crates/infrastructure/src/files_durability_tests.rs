use super::*;

#[test]
fn document_directory_chain_is_synced_for_new_and_reused_blobs() {
    let temp = tempfile::tempdir().unwrap();
    let generation = temp.path().join("generation");
    directory(&generation).unwrap();
    let parent = generation.join("documents/sha256/ab");
    for _ in 0..2 {
        test_faults::take_syncs();
        sync_blob_directory(&generation, &parent).unwrap();
        let syncs = test_faults::take_syncs();
        assert!(syncs.ends_with(&[
            parent.clone(),
            generation.join("documents/sha256"),
            generation.join("documents"),
            generation.clone(),
        ]));
    }
}

#[test]
fn exclusive_publication_moves_a_complete_file_and_preserves_a_racing_destination() {
    let temp = tempfile::tempdir().unwrap();
    let staged = temp.path().join("staged");
    let target = temp.path().join("target");
    fs::write(&staged, b"complete bytes").unwrap();
    publish_file(&staged, &target).unwrap();
    assert!(!staged.exists());
    assert_eq!(fs::read(&target).unwrap(), b"complete bytes");
    fs::write(&staged, b"different bytes").unwrap();
    assert!(publish_file(&staged, &target).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"complete bytes");
    assert_eq!(fs::read(&staged).unwrap(), b"different bytes");
}

#[test]
fn a_parent_sync_failure_reports_the_already_published_destination() {
    let temp = tempfile::tempdir().unwrap();
    let staged = temp.path().join("staged");
    let target = temp.path().join("backup.zip");
    fs::write(&staged, b"complete bytes").unwrap();
    test_faults::fail("sync", temp.path());
    let error = publish_file(&staged, &target).unwrap_err();
    assert_eq!(error.code, "publication_uncertain");
    assert!(error.message.contains(target.to_str().unwrap()));
    assert!(error.message.contains("nouveau nom"));
    assert_eq!(fs::read(&target).unwrap(), b"complete bytes");
    assert!(!staged.exists());
}
