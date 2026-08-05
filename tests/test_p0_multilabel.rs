// Multi-label P0 Bead Creation Tests
// Tests that P0 (critical priority) beads can be created with multiple labels
// and that all labels are correctly preserved through storage and retrieval

use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;

#[test]
fn test_p0_bead_with_multiple_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create a P0 bead with multiple labels
    let bead = Issue {
        id: "bf-p0-multilabel".to_string(),
        title: "P0 Bead with Multiple Labels".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL, // P0
        labels: vec![
            "critical".to_string(),
            "security".to_string(),
            "urgent".to_string(),
            "backend".to_string(),
        ],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Retrieve and verify all labels are preserved
    let retrieved = storage.get_issue("bf-p0-multilabel").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 4);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"security".to_string()));
    assert!(retrieved.labels.contains(&"urgent".to_string()));
    assert!(retrieved.labels.contains(&"backend".to_string()));
}

#[test]
fn test_p0_bead_with_single_label() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create a P0 bead with single label
    let bead = Issue {
        id: "bf-p0-single".to_string(),
        title: "P0 Bead with Single Label".to_string(),
        issue_type: IssueType::Bug,
        status: Status::Open,
        priority: Priority::CRITICAL, // P0
        labels: vec!["p0".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Retrieve and verify single label is preserved
    let retrieved = storage.get_issue("bf-p0-single").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 1);
    assert_eq!(retrieved.labels[0], "p0");
}

#[test]
fn test_p0_bead_with_no_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create a P0 bead with no labels
    let bead = Issue {
        id: "bf-p0-nolabels".to_string(),
        title: "P0 Bead with No Labels".to_string(),
        issue_type: IssueType::Feature,
        status: Status::Open,
        priority: Priority::CRITICAL, // P0
        labels: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Retrieve and verify no labels
    let retrieved = storage.get_issue("bf-p0-nolabels").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 0);
}

#[test]
fn test_p0_multilabel_serialization() {
    // Create a P0 bead with multiple labels
    let bead = Issue {
        id: "bf-p0-serialize".to_string(),
        title: "P0 Multi-label Serialization Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::InProgress,
        priority: Priority::CRITICAL, // P0
        labels: vec![
            "critical".to_string(),
            "frontend".to_string(),
            "performance".to_string(),
        ],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&bead).unwrap();

    // Verify P0 priority and all labels are serialized correctly
    assert!(json.contains(r#""priority":0"#)); // CRITICAL = 0
    assert!(json.contains(r#""labels":["critical","frontend","performance"]"#));

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.labels.len(), 3);
    assert!(deserialized.labels.contains(&"critical".to_string()));
    assert!(deserialized.labels.contains(&"frontend".to_string()));
    assert!(deserialized.labels.contains(&"performance".to_string()));
}

#[test]
fn test_p0_multilabel_with_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create a P0 bead with duplicate labels (should be stored as-is)
    let bead = Issue {
        id: "bf-p0-duplicates".to_string(),
        title: "P0 Bead with Duplicate Labels".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![
            "critical".to_string(),
            "urgent".to_string(),
            "critical".to_string(), // duplicate
            "urgent".to_string(),   // duplicate
        ],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Retrieve and verify duplicates are preserved
    let retrieved = storage.get_issue("bf-p0-duplicates").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 4); // all 4 stored including duplicates
}

#[test]
fn test_p0_multilabel_special_characters() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create a P0 bead with labels containing special characters
    let bead = Issue {
        id: "bf-p0-special".to_string(),
        title: "P0 Bead with Special Character Labels".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![
            "critical:security".to_string(),
            "urgent-fix".to_string(),
            "p0-blocker".to_string(),
            "team:backend".to_string(),
        ],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Retrieve and verify special characters are preserved
    let retrieved = storage.get_issue("bf-p0-special").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 4);
    assert!(retrieved.labels.contains(&"critical:security".to_string()));
    assert!(retrieved.labels.contains(&"urgent-fix".to_string()));
    assert!(retrieved.labels.contains(&"p0-blocker".to_string()));
    assert!(retrieved.labels.contains(&"team:backend".to_string()));
}

#[test]
fn test_p0_multilabel_json_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create a P0 bead with multiple labels
    let bead = Issue {
        id: "bf-p0-roundtrip".to_string(),
        title: "P0 Multi-label Roundtrip Test".to_string(),
        issue_type: IssueType::Bug,
        status: Status::Blocked,
        priority: Priority::CRITICAL,
        labels: vec![
            "critical".to_string(),
            "database".to_string(),
            "migration".to_string(),
        ],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Retrieve and serialize
    let retrieved = storage.get_issue("bf-p0-roundtrip").unwrap().unwrap();
    let json = serde_json::to_string(&retrieved).unwrap();

    // Deserialize
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    // Verify all fields preserved including P0 priority and all labels
    assert_eq!(deserialized.id, "bf-p0-roundtrip");
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.issue_type, IssueType::Bug);
    assert_eq!(deserialized.status, Status::Blocked);
    assert_eq!(deserialized.labels.len(), 3);
    assert!(deserialized.labels.contains(&"critical".to_string()));
    assert!(deserialized.labels.contains(&"database".to_string()));
    assert!(deserialized.labels.contains(&"migration".to_string()));
}

#[test]
fn test_multiple_p0_beads_different_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple P0 beads with different label combinations
    let beads = vec![
        ("bf-p0-1", vec!["critical".to_string(), "security".to_string()]),
        (
            "bf-p0-2",
            vec![
                "urgent".to_string(),
                "performance".to_string(),
                "frontend".to_string(),
            ],
        ),
        ("bf-p0-3", vec!["p0".to_string()]),
        (
            "bf-p0-4",
            vec![
                "critical".to_string(),
                "database".to_string(),
                "migration".to_string(),
                "downtime".to_string(),
            ],
        ),
    ];

    for (id, labels) in &beads {
        let bead = Issue {
            id: id.to_string(),
            title: format!("P0 Bead {}", id),
            issue_type: IssueType::Task,
            status: Status::Open,
            priority: Priority::CRITICAL,
            labels: labels.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };
        storage.create_issue(&bead).unwrap();
    }

    // Verify each P0 bead has its specific label combination
    for (id, expected_labels) in &beads {
        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.priority, Priority::CRITICAL);
        assert_eq!(retrieved.labels.len(), expected_labels.len());
        for label in expected_labels {
            assert!(retrieved.labels.contains(label));
        }
    }
}

#[test]
fn test_p0_multilabel_sync_equals() {
    let mut bead1 = Issue {
        id: "bf-p0-sync1".to_string(),
        title: "P0 Multi-label Sync Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![
            "critical".to_string(),
            "urgent".to_string(),
            "backend".to_string(),
        ],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    let mut bead2 = bead1.clone();

    // Modify some non-label fields
    bead2.created_at = Utc::now();
    bead2.updated_at = Utc::now();
    bead2.labels.reverse(); // different order

    // sync_equals should still return true (order-independent for labels)
    assert!(bead1.sync_equals(&bead2));
    assert!(bead2.sync_equals(&bead1));

    // Change a label and verify sync_equals detects the difference
    bead2.labels = vec!["critical".to_string(), "different".to_string()];
    assert!(!bead1.sync_equals(&bead2));
}

#[test]
fn test_p0_multilabel_all_priority_levels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create beads at all priority levels with multiple labels
    let priorities = vec![
        (Priority::CRITICAL, "bf-prio-critical"),
        (Priority::HIGH, "bf-prio-high"),
        (Priority::MEDIUM, "bf-prio-medium"),
        (Priority::LOW, "bf-prio-low"),
        (Priority::BACKLOG, "bf-prio-backlog"),
    ];

    for (priority, id) in &priorities {
        let bead = Issue {
            id: id.to_string(),
            title: format!("Bead at {:?}", priority),
            issue_type: IssueType::Task,
            status: Status::Open,
            priority: *priority,
            labels: vec![
                format!("{:?}", priority),
                "test-label".to_string(),
                "multi-label".to_string(),
            ],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };
        storage.create_issue(&bead).unwrap();
    }

    // Verify each bead has correct priority and all labels
    for (expected_priority, id) in &priorities {
        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.priority, *expected_priority);
        assert_eq!(retrieved.labels.len(), 3);
        assert!(retrieved.labels.contains(&"test-label".to_string()));
        assert!(retrieved.labels.contains(&"multi-label".to_string()));
    }

    // Specifically verify P0 (CRITICAL) bead
    let p0_bead = storage.get_issue("bf-prio-critical").unwrap().unwrap();
    assert_eq!(p0_bead.priority, Priority::CRITICAL);
    assert_eq!(p0_bead.priority.0, 0);
}
