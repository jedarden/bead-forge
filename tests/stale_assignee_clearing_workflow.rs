//! End-to-end test for stale assignee clearing workflow
//!
//! This test validates the complete workflow for handling stale assignees:
//! 1. Create a test bead with assignee 'dead-worker-X' and status 'open'
//! 2. Verify that NEEDLE explore strand would exclude it (assignee is non-empty)
//! 3. Use 'bf update --clear-assignee' to clear the assignee
//! 4. Verify the bead is now discoverable (assignee is NULL)
//! 5. Document the workflow for fixing stale assignees fleet-wide
//!
//! Background: When NEEDLE workers crash or abandon beads, they leave stale
//! assignees that prevent the beads from being discovered by new workers.
//! This test ensures the workflow for clearing stale assignees works correctly.

use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_stale_assignee_clearing_workflow() {
    // Step 1: Set up test workspace
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace_dir = temp_dir.path().join("test-workspace");
    std::fs::create_dir_all(&workspace_dir).expect("Failed to create workspace");

    // Initialize bf workspace
    let init_output = Command::new("bf")
        .arg("init")
        .arg("--prefix")
        .arg("test")
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf init");

    assert!(
        init_output.status.success(),
        "bf init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    // Step 2: Create a test bead with assignee 'dead-worker-X' and status 'open'
    println!("Step 1: Creating test bead with stale assignee...");
    let create_output = Command::new("bf")
        .arg("create")
        .arg("--title")
        .arg("Test stale assignee clearing")
        .arg("--type")
        .arg("task")
        .arg("--priority")
        .arg("2")
        .arg("--assignee")
        .arg("dead-worker-X")
        .arg("--json")
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf create");

    assert!(
        create_output.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&create_output.stderr)
    );

    // Extract bead ID from JSON output
    let create_json: serde_json::Value = serde_json::from_slice(&create_output.stdout)
        .expect("Failed to parse bf create JSON output");
    let bead_id = create_json["id"]
        .as_str()
        .expect("Bead ID not found in create output");
    println!("Created bead: {}", bead_id);

    // Step 3: Verify that NEEDLE explore strand would exclude it (assignee is non-empty)
    println!("Step 2: Verifying bead has stale assignee (would be excluded from NEEDLE explore)...");
    let show_output = Command::new("bf")
        .arg("show")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .arg("--envelope")
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf show");

    assert!(
        show_output.status.success(),
        "bf show failed: {}",
        String::from_utf8_lossy(&show_output.stderr)
    );

    let show_json: serde_json::Value = serde_json::from_slice(&show_output.stdout)
        .expect("Failed to parse bf show JSON output");
    let initial_assignee = show_json["data"]["assignee"]
        .as_str();

    assert_eq!(
        initial_assignee,
        Some("dead-worker-X"),
        "Initial assignee should be 'dead-worker-X'"
    );
    println!("✓ Bead has stale assignee: {}", initial_assignee.unwrap());

    // Verify it would be excluded from discoverable beads
    let ready_output = Command::new("bf")
        .arg("ready")
        .arg("--format")
        .arg("json")
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf ready");

    assert!(
        ready_output.status.success(),
        "bf ready failed: {}",
        String::from_utf8_lossy(&ready_output.stderr)
    );

    let ready_json: serde_json::Value = serde_json::from_slice(&ready_output.stdout)
        .expect("Failed to parse bf ready JSON output");
    let ready_beads = ready_json
        .as_array()
        .expect("Ready beads should be an array");

    // The bead should NOT appear in ready list because it has a non-empty assignee
    let is_excluded = !ready_beads.iter().any(|b| b["id"].as_str() == Some(bead_id));
    assert!(
        is_excluded,
        "Bead with stale assignee should be excluded from ready/discoverable list"
    );
    println!("✓ Bead is correctly excluded from ready/discoverable list");

    // Step 4: Use 'bf update --clear-assignee' to clear the assignee
    println!("Step 3: Clearing stale assignee with --clear-assignee...");
    let update_output = Command::new("bf")
        .arg("update")
        .arg(&bead_id)
        .arg("--clear-assignee")
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf update --clear-assignee");

    assert!(
        update_output.status.success(),
        "bf update --clear-assignee failed: {}",
        String::from_utf8_lossy(&update_output.stderr)
    );
    println!("✓ Command executed successfully");

    // Step 5: Verify the bead is now discoverable (assignee is NULL)
    println!("Step 4: Verifying assignee is cleared and bead is discoverable...");
    let show_after_output = Command::new("bf")
        .arg("show")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .arg("--envelope")
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf show after clear");

    assert!(
        show_after_output.status.success(),
        "bf show after clear failed: {}",
        String::from_utf8_lossy(&show_after_output.stderr)
    );

    let show_after_json: serde_json::Value = serde_json::from_slice(&show_after_output.stdout)
        .expect("Failed to parse bf show after clear JSON output");
    let cleared_assignee = show_after_json["data"]["assignee"]
        .as_str();

    assert_eq!(
        cleared_assignee,
        None,
        "Assignee should be NULL (null in JSON) after clearing"
    );
    println!("✓ Assignee successfully cleared (is NULL)");

    // Verify the bead is now discoverable in the ready list
    let ready_after_output = Command::new("bf")
        .arg("ready")
        .arg("--format")
        .arg("json")
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf ready after clear");

    assert!(
        ready_after_output.status.success(),
        "bf ready after clear failed: {}",
        String::from_utf8_lossy(&ready_after_output.stderr)
    );

    let ready_after_json: serde_json::Value = serde_json::from_slice(&ready_after_output.stdout)
        .expect("Failed to parse bf ready after clear JSON output");
    let ready_beads_after = ready_after_json
        .as_array()
        .expect("Ready beads after clear should be an array");

    // The bead SHOULD now appear in ready list because assignee is NULL
    let is_discoverable = ready_beads_after.iter().any(|b| b["id"].as_str() == Some(bead_id));
    assert!(
        is_discoverable,
        "Bead with cleared assignee should be discoverable in ready list"
    );
    println!("✓ Bead is now discoverable in ready/claim list");

    println!("\n✅ All acceptance criteria passed!");
    println!("Workflow summary:");
    println!("1. Created bead with stale assignee 'dead-worker-X'");
    println!("2. Verified bead was excluded from discoverable list");
    println!("3. Used 'bf update --clear-assignee' to clear assignee");
    println!("4. Verified assignee is NULL and bead is discoverable");
}

#[test]
fn test_clear_assignee_via_empty_string() {
    // Test that --assignee "" works the same as --clear-assignee
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace_dir = temp_dir.path().join("test-workspace-2");
    std::fs::create_dir_all(&workspace_dir).expect("Failed to create workspace");

    // Initialize and create bead
    Command::new("bf")
        .arg("init")
        .arg("--prefix")
        .arg("test")
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf init");

    let create_output = Command::new("bf")
        .arg("create")
        .arg("--title")
        .arg("Test empty string clear")
        .arg("--assignee")
        .arg("another-dead-worker")
        .arg("--json")
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf create");

    let create_json: serde_json::Value = serde_json::from_slice(&create_output.stdout)
        .expect("Failed to parse create JSON");
    let bead_id = create_json["id"].as_str().expect("No bead ID");

    // Clear using empty string instead of --clear-assignee
    let update_output = Command::new("bf")
        .arg("update")
        .arg(bead_id)
        .arg("--assignee")
        .arg("")
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf update with empty assignee");

    assert!(
        update_output.status.success(),
        "bf update with empty assignee failed: {}",
        String::from_utf8_lossy(&update_output.stderr)
    );

    // Verify assignee is cleared
    let show_output = Command::new("bf")
        .arg("show")
        .arg(bead_id)
        .arg("--format")
        .arg("json")
        .arg("--envelope")
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf show");

    let show_json: serde_json::Value = serde_json::from_slice(&show_output.stdout)
        .expect("Failed to parse show JSON");

    assert_eq!(
        show_json["data"]["assignee"].as_str(),
        None,
        "Assignee should be NULL after clearing with empty string"
    );
}

#[test]
fn test_clear_assignee_conflicts_with_assignee_flag() {
    // Test that --clear-assignee conflicts with --assignee
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace_dir = temp_dir.path().join("test-workspace-3");
    std::fs::create_dir_all(&workspace_dir).expect("Failed to create workspace");

    Command::new("bf")
        .arg("init")
        .arg("--prefix")
        .arg("test")
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf init");

    let create_output = Command::new("bf")
        .arg("create")
        .arg("--title")
        .arg("Test conflict")
        .arg("--json")
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf create");

    let create_json: serde_json::Value = serde_json::from_slice(&create_output.stdout)
        .expect("Failed to parse create JSON");
    let bead_id = create_json["id"].as_str().expect("No bead ID");

    // Try to use both flags together - should fail
    let update_output = Command::new("bf")
        .arg("update")
        .arg(bead_id)
        .arg("--clear-assignee")
        .arg("--assignee")
        .arg("some-worker")
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to execute bf update");

    assert!(
        !update_output.status.success(),
        "bf update should fail when both --clear-assignee and --assignee are used"
    );

    let stderr = String::from_utf8_lossy(&update_output.stderr);
    assert!(
        stderr.contains("cannot be used with") || stderr.contains("conflicts"),
        "Error message should mention conflict between flags"
    );
}

#[test]
fn test_programmatic_clear_assignee_workflow() {
    // Test the same workflow using the programmatic API
    use bead_forge::config::load_config;
    use bead_forge::model::{Issue, IssueChanges, IssueType, Priority, Status};
    use bead_forge::storage::Storage;
    use chrono::Utc;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace_dir = temp_dir.path().join("test-workspace-4");
    std::fs::create_dir_all(&workspace_dir).expect("Failed to create workspace");

    let beads_dir = workspace_dir.join(".beads");
    std::fs::create_dir_all(&beads_dir).expect("Failed to create .beads");

    // Initialize workspace
    bead_forge::config::init_workspace(&beads_dir, "test").expect("Failed to init workspace");
    let config = load_config(&beads_dir).expect("Failed to load config");
    let metadata = bead_forge::config::load_metadata(&beads_dir).expect("Failed to load metadata");
    let db_path = beads_dir.join(&metadata.database);

    let storage = Storage::open_with_config(&db_path, &config)
        .expect("Failed to open storage");

    // Create bead with stale assignee
    let now = Utc::now();
    let issue = Issue {
        id: "test-stale-assignee".to_string(),
        title: "Test programmatic clear".to_string(),
        description: Some("Test description".to_string()),
        acceptance_criteria: None,
        design: None,
        notes: None,
        status: Status::Open,
        priority: Priority(2),
        issue_type: IssueType::Task,
        assignee: Some("dead-worker-programmatic".to_string()),
        owner: None,
        estimated_minutes: None,
        created_at: now,
        created_by: Some("test".to_string()),
        updated_at: now,
        closed_at: None,
        close_reason: None,
        closed_by_session: None,
        due_at: None,
        defer_until: None,
        external_ref: None,
        source_system: None,
        source_repo: None,
        deleted_at: None,
        deleted_by: None,
        delete_reason: None,
        original_type: None,
        compaction_level: None,
        compacted_at: None,
        compacted_at_commit: None,
        original_size: None,
        sender: None,
        ephemeral: false,
        pinned: false,
        is_template: false,
        content_hash: None,
        labels: vec![],
        dependencies: vec![],
        comments: vec![],
        annotations: Default::default(),
    };

    storage.create_issue(&issue).expect("Failed to create issue");

    // Verify initial state
    let retrieved = storage.get_issue("test-stale-assignee")
        .expect("Failed to get issue")
        .expect("Issue not found");
    assert_eq!(
        retrieved.assignee.as_deref(),
        Some("dead-worker-programmatic"),
        "Initial assignee should be set"
    );

    // Clear assignee using empty string (API equivalent of --clear-assignee)
    let changes = IssueChanges {
        assignee: Some(String::new()), // Empty string signals "clear to NULL"
        ..Default::default()
    };
    storage.update_issue("test-stale-assignee", &changes)
        .expect("Failed to update issue");

    // Verify assignee is cleared
    let cleared = storage.get_issue("test-stale-assignee")
        .expect("Failed to get cleared issue")
        .expect("Cleared issue not found");
    assert_eq!(
        cleared.assignee,
        None,
        "Assignee should be NULL after clearing"
    );

    // Verify the bead is now discoverable (no assignee filter returns it)
    let filter = bead_forge::model::IssueFilter {
        assignee: None, // No assignee filter - should return all unassigned beads
        ..Default::default()
    };
    let unassigned_beads = storage.list_issues(&filter)
        .expect("Failed to list issues");

    let is_discoverable = unassigned_beads.iter()
        .any(|b| b.id == "test-stale-assignee");
    assert!(
        is_discoverable,
        "Bead with cleared assignee should be discoverable"
    );
}
