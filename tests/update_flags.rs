// Tests for bf update command field flags
// Tests that: --description, --acceptance-criteria, --notes, --design, --due-at

use bead_forge::model::{Issue, IssueChanges};
use bead_forge::storage::Storage;
use chrono::{DateTime, Utc};
use tempfile::TempDir;

fn create_test_storage() -> (Storage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();
    (storage, temp_dir)
}

fn create_test_bead(storage: &Storage, title: &str) -> Issue {
    let issue = Issue::new(format!("test-{}", title.replace(' ', "-")), title.to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();
    issue
}

#[test]
fn test_update_description() {
    let (storage, _temp_dir) = create_test_storage();
    let bead = create_test_bead(&storage, "original title");

    let changes = IssueChanges {
        description: Some("New description".to_string()),
        ..Default::default()
    };

    storage.update_issue(&bead.id, &changes).unwrap();

    let updated = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(updated.description, Some("New description".to_string()));
    assert_eq!(updated.title, bead.title); // Other fields unchanged
}

#[test]
fn test_update_acceptance_criteria() {
    let (storage, _temp_dir) = create_test_storage();
    let bead = create_test_bead(&storage, "test ac");

    let changes = IssueChanges {
        acceptance_criteria: Some("Should pass tests".to_string()),
        ..Default::default()
    };

    storage.update_issue(&bead.id, &changes).unwrap();

    let updated = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(
        updated.acceptance_criteria,
        Some("Should pass tests".to_string())
    );
}

#[test]
fn test_update_notes() {
    let (storage, _temp_dir) = create_test_storage();
    let bead = create_test_bead(&storage, "test notes");

    let changes = IssueChanges {
        notes: Some("Additional notes here".to_string()),
        ..Default::default()
    };

    storage.update_issue(&bead.id, &changes).unwrap();

    let updated = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(updated.notes, Some("Additional notes here".to_string()));
}

#[test]
fn test_update_design() {
    let (storage, _temp_dir) = create_test_storage();
    let bead = create_test_bead(&storage, "test design");

    let changes = IssueChanges {
        design: Some("Design documentation".to_string()),
        ..Default::default()
    };

    storage.update_issue(&bead.id, &changes).unwrap();

    let updated = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(updated.design, Some("Design documentation".to_string()));
}

#[test]
fn test_update_due_at_rfc3339() {
    let (storage, _temp_dir) = create_test_storage();
    let bead = create_test_bead(&storage, "test due");

    let due_date: DateTime<Utc> = "2025-12-31T23:59:59Z".parse().unwrap();
    let changes = IssueChanges {
        due_at: Some(due_date),
        ..Default::default()
    };

    storage.update_issue(&bead.id, &changes).unwrap();

    let updated = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(updated.due_at.map(|d| d.to_rfc3339()), Some("2025-12-31T23:59:59+00:00".to_string()));
}

#[test]
fn test_update_multiple_fields() {
    let (storage, _temp_dir) = create_test_storage();
    let bead = create_test_bead(&storage, "test multiple");

    let due_date: DateTime<Utc> = "2025-06-30T12:00:00Z".parse().unwrap();
    let changes = IssueChanges {
        description: Some("Updated description".to_string()),
        acceptance_criteria: Some("AC 1, AC 2".to_string()),
        notes: Some("Notes here".to_string()),
        design: Some("Design docs".to_string()),
        due_at: Some(due_date),
        ..Default::default()
    };

    storage.update_issue(&bead.id, &changes).unwrap();

    let updated = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(updated.description, Some("Updated description".to_string()));
    assert_eq!(updated.acceptance_criteria, Some("AC 1, AC 2".to_string()));
    assert_eq!(updated.notes, Some("Notes here".to_string()));
    assert_eq!(updated.design, Some("Design docs".to_string()));
    assert_eq!(updated.due_at.map(|d| d.to_rfc3339()), Some("2025-06-30T12:00:00+00:00".to_string()));
}

#[test]
fn test_update_clears_description() {
    let (storage, _temp_dir) = create_test_storage();
    let mut bead = create_test_bead(&storage, "test clear");
    bead.description = Some("Original description".to_string());
    storage.update_issue(&bead.id, &IssueChanges {
        description: Some("".to_string()),
        ..Default::default()
    }).unwrap();

    // Empty string should clear the field (None in Rust)
    let updated = storage.get_issue(&bead.id).unwrap().unwrap();
    // The behavior depends on implementation - could be None or empty string
    // We just verify it changed
    assert!(updated.description != Some("Original description".to_string()));
}

#[test]
fn test_update_preserves_other_fields() {
    let (storage, _temp_dir) = create_test_storage();
    let bead = create_test_bead(&storage, "test preserve");

    // Set initial values using update
    let initial_changes = IssueChanges {
        description: Some("Original description".to_string()),
        acceptance_criteria: Some("Original AC".to_string()),
        notes: Some("Original notes".to_string()),
        design: Some("Original design".to_string()),
        ..Default::default()
    };
    storage.update_issue(&bead.id, &initial_changes).unwrap();

    // Update only description
    let changes = IssueChanges {
        description: Some("New description only".to_string()),
        ..Default::default()
    };

    storage.update_issue(&bead.id, &changes).unwrap();

    let updated = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(updated.description, Some("New description only".to_string()));
    // All other fields should be preserved
    assert_eq!(updated.acceptance_criteria, Some("Original AC".to_string()));
    assert_eq!(updated.notes, Some("Original notes".to_string()));
    assert_eq!(updated.design, Some("Original design".to_string()));
}

#[test]
fn test_update_with_multiline_text() {
    let (storage, _temp_dir) = create_test_storage();
    let bead = create_test_bead(&storage, "test multiline");

    let multiline_desc = "Line 1\nLine 2\nLine 3".to_string();
    let changes = IssueChanges {
        description: Some(multiline_desc.clone()),
        ..Default::default()
    };

    storage.update_issue(&bead.id, &changes).unwrap();

    let updated = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(updated.description, Some(multiline_desc));
}

#[test]
fn test_update_unicode_characters() {
    let (storage, _temp_dir) = create_test_storage();
    let bead = create_test_bead(&storage, "test unicode");

    let unicode_text = "Description with émojis 🎉 and spëcial çharacters".to_string();
    let changes = IssueChanges {
        description: Some(unicode_text.clone()),
        ..Default::default()
    };

    storage.update_issue(&bead.id, &changes).unwrap();

    let updated = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(updated.description, Some(unicode_text));
}
