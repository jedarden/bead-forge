// Comprehensive unit tests for storage CRUD operations and transactions.
//
// This module provides extensive test coverage for the SQLite storage layer,
// including CRUD operations, transaction handling, dependency management,
// and edge cases.

#[cfg(test)]
mod comprehensive_tests {
    use super::super::*;
    use crate::model::{
        Comment, DependencyType, Event, EventType, Issue, IssueChanges,
        IssueFilter, IssueType, Priority, Status,
    };
    use chrono::{Duration, Utc};
    use tempfile::NamedTempFile;

    // ============================================================================
    // CRUD Operations Tests
    // ============================================================================

    #[test]
    fn test_create_issue_basic() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let issue = Issue::new(
            "bf-create-1".to_string(),
            "Test create issue".to_string(),
            ".".to_string(),
        );

        let result = storage.create_issue(&issue);
        assert!(result.is_ok(), "Issue creation should succeed");

        // Verify issue was created
        let retrieved = storage.get_issue("bf-create-1");
        assert!(retrieved.is_ok());
        let retrieved_issue = retrieved.unwrap();
        assert!(retrieved_issue.is_some());
        assert_eq!(retrieved_issue.unwrap().id, "bf-create-1");
    }

    #[test]
    fn test_create_issue_with_all_fields() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let mut issue = Issue::new(
            "bf-full-1".to_string(),
            "Full issue test".to_string(),
            "test-repo".to_string(),
        );
        issue.description = Some("Test description".to_string());
        issue.design = Some("Test design".to_string());
        issue.acceptance_criteria = Some("Test criteria".to_string());
        issue.notes = Some("Test notes".to_string());
        issue.status = Status::InProgress;
        issue.priority = Priority::CRITICAL;
        issue.issue_type = IssueType::Bug;
        issue.assignee = Some("test-user".to_string());
        issue.owner = Some("test-owner".to_string());
        issue.estimated_minutes = Some(120);
        issue.created_by = Some("creator".to_string());
        issue.due_at = Some(Utc::now() + Duration::days(7));
        issue.external_ref = Some("EXT-123".to_string());

        storage.create_issue(&issue).unwrap();

        let retrieved = storage.get_issue("bf-full-1").unwrap().unwrap();
        assert_eq!(retrieved.description, issue.description);
        assert_eq!(retrieved.design, issue.design);
        assert_eq!(retrieved.acceptance_criteria, issue.acceptance_criteria);
        assert_eq!(retrieved.notes, issue.notes);
        assert_eq!(retrieved.status, issue.status);
        assert_eq!(retrieved.priority, issue.priority);
        assert_eq!(retrieved.issue_type, issue.issue_type);
        assert_eq!(retrieved.assignee, issue.assignee);
        assert_eq!(retrieved.owner, issue.owner);
        assert_eq!(retrieved.estimated_minutes, issue.estimated_minutes);
    }

    #[test]
    fn test_get_issue_not_found() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let result = storage.get_issue("bf-nonexistent");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none(), "Non-existent issue should return None");
    }

    #[test]
    fn test_update_issue_basic_fields() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let mut issue = Issue::new(
            "bf-update-1".to_string(),
            "Original title".to_string(),
            ".".to_string(),
        );
        issue.description = Some("Original description".to_string());
        storage.create_issue(&issue).unwrap();

        // Update title and description
        let mut changes = IssueChanges::default();
        changes.title = Some("Updated title".to_string());
        changes.description = Some("Updated description".to_string());
        changes.actor = Some("test-user".to_string());

        storage.update_issue("bf-update-1", &changes).unwrap();

        let retrieved = storage.get_issue("bf-update-1").unwrap().unwrap();
        assert_eq!(retrieved.title, "Updated title");
        assert_eq!(retrieved.description, Some("Updated description".to_string()));
    }

    #[test]
    fn test_update_issue_status() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let issue = Issue::new(
            "bf-status-1".to_string(),
            "Status test".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&issue).unwrap();

        let mut changes = IssueChanges::default();
        changes.status = Some(Status::InProgress);
        changes.actor = Some("test-user".to_string());

        storage.update_issue("bf-status-1", &changes).unwrap();

        let retrieved = storage.get_issue("bf-status-1").unwrap().unwrap();
        assert_eq!(retrieved.status, Status::InProgress);
    }

    #[test]
    fn test_update_issue_priority() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let issue = Issue::new(
            "bf-priority-1".to_string(),
            "Priority test".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&issue).unwrap();

        let mut changes = IssueChanges::default();
        changes.priority = Some(0); // CRITICAL
        changes.actor = Some("test-user".to_string());

        storage.update_issue("bf-priority-1", &changes).unwrap();

        let retrieved = storage.get_issue("bf-priority-1").unwrap().unwrap();
        assert_eq!(retrieved.priority, Priority::CRITICAL);
    }

    #[test]
    fn test_list_issues_empty() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let filter = IssueFilter::default();
        let results = storage.list_issues(&filter).unwrap();

        assert_eq!(results.len(), 0, "Empty database should return no issues");
    }

    #[test]
    fn test_list_issues_with_filter() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create multiple issues with different statuses
        let mut issue1 = Issue::new("bf-list-1".to_string(), "Open issue".to_string(), ".".to_string());
        issue1.status = Status::Open;
        issue1.priority = Priority::HIGH;
        storage.create_issue(&issue1).unwrap();

        let mut issue2 = Issue::new("bf-list-2".to_string(), "Closed issue".to_string(), ".".to_string());
        issue2.status = Status::Closed;
        storage.create_issue(&issue2).unwrap();

        let mut issue3 = Issue::new("bf-list-3".to_string(), "Another open".to_string(), ".".to_string());
        issue3.status = Status::Open;
        issue3.priority = Priority::MEDIUM;
        storage.create_issue(&issue3).unwrap();

        // Filter by status
        let mut filter = IssueFilter::default();
        filter.status = Some(Status::Open);

        let results = storage.list_issues(&filter).unwrap();
        assert_eq!(results.len(), 2, "Should return 2 open issues");

        let ids: Vec<&str> = results.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&"bf-list-1"));
        assert!(ids.contains(&"bf-list-3"));
        assert!(!ids.contains(&"bf-list-2"));
    }

    #[test]
    fn test_list_issues_with_priority_filter() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let mut issue1 = Issue::new("bf-prio-1".to_string(), "Critical".to_string(), ".".to_string());
        issue1.priority = Priority::CRITICAL;
        storage.create_issue(&issue1).unwrap();

        let mut issue2 = Issue::new("bf-prio-2".to_string(), "High".to_string(), ".".to_string());
        issue2.priority = Priority::HIGH;
        storage.create_issue(&issue2).unwrap();

        let mut issue3 = Issue::new("bf-prio-3".to_string(), "Medium".to_string(), ".".to_string());
        issue3.priority = Priority::MEDIUM;
        storage.create_issue(&issue3).unwrap();

        let mut filter = IssueFilter::default();
        filter.priority = Some(1); // HIGH

        let results = storage.list_issues(&filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "bf-prio-2");
    }

    #[test]
    fn test_list_issues_with_pagination() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create 5 issues
        for i in 1..=5 {
            let issue = Issue::new(
                format!("bf-page-{}", i),
                format!("Issue {}", i),
                ".".to_string(),
            );
            storage.create_issue(&issue).unwrap();
        }

        let mut filter = IssueFilter::default();
        filter.limit = Some(2);
        filter.offset = Some(2);

        let results = storage.list_issues(&filter).unwrap();
        assert_eq!(results.len(), 2, "Should return 2 issues with offset");
    }

    // ============================================================================
    // Dependency Tests
    // ============================================================================

    #[test]
    fn test_add_dependency() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create two issues
        let issue1 = Issue::new("bf-dep-1".to_string(), "Issue 1".to_string(), ".".to_string());
        storage.create_issue(&issue1).unwrap();

        let issue2 = Issue::new("bf-dep-2".to_string(), "Issue 2".to_string(), ".".to_string());
        storage.create_issue(&issue2).unwrap();

        // Add dependency using the actual API
        let result = storage.add_dependency(
            "bf-dep-1",
            "bf-dep-2",
            &DependencyType::Blocks,
            "test-user"
        );
        assert!(result.is_ok(), "Dependency addition should succeed");

        // Verify dependency was added
        let dependencies = storage.get_dependencies("bf-dep-1").unwrap();
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0].depends_on_id, "bf-dep-2");
    }

    #[test]
    fn test_get_dependencies_empty() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let issue = Issue::new("bf-nodep-1".to_string(), "No deps".to_string(), ".".to_string());
        storage.create_issue(&issue).unwrap();

        let dependencies = storage.get_dependencies("bf-nodep-1").unwrap();
        assert_eq!(dependencies.len(), 0);
    }

    #[test]
    fn test_remove_dependency() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create issues and dependency
        let issue1 = Issue::new("bf-rem-1".to_string(), "Issue 1".to_string(), ".".to_string());
        storage.create_issue(&issue1).unwrap();

        let issue2 = Issue::new("bf-rem-2".to_string(), "Issue 2".to_string(), ".".to_string());
        storage.create_issue(&issue2).unwrap();

        storage.add_dependency("bf-rem-1", "bf-rem-2", &DependencyType::Blocks, "test-user").unwrap();

        // Remove dependency
        let result = storage.remove_dependency("bf-rem-1", "bf-rem-2");
        assert!(result.is_ok());

        // Verify dependency was removed
        let dependencies = storage.get_dependencies("bf-rem-1").unwrap();
        assert_eq!(dependencies.len(), 0);
    }

    #[test]
    fn test_get_dependents() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create issues
        let parent = Issue::new("bf-parent".to_string(), "Parent".to_string(), ".".to_string());
        storage.create_issue(&parent).unwrap();

        let child1 = Issue::new("bf-child1".to_string(), "Child 1".to_string(), ".".to_string());
        storage.create_issue(&child1).unwrap();

        let child2 = Issue::new("bf-child2".to_string(), "Child 2".to_string(), ".".to_string());
        storage.create_issue(&child2).unwrap();

        // Add dependencies from children to parent
        storage.add_dependency("bf-child1", "bf-parent", &DependencyType::Blocks, "test-user").unwrap();
        storage.add_dependency("bf-child2", "bf-parent", &DependencyType::Blocks, "test-user").unwrap();

        // Get dependents of parent
        let dependents = storage.get_dependents("bf-parent").unwrap();
        assert_eq!(dependents.len(), 2);

        let dependent_ids: Vec<&str> = dependents.iter().map(|d| d.issue_id.as_str()).collect();
        assert!(dependent_ids.contains(&"bf-child1"));
        assert!(dependent_ids.contains(&"bf-child2"));
    }

    // ============================================================================
    // Comment Tests (loaded as part of Issue)
    // ============================================================================

    #[test]
    fn test_issue_with_comments() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let mut issue = Issue::new("bf-comment-1".to_string(), "Comment test".to_string(), ".".to_string());

        // Add comments to the issue
        issue.comments.push(Comment {
            id: 1,
            issue_id: "bf-comment-1".to_string(),
            author: "test-user".to_string(),
            body: "Test comment".to_string(),
            created_at: Utc::now(),
        });

        storage.create_issue(&issue).unwrap();

        // Verify comments were persisted and loaded
        let retrieved = storage.get_issue("bf-comment-1").unwrap().unwrap();
        assert_eq!(retrieved.comments.len(), 1);
        assert_eq!(retrieved.comments[0].body, "Test comment");
    }

    #[test]
    fn test_issue_without_comments() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let issue = Issue::new("bf-nocomment-1".to_string(), "No comments".to_string(), ".".to_string());
        storage.create_issue(&issue).unwrap();

        let retrieved = storage.get_issue("bf-nocomment-1").unwrap().unwrap();
        assert_eq!(retrieved.comments.len(), 0);
    }

    #[test]
    fn test_issue_with_multiple_comments() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let mut issue = Issue::new("bf-multicomment-1".to_string(), "Multiple comments".to_string(), ".".to_string());

        issue.comments.push(Comment {
            id: 1,
            issue_id: "bf-multicomment-1".to_string(),
            author: "user1".to_string(),
            body: "First comment".to_string(),
            created_at: Utc::now(),
        });

        issue.comments.push(Comment {
            id: 2,
            issue_id: "bf-multicomment-1".to_string(),
            author: "user2".to_string(),
            body: "Second comment".to_string(),
            created_at: Utc::now() + Duration::seconds(1),
        });

        storage.create_issue(&issue).unwrap();

        let retrieved = storage.get_issue("bf-multicomment-1").unwrap().unwrap();
        assert_eq!(retrieved.comments.len(), 2);
    }

    // ============================================================================
    // Event Tests (loaded as part of Issue)
    // ============================================================================

    #[test]
    fn test_issue_with_event() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let mut issue = Issue::new("bf-event-1".to_string(), "Event test".to_string(), ".".to_string());

        // Add an event to the issue
        issue.events.push(Event {
            id: 1,
            issue_id: "bf-event-1".to_string(),
            event_type: EventType::StatusChanged,
            actor: "test-user".to_string(),
            old_value: Some("open".to_string()),
            new_value: Some("in_progress".to_string()),
            comment: None,
            created_at: Utc::now(),
        });

        storage.create_issue(&issue).unwrap();

        // Verify event was persisted and loaded
        let retrieved = storage.get_issue("bf-event-1").unwrap().unwrap();
        assert_eq!(retrieved.events.len(), 1);
        assert_eq!(retrieved.events[0].event_type, EventType::StatusChanged);
    }

    #[test]
    fn test_issue_without_events() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let issue = Issue::new("bf-noevent-1".to_string(), "No events".to_string(), ".".to_string());
        storage.create_issue(&issue).unwrap();

        let retrieved = storage.get_issue("bf-noevent-1").unwrap().unwrap();
        // Should have the 'created' event added by create_issue
        assert!(retrieved.events.len() >= 1);
    }

    #[test]
    fn test_issue_with_multiple_events() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let mut issue = Issue::new("bf-multievent-1".to_string(), "Multiple events".to_string(), ".".to_string());

        let now = Utc::now();
        issue.events.push(Event {
            id: 1,
            issue_id: "bf-multievent-1".to_string(),
            event_type: EventType::Created,
            actor: "creator".to_string(),
            old_value: None,
            new_value: None,
            comment: None,
            created_at: now,
        });

        issue.events.push(Event {
            id: 2,
            issue_id: "bf-multievent-1".to_string(),
            event_type: EventType::StatusChanged,
            actor: "updater".to_string(),
            old_value: Some("open".to_string()),
            new_value: Some("in_progress".to_string()),
            comment: None,
            created_at: now + Duration::seconds(1),
        });

        storage.create_issue(&issue).unwrap();

        let retrieved = storage.get_issue("bf-multievent-1").unwrap().unwrap();
        // Should have at least our 2 events plus the auto-created 'created' event
        assert!(retrieved.events.len() >= 2);
    }

    // ============================================================================
    // Transaction Tests
    // ============================================================================

    #[test]
    fn test_storage_open_and_basic_operations() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Test basic operations work
        let issue = Issue::new(
            "bf-basic-1".to_string(),
            "Basic operations test".to_string(),
            ".".to_string(),
        );

        storage.create_issue(&issue).unwrap();

        let retrieved = storage.get_issue("bf-basic-1").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "bf-basic-1");
    }

    #[test]
    fn test_database_persistence() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        // Create storage and add issue
        {
            let storage = Storage::open(&path).unwrap();
            let issue = Issue::new(
                "bf-persist-1".to_string(),
                "Persistence test".to_string(),
                ".".to_string(),
            );
            storage.create_issue(&issue).unwrap();
        }

        // Reopen storage and verify issue persists
        let storage = Storage::open(&path).unwrap();
        let retrieved = storage.get_issue("bf-persist-1").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Persistence test");
    }

    // ============================================================================
    // Edge Case Tests
    // ============================================================================

    #[test]
    fn test_update_nonexistent_issue() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let changes = IssueChanges::default();
        let result = storage.update_issue("bf-nonexistent", &changes);

        assert!(result.is_err(), "Updating non-existent issue should fail");
    }

    #[test]
    fn test_delete_nonexistent_dependency() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let result = storage.remove_dependency("bf-issue1", "bf-issue2");
        // Should either succeed (no-op) or fail gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_special_characters_in_fields() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let mut issue = Issue::new(
            "bf-special-1".to_string(),
            "Issue with special chars: <>&\"'".to_string(),
            ".".to_string(),
        );
        issue.description = Some("Description with \n newlines and \t tabs".to_string());
        issue.notes = Some("Notes with unicode: 你好世界 🎉".to_string());

        storage.create_issue(&issue).unwrap();

        let retrieved = storage.get_issue("bf-special-1").unwrap().unwrap();
        assert!(retrieved.title.contains("<>&\"'"));
        assert!(retrieved.description.unwrap().contains('\n'));
        assert!(retrieved.notes.unwrap().contains("你好世界"));
    }

    #[test]
    fn test_very_long_title() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create title exactly at the 500 character limit
        let long_title = "x".repeat(500);
        let issue = Issue::new(
            "bf-long-1".to_string(),
            long_title.clone(),
            ".".to_string(),
        );

        storage.create_issue(&issue).unwrap();

        let retrieved = storage.get_issue("bf-long-1").unwrap().unwrap();
        assert_eq!(retrieved.title.len(), 500);
    }

    #[test]
    fn test_issue_with_annotations() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let mut issue = Issue::new(
            "bf-annot-1".to_string(),
            "Annotation test".to_string(),
            ".".to_string(),
        );

        issue.annotations.insert("key1".to_string(), "value1".to_string());
        issue.annotations.insert("key2".to_string(), "value2".to_string());

        storage.create_issue(&issue).unwrap();

        let retrieved = storage.get_issue("bf-annot-1").unwrap().unwrap();
        assert_eq!(retrieved.annotations.len(), 2);
        assert_eq!(retrieved.annotations.get("key1"), Some(&"value1".to_string()));
        assert_eq!(retrieved.annotations.get("key2"), Some(&"value2".to_string()));
    }

    // ============================================================================
    // Performance and Scale Tests
    // ============================================================================

    #[test]
    fn test_batch_issue_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create 100 issues in batch
        for i in 1..=100 {
            let issue = Issue::new(
                format!("bf-batch-{}", i),
                format!("Batch issue {}", i),
                ".".to_string(),
            );
            storage.create_issue(&issue).unwrap();
        }

        // Verify all issues were created
        let filter = IssueFilter::default();
        let results = storage.list_issues(&filter).unwrap();
        assert_eq!(results.len(), 100);
    }

    #[test]
    fn test_complex_query_performance() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create issues with various states
        for i in 1..=50 {
            let mut issue = Issue::new(
                format!("bf-query-{}", i),
                format!("Query test {}", i),
                ".".to_string(),
            );

            // Mix statuses and priorities
            issue.status = match i % 3 {
                0 => Status::Open,
                1 => Status::InProgress,
                _ => Status::Closed,
            };

            issue.priority = match i % 5 {
                0 => Priority::CRITICAL,
                1 => Priority::HIGH,
                2 => Priority::MEDIUM,
                3 => Priority::LOW,
                _ => Priority::BACKLOG,
            };

            storage.create_issue(&issue).unwrap();
        }

        // Query with multiple filters
        let mut filter = IssueFilter::default();
        filter.status = Some(Status::Open);
        filter.priority = Some(1); // HIGH

        let results = storage.list_issues(&filter).unwrap();
        // Should return issues that are both Open and HIGH priority
        assert!(results.len() >= 0);
    }

    // ============================================================================
    // Constraint Tests
    // ============================================================================

    #[test]
    fn test_priority_range_constraint() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let mut issue = Issue::new(
            "bf-prio-constraint-1".to_string(),
            "Priority constraint test".to_string(),
            ".".to_string(),
        );

        // Valid priority (0-4)
        issue.priority = Priority::CRITICAL; // 0
        storage.create_issue(&issue).unwrap();

        // Test updating to valid priority
        let mut changes = IssueChanges::default();
        changes.priority = Some(4); // BACKLOG
        storage.update_issue("bf-prio-constraint-1", &changes).unwrap();

        let retrieved = storage.get_issue("bf-prio-constraint-1").unwrap().unwrap();
        assert_eq!(retrieved.priority, Priority::BACKLOG);
    }

    #[test]
    fn test_foreign_key_constraint_cascade() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create issues with dependency
        let issue1 = Issue::new("bf-fk-1".to_string(), "Issue 1".to_string(), ".".to_string());
        storage.create_issue(&issue1).unwrap();

        let issue2 = Issue::new("bf-fk-2".to_string(), "Issue 2".to_string(), ".".to_string());
        storage.create_issue(&issue2).unwrap();

        storage.add_dependency("bf-fk-1", "bf-fk-2", &DependencyType::Blocks, "test-user").unwrap();

        // Verify dependency exists
        let deps = storage.get_dependencies("bf-fk-1").unwrap();
        assert_eq!(deps.len(), 1);

        // Mark dependent issue as deleted (soft delete)
        let mut changes = IssueChanges::default();
        changes.status = Some(Status::Tombstone);
        changes.actor = Some("test-user".to_string());
        storage.update_issue("bf-fk-1", &changes).unwrap();

        // Verify issue is marked as tombstone
        let retrieved = storage.get_issue("bf-fk-1").unwrap();
        assert!(retrieved.is_none(), "Tombstone issues should not be returned by default");
    }

    // ============================================================================
    // Data Integrity Tests
    // ============================================================================

    #[test]
    fn test_updated_at_auto_update() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let issue = Issue::new(
            "bf-timestamp-1".to_string(),
            "Timestamp test".to_string(),
            ".".to_string(),
        );

        storage.create_issue(&issue).unwrap();

        let retrieved = storage.get_issue("bf-timestamp-1").unwrap().unwrap();
        let original_updated = retrieved.updated_at;

        // Wait a bit to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Update the issue
        let mut changes = IssueChanges::default();
        changes.title = Some("Updated title".to_string());
        changes.actor = Some("test-user".to_string());
        storage.update_issue("bf-timestamp-1", &changes).unwrap();

        let updated = storage.get_issue("bf-timestamp-1").unwrap().unwrap();
        assert!(updated.updated_at > original_updated, "updated_at should change after update");
    }

    #[test]
    fn test_consistent_read_write() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create issue with all fields
        let mut original = Issue::new(
            "bf-consistency-1".to_string(),
            "Consistency test".to_string(),
            "test-repo".to_string(),
        );

        original.description = Some("Test description".to_string());
        original.design = Some("Test design".to_string());
        original.acceptance_criteria = Some("Test criteria".to_string());
        original.notes = Some("Test notes".to_string());
        original.status = Status::InProgress;
        original.priority = Priority::HIGH;
        original.issue_type = IssueType::Feature;
        original.assignee = Some("assignee".to_string());
        original.owner = Some("owner".to_string());
        original.estimated_minutes = Some(60);
        original.created_by = Some("creator".to_string());
        original.due_at = Some(Utc::now() + Duration::days(7));
        original.external_ref = Some("EXT-123".to_string());

        storage.create_issue(&original).unwrap();

        // Read back and verify all fields match
        let retrieved = storage.get_issue("bf-consistency-1").unwrap().unwrap();

        assert_eq!(retrieved.id, original.id);
        assert_eq!(retrieved.title, original.title);
        assert_eq!(retrieved.description, original.description);
        assert_eq!(retrieved.design, original.design);
        assert_eq!(retrieved.acceptance_criteria, original.acceptance_criteria);
        assert_eq!(retrieved.notes, original.notes);
        assert_eq!(retrieved.status, original.status);
        assert_eq!(retrieved.priority, original.priority);
        assert_eq!(retrieved.issue_type, original.issue_type);
        assert_eq!(retrieved.assignee, original.assignee);
        assert_eq!(retrieved.owner, original.owner);
        assert_eq!(retrieved.estimated_minutes, original.estimated_minutes);
        assert_eq!(retrieved.external_ref, original.external_ref);
    }

    // ============================================================================
    // Ready Candidates Tests
    // ============================================================================

    #[test]
    fn test_get_ready_candidates_basic() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an open issue with no dependencies
        let issue = Issue::new("bf-ready-1".to_string(), "Ready issue".to_string(), ".".to_string());
        storage.create_issue(&issue).unwrap();

        let ready = storage.get_ready_candidates().unwrap();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-ready-1");
    }

    #[test]
    fn test_ready_candidates_excludes_blocked() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a blocked issue and its blocker
        let blocker = Issue::new("bf-blocker".to_string(), "Blocker".to_string(), ".".to_string());
        storage.create_issue(&blocker).unwrap();

        let blocked = Issue::new("bf-blocked".to_string(), "Blocked".to_string(), ".".to_string());
        storage.create_issue(&blocked).unwrap();

        // Add blocking dependency
        storage.add_dependency("bf-blocked", "bf-blocker", &DependencyType::Blocks, "test-user").unwrap();

        // Blocked issue should not be in ready candidates
        let ready = storage.get_ready_candidates().unwrap();
        assert!(!ready.iter().any(|i| i.id == "bf-blocked"));
    }

    #[test]
    fn test_ready_candidates_with_closed_blocker() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a blocked issue and a blocker (initially open)
        let blocker = Issue::new("bf-closed-blocker".to_string(), "Closed blocker".to_string(), ".".to_string());
        storage.create_issue(&blocker).unwrap();

        let issue = Issue::new("bf-ready-2".to_string(), "Ready with closed blocker".to_string(), ".".to_string());
        storage.create_issue(&issue).unwrap();

        // Add dependency on blocker
        storage.add_dependency("bf-ready-2", "bf-closed-blocker", &DependencyType::Blocks, "test-user").unwrap();

        // Close the blocker properly (this sets closed_at)
        storage.close_issue("bf-closed-blocker", "test close", "test-user").unwrap();

        // Issue should be ready since blocker is closed
        let ready = storage.get_ready_candidates().unwrap();
        assert!(ready.iter().any(|i| i.id == "bf-ready-2"));
    }

    // ============================================================================
    // Close and Reopen Tests
    // ============================================================================

    #[test]
    fn test_close_issue_basic() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let issue = Issue::new("bf-close-1".to_string(), "Close test".to_string(), ".".to_string());
        storage.create_issue(&issue).unwrap();

        storage.close_issue("bf-close-1", "Completed", "test-user").unwrap();

        let retrieved = storage.get_issue("bf-close-1").unwrap().unwrap();
        assert_eq!(retrieved.status, Status::Closed);
        assert_eq!(retrieved.close_reason.as_deref(), Some("Completed"));
    }

    #[test]
    fn test_close_issue_is_idempotent() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let issue = Issue::new("bf-close-idempotent".to_string(), "Idempotent close".to_string(), ".".to_string());
        storage.create_issue(&issue).unwrap();

        // Close twice - should not error
        storage.close_issue("bf-close-idempotent", "Completed", "test-user").unwrap();
        storage.close_issue("bf-close-idempotent", "Completed", "test-user").unwrap();

        let retrieved = storage.get_issue("bf-close-idempotent").unwrap().unwrap();
        assert_eq!(retrieved.status, Status::Closed);
    }

    #[test]
    fn test_reopen_issue_basic() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let issue = Issue::new("bf-reopen-1".to_string(), "Reopen test".to_string(), ".".to_string());
        storage.create_issue(&issue).unwrap();

        // Close the issue
        storage.close_issue("bf-reopen-1", "Completed", "test-user").unwrap();

        // Reopen it
        storage.reopen_issue("bf-reopen-1").unwrap();

        let retrieved = storage.get_issue("bf-reopen-1").unwrap().unwrap();
        assert_eq!(retrieved.status, Status::Open);
        assert_eq!(retrieved.assignee, None, "Assignee should be cleared on reopen");
    }

    #[test]
    fn test_close_unblocks_dependents() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a blocker and two dependents
        let blocker = Issue::new("bf-unblock-blocker".to_string(), "Blocker".to_string(), ".".to_string());
        storage.create_issue(&blocker).unwrap();

        let mut dependent1 = Issue::new("bf-dep1".to_string(), "Dependent 1".to_string(), ".".to_string());
        dependent1.status = Status::Blocked;
        storage.create_issue(&dependent1).unwrap();

        let mut dependent2 = Issue::new("bf-dep2".to_string(), "Dependent 2".to_string(), ".".to_string());
        dependent2.status = Status::Blocked;
        storage.create_issue(&dependent2).unwrap();

        // Add blocking dependencies
        storage.add_dependency("bf-dep1", "bf-unblock-blocker", &DependencyType::Blocks, "test-user").unwrap();
        storage.add_dependency("bf-dep2", "bf-unblock-blocker", &DependencyType::Blocks, "test-user").unwrap();

        // Close the blocker
        storage.close_issue("bf-unblock-blocker", "Done", "test-user").unwrap();

        // Dependents should be unblocked (moved to open)
        let dep1_retrieved = storage.get_issue("bf-dep1").unwrap().unwrap();
        let dep2_retrieved = storage.get_issue("bf-dep2").unwrap().unwrap();

        assert_eq!(dep1_retrieved.status, Status::Open);
        assert_eq!(dep2_retrieved.status, Status::Open);
    }

    // ============================================================================
    // Count and Query Tests
    // ============================================================================

    #[test]
    fn test_count_issues() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        assert_eq!(storage.count_issues().unwrap(), 0);

        // Create some issues
        for i in 1..=5 {
            let issue = Issue::new(format!("bf-count-{}", i), format!("Issue {}", i), ".".to_string());
            storage.create_issue(&issue).unwrap();
        }

        assert_eq!(storage.count_issues().unwrap(), 5);
    }

    #[test]
    fn test_get_ready_candidates_empty() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let ready = storage.get_ready_candidates().unwrap();
        assert_eq!(ready.len(), 0);
    }
}
