use bead_forge::storage::Storage;

#[test]
fn test_remove_label_from_nonexistent_bead() {
    let temp_dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&temp_dir.path().join("test.db")).unwrap();
    
    // Try to remove a label from a non-existent bead
    let result = storage.remove_label("non-existent-bead", "some-label");
    
    // Should succeed idempotently (no-op)
    assert!(result.is_ok(), "remove_label should succeed for non-existent bead");
}
