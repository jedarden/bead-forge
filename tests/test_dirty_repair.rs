//! Test for dirty bead tracking after repair
use bead_forge::config::init_workspace;
use bead_forge::doctor::{check, count_unflushed, repair};
use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use bead_forge::sync::flush;
use tempfile::TempDir;

#[test]
fn test_dirty_after_repair_cycle() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    init_workspace(&beads_dir, "bf").unwrap();
    let metadata = bead_forge::config::load_metadata(&beads_dir).unwrap();
    let db_path = beads_dir.join(&metadata.database);

    // Create initial bead
    let storage = Storage::open(&db_path).unwrap();
    let issue = Issue {
        id: "bf-test".to_string(),
        title: "Test".to_string(),
        status: Status::Open,
        priority: Priority::MEDIUM,
        issue_type: IssueType::Task,
        source_repo: Some(".".to_string()),
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Flush to JSONL
    flush(workspace).unwrap();

    // Check dirty_issues table directly via count_unflushed
    let unflushed_before = count_unflushed(&db_path).unwrap();
    println!("Before repair: count_unflushed = {}", unflushed_before);
    assert_eq!(unflushed_before, 0, "Should be 0 after flush");

    // Run repair
    let imported = repair(workspace, false, false).unwrap();
    println!("Repair imported {} beads", imported);

    // Check dirty_issues table after repair
    let unflushed_after = count_unflushed(&db_path).unwrap();
    println!("After repair: count_unflushed = {}", unflushed_after);
    assert_eq!(
        unflushed_after, 0,
        "Should be 0 after repair (beads came from JSONL)"
    );

    // Run doctor check to verify
    let result = check(workspace).unwrap();
    println!("Doctor check unflushed_count = {}", result.unflushed_count);
    assert_eq!(
        result.unflushed_count, 0,
        "Doctor should report 0 unflushed after repair"
    );
}
