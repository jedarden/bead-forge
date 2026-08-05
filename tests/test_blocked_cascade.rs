//! Regression test for bf-5id: blocked->open status cascade on close.
//!
//! Tests that when a bead closes, its dependent beads automatically transition
//! from status='blocked' to status='open' if they have no remaining blockers.
//!
//! This is a critical bug fix - without this cascade, sequential dependency chains
//! (genesis + phase beads) freeze after the first phase closes.

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::TempDir;

    /// Create a temporary test workspace with bf configuration
    fn setup_test_workspace() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path();
        let beads_dir = workspace_dir.join(".beads");
        fs::create_dir(&beads_dir).unwrap();

        // Initialize workspace with bf config
        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            "# Test workspace config\nissue_prefixes:\n- test\n",
        )
        .unwrap();

        // Initialize metadata
        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )
        .unwrap();

        // Initialize empty issues.jsonl
        let issues_path = beads_dir.join("issues.jsonl");
        fs::write(&issues_path, "").unwrap();

        (temp_dir, beads_dir)
    }

    /// Run bf command with the given arguments
    fn run_bf(beads_dir: &PathBuf, args: &[&str]) -> (String, String, bool) {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_bf"));
        cmd.current_dir(beads_dir.parent().unwrap());
        cmd.args(args);

        let output = cmd.output().expect("Failed to execute bf command");
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let success = output.status.success();

        (stdout, stderr, success)
    }

    /// Extract bead ID from bf create output
    fn extract_bead_id(output: &str) -> Option<String> {
        // Look for patterns like "Created bead: test-xxxx" or just "test-xxxx"
        for line in output.lines() {
            if let Some(id) = line.split("Created bead: ").nth(1) {
                return Some(id.trim().to_string());
            }
            if let Some(id) = line.split("Bead ID: ").nth(1) {
                return Some(id.trim().to_string());
            }
            // Also try to find a pattern like "test-xxxx" directly
            if line.starts_with("test-") && line.len() < 20 {
                return Some(line.trim().to_string());
            }
        }
        None
    }

    /// Get bead status as JSON value
    fn get_bead_status(beads_dir: &PathBuf, bead_id: &str) -> String {
        let (stdout, _, success) = run_bf(beads_dir, &["show", bead_id, "--format", "json"]);
        assert!(success, "Show failed for bead {}", bead_id);

        let json: serde_json::Value =
            serde_json::from_str(&stdout).expect("Failed to parse show output as JSON");
        // bf show --json emits an array (NEEDLE contract); unwrap the first element.
        let json = json.get(0).cloned().unwrap_or(json);

        json["status"].as_str().unwrap().to_string()
    }

    /// Check if a bead appears in blocked_issues_cache
    fn is_in_blocked_cache(beads_dir: &PathBuf, bead_id: &str) -> bool {
        use std::io::BufRead;

        let db_path = beads_dir.join("beads.db");
        let output = std::process::Command::new("sqlite3")
            .arg(&db_path)
            .arg("SELECT issue_id FROM blocked_issues_cache WHERE issue_id = ?")
            .arg(bead_id)
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                stdout.trim().contains(bead_id)
            }
            Err(_) => false,
        }
    }

    #[test]
    fn test_close_cascades_blocked_to_open_single_blocker() {
        // Regression test for bf-5id: closing a bead should unblock dependents
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Create bead A (the blocker)
        let (stdout, _, success) = run_bf(
            &beads_dir,
            &[
                "create",
                "--title",
                "Phase 1",
                "--description",
                "First phase",
                "--type",
                "task",
            ],
        );
        assert!(success, "Create A failed");
        let bead_a = extract_bead_id(&stdout).expect("Could not extract bead A ID");

        // Create bead B (blocked by A)
        let (stdout, _, success) = run_bf(
            &beads_dir,
            &[
                "create",
                "--title",
                "Phase 2",
                "--description",
                "Second phase, blocked by Phase 1",
                "--type",
                "task",
            ],
        );
        assert!(success, "Create B failed");
        let bead_b = extract_bead_id(&stdout).expect("Could not extract bead B ID");

        // Add blocking dependency: A blocks B
        let (_, _, success) = run_bf(&beads_dir, &["dep", "add", "--blocks", &bead_b, &bead_a]);
        assert!(success, "dep add failed");

        // Verify B is blocked
        let status_b = get_bead_status(&beads_dir, &bead_b);
        assert_eq!(status_b, "blocked", "B should be blocked while A is open");

        // Close A
        let (_, _, success) = run_bf(
            &beads_dir,
            &["close", &bead_a, "--reason", "Phase 1 complete"],
        );
        assert!(success, "Close A failed");

        // CRITICAL: B should now be open (cascaded from blocked->open)
        let status_b = get_bead_status(&beads_dir, &bead_b);
        assert_eq!(
            status_b, "open",
            "B should transition to open when A closes (bf-5id)"
        );

        // Verify B is not in blocked_issues_cache
        assert!(
            !is_in_blocked_cache(&beads_dir, &bead_b),
            "B should not appear in blocked_issues_cache after A closes"
        );
    }

    #[test]
    fn test_close_does_not_open_with_remaining_blockers() {
        // Negative case: B blocked by both A and C; closing A should NOT open B
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Create beads A, B, C
        let (stdout, _, success) = run_bf(
            &beads_dir,
            &["create", "--title", "Blocker A", "--type", "task"],
        );
        assert!(success, "Create A failed");
        let bead_a = extract_bead_id(&stdout).expect("Could not extract bead A ID");

        let (stdout, _, success) = run_bf(
            &beads_dir,
            &["create", "--title", "Dependent B", "--type", "task"],
        );
        assert!(success, "Create B failed");
        let bead_b = extract_bead_id(&stdout).expect("Could not extract bead B ID");

        let (stdout, _, success) = run_bf(
            &beads_dir,
            &["create", "--title", "Blocker C", "--type", "task"],
        );
        assert!(success, "Create C failed");
        let bead_c = extract_bead_id(&stdout).expect("Could not extract bead C ID");

        // B is blocked by both A and C
        let (_, _, success) = run_bf(&beads_dir, &["dep", "add", "--blocks", &bead_b, &bead_a]);
        assert!(success, "dep add A failed");
        let (_, _, success) = run_bf(&beads_dir, &["dep", "add", "--blocks", &bead_b, &bead_c]);
        assert!(success, "dep add C failed");

        // Verify B is blocked
        let status_b = get_bead_status(&beads_dir, &bead_b);
        assert_eq!(
            status_b, "blocked",
            "B should be blocked while both A and C are open"
        );

        // Close A
        let (_, _, success) = run_bf(&beads_dir, &["close", &bead_a, "--reason", "A complete"]);
        assert!(success, "Close A failed");

        // B should STILL be blocked (C is still open)
        let status_b = get_bead_status(&beads_dir, &bead_b);
        assert_eq!(
            status_b, "blocked",
            "B should remain blocked when C is still open (bf-5id negative case)"
        );

        // Now close C
        let (_, _, success) = run_bf(&beads_dir, &["close", &bead_c, "--reason", "C complete"]);
        assert!(success, "Close C failed");

        // NOW B should open (both blockers gone)
        let status_b = get_bead_status(&beads_dir, &bead_b);
        assert_eq!(
            status_b, "open",
            "B should open only after both A and C close"
        );
    }

    #[test]
    fn test_close_cascade_does_not_touch_non_blocked_statuses() {
        // Test that cascade only affects beads with status='blocked'
        // When a bead has a blocking dependency added, it becomes 'blocked'
        // This test verifies that only beads currently at 'blocked' get transitioned
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Create bead A (blocker)
        let (stdout, _, success) =
            run_bf(&beads_dir, &["create", "--title", "A", "--type", "task"]);
        assert!(success, "Create A failed");
        let bead_a = extract_bead_id(&stdout).expect("Could not extract bead A ID");

        // Create bead C (another blocker that will stay open)
        let (stdout, _, success) =
            run_bf(&beads_dir, &["create", "--title", "C", "--type", "task"]);
        assert!(success, "Create C failed");
        let bead_c = extract_bead_id(&stdout).expect("Could not extract bead C ID");

        // Create bead B and add TWO blocking dependencies (A and C both block B)
        let (stdout, _, success) =
            run_bf(&beads_dir, &["create", "--title", "B", "--type", "task"]);
        assert!(success, "Create B failed");
        let bead_b = extract_bead_id(&stdout).expect("Could not extract bead B ID");

        // Set B to in_progress manually BEFORE adding dependencies
        let (_, _, success) = run_bf(&beads_dir, &["update", &bead_b, "--status", "in_progress"]);
        assert!(success, "Update B to in_progress failed");

        // Add blocking dependencies (A blocks B, C blocks B)
        let (_, _, success) = run_bf(&beads_dir, &["dep", "add", "--blocks", &bead_b, &bead_a]);
        assert!(success, "dep add A failed");
        let (_, _, success) = run_bf(&beads_dir, &["dep", "add", "--blocks", &bead_b, &bead_c]);
        assert!(success, "dep add C failed");

        // After adding blockers, B should become 'blocked' (it now has active blockers)
        let status_b = get_bead_status(&beads_dir, &bead_b);
        assert_eq!(
            status_b, "blocked",
            "B should be blocked after adding blockers"
        );

        // Close A (but C remains open)
        let (_, _, success) = run_bf(&beads_dir, &["close", &bead_a, "--reason", "A done"]);
        assert!(success, "Close A failed");

        // B should STILL be blocked (C is still open - cascade only affects fully unblocked)
        let status_b = get_bead_status(&beads_dir, &bead_b);
        assert_eq!(
            status_b, "blocked",
            "B should remain blocked while C is still open (cascade only opens fully unblocked beads)"
        );
    }

    #[test]
    fn test_three_phase_chain() {
        // Test a realistic genesis + phase 1 + phase 2 + phase 3 chain
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Create genesis bead
        let (stdout, _, success) = run_bf(
            &beads_dir,
            &[
                "create",
                "--title",
                "Genesis: Project X",
                "--type",
                "genesis",
            ],
        );
        assert!(success, "Create genesis failed");
        let genesis = extract_bead_id(&stdout).expect("Could not extract genesis ID");

        // Create Phase 1, blocked by genesis
        let (stdout, _, success) = run_bf(
            &beads_dir,
            &["create", "--title", "Phase 1: Core", "--type", "task"],
        );
        assert!(success, "Create phase 1 failed");
        let phase1 = extract_bead_id(&stdout).expect("Could not extract phase1 ID");

        let (_, _, success) = run_bf(&beads_dir, &["dep", "add", "--blocks", &phase1, &genesis]);
        assert!(success, "phase1 blocked by genesis failed");

        // Create Phase 2, blocked by phase 1
        let (stdout, _, success) = run_bf(
            &beads_dir,
            &["create", "--title", "Phase 2: Advanced", "--type", "task"],
        );
        assert!(success, "Create phase 2 failed");
        let phase2 = extract_bead_id(&stdout).expect("Could not extract phase2 ID");

        let (_, _, success) = run_bf(&beads_dir, &["dep", "add", "--blocks", &phase2, &phase1]);
        assert!(success, "phase2 blocked by phase1 failed");

        // Verify all phases blocked
        assert_eq!(get_bead_status(&beads_dir, &phase1), "blocked");
        assert_eq!(get_bead_status(&beads_dir, &phase2), "blocked");

        // Close genesis → phase1 should open, phase2 still blocked
        let (_, _, success) = run_bf(
            &beads_dir,
            &["close", &genesis, "--reason", "Genesis complete"],
        );
        assert!(success, "Close genesis failed");

        assert_eq!(
            get_bead_status(&beads_dir, &phase1),
            "open",
            "Phase 1 should open after genesis closes"
        );
        assert_eq!(
            get_bead_status(&beads_dir, &phase2),
            "blocked",
            "Phase 2 should still be blocked (phase1 is open)"
        );

        // Close phase1 → phase2 should open
        let (_, _, success) = run_bf(
            &beads_dir,
            &["close", &phase1, "--reason", "Phase 1 complete"],
        );
        assert!(success, "Close phase1 failed");

        assert_eq!(
            get_bead_status(&beads_dir, &phase2),
            "open",
            "Phase 2 should open after phase1 closes"
        );
    }
}
