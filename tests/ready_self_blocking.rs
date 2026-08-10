/// Quick manual test to verify self-blocking prevention works
/// This is a simplified test to verify the implementation without relying on the full test suite

use bead_forge::model::{DependencyType, Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use tempfile::NamedTempFile;

#[test]
fn quick_test_self_blocking_prevention() {
    // Create temporary database
    let temp_file = NamedTempFile::new().unwrap();
    let storage = Storage::open(temp_file.path()).unwrap();

    // Create a test bead
    let issue = Issue {
        id: "bf-test".to_string(),
        title: "Test bead".to_string(),
        priority: Priority::MEDIUM,
        status: Status::Open,
        issue_type: IssueType::Task,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        source_repo: Some(".".to_string()),
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Try to add self-blocking dependency
    let result = storage.add_dependency(
        "bf-test",
        "bf-test",
        &DependencyType::Blocks,
        "test"
    );

    // Should fail with informative error
    assert!(result.is_err(), "Storage should reject self-blocking dependency");

    let error_msg = result.unwrap_err().to_string();
    println!("Error message: {}", error_msg);

    assert!(
        error_msg.to_lowercase().contains("cannot") ||
        error_msg.to_lowercase().contains("block itself") ||
        error_msg.to_lowercase().contains("self-blocking"),
        "Error message should mention self-blocking prevention: {}",
        error_msg
    );

    println!("✓ Self-blocking prevention works correctly");
}

#[test]
fn quick_test_valid_blocking_still_works() {
    // Create temporary database
    let temp_file = NamedTempFile::new().unwrap();
    let storage = Storage::open(temp_file.path()).unwrap();

    // Create two beads
    let blocker = Issue {
        id: "bf-blocker".to_string(),
        title: "Blocker bead".to_string(),
        priority: Priority::MEDIUM,
        status: Status::Open,
        issue_type: IssueType::Task,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        source_repo: Some(".".to_string()),
        ..Default::default()
    };
    storage.create_issue(&blocker).unwrap();

    let dependent = Issue {
        id: "bf-dependent".to_string(),
        title: "Dependent bead".to_string(),
        priority: Priority::MEDIUM,
        status: Status::Open,
        issue_type: IssueType::Task,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        source_repo: Some(".".to_string()),
        ..Default::default()
    };
    storage.create_issue(&dependent).unwrap();

    // Add valid blocking dependency (different beads)
    let result = storage.add_dependency(
        "bf-dependent",
        "bf-blocker",
        &DependencyType::Blocks,
        "test"
    );

    // Should succeed
    assert!(result.is_ok(), "Storage should allow blocking between different beads");
    println!("✓ Valid blocking dependencies still work");
}
