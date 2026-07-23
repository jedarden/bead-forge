//! Integration tests for `bf claim --fallback any` behavior.
//!
//! Tests the fallback mechanism where a worker prefers their assigned workspace
//! but falls back to cross-workspace claiming when their workspace is empty.

mod common;

#[test]
fn test_claim_fallback_any_exhausted_primary_workspace() {
    // Main test: workspace A is empty, workspace B has beads.
    // Claiming with --fallback any should return a bead from workspace B.
    let workspace_a = common::TempWorkspace::new().unwrap();
    let workspace_b = common::TempWorkspace::new().unwrap();

    // Workspace A: no beads (exhausted)
    // Workspace B: 2 beads available
    workspace_b.create_bead("bf-b1", "Bead in B").unwrap();
    workspace_b
        .create_bead("bf-b2", "Another bead in B")
        .unwrap();

    // Set up worker metadata
    let worker_metadata = bead_forge::claim::WorkerMetadata {
        worker_id: "worker-1".to_string(),
        model: None,
        harness: None,
        harness_version: None,
    };

    // Claim with fallback: primary workspace is A (empty), should fallback to B
    let workspace_paths = vec![
        workspace_a.workspace_path().to_path_buf(),
        workspace_b.workspace_path().to_path_buf(),
    ];

    let result =
        bead_forge::claim::claim_any(&workspace_paths, "worker-1", 30, Some(&worker_metadata));

    assert!(result.is_ok());
    let claim_result = result.unwrap();

    // Should claim a bead from workspace B
    assert!(
        claim_result.is_some(),
        "Should claim a bead from workspace B"
    );

    let claimed = claim_result.unwrap();

    // The claimed bead should be from workspace B
    let workspace_path = claimed.workspace_path.unwrap();
    assert_eq!(workspace_path, workspace_b.workspace_path());

    // Verify the bead was actually claimed
    let bead = workspace_b.get_bead(&claimed.bead_id).unwrap().unwrap();
    assert_eq!(bead.status.to_string(), "in_progress");
    assert_eq!(bead.assignee.as_ref().unwrap(), "worker-1");
}

#[test]
fn test_claim_fallback_any_primary_has_beads_no_fallback() {
    // When primary workspace has beads, it should claim from there (no fallback).
    let workspace_a = common::TempWorkspace::new().unwrap();
    let workspace_b = common::TempWorkspace::new().unwrap();

    // Workspace A: 1 bead (should be claimed here)
    workspace_a.create_bead("bf-a1", "Bead in A").unwrap();

    // Workspace B: 2 beads (should NOT be claimed from)
    workspace_b.create_bead("bf-b1", "Bead in B").unwrap();
    workspace_b
        .create_bead("bf-b2", "Another bead in B")
        .unwrap();

    let worker_metadata = bead_forge::claim::WorkerMetadata {
        worker_id: "worker-1".to_string(),
        model: None,
        harness: None,
        harness_version: None,
    };

    let workspace_paths = vec![
        workspace_a.workspace_path().to_path_buf(),
        workspace_b.workspace_path().to_path_buf(),
    ];

    let result = bead_forge::claim_any(&workspace_paths, "worker-1", 30, Some(&worker_metadata));

    assert!(result.is_ok());
    let claim_result = result.unwrap();

    assert!(claim_result.is_some());

    let claimed = claim_result.unwrap();

    // Should claim from workspace A (has beads)
    let workspace_path = claimed.workspace_path.unwrap();
    assert_eq!(workspace_path, workspace_a.workspace_path());
    assert_eq!(claimed.bead_id, "bf-a1");

    // Verify workspace B beads are still open
    let bead_b1 = workspace_b.get_bead("bf-b1").unwrap().unwrap();
    assert_eq!(bead_b1.status.to_string(), "open");
    assert!(bead_b1.assignee.is_none());
}

#[test]
fn test_claim_fallback_any_empty_all_workspaces() {
    // When all workspaces are empty, should return None.
    let workspace_a = common::TempWorkspace::new().unwrap();
    let workspace_b = common::TempWorkspace::new().unwrap();

    // Both workspaces empty: no beads

    let worker_metadata = bead_forge::claim::WorkerMetadata {
        worker_id: "worker-1".to_string(),
        model: None,
        harness: None,
        harness_version: None,
    };

    let workspace_paths = vec![
        workspace_a.workspace_path().to_path_buf(),
        workspace_b.workspace_path().to_path_buf(),
    ];

    let result = bead_forge::claim_any(&workspace_paths, "worker-1", 30, Some(&worker_metadata));

    assert!(result.is_ok());
    let claim_result = result.unwrap();

    // No beads available in any workspace
    assert!(
        claim_result.is_none(),
        "Should return None when all workspaces empty"
    );
}

#[test]
fn test_claim_fallback_any_selects_from_available_workspace() {
    // Verify that when one workspace has beads and another doesn't,
    // a bead from the available workspace is claimed.
    let workspace_a = common::TempWorkspace::new().unwrap();
    let workspace_b = common::TempWorkspace::new().unwrap();

    // Workspace A: has a bead
    workspace_a.create_bead("bf-a1", "Bead in A").unwrap();

    // Workspace B: empty

    let worker_metadata = bead_forge::claim::WorkerMetadata {
        worker_id: "worker-1".to_string(),
        model: None,
        harness: None,
        harness_version: None,
    };

    let workspace_paths = vec![
        workspace_a.workspace_path().to_path_buf(),
        workspace_b.workspace_path().to_path_buf(),
    ];

    let result = bead_forge::claim_any(&workspace_paths, "worker-1", 30, Some(&worker_metadata));

    assert!(result.is_ok());
    let claim_result = result.unwrap();

    assert!(claim_result.is_some());

    let claimed = claim_result.unwrap();

    // Should claim from workspace A (the only one with beads)
    assert_eq!(claimed.bead_id, "bf-a1");
    let workspace_path = claimed.workspace_path.unwrap();
    assert_eq!(workspace_path, workspace_a.workspace_path());
}

#[test]
fn test_claim_fallback_any_with_dependencies() {
    // Verify that blocked beads are not claimed even with fallback.
    let workspace_a = common::TempWorkspace::new().unwrap();
    let workspace_b = common::TempWorkspace::new().unwrap();

    // Workspace A: empty

    // Workspace B: parent and child beads
    workspace_b.create_bead("bf-parent", "Parent").unwrap();
    workspace_b.create_bead("bf-child", "Child").unwrap();

    // Block child on parent
    workspace_b
        .storage()
        .unwrap()
        .add_dependency(
            "bf-child",
            "bf-parent",
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();

    let worker_metadata = bead_forge::claim::WorkerMetadata {
        worker_id: "worker-1".to_string(),
        model: None,
        harness: None,
        harness_version: None,
    };

    let workspace_paths = vec![
        workspace_a.workspace_path().to_path_buf(),
        workspace_b.workspace_path().to_path_buf(),
    ];

    // First claim: should get bf-parent (child is blocked)
    let result1 = bead_forge::claim_any(&workspace_paths, "worker-1", 30, Some(&worker_metadata));

    assert!(result1.is_ok());
    let claim_result1 = result1.unwrap().unwrap();
    assert_eq!(claim_result1.bead_id, "bf-parent");

    // Second claim: should get None (child still blocked)
    let result2 = bead_forge::claim_any(&workspace_paths, "worker-2", 30, Some(&worker_metadata));

    assert!(result2.is_ok());
    let claim_result2 = result2.unwrap();

    // Child should not be claimable (blocked by parent)
    assert!(
        claim_result2.is_none(),
        "Child bead should not be claimed (blocked by parent)"
    );
}

#[test]
fn test_claim_fallback_any_pinned_beads_respected() {
    // Verify that pinned beads are not claimed even with fallback.
    let workspace_a = common::TempWorkspace::new().unwrap();
    let workspace_b = common::TempWorkspace::new().unwrap();

    // Workspace A: empty

    // Workspace B: mix of pinned and unpinned
    let mut pinned = bead_forge::Issue::new(
        "bf-pinned".to_string(),
        "Pinned bead".to_string(),
        ".".to_string(),
    );
    pinned.pinned = true;
    workspace_b
        .storage()
        .unwrap()
        .create_issue(&pinned)
        .unwrap();

    workspace_b.create_bead("bf-open", "Open bead").unwrap();

    let worker_metadata = bead_forge::claim::WorkerMetadata {
        worker_id: "worker-1".to_string(),
        model: None,
        harness: None,
        harness_version: None,
    };

    let workspace_paths = vec![
        workspace_a.workspace_path().to_path_buf(),
        workspace_b.workspace_path().to_path_buf(),
    ];

    let result = bead_forge::claim_any(&workspace_paths, "worker-1", 30, Some(&worker_metadata));

    assert!(result.is_ok());
    let claim_result = result.unwrap();

    assert!(claim_result.is_some());

    let claimed = claim_result.unwrap();

    // Should claim the unpinned bead
    assert_eq!(claimed.bead_id, "bf-open");

    // Verify pinned bead is still open
    let pinned_bead = workspace_b.get_bead("bf-pinned").unwrap().unwrap();
    assert_eq!(pinned_bead.status.to_string(), "open");
    assert!(pinned_bead.assignee.is_none());
    assert!(pinned_bead.pinned);
}

#[test]
fn test_claim_fallback_any_multiple_workspaces() {
    // Test fallback across more than 2 workspaces.
    let workspace_a = common::TempWorkspace::new().unwrap();
    let workspace_b = common::TempWorkspace::new().unwrap();
    let workspace_c = common::TempWorkspace::new().unwrap();

    // Workspace A: empty
    // Workspace B: empty
    // Workspace C: has beads
    workspace_c.create_bead("bf-c1", "Bead in C").unwrap();

    let worker_metadata = bead_forge::claim::WorkerMetadata {
        worker_id: "worker-1".to_string(),
        model: None,
        harness: None,
        harness_version: None,
    };

    let workspace_paths = vec![
        workspace_a.workspace_path().to_path_buf(),
        workspace_b.workspace_path().to_path_buf(),
        workspace_c.workspace_path().to_path_buf(),
    ];

    let result = bead_forge::claim_any(&workspace_paths, "worker-1", 30, Some(&worker_metadata));

    assert!(result.is_ok());
    let claim_result = result.unwrap();

    assert!(claim_result.is_some());

    let claimed = claim_result.unwrap();

    // Should claim from workspace C (the only one with beads)
    let workspace_path = claimed.workspace_path.unwrap();
    assert_eq!(workspace_path, workspace_c.workspace_path());
    assert_eq!(claimed.bead_id, "bf-c1");
}

#[test]
fn test_cli_claim_fallback_any_exhausted_workspace() {
    // CLI-level integration test for `bf claim --fallback any`.
    // Creates two workspaces, exhausts workspace A, verifies that
    // `bf claim --workspace A --fallback any` returns a bead from workspace B.

    use std::path::PathBuf;
    use std::process::Command;

    let workspace_a = common::TempWorkspace::new().unwrap();
    let workspace_b = common::TempWorkspace::new().unwrap();

    // Workspace A: no beads (exhausted)
    // Workspace B: 2 beads available
    workspace_b.create_bead("bf-b1", "Bead in B").unwrap();
    workspace_b
        .create_bead("bf-b2", "Another bead in B")
        .unwrap();

    // Build the bf binary if it doesn't exist
    // Note: cargo tests run with cwd = project root, but we change current_dir for the command
    // so we need an absolute path to the binary
    let project_root = std::env::current_dir().unwrap();
    let bf_binary: PathBuf = if cfg!(debug_assertions) {
        project_root.join("target/debug/bf")
    } else {
        project_root.join("target/release/bf")
    };

    // If binary doesn't exist, skip this test (requires cargo build)
    if !bf_binary.exists() {
        println!(
            "CLI binary not found at {:?}, skipping CLI test. Run 'cargo build' first.",
            bf_binary
        );
        return;
    }

    // Run: bf claim --workspace <workspace_a> --fallback any --workspace-paths <workspace_a> --workspace-paths <workspace_b> --assignee test-worker --format json
    // Note: --workspace-paths is needed because find_workspaces searches upward from cwd
    // and won't find sibling temp workspaces. Each workspace needs its own --workspace-paths flag.
    let output = Command::new(&bf_binary)
        .arg("--workspace")
        .arg(workspace_a.workspace_path())
        .arg("claim")
        .arg("--fallback")
        .arg("any")
        .arg("--workspace-paths")
        .arg(workspace_a.workspace_path())
        .arg("--workspace-paths")
        .arg(workspace_b.workspace_path())
        .arg("--assignee")
        .arg("test-worker")
        .arg("--format")
        .arg("json")
        .current_dir(workspace_a.workspace_path())
        .output();

    match output {
        Ok(output) => {
            // Check that command succeeded
            assert!(
                output.status.success(),
                "bf claim command failed: stderr={}",
                String::from_utf8_lossy(&output.stderr)
            );

            // Parse JSON output (may be wrapped in envelope)
            let stdout = String::from_utf8(output.stdout).unwrap();
            let json: serde_json::Value =
                serde_json::from_str(&stdout).expect("Output should be valid JSON");

            // Handle envelope format: {version, kind, data}
            let data = if json.get("version").is_some() && json.get("data").is_some() {
                &json["data"]
            } else {
                &json
            };

            // Verify a bead was claimed
            let bead_id = data["bead_id"].as_str();
            assert!(
                bead_id.is_some(),
                "Expected 'bead_id' in JSON output, got: {}",
                stdout
            );

            // The claimed bead should be from workspace B
            let claimed_bead_id = bead_id.unwrap();
            assert!(
                claimed_bead_id.starts_with("bf-b"),
                "Expected bead from workspace B (bf-b*), got: {}",
                claimed_bead_id
            );

            // Verify workspace is in output (should be workspace B's path)
            let workspace_path = data["workspace"].as_str();
            assert!(
                workspace_path.is_some(),
                "Expected 'workspace' in JSON output when claiming via fallback"
            );

            // Verify the bead was actually claimed in workspace B
            let bead = workspace_b.get_bead(claimed_bead_id).unwrap().unwrap();
            assert_eq!(bead.status.to_string(), "in_progress");
            assert_eq!(bead.assignee.as_ref().unwrap(), "test-worker");
        }
        Err(e) => {
            panic!("Failed to execute bf binary: {}", e);
        }
    }
}

#[test]
fn test_claim_fallback_to_1800s_when_velocity_stats_empty() {
    // Regression test for plan §4B.6: verify claim scorer uses COALESCE(vs.p50_seconds, 1800)
    // correctly when no velocity_stats rows exist for the requesting worker model/harness.

    use chrono::Utc;

    let workspace = common::TempWorkspace::new().unwrap();

    // Create some test beads
    workspace.create_bead("bf-1", "First bead").unwrap();
    workspace.create_bead("bf-2", "Second bead").unwrap();
    workspace.create_bead("bf-3", "Third bead").unwrap();

    // Set up worker metadata with an unknown model/harness combination
    // that has no entries in velocity_stats
    let worker_metadata = bead_forge::claim::WorkerMetadata {
        worker_id: "test-worker".to_string(),
        model: Some("unknown-model".to_string()),
        harness: Some("unknown-harness".to_string()),
        harness_version: None,
    };

    // Claim should succeed and return a bead, using 1800s as the default p50_seconds
    let storage = workspace.storage().unwrap();
    let claim_result = storage
        .with_immediate_transaction(|tx| {
            bead_forge::claim::claim(tx, "test-worker", 30, Utc::now(), Some(&worker_metadata))
        })
        .unwrap();

    // Should claim a bead successfully (no error despite empty velocity_stats)
    assert!(
        claim_result.is_some(),
        "Should claim a bead even when velocity_stats is empty for the model/harness"
    );

    let claimed = claim_result.unwrap();
    let bead = workspace.get_bead(&claimed.bead_id).unwrap().unwrap();

    // Verify the bead was actually claimed
    assert_eq!(bead.status.to_string(), "in_progress");
    assert_eq!(bead.assignee.as_ref().unwrap(), "test-worker");

    // Verify velocity_stats is still empty for this model/harness
    let count = storage
        .with_immediate_transaction(|tx| {
            Ok(tx.query_row(
                "SELECT COUNT(*) FROM velocity_stats WHERE model = ?1 AND harness = ?2",
                [&"unknown-model", &"unknown-harness"],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .unwrap();

    assert_eq!(
        count, 0,
        "velocity_stats should still be empty - fallback used 1800s default"
    );
}
