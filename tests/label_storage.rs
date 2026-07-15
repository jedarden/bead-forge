// Label storage tests - duplicate of label_list.rs
// Tests label creation, listing, aggregation, counting, and ordering

use bead_forge::model::{Issue, IssueChanges, IssueType, Status};
use bead_forge::storage::Storage;

#[test]
fn test_label_add_and_list() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create an issue
    let issue = Issue {
        id: "test-1".to_string(),
        title: "Test Issue".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Add labels one by one
    storage.add_label("test-1", "bug").unwrap();
    storage.add_label("test-1", "urgent").unwrap();

    // List all labels
    let labels = storage.list_all_labels().unwrap();
    assert_eq!(labels.len(), 2);

    // Get labels for specific issue
    let issue_labels = storage.get_labels("test-1").unwrap();
    assert_eq!(issue_labels.len(), 2);
    assert!(issue_labels.contains(&"bug".to_string()));
    assert!(issue_labels.contains(&"urgent".to_string()));
}

#[test]
fn test_label_all_unique() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple issues with various labels
    let issue1 = Issue {
        id: "issue-1".to_string(),
        title: "Issue 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["bug".to_string(), "urgent".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue1).unwrap();

    let issue2 = Issue {
        id: "issue-2".to_string(),
        title: "Issue 2".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["bug".to_string(), "feature".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue2).unwrap();

    // List all labels - should return unique labels with counts
    let labels = storage.list_all_labels().unwrap();

    // Verify all labels are unique (no duplicate label names)
    let label_names: Vec<&String> = labels.iter().map(|(name, _)| name).collect();
    let unique_labels: std::collections::HashSet<_> = label_names.iter().collect();

    assert_eq!(
        label_names.len(),
        unique_labels.len(),
        "All labels should be unique"
    );
    assert_eq!(
        labels.len(),
        3,
        "Should have 3 unique labels: bug, urgent, feature"
    );
}

#[test]
fn test_label_duplicate_handling() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create an issue
    let issue = Issue {
        id: "dup-test".to_string(),
        title: "Duplicate Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["bug".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Try to add the same label twice
    storage.add_label("dup-test", "urgent").unwrap();
    storage.add_label("dup-test", "urgent").unwrap();

    // List labels for the issue - should not have duplicates
    let issue_labels = storage.get_labels("dup-test").unwrap();

    // Count occurrences of "urgent"
    let urgent_count = issue_labels
        .iter()
        .filter(|l| l.as_str() == "urgent")
        .count();
    assert_eq!(
        urgent_count, 1,
        "Label 'urgent' should appear only once (no duplicates)"
    );

    // List all labels globally - count should be 1 for urgent
    let all_labels = storage.list_all_labels().unwrap();
    let urgent_entry = all_labels.iter().find(|(name, _)| name == "urgent");
    assert!(urgent_entry.is_some(), "Urgent label should exist");
    assert_eq!(
        urgent_entry.unwrap().1,
        1,
        "Urgent label count should be 1 (not duplicated)"
    );
}

#[test]
fn test_label_empty_bead() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create an issue with no labels
    let issue = Issue {
        id: "empty-bead".to_string(),
        title: "Empty Bead".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Get labels for bead with no labels
    let labels = storage.get_labels("empty-bead").unwrap();
    assert_eq!(
        labels.len(),
        0,
        "Bead with no labels should return empty list"
    );

    // List all labels globally - should not include anything from this bead
    let all_labels = storage.list_all_labels().unwrap();
    assert_eq!(all_labels.len(), 0, "Global label list should be empty");

    // Add a label to the empty bead
    storage.add_label("empty-bead", "first-label").unwrap();

    // Now we should have one label
    let labels = storage.get_labels("empty-bead").unwrap();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0], "first-label");

    let all_labels = storage.list_all_labels().unwrap();
    assert_eq!(all_labels.len(), 1);
    assert_eq!(all_labels[0].0, "first-label");
    assert_eq!(all_labels[0].1, 1);
}

#[test]
fn test_label_list_empty_database() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Empty database should return empty list
    let labels = storage.list_all_labels().unwrap();
    assert_eq!(labels.len(), 0);
}

#[test]
fn test_label_list_single_label() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create issue with label
    let issue = Issue {
        id: "test-1".to_string(),
        title: "Test Issue".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["bug".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // List labels
    let labels = storage.list_all_labels().unwrap();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].0, "bug");
    assert_eq!(labels[0].1, 1);
}

#[test]
fn test_label_list_multiple_issues_same_label() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple issues with the same label
    for i in 1..=5 {
        let issue = Issue {
            id: format!("issue-{}", i),
            title: format!("Issue {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            labels: vec!["urgent".to_string()],
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();
    }

    // List labels - should aggregate
    let labels = storage.list_all_labels().unwrap();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].0, "urgent");
    assert_eq!(labels[0].1, 5);
}

#[test]
fn test_label_list_multiple_labels_same_issue() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create issue with multiple labels
    let issue = Issue {
        id: "multi-label".to_string(),
        title: "Multi Label Issue".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![
            "bug".to_string(),
            "urgent".to_string(),
            "frontend".to_string(),
        ],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // List labels
    let labels = storage.list_all_labels().unwrap();
    assert_eq!(labels.len(), 3);

    // Each label should have count 1
    for (_label, count) in &labels {
        assert_eq!(*count, 1);
    }
}

#[test]
fn test_label_list_ordering_by_count() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create issues with different label frequencies
    // "common" - 5 issues
    for i in 1..=5 {
        let issue = Issue {
            id: format!("common-{}", i),
            title: format!("Common {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            labels: vec!["common".to_string()],
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();
    }

    // "medium" - 3 issues
    for i in 1..=3 {
        let issue = Issue {
            id: format!("medium-{}", i),
            title: format!("Medium {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            labels: vec!["medium".to_string()],
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();
    }

    // "rare" - 1 issue
    let issue = Issue {
        id: "rare-1".to_string(),
        title: "Rare".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["rare".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // List labels - should be ordered by count DESC
    let labels = storage.list_all_labels().unwrap();
    assert_eq!(labels.len(), 3);

    // Check ordering
    assert_eq!(labels[0].0, "common");
    assert_eq!(labels[0].1, 5);

    assert_eq!(labels[1].0, "medium");
    assert_eq!(labels[1].1, 3);

    assert_eq!(labels[2].0, "rare");
    assert_eq!(labels[2].1, 1);
}

#[test]
fn test_label_list_mixed_distribution() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create complex label distribution
    let test_data = vec![
        ("issue-1", vec!["bug", "urgent", "frontend"]),
        ("issue-2", vec!["bug", "backend"]),
        ("issue-3", vec!["urgent", "frontend"]),
        ("issue-4", vec!["bug"]),
        ("issue-5", vec!["docs", "low-priority"]),
    ];

    for (id, labels) in test_data {
        let issue = Issue {
            id: id.to_string(),
            title: id.to_string(),
            issue_type: IssueType::Task,
            status: Status::Open,
            labels: labels.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();
    }

    // List and verify
    let labels = storage.list_all_labels().unwrap();
    assert_eq!(labels.len(), 6);

    // Find specific labels and verify counts
    let label_map: std::collections::HashMap<String, i64> = labels.into_iter().collect();
    assert_eq!(label_map.get("bug"), Some(&3));
    assert_eq!(label_map.get("urgent"), Some(&2));
    assert_eq!(label_map.get("frontend"), Some(&2));
    assert_eq!(label_map.get("backend"), Some(&1));
    assert_eq!(label_map.get("docs"), Some(&1));
    assert_eq!(label_map.get("low-priority"), Some(&1));
}

#[test]
fn test_label_list_after_add() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create issue without labels
    let issue = Issue {
        id: "no-labels".to_string(),
        title: "No Labels".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Initially empty
    let labels = storage.list_all_labels().unwrap();
    assert_eq!(labels.len(), 0);

    // Add labels via update (replace entire list)
    let changes = IssueChanges {
        labels: Some(vec!["bug".to_string(), "urgent".to_string()]),
        ..Default::default()
    };
    storage.update_issue("no-labels", &changes).unwrap();

    // Should now have 2 labels
    let labels = storage.list_all_labels().unwrap();
    assert_eq!(labels.len(), 2);
}

#[test]
fn test_label_list_after_remove() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create issue with labels
    let issue = Issue {
        id: "remove-test".to_string(),
        title: "Remove Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![
            "bug".to_string(),
            "urgent".to_string(),
            "frontend".to_string(),
        ],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Verify initial state
    let labels = storage.list_all_labels().unwrap();
    assert_eq!(labels.len(), 3);

    // Remove one label directly
    storage.remove_label("remove-test", "urgent").unwrap();

    // Should now have 2 labels
    let labels = storage.list_all_labels().unwrap();
    assert_eq!(labels.len(), 2);

    let label_set: std::collections::HashSet<String> = labels.into_iter().map(|(l, _)| l).collect();
    assert!(label_set.contains("bug"));
    assert!(label_set.contains("frontend"));
    assert!(!label_set.contains("urgent"));
}

#[test]
fn test_label_list_after_issue_close() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create two issues with labels
    let issue1 = Issue {
        id: "close-1".to_string(),
        title: "Close 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["bug".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue1).unwrap();

    let issue2 = Issue {
        id: "keep-1".to_string(),
        title: "Keep 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["bug".to_string(), "urgent".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue2).unwrap();

    // Verify initial counts
    let labels = storage.list_all_labels().unwrap();
    assert_eq!(labels.len(), 2);
    let label_map: std::collections::HashMap<String, i64> = labels.into_iter().collect();
    assert_eq!(label_map.get("bug"), Some(&2));
    assert_eq!(label_map.get("urgent"), Some(&1));

    // Close one issue (labels should still be counted)
    storage.close_issue("close-1", "completed", "test").unwrap();

    // Labels are still present even on closed issues
    let labels = storage.list_all_labels().unwrap();
    assert_eq!(labels.len(), 2);
    let label_map: std::collections::HashMap<String, i64> = labels.into_iter().collect();
    assert_eq!(label_map.get("bug"), Some(&2)); // Still 2
    assert_eq!(label_map.get("urgent"), Some(&1));
}

#[test]
fn test_label_list_case_sensitivity() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create issues with different case labels
    let issue1 = Issue {
        id: "case-1".to_string(),
        title: "Case 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["Bug".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue1).unwrap();

    let issue2 = Issue {
        id: "case-2".to_string(),
        title: "Case 2".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["bug".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue2).unwrap();

    // Should treat as different labels
    let labels = storage.list_all_labels().unwrap();
    assert_eq!(labels.len(), 2);

    let label_set: std::collections::HashSet<String> = labels.into_iter().map(|(l, _)| l).collect();
    assert!(label_set.contains("Bug"));
    assert!(label_set.contains("bug"));
}

#[test]
fn test_label_list_special_characters() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create issues with special character labels
    let issue = Issue {
        id: "special".to_string(),
        title: "Special".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![
            "high-priority".to_string(),
            "needs-review".to_string(),
            "API:breaking".to_string(),
        ],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // List labels
    let labels = storage.list_all_labels().unwrap();
    assert_eq!(labels.len(), 3);

    let label_set: std::collections::HashSet<String> = labels.into_iter().map(|(l, _)| l).collect();
    assert!(label_set.contains("high-priority"));
    assert!(label_set.contains("needs-review"));
    assert!(label_set.contains("API:breaking"));
}

#[test]
fn test_label_list_empty_label() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create issue with empty label string
    let issue = Issue {
        id: "empty-label".to_string(),
        title: "Empty Label".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["".to_string()],
        ..Default::default()
    };

    // This should either fail or handle gracefully
    // Depending on implementation, we might want to reject empty labels
    let result = storage.create_issue(&issue);

    // If it succeeds, verify the empty label is stored
    if result.is_ok() {
        let labels = storage.list_all_labels().unwrap();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].0, "");
        assert_eq!(labels[0].1, 1);
    }
}

#[test]
fn test_label_list_unicode() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create issues with unicode labels
    let issue = Issue {
        id: "unicode".to_string(),
        title: "Unicode".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![
            "🐛-bug".to_string(),
            "高优先级".to_string(),
            "critique".to_string(),
        ],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // List labels
    let labels = storage.list_all_labels().unwrap();
    assert_eq!(labels.len(), 3);

    let label_set: std::collections::HashSet<String> = labels.into_iter().map(|(l, _)| l).collect();
    assert!(label_set.contains("🐛-bug"));
    assert!(label_set.contains("高优先级"));
    assert!(label_set.contains("critique"));
}

#[test]
fn test_label_list_get_individual_issue_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple issues with different labels
    let issue1 = Issue {
        id: "issue-1".to_string(),
        title: "Issue 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["bug".to_string(), "urgent".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue1).unwrap();

    let issue2 = Issue {
        id: "issue-2".to_string(),
        title: "Issue 2".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["feature".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue2).unwrap();

    // Get labels for specific issue
    let labels1 = storage.get_labels("issue-1").unwrap();
    assert_eq!(labels1.len(), 2);
    assert!(labels1.contains(&"bug".to_string()));
    assert!(labels1.contains(&"urgent".to_string()));

    let labels2 = storage.get_labels("issue-2").unwrap();
    assert_eq!(labels2.len(), 1);
    assert!(labels2.contains(&"feature".to_string()));
}

#[test]
fn test_label_list_large_scale() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create many issues with labels to test performance
    let label_options = vec!["bug", "feature", "urgent", "docs", "refactor", "test"];

    for i in 1..=100 {
        let labels = vec![
            label_options[i % label_options.len()].to_string(),
            label_options[(i + 1) % label_options.len()].to_string(),
        ];
        let issue = Issue {
            id: format!("large-{}", i),
            title: format!("Large Scale {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            labels,
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();
    }

    // List all labels
    let labels = storage.list_all_labels().unwrap();
    assert_eq!(labels.len(), 6);

    // All labels should appear at least once
    let label_map: std::collections::HashMap<String, i64> = labels.into_iter().collect();
    for label in &label_options {
        assert!(label_map.contains_key(&label.to_string()));
    }
}
