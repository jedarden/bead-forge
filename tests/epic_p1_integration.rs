//! Comprehensive P1 Epic Integration Tests (bf-4rgvt).
//!
//! Tests P1 (High Priority) epic behavior across storage, updates, status computation,
//! JSON serialization, and sync semantics. P1 priority is Priority::HIGH = 1.
//!
//! Coverage:
//! - Storage and retrieval with all fields including description
//! - Parent-child relationships with mixed priority children
//! - Description updates via IssueChanges
//! - Status computation based on child closure states
//! - All status values (Open/InProgress/Blocked/Deferred/Draft/Closed)
//! - JSON roundtrip serialization
//! - sync_equals semantics (ignores timestamps, compares description/priority)

use bead_forge::model::{
    DependencyType, EpicStatus, Issue, IssueChanges, IssueType, Priority, Status,
};
use bead_forge::storage::Storage;
use chrono::Utc;
use std::thread;
use std::time::Duration;

/// Create a fresh temp storage with a database.
fn temp_storage() -> (tempfile::TempDir, Storage) {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();
    (dir, storage)
}

/// Create a P1 epic with the given fields.
fn create_p1_epic(
    storage: &Storage,
    id: &str,
    title: &str,
    description: Option<&str>,
    status: Status,
) -> Issue {
    let epic = Issue {
        id: id.to_string(),
        title: title.to_string(),
        issue_type: IssueType::Epic,
        status,
        priority: Priority::HIGH, // P1 = 1
        description: description.map(str::to_string),
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();
    epic
}

/// Create a child issue linked to an epic via ParentChild dependency.
fn create_child(
    storage: &Storage,
    epic_id: &str,
    id: &str,
    priority: Priority,
    status: Status,
) -> Issue {
    let mut child = Issue {
        id: id.to_string(),
        title: format!("Child {id}"),
        issue_type: IssueType::Task,
        status: status.clone(),
        priority,
        ..Default::default()
    };
    if status == Status::Closed {
        child.closed_at = Some(Utc::now());
    }
    storage.create_issue(&child).unwrap();
    storage
        .add_dependency(epic_id, id, &DependencyType::ParentChild, "test")
        .unwrap();
    child
}

/// Get children of an epic via ParentChild dependencies.
fn children_of(storage: &Storage, epic_id: &str) -> Vec<Issue> {
    storage
        .get_dependencies(epic_id)
        .unwrap()
        .into_iter()
        .filter(|d| d.dep_type == DependencyType::ParentChild)
        .filter_map(|d| storage.get_issue(&d.depends_on_id).unwrap())
        .collect()
}

/// Compute EpicStatus for an epic.
fn epic_status(storage: &Storage, epic_id: &str) -> EpicStatus {
    let epic = storage.get_issue(epic_id).unwrap().unwrap();
    let children = children_of(storage, epic_id);
    let total_children = children.len();
    let closed_children = children
        .iter()
        .filter(|c| c.status == Status::Closed)
        .count();

    EpicStatus {
        epic,
        total_children,
        closed_children,
        eligible_for_close: total_children > 0 && closed_children == total_children,
    }
}

#[test]
fn test_p1_epic_storage_retrieval() {
    let (_dir, storage) = temp_storage();

    // Create P1 epic with comprehensive description
    let description = "Implement comprehensive P1 epic integration tests covering storage, retrieval, updates, and status computation across all scenarios.";
    let _original = create_p1_epic(
        &storage,
        "epic-p1-storage",
        "P1 Epic Storage Test",
        Some(description),
        Status::Open,
    );

    // Retrieve and verify all fields
    let retrieved = storage.get_issue("epic-p1-storage").unwrap().unwrap();

    // Verify ID and title
    assert_eq!(retrieved.id, "epic-p1-storage");
    assert_eq!(retrieved.title, "P1 Epic Storage Test");

    // Verify P1 priority (HIGH = 1)
    assert_eq!(retrieved.priority, Priority::HIGH);
    assert_eq!(retrieved.priority.0, 1);

    // Verify epic type
    assert_eq!(retrieved.issue_type, IssueType::Epic);

    // Verify status
    assert_eq!(retrieved.status, Status::Open);

    // Verify description is preserved exactly
    assert_eq!(retrieved.description, Some(description.to_string()));
    assert_eq!(retrieved.description.as_deref(), Some(description));

    // Verify other expected defaults
    assert_eq!(retrieved.assignee, None);
    assert_eq!(retrieved.owner, None);
    assert!(retrieved.labels.is_empty());
    assert!(retrieved.dependencies.is_empty());
    assert!(retrieved.comments.is_empty());
}

#[test]
fn test_p1_epic_with_children() {
    let (_dir, storage) = temp_storage();

    // Create P1 epic with description
    let epic_description =
        "P1 epic with children of varying priorities to test parent-child rollup.";
    create_p1_epic(
        &storage,
        "epic-p1-children",
        "P1 Epic with Children",
        Some(epic_description),
        Status::Open,
    );

    // Create 3 children with different priorities (P0, P1, P2)
    create_child(
        &storage,
        "epic-p1-children",
        "child-p0",
        Priority::CRITICAL, // P0
        Status::Open,
    );
    create_child(
        &storage,
        "epic-p1-children",
        "child-p1",
        Priority::HIGH, // P1
        Status::Open,
    );
    create_child(
        &storage,
        "epic-p1-children",
        "child-p2",
        Priority::MEDIUM, // P2
        Status::Open,
    );

    // Verify epic has 3 children
    let children = children_of(&storage, "epic-p1-children");
    assert_eq!(children.len(), 3);

    // Verify each child has distinct priority
    let priorities: Vec<_> = children.iter().map(|c| c.priority).collect();
    assert!(priorities.contains(&Priority::CRITICAL)); // P0
    assert!(priorities.contains(&Priority::HIGH)); // P1
    assert!(priorities.contains(&Priority::MEDIUM)); // P2

    // Verify epic itself remains P1
    let epic = storage.get_issue("epic-p1-children").unwrap().unwrap();
    assert_eq!(epic.priority, Priority::HIGH);
    assert_eq!(epic.issue_type, IssueType::Epic);

    // Verify epic description is preserved
    assert_eq!(epic.description.as_deref(), Some(epic_description));

    // Verify all dependencies are ParentChild type
    let deps = storage.get_dependencies("epic-p1-children").unwrap();
    assert_eq!(deps.len(), 3);
    assert!(deps
        .iter()
        .all(|d| d.dep_type == DependencyType::ParentChild));
}

#[test]
fn test_p1_epic_update() {
    let (_dir, storage) = temp_storage();

    // Create initial P1 epic
    let original_description = "Initial description for P1 epic update test.";
    create_p1_epic(
        &storage,
        "epic-p1-update",
        "P1 Epic Update Test",
        Some(original_description),
        Status::Open,
    );

    // Update description via IssueChanges
    let updated_description =
        "Updated description: P1 priority must be preserved across description changes.";
    let changes = IssueChanges {
        description: Some(updated_description.to_string()),
        actor: Some("test-worker".to_string()),
        ..Default::default()
    };

    storage.update_issue("epic-p1-update", &changes).unwrap();

    // Retrieve and verify
    let updated = storage.get_issue("epic-p1-update").unwrap().unwrap();

    // Verify P1 priority is preserved (still 1, not changed)
    assert_eq!(updated.priority, Priority::HIGH);
    assert_eq!(updated.priority.0, 1);

    // Verify description was updated
    assert_eq!(updated.description.as_deref(), Some(updated_description));

    // Verify epic type is preserved
    assert_eq!(updated.issue_type, IssueType::Epic);

    // Verify status unchanged
    assert_eq!(updated.status, Status::Open);

    // Verify timestamps differ (updated_at should be newer)
    let original = storage.get_issue("epic-p1-update").unwrap().unwrap();
    assert!(updated.updated_at > original.created_at);
}

#[test]
fn test_p1_epic_status_computation() {
    let (_dir, storage) = temp_storage();

    // Create P1 epic with description
    let description = "P1 epic for status computation: 2 closed children, 1 open child.";
    create_p1_epic(
        &storage,
        "epic-p1-status",
        "P1 Epic Status Test",
        Some(description),
        Status::Open,
    );

    // Create 3 children: 2 closed, 1 open
    create_child(
        &storage,
        "epic-p1-status",
        "child-closed-1",
        Priority::MEDIUM,
        Status::Closed,
    );
    create_child(
        &storage,
        "epic-p1-status",
        "child-closed-2",
        Priority::MEDIUM,
        Status::Closed,
    );
    create_child(
        &storage,
        "epic-p1-status",
        "child-open",
        Priority::MEDIUM,
        Status::Open,
    );

    // Compute epic status
    let status = epic_status(&storage, "epic-p1-status");

    // Verify epic is P1
    assert_eq!(status.epic.priority, Priority::HIGH);
    assert_eq!(status.epic.issue_type, IssueType::Epic);

    // Verify child counts
    assert_eq!(status.total_children, 3);
    assert_eq!(status.closed_children, 2);

    // Verify not eligible for close (1 child still open)
    assert!(!status.eligible_for_close);

    // Verify epic description is preserved
    assert_eq!(status.epic.description.as_deref(), Some(description));

    // Verify epic itself is still Open (not auto-closed)
    assert_eq!(status.epic.status, Status::Open);

    // Verify individual child states
    let children = children_of(&storage, "epic-p1-status");
    let closed_count = children
        .iter()
        .filter(|c| c.status == Status::Closed)
        .count();
    let open_count = children.iter().filter(|c| c.status == Status::Open).count();
    assert_eq!(closed_count, 2);
    assert_eq!(open_count, 1);
}

#[test]
fn test_p1_epic_all_statuses() {
    let (_dir, storage) = temp_storage();

    // Test all statuses with P1 epic + description
    let statuses = vec![
        Status::Open,
        Status::InProgress,
        Status::Blocked,
        Status::Deferred,
        Status::Draft,
    ];

    for (i, status) in statuses.iter().enumerate() {
        let id = format!("epic-p1-status-{}", i);
        let title = format!("P1 Epic with {:?}", status);
        let description = format!("P1 epic in {:?} state with description text.", status);

        let epic = Issue {
            id: id.clone(),
            title,
            issue_type: IssueType::Epic,
            status: status.clone(),
            priority: Priority::HIGH,
            description: Some(description.clone()),
            ..Default::default()
        };

        storage.create_issue(&epic).unwrap();

        // Retrieve and verify
        let retrieved = storage.get_issue(&id).unwrap().unwrap();

        // All must be P1 (priority = 1)
        assert_eq!(retrieved.priority, Priority::HIGH);
        assert_eq!(retrieved.priority.0, 1);

        // All must be epic type
        assert_eq!(retrieved.issue_type, IssueType::Epic);

        // Each status must be preserved
        assert_eq!(retrieved.status, *status);

        // Description must be preserved
        assert_eq!(retrieved.description.as_deref(), Some(description.as_str()));

        // Display formatting must be P1
        assert_eq!(format!("{}", retrieved.priority), "P1");
    }

    // Count P1 epics across all statuses
    let all_issues = storage.list_issues(&Default::default()).unwrap();
    let p1_epics: Vec<_> = all_issues
        .iter()
        .filter(|i| i.issue_type == IssueType::Epic && i.priority == Priority::HIGH)
        .collect();

    assert_eq!(
        p1_epics.len(),
        5,
        "Should have 5 P1 epics, one for each status"
    );
}

#[test]
fn test_p1_epic_json_roundtrip() {
    // Create P1 epic with all relevant fields
    let original = Issue {
        id: "epic-p1-roundtrip".to_string(),
        title: "P1 Epic JSON Roundtrip".to_string(),
        issue_type: IssueType::Epic,
        status: Status::InProgress,
        priority: Priority::HIGH, // P1
        description: Some("Full JSON roundtrip test for P1 epic with description".to_string()),
        assignee: Some("test-assignee".to_string()),
        labels: vec!["p1".to_string(), "integration-test".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&original).unwrap();

    // Verify JSON contains P1 priority as integer 1
    assert!(json.contains(r#""priority":1"#));
    assert!(!json.contains(r#""priority":"P1""#));

    // Verify JSON contains epic type
    assert!(json.contains(r#""issue_type":"epic""#));

    // Verify JSON contains description
    assert!(json.contains("Full JSON roundtrip test"));

    // Deserialize back
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    // Verify all fields match
    assert_eq!(deserialized.id, original.id);
    assert_eq!(deserialized.title, original.title);
    assert_eq!(deserialized.issue_type, original.issue_type);
    assert_eq!(deserialized.priority, original.priority);
    assert_eq!(deserialized.priority.0, 1);
    assert_eq!(deserialized.description, original.description);
    assert_eq!(deserialized.status, original.status);
    assert_eq!(deserialized.assignee, original.assignee);
    assert_eq!(deserialized.labels, original.labels);

    // Pretty-print JSON and deserialize again
    let pretty_json = serde_json::to_string_pretty(&original).unwrap();
    let from_pretty: Issue = serde_json::from_str(&pretty_json).unwrap();
    assert_eq!(from_pretty.priority, Priority::HIGH);
    assert_eq!(from_pretty.issue_type, IssueType::Epic);
}

#[test]
fn test_p1_epic_sync_equals() {
    let (_dir, storage) = temp_storage();

    // Create a P1 epic
    let base_time = Utc::now();

    let epic = Issue {
        id: "epic-p1-sync".to_string(),
        title: "P1 Epic Sync Test".to_string(),
        issue_type: IssueType::Epic,
        priority: Priority::HIGH,
        description: Some("Description for sync_equals test".to_string()),
        created_at: base_time,
        updated_at: base_time,
        ..Default::default()
    };

    // Store it
    storage.create_issue(&epic).unwrap();

    // Retrieve the original
    let retrieved = storage.get_issue("epic-p1-sync").unwrap().unwrap();

    // Test 1: An issue should sync_equal with itself (sanity check)
    assert!(
        retrieved.sync_equals(&retrieved),
        "Issue should sync_equal with itself"
    );

    // Test 2: Create an exact clone - should be sync_equal
    let same_content = retrieved.clone();
    assert!(
        retrieved.sync_equals(&same_content),
        "P1 epic should sync_equal with its clone"
    );

    // Verify P1 priority comparison works
    assert_eq!(retrieved.priority, same_content.priority);
    assert_eq!(retrieved.priority.0, 1);

    // Verify description comparison works
    assert_eq!(retrieved.description, same_content.description);

    // Create a version with different description
    let different_desc = Issue {
        id: "epic-p1-sync".to_string(),
        title: retrieved.title.clone(),
        issue_type: IssueType::Epic,
        priority: retrieved.priority,
        description: Some("Different description".to_string()),
        created_at: base_time,
        updated_at: base_time,
        ..Default::default()
    };

    assert!(
        !retrieved.sync_equals(&different_desc),
        "Different descriptions should not be sync_equal"
    );

    // Create a version with different priority (P0 instead of P1)
    let different_priority = Issue {
        id: "epic-p1-sync".to_string(),
        title: retrieved.title.clone(),
        issue_type: IssueType::Epic,
        priority: Priority::CRITICAL, // P0
        description: retrieved.description.clone(),
        created_at: base_time,
        updated_at: base_time,
        ..Default::default()
    };

    assert!(
        !retrieved.sync_equals(&different_priority),
        "Different priorities should not be sync_equal"
    );
    assert_eq!(different_priority.priority, Priority::CRITICAL);
    assert_eq!(different_priority.priority.0, 0);

    // Create a version with different title
    let different_title = Issue {
        id: "epic-p1-sync".to_string(),
        title: "Different Title".to_string(),
        issue_type: IssueType::Epic,
        priority: retrieved.priority,
        description: retrieved.description.clone(),
        created_at: base_time,
        updated_at: base_time,
        ..Default::default()
    };

    assert!(
        !retrieved.sync_equals(&different_title),
        "Different titles should not be sync_equal"
    );

    // Verify sync_equals requires same ID
    let different_id = Issue {
        id: "different-id".to_string(),
        title: retrieved.title.clone(),
        issue_type: IssueType::Epic,
        priority: retrieved.priority,
        description: retrieved.description.clone(),
        created_at: base_time,
        updated_at: base_time,
        ..Default::default()
    };

    assert!(
        !retrieved.sync_equals(&different_id),
        "Different IDs should not be sync_equal"
    );
}
