// Integration test for bf-5sw6: bf ready --limit 0 should return unlimited beads

use std::process::Command;

// NOTE: do not add a helper that runs `bf` without `--workspace`/`current_dir`.
// A previous `bf_cmd()` here did `current_dir("..").env("BEADS_DIR", ".beads")`,
// which resolves to a store outside the repo. It was dead code, but it is
// exactly the shape that lets a test mutate a real workspace. Every `bf`
// invocation must be scoped to a temp workspace — see
// `tests/workspace_isolation_guard.rs`.

fn bf_absolute_cmd() -> Command {
    let bf_path = if let Ok(exe) = std::env::current_exe() {
        // Get the cargo target directory from the test executable path
        let target_dir = exe
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or_else(|| exe.parent().unwrap());
        target_dir.join("bf").to_str().unwrap().to_string()
    } else {
        "target/debug/bf".to_string()
    };
    let mut cmd = Command::new(bf_path);
    cmd
}

#[test]
fn test_ready_limit_zero_returns_unlimited() {
    // Create a test workspace with multiple ready beads
    let temp_dir = tempfile::tempdir().unwrap();
    let workspace = temp_dir.path();

    // Initialize workspace
    let mut bf_cmd = bf_absolute_cmd();
    let output = bf_cmd
        .current_dir(workspace)
        .args(["init", "--prefix", "bf"])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Create 15 open beads
    for i in 0..15 {
        let mut bf_cmd = bf_absolute_cmd();
        let output = bf_cmd
            .current_dir(workspace)
            .args([
                "create",
                &format!("--title=Bead {}", i),
                "--type",
                "task",
                &format!("--priority={}", i % 5),
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
    }

    // Test with --limit 0 (should return all 15 beads)
    let mut bf_cmd = bf_absolute_cmd();
    let output = bf_cmd
        .current_dir(workspace)
        .args(["ready", "--limit", "0"])
        .output()
        .unwrap();

    assert!(output.status.success(), "bf ready --limit 0 should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bead_count = stdout.lines().filter(|line| line.contains("[bf-")).count();
    assert_eq!(
        bead_count, 15,
        "Expected all 15 beads with --limit 0 (unlimited)"
    );

    // Test with --limit 5 (should return exactly 5 beads)
    let mut bf_cmd = bf_absolute_cmd();
    let output = bf_cmd
        .current_dir(workspace)
        .args(["ready", "--limit", "5"])
        .output()
        .unwrap();

    assert!(output.status.success(), "bf ready --limit 5 should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bead_count = stdout.lines().filter(|line| line.contains("[bf-")).count();
    assert_eq!(bead_count, 5, "Expected exactly 5 beads with --limit 5");
}

#[test]
fn test_ready_default_limit() {
    // Test that the default limit of 10 is applied
    let temp_dir = tempfile::tempdir().unwrap();
    let workspace = temp_dir.path();

    let mut bf_cmd = bf_absolute_cmd();
    let output = bf_cmd
        .current_dir(workspace)
        .args(["init", "--prefix", "bf"])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Create 20 open beads
    for i in 0..20 {
        let mut bf_cmd = bf_absolute_cmd();
        let output = bf_cmd
            .current_dir(workspace)
            .args(["create", &format!("--title=Bead {}", i), "--type", "task"])
            .output()
            .unwrap();
        assert!(output.status.success());
    }

    // Test without --limit (should use default of 10)
    let mut bf_cmd = bf_absolute_cmd();
    let output = bf_cmd
        .current_dir(workspace)
        .args(["ready"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "bf ready should succeed with default limit"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bead_count = stdout.lines().filter(|line| line.contains("[bf-")).count();
    assert_eq!(bead_count, 10, "Expected default limit of 10 beads");
}

#[test]
fn test_list_limit_zero_returns_unlimited() {
    // Test that --limit 0 also works for bf list
    let temp_dir = tempfile::tempdir().unwrap();
    let workspace = temp_dir.path();

    let mut bf_cmd = bf_absolute_cmd();
    let output = bf_cmd
        .current_dir(workspace)
        .args(["init", "--prefix", "bf"])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Create 15 open beads
    for i in 0..15 {
        let mut bf_cmd = bf_absolute_cmd();
        let output = bf_cmd
            .current_dir(workspace)
            .args(["create", &format!("--title=Bead {}", i), "--type", "task"])
            .output()
            .unwrap();
        assert!(output.status.success());
    }

    // Test list with --limit 0 (should return all 15 beads)
    let mut bf_cmd = bf_absolute_cmd();
    let output = bf_cmd
        .current_dir(workspace)
        .args(["list", "--limit", "0"])
        .output()
        .unwrap();

    assert!(output.status.success(), "bf list --limit 0 should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bead_count = stdout.lines().filter(|line| line.contains("[bf-")).count();
    assert_eq!(bead_count, 15, "Expected all 15 beads with list --limit 0");
}
