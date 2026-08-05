// Comprehensive test for P0 beads with multiple labels
// This test verifies that beads with P0 (critical) priority can have multiple labels
// and that those labels are correctly stored, retrieved, and serialized.

use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;

#[test]
fn test_p0_bead_with_multiple_labels_creation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead with multiple labels
    let bead = Issue {
        id: "bf-p0-multi".to_string(),
        title: "P0 Bead with Multiple Labels".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL, // P0
        labels: vec![
            "critical".to_string(),
            "urgent".to_string(),
            "security".to_string(),
        ],
        description: Some("Testing P0 bead with multiple labels".to_string()),
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Verify storage and retrieval
    let retrieved = storage.get_issue("bf-p0-multi").unwrap().unwrap();
    
    // Test P0 priority
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    
    // Test multiple labels
    assert_eq!(retrieved.labels.len(), 3);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"urgent".to_string()));
    assert!(retrieved.labels.contains(&"security".to_string()));
}

#[test]
fn test_p0_bead_multiple_labels_serialization() {
    // Create P0 bead with multiple labels
    let bead = Issue {
        id: "bf-p0-serialize".to_string(),
        title: "P0 Serialization Test".to_string(),
        issue_type: IssueType::Bug,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["p0".to_string(), "critical".to_string(), "hotfix".to_string()],
        ..Default::default()
    };

    // Test JSON serialization
    let json = serde_json::to_string(&bead).unwrap();
    
    // Verify P0 priority is serialized as 0
    assert!(json.contains("\"priority\":0"));
    
    // Verify all labels are in the JSON
    assert!(json.contains("p0"));
    assert!(json.contains("critical"));
    assert!(json.contains("hotfix"));

    // Test deserialization
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.labels.len(), 3);
}

#[test]
fn test_p0_bead_label_operations() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead with initial labels
    let bead = Issue {
        id: "bf-p0-ops".to_string(),
        title: "P0 Label Operations".to_string(),
        issue_type: IssueType::Feature,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string()],
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Add more labels
    storage.add_label("bf-p0-ops", "urgent").unwrap();
    storage.add_label("bf-p0-ops", "security").unwrap();
    storage.add_label("bf-p0-ops", "performance").unwrap();

    // Verify all labels
    let retrieved = storage.get_issue("bf-p0-ops").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 4);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
}

#[test]
fn test_p0_bead_multiple_labels_filtering() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple P0 beads with different labels
    for i in 1..=3 {
        let bead = Issue {
            id: format!("bf-p0-filter-{}", i),
            title: format!("P0 Filter Test {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            priority: Priority::CRITICAL,
            labels: vec![
                "critical".to_string(),
                format!("label-{}", i),
            ],
            ..Default::default()
        };
        storage.create_issue(&bead).unwrap();
    }

    // Filter by P0 priority
    let filter = bead_forge::model::IssueFilter {
        priority: Some(0),
        ..Default::default()
    };
    let p0_beads = storage.list_issues(&filter).unwrap();
    
    // Should have 3 P0 beads
    assert_eq!(p0_beads.len(), 3);
    
    // All should have "critical" label
    for bead in p0_beads {
        assert_eq!(bead.priority, Priority::CRITICAL);
        assert!(bead.labels.contains(&"critical".to_string()));
    }
}

#[test]
fn test_p0_bead_label_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead with multiple labels
    let bead = Issue {
        id: "bf-p0-persist".to_string(),
        title: "P0 Label Persistence".to_string(),
        issue_type: IssueType::Task,
        status: Status::InProgress,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "wip".to_string()],
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Simulate database close and reopen
    drop(storage);
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Verify labels persist
    let retrieved = storage.get_issue("bf-p0-persist").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 2);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"wip".to_string()));
}

#[test]
fn test_p0_bead_with_various_label_counts() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Test with varying numbers of labels
    let label_counts = vec![1, 3, 5, 10];
    
    for (i, count) in label_counts.iter().enumerate() {
        let labels: Vec<String> = (0..*count)
            .map(|j| format!("label-{}", j))
            .collect();
        
        let bead = Issue {
            id: format!("bf-p0-var-{}", i),
            title: format!("P0 with {} labels", count),
            issue_type: IssueType::Task,
            status: Status::Open,
            priority: Priority::CRITICAL,
            labels: labels.clone(),
            ..Default::default()
        };

        storage.create_issue(&bead).unwrap();

        let retrieved = storage.get_issue(&format!("bf-p0-var-{}", i)).unwrap().unwrap();
        assert_eq!(retrieved.labels.len(), *count);
        assert_eq!(retrieved.priority, Priority::CRITICAL);
    }
}

#[test]
fn test_p0_priority_multiple_labels_integration() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic with child beads, all with labels
    let epic = Issue {
        id: "bf-p0-epic".to_string(),
        title: "P0 Epic with Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["epic".to_string(), "critical".to_string()],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Create child beads
    for i in 1..=2 {
        let child = Issue {
            id: format!("bf-p0-child-{}", i),
            title: format!("P0 Child {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            priority: Priority::CRITICAL,
            labels: vec!["child".to_string(), format!("group-{}", i)],
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();
        
        // Add dependency
        storage.add_dependency(
            &format!("bf-p0-child-{}", i),
            "bf-p0-epic",
            &bead_forge::model::DependencyType::ParentChild,
            "test",
        ).unwrap();
    }

    // Verify all have P0 priority and labels
    let p0_items = storage.list_issues(&bead_forge::model::IssueFilter {
        priority: Some(0),
        ..Default::default()
    }).unwrap();

    assert_eq!(p0_items.len(), 3);
    for item in p0_items {
        assert_eq!(item.priority, Priority::CRITICAL);
        assert!(!item.labels.is_empty());
    }
}
