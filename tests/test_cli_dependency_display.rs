//! CLI integration tests for dependency display output
//!
//! Tests the `bf show` command output for dependencies:
//! - Bead with no dependencies shows nothing
//! - Bead with dependencies shows formatted list with titles
//! - Dependency titles are properly escaped/rendered
//! - Output formatting matches expected CLI display style
//!
//! Acceptance Criteria:
//! - Test `bf show <id>` output includes dependencies section when present
//! - Test scenarios: no dependencies, with dependencies, special characters
//! - Test output formatting matches expected CLI display style
//! - All tests pass with `cargo test`

mod common;

use std::process::Command;

/// Get the path to the bf binary
fn bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Create a Command builder for bf with workspace configured
fn bf_command(workspace: &common::TempWorkspace) -> Command {
    let mut cmd = Command::new(&bf_binary());
    cmd.arg("-w").arg(&workspace.beads_dir);
    cmd.current_dir(workspace.workspace_path());
    cmd
}

/// Helper to check if binary exists
fn require_binary() {
    let binary = bf_binary();
    if !std::path::Path::new(&binary).exists() {
        eprintln!(
            "Skipping test - binary not found at: {}. Run 'cargo build' first.",
            binary
        );
        panic!("Binary not found");
    }
}

// ============================================================================
// Text format dependency display tests
// ============================================================================

#[test]
fn test_show_text_no_dependencies() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create a bead without dependencies
    let bead_id = "bf-no-deps";
    ws.create_bead(bead_id, "Bead without dependencies")
        .unwrap();

    // Get text output from show command
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should NOT contain "Dependencies:" section
    assert!(!stdout.contains("Dependencies:"),
            "Bead without dependencies should not show Dependencies section");

    // Should contain normal fields
    assert!(stdout.contains(&format!("ID: {}", bead_id)));
    assert!(stdout.contains("Title: Bead without dependencies"));
}

#[test]
fn test_show_text_with_single_dependency() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create a dependency bead
    let dep_id = "bf-dep-1";
    ws.create_bead(dep_id, "First dependency").unwrap();

    // Create a bead with a dependency
    let bead_id = "bf-with-deps";
    ws.create_bead(bead_id, "Bead with dependencies").unwrap();

    // Add the dependency using storage API
    let storage = ws.storage().unwrap();
    storage
        .add_dependency(
            bead_id,
            dep_id,
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();

    // Get text output from show command
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should contain "Dependencies:" section
    assert!(stdout.contains("Dependencies:"),
            "Bead with dependencies should show Dependencies section");

    // Should show the dependency with ID and title
    assert!(stdout.contains(dep_id),
            "Dependencies should include dependency ID");
    assert!(stdout.contains("First dependency"),
            "Dependencies should include dependency title");

    // Should show (blocks) marker
    assert!(stdout.contains("(blocks)"),
            "Blocking dependencies should show (blocks) marker");
}

#[test]
fn test_show_text_with_multiple_dependencies() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create multiple dependency beads
    let dep1_id = "bf-dep-1";
    ws.create_bead(dep1_id, "First blocker").unwrap();

    let dep2_id = "bf-dep-2";
    ws.create_bead(dep2_id, "Second blocker").unwrap();

    let dep3_id = "bf-dep-3";
    ws.create_bead(dep3_id, "Related task").unwrap();

    // Create a bead with multiple dependencies
    let bead_id = "bf-multi-deps";
    ws.create_bead(bead_id, "Bead with multiple dependencies").unwrap();

    // Add dependencies using storage API
    let storage = ws.storage().unwrap();
    storage
        .add_dependency(
            bead_id,
            dep1_id,
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();
    storage
        .add_dependency(
            bead_id,
            dep2_id,
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();
    storage
        .add_dependency(
            bead_id,
            dep3_id,
            &bead_forge::model::DependencyType::Related,
            "test",
        )
        .unwrap();

    // Get text output from show command
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should contain "Dependencies:" section
    assert!(stdout.contains("Dependencies:"),
            "Bead with dependencies should show Dependencies section");

    // Should show all dependencies
    assert!(stdout.contains(dep1_id));
    assert!(stdout.contains("First blocker"));
    assert!(stdout.contains(dep2_id));
    assert!(stdout.contains("Second blocker"));
    assert!(stdout.contains(dep3_id));
    assert!(stdout.contains("Related task"));

    // Should show (blocks) marker for blocking dependencies
    assert!(stdout.contains("(blocks)"));

    // Should be comma-separated
    assert!(stdout.contains(","));
}

#[test]
fn test_show_text_dependencies_formatting() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create a dependency bead
    let dep_id = "bf-dep-blocker";
    ws.create_bead(dep_id, "Blocker task").unwrap();

    // Create a bead with a blocking dependency
    let bead_id = "bf-formatted";
    ws.create_bead(bead_id, "Test formatting").unwrap();

    // Add the dependency
    let storage = ws.storage().unwrap();
    storage
        .add_dependency(
            bead_id,
            dep_id,
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();

    // Get text output from show command
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Check the exact format: "Depends: bf-id (title) (blocks)"
    assert!(stdout.contains(&format!("Depends: {} (Blocker task) (blocks)", dep_id)),
            "Dependency should be formatted as: Depends: bf-id (title) (blocks)");
}

#[test]
fn test_show_text_non_blocking_dependency() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create a dependency bead
    let dep_id = "bf-related";
    ws.create_bead(dep_id, "Related task").unwrap();

    // Create a bead with a non-blocking dependency
    let bead_id = "bf-non-blocking";
    ws.create_bead(bead_id, "Bead with related task").unwrap();

    // Add the non-blocking dependency
    let storage = ws.storage().unwrap();
    storage
        .add_dependency(
            bead_id,
            dep_id,
            &bead_forge::model::DependencyType::Related,
            "test",
        )
        .unwrap();

    // Get text output from show command
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should show the dependency without (blocks) marker
    assert!(stdout.contains(&format!("Depends: {} (Related task)", dep_id)),
            "Non-blocking dependency should be formatted as: Depends: bf-id (title)");

    // Should NOT contain (blocks) for related dependencies
    let lines: Vec<&str> = stdout.lines().collect();
    let dep_lines: Vec<&str> = lines.iter()
        .filter(|line| line.contains(dep_id))
        .cloned()
        .collect();

    assert!(!dep_lines.is_empty(), "Should have dependency line");

    // The line with the dependency should not have (blocks) unless it's a blocking dep
    let dep_line = dep_lines.join(" ");
    if dep_line.contains("(blocks)") {
        panic!("Related dependency should not show (blocks) marker");
    }
}

#[test]
fn test_show_text_dependency_title_special_characters() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create dependency beads with special characters in titles
    let dep1_id = "bf-special-1";
    ws.create_bead(dep1_id, "Task with <quotes> & \"double\"").unwrap();

    let dep2_id = "bf-special-2";
    ws.create_bead(dep2_id, "Task with emoji 🎉 and unicode Ñ").unwrap();

    // Create a bead with these dependencies
    let bead_id = "bf-special-deps";
    ws.create_bead(bead_id, "Bead with special dependency titles").unwrap();

    // Add dependencies
    let storage = ws.storage().unwrap();
    storage
        .add_dependency(
            bead_id,
            dep1_id,
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();
    storage
        .add_dependency(
            bead_id,
            dep2_id,
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();

    // Get text output from show command
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should preserve special characters
    assert!(stdout.contains("<quotes>"),
            "Dependency titles should preserve special characters");
    assert!(stdout.contains("🎉"),
            "Dependency titles should preserve emoji");
    assert!(stdout.contains("Ñ"),
            "Dependency titles should preserve unicode characters");
}

#[test]
fn test_show_text_dependency_title_multiline() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create a dependency bead with multiline title
    let dep_id = "bf-multiline";
    let multiline_title = "Line 1\nLine 2\nLine 3";

    // Create bead with custom title using full Issue creation
    let dep_issue = bead_forge::Issue {
        id: dep_id.to_string(),
        title: multiline_title.to_string(),
        ..Default::default()
    };
    ws.create_issue(&dep_issue).unwrap();

    // Create a bead that depends on it
    let bead_id = "bf-dep-multiline";
    ws.create_bead(bead_id, "Bead depending on multiline title").unwrap();

    // Add the dependency
    let storage = ws.storage().unwrap();
    storage
        .add_dependency(
            bead_id,
            dep_id,
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();

    // Get text output from show command
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should preserve newlines in dependency title
    assert!(stdout.contains("Line 1"),
            "Dependency titles should preserve newlines (line 1)");
    assert!(stdout.contains("Line 2"),
            "Dependency titles should preserve newlines (line 2)");
    assert!(stdout.contains("Line 3"),
            "Dependency titles should preserve newlines (line 3)");
}

#[test]
fn test_show_text_dependency_section_indentation() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create dependency beads
    let dep_id = "bf-dep-indent";
    ws.create_bead(dep_id, "Dependency for indentation test").unwrap();

    // Create a bead with dependency
    let bead_id = "bf-indent-test";
    ws.create_bead(bead_id, "Test indentation").unwrap();

    // Add the dependency
    let storage = ws.storage().unwrap();
    storage
        .add_dependency(
            bead_id,
            dep_id,
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();

    // Get text output from show command
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Check that dependencies are properly indented
    let lines: Vec<&str> = stdout.lines().collect();

    // Find the "Dependencies:" line
    let mut deps_line_idx = None;
    for (i, line) in lines.iter().enumerate() {
        if line.contains("Dependencies:") {
            deps_line_idx = Some(i);
            break;
        }
    }

    assert!(deps_line_idx.is_some(), "Should have Dependencies section");

    // The next line(s) should be indented (contain spaces at the start)
    if let Some(deps_idx) = deps_line_idx {
        if deps_idx + 1 < lines.len() {
            let next_line = lines[deps_idx + 1];
            // Dependency lines should be indented
            if !next_line.trim().is_empty() {
                assert!(next_line.starts_with("  ") || next_line.starts_with("\t"),
                        "Dependency lines should be indented");
            }
        }
    }
}

#[test]
fn test_show_text_empty_dependency_title() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create a dependency bead with empty title
    let dep_id = "bf-empty-title";
    let empty_title_issue = bead_forge::Issue {
        id: dep_id.to_string(),
        title: "".to_string(),
        ..Default::default()
    };
    ws.create_issue(&empty_title_issue).unwrap();

    // Create a bead that depends on it
    let bead_id = "bf-dep-empty-title";
    ws.create_bead(bead_id, "Bead depending on empty title").unwrap();

    // Add the dependency
    let storage = ws.storage().unwrap();
    storage
        .add_dependency(
            bead_id,
            dep_id,
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();

    // Get text output from show command
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should show dependency even with empty title
    assert!(stdout.contains("Dependencies:"),
            "Should show Dependencies section even with empty title");
    assert!(stdout.contains(dep_id),
            "Should show dependency ID even with empty title");
    // Should show empty parens for title: "bf-empty-title () (blocks)"
    assert!(stdout.contains("() (blocks)"),
            "Should show empty parens for empty title");
}

// ============================================================================
// Toon format dependency display tests
// ============================================================================

#[test]
fn test_show_toon_with_dependencies() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create dependency beads
    let dep_id = "bf-dep-toon";
    ws.create_bead(dep_id, "Toon format dependency").unwrap();

    // Create a bead with dependency
    let bead_id = "bf-toon-deps";
    ws.create_bead(bead_id, "Bead for toon format").unwrap();

    // Add the dependency
    let storage = ws.storage().unwrap();
    storage
        .add_dependency(
            bead_id,
            dep_id,
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();

    // Get toon output from show command
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .arg("--format")
        .arg("toon")
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should contain "Dependencies:" section
    assert!(stdout.contains("Dependencies:"),
            "Toon format should show Dependencies section");

    // Should show the dependency
    assert!(stdout.contains(dep_id),
            "Toon format should include dependency ID");
    assert!(stdout.contains("Toon format dependency"),
            "Toon format should include dependency title");
}

#[test]
fn test_show_toon_no_dependencies() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create a bead without dependencies
    let bead_id = "bf-toon-no-deps";
    ws.create_bead(bead_id, "Bead without dependencies for toon")
        .unwrap();

    // Get toon output from show command
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .arg("--format")
        .arg("toon")
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should NOT contain "Dependencies:" section
    assert!(!stdout.contains("Dependencies:"),
            "Toon format should not show Dependencies section when no dependencies");
}

// ============================================================================
// JSON format dependency display tests
// ============================================================================

#[test]
fn test_show_json_dependencies_stripped() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create dependency beads
    let dep_id = "bf-dep-json";
    ws.create_bead(dep_id, "JSON dependency").unwrap();

    // Create a bead with dependency
    let bead_id = "bf-json-deps";
    ws.create_bead(bead_id, "Bead for JSON format").unwrap();

    // Add the dependency
    let storage = ws.storage().unwrap();
    storage
        .add_dependency(
            bead_id,
            dep_id,
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();

    // Get JSON output from show command
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(stdout.trim())
        .expect("Invalid JSON output");

    let array = json.as_array().expect("show should return array");
    let issue_json = &array[0];

    // Dependencies and comments should be stripped/empty in JSON output
    // due to NEEDLE compatibility (as noted in cmd_show)
    let deps = issue_json.get("dependencies");
    match deps {
        Some(dep_value) => {
            let dep_array = dep_value.as_array()
                .expect("dependencies should be an array if present");
            assert_eq!(dep_array.len(), 0,
                       "dependencies should be empty in JSON show output");
        }
        None => {
            // Field is absent when empty, which is acceptable
        }
    }
}
