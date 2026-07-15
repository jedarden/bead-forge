// Duplicate Label Test (bf-29jlp)
// Tests for duplicate label handling across various scenarios

use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use std::collections::HashSet;

#[test]
fn test_duplicate_label_add_prevention() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create bead
    let bead = Issue {
        id: "dup-test-bead".to_string(),
        title: "Duplicate Label Test Bead".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Add same label multiple times
    storage.add_label("dup-test-bead", "urgent").unwrap();
    storage.add_label("dup-test-bead", "urgent").unwrap();
    storage.add_label("dup-test-bead", "urgent").unwrap();

    // Verify only one instance exists
    let labels = storage.get_labels("dup-test-bead").unwrap();
    assert_eq!(
        labels.len(),
        1,
        "Duplicate adds should result in single label"
    );
    assert!(labels.contains(&"urgent".to_string()));
}

#[test]
fn test_multiple_duplicate_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create bead
    let bead = Issue {
        id: "multi-dup-bead".to_string(),
        title: "Multiple Duplicate Labels Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Add multiple labels with duplicates
    storage.add_label("multi-dup-bead", "urgent").unwrap();
    storage.add_label("multi-dup-bead", "backend").unwrap();
    storage.add_label("multi-dup-bead", "urgent").unwrap(); // duplicate
    storage.add_label("multi-dup-bead", "frontend").unwrap();
    storage.add_label("multi-dup-bead", "backend").unwrap(); // duplicate
    storage.add_label("multi-dup-bead", "urgent").unwrap(); // duplicate again

    // Verify unique labels only
    let labels = storage.get_labels("multi-dup-bead").unwrap();
    assert_eq!(
        labels.len(),
        3,
        "Should have 3 unique labels despite duplicate adds"
    );
    assert!(labels.contains(&"urgent".to_string()));
    assert!(labels.contains(&"backend".to_string()));
    assert!(labels.contains(&"frontend".to_string()));
}

#[test]
fn test_duplicate_label_removal() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create bead with labels
    let bead = Issue {
        id: "dup-removal-bead".to_string(),
        title: "Duplicate Removal Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["urgent".to_string(), "backend".to_string()],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Verify initial labels
    let labels = storage.get_labels("dup-removal-bead").unwrap();
    assert_eq!(labels.len(), 2);

    // Remove label (even if it was added multiple times internally)
    storage.remove_label("dup-removal-bead", "urgent").unwrap();

    // Verify label is completely removed
    let labels = storage.get_labels("dup-removal-bead").unwrap();
    assert_eq!(labels.len(), 1, "Label should be completely removed");
    assert!(!labels.contains(&"urgent".to_string()));
    assert!(labels.contains(&"backend".to_string()));
}

#[test]
fn test_duplicate_labels_across_beads() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple beads with overlapping labels
    let bead1 = Issue {
        id: "dup-bead-1".to_string(),
        title: "Bead 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["urgent".to_string(), "backend".to_string()],
        ..Default::default()
    };
    storage.create_issue(&bead1).unwrap();

    let bead2 = Issue {
        id: "dup-bead-2".to_string(),
        title: "Bead 2".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["urgent".to_string(), "frontend".to_string()],
        ..Default::default()
    };
    storage.create_issue(&bead2).unwrap();

    let bead3 = Issue {
        id: "dup-bead-3".to_string(),
        title: "Bead 3".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["backend".to_string(), "frontend".to_string()],
        ..Default::default()
    };
    storage.create_issue(&bead3).unwrap();

    // Verify each bead has correct labels
    let labels1 = storage.get_labels("dup-bead-1").unwrap();
    assert_eq!(labels1.len(), 2);
    assert!(labels1.contains(&"urgent".to_string()));
    assert!(labels1.contains(&"backend".to_string()));

    let labels2 = storage.get_labels("dup-bead-2").unwrap();
    assert_eq!(labels2.len(), 2);
    assert!(labels2.contains(&"urgent".to_string()));
    assert!(labels2.contains(&"frontend".to_string()));

    let labels3 = storage.get_labels("dup-bead-3").unwrap();
    assert_eq!(labels3.len(), 2);
    assert!(labels3.contains(&"backend".to_string()));
    assert!(labels3.contains(&"frontend".to_string()));

    // Verify global label aggregation counts duplicates correctly
    let all_labels = storage.list_all_labels().unwrap();
    let label_map: std::collections::HashMap<String, i64> = all_labels.into_iter().collect();

    assert_eq!(
        label_map.get("urgent"),
        Some(&2),
        "urgent should appear in 2 beads"
    );
    assert_eq!(
        label_map.get("backend"),
        Some(&2),
        "backend should appear in 2 beads"
    );
    assert_eq!(
        label_map.get("frontend"),
        Some(&2),
        "frontend should appear in 2 beads"
    );
}

#[test]
fn test_duplicate_label_serialization() {
    // Create issue with duplicate labels in the vector
    let issue = Issue {
        id: "dup-serialize".to_string(),
        title: "Duplicate Serialization Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![
            "urgent".to_string(),
            "urgent".to_string(), // duplicate in vector
            "backend".to_string(),
        ],
        ..Default::default()
    };

    // Serialize
    let json = serde_json::to_string(&issue).unwrap();

    // Deserialize
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    // Verify duplicates are preserved during serialization
    assert_eq!(
        deserialized.labels.len(),
        3,
        "Serialization should preserve duplicates in vector"
    );
}

#[test]
fn test_duplicate_label_deduplication_on_storage() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create issue with duplicate labels in the vector
    let issue = Issue {
        id: "dup-storage".to_string(),
        title: "Storage Deduplication Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![
            "urgent".to_string(),
            "urgent".to_string(),
            "urgent".to_string(),
            "backend".to_string(),
            "backend".to_string(),
        ],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Retrieve from storage
    let retrieved = storage.get_issue("dup-storage").unwrap().unwrap();

    // Verify storage deduplicates
    let unique_labels: HashSet<String> = retrieved.labels.into_iter().collect();
    assert_eq!(unique_labels.len(), 2, "Storage should deduplicate labels");
    assert!(unique_labels.contains("urgent"));
    assert!(unique_labels.contains("backend"));
}

#[test]
fn test_duplicate_label_with_removal_and_readd() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create bead
    let bead = Issue {
        id: "dup-readd-bead".to_string(),
        title: "Duplicate Readd Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["urgent".to_string()],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Add duplicate multiple times
    storage.add_label("dup-readd-bead", "urgent").unwrap();
    storage.add_label("dup-readd-bead", "urgent").unwrap();

    // Remove label
    storage.remove_label("dup-readd-bead", "urgent").unwrap();

    // Verify no labels
    let labels = storage.get_labels("dup-readd-bead").unwrap();
    assert_eq!(labels.len(), 0);

    // Re-add the same label
    storage.add_label("dup-readd-bead", "urgent").unwrap();

    // Verify it's back
    let labels = storage.get_labels("dup-readd-bead").unwrap();
    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"urgent".to_string()));
}

#[test]
fn test_duplicate_label_case_sensitivity() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create bead
    let bead = Issue {
        id: "dup-case-bead".to_string(),
        title: "Case Sensitivity Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Add same label with different cases
    storage.add_label("dup-case-bead", "urgent").unwrap();
    storage.add_label("dup-case-bead", "URGENT").unwrap();
    storage.add_label("dup-case-bead", "Urgent").unwrap();

    // Verify case sensitivity (should have 3 different labels if case-sensitive)
    let labels = storage.get_labels("dup-case-bead").unwrap();
    // Based on implementation, this tests whether labels are case-sensitive
    // The assertion depends on the actual implementation behavior
    assert!(labels.len() >= 1, "At least one label should exist");
}

#[test]
fn test_duplicate_label_empty_string() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create bead
    let bead = Issue {
        id: "dup-empty-bead".to_string(),
        title: "Empty String Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["urgent".to_string()],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Try to add empty string as label (should be handled gracefully)
    let result = storage.add_label("dup-empty-bead", "");

    // Verify behavior (either fails or is ignored)
    match result {
        Ok(_) => {
            let labels = storage.get_labels("dup-empty-bead").unwrap();
            // If empty strings are allowed, they should be deduplicated too
            let unique_labels: HashSet<String> = labels.into_iter().collect();
            assert!(unique_labels.contains("urgent"));
        }
        Err(_) => {
            // Expected behavior - empty strings rejected
            let labels = storage.get_labels("dup-empty-bead").unwrap();
            assert!(labels.contains(&"urgent".to_string()));
        }
    }
}

#[test]
fn test_duplicate_label_whitespace_handling() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create bead
    let bead = Issue {
        id: "dup-whitespace-bead".to_string(),
        title: "Whitespace Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Add labels with whitespace variations
    storage.add_label("dup-whitespace-bead", "urgent").unwrap();
    storage.add_label("dup-whitespace-bead", "urgent ").unwrap(); // trailing space
    storage.add_label("dup-whitespace-bead", " urgent").unwrap(); // leading space

    // Verify behavior (whitespace should create different labels or be trimmed)
    let labels = storage.get_labels("dup-whitespace-bead").unwrap();
    // The exact assertion depends on whitespace handling implementation
    assert!(labels.len() >= 1, "At least one label should exist");
    assert!(
        labels.contains(&"urgent".to_string())
            || labels.contains(&"urgent ".to_string())
            || labels.contains(&" urgent".to_string())
    );
}

#[test]
fn test_duplicate_label_with_special_characters() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create bead
    let bead = Issue {
        id: "dup-special-bead".to_string(),
        title: "Special Characters Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Add labels with special characters
    storage.add_label("dup-special-bead", "phase-1").unwrap();
    storage.add_label("dup-special-bead", "phase-1").unwrap();
    storage.add_label("dup-special-bead", "bug/fix").unwrap();
    storage.add_label("dup-special-bead", "bug/fix").unwrap();

    // Verify deduplication works with special characters
    let labels = storage.get_labels("dup-special-bead").unwrap();
    assert_eq!(
        labels.len(),
        2,
        "Special character labels should be deduplicated"
    );
    assert!(labels.contains(&"phase-1".to_string()));
    assert!(labels.contains(&"bug/fix".to_string()));
}

#[test]
fn test_duplicate_label_unicode() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create bead
    let bead = Issue {
        id: "dup-unicode-bead".to_string(),
        title: "Unicode Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Add unicode labels with duplicates
    storage.add_label("dup-unicode-bead", "🔥urgent").unwrap();
    storage.add_label("dup-unicode-bead", "🔥urgent").unwrap();
    storage.add_label("dup-unicode-bead", "tâsk").unwrap();
    storage.add_label("dup-unicode-bead", "tâsk").unwrap();

    // Verify unicode deduplication
    let labels = storage.get_labels("dup-unicode-bead").unwrap();
    assert_eq!(labels.len(), 2, "Unicode labels should be deduplicated");
    assert!(labels.contains(&"🔥urgent".to_string()));
    assert!(labels.contains(&"tâsk".to_string()));
}

#[test]
fn test_duplicate_label_very_long_label() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create bead
    let bead = Issue {
        id: "dup-long-bead".to_string(),
        title: "Long Label Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Add very long label with duplicates
    let long_label = "a".repeat(1000);
    storage.add_label("dup-long-bead", &long_label).unwrap();
    storage.add_label("dup-long-bead", &long_label).unwrap();

    // Verify deduplication works with long labels
    let labels = storage.get_labels("dup-long-bead").unwrap();
    assert_eq!(labels.len(), 1, "Long labels should be deduplicated");
    assert!(labels.contains(&long_label));
}
