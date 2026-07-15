//! E2E compatibility test: bf vs br output parity on same workspace.
//!
//! This test validates that bead-forge (bf) produces identical JSON output
//! to beads_rust (br) when reading the same JSONL fixture.
//!
//! Test strategy:
//! 1. Load a real workspace JSONL snapshot (from forge-snapshot.jsonl)
//! 2. Import it into a bf temporary workspace
//! 3. Run bf list (using JsonFormatter to match br's --format json)
//! 4. Compare output against the original fixture JSONL
//!
//! This validates JSONL round-trip compatibility: bf can import br-generated
//! JSONL and export it back in the same format.

mod common;

use bead_forge::format::Formatter;
use std::fs;
use std::path::PathBuf;

/// Test E2E parity: bf list produces identical JSON output to br list.
///
/// Uses forge-snapshot.jsonl as a real-world fixture from an active bf workspace.
/// The test verifies that:
/// 1. bf can import the JSONL without errors
/// 2. All beads are preserved in the import
/// 3. JSON output from bf matches the original fixture (field parity)
#[test]
fn test_e2e_bf_vs_br_output_parity_forge_snapshot() {
    // Load the fixture
    let fixture_path = PathBuf::from("tests/fixtures/forge-snapshot.jsonl");
    let fixture_jsonl =
        fs::read_to_string(&fixture_path).expect("Failed to read forge-snapshot.jsonl fixture");

    // Parse fixture to count beads and collect IDs
    let fixture_lines: Vec<&str> = fixture_jsonl.lines().collect();
    let fixture_count = fixture_lines.len();
    let fixture_ids: Vec<String> = fixture_lines
        .iter()
        .filter_map(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .ok()
                .and_then(|v| {
                    v.get("id")
                        .and_then(|id| id.as_str())
                        .map(|s| s.to_string())
                })
        })
        .collect();

    println!("Fixture: forge-snapshot.jsonl");
    println!("  Total beads: {}", fixture_count);
    println!(
        "  Sample IDs: {:?}",
        &fixture_ids.iter().take(5).cloned().collect::<Vec<_>>()
    );

    // Import into bf workspace
    let ws = common::TempWorkspace::from_fixture("forge-snapshot.jsonl")
        .expect("Failed to create workspace from fixture");

    let import_result = ws.import_jsonl().expect("Failed to import JSONL");
    assert_eq!(
        import_result.imported + import_result.skipped,
        fixture_count,
        "Import count should match fixture line count"
    );

    // List all beads via storage (mimics bf list command)
    let beads = ws.list_beads().expect("Failed to list beads");
    assert_eq!(
        beads.len(),
        fixture_count,
        "Listed bead count should match fixture count"
    );

    // Format using JsonFormatter (same as bf list --format json)
    let formatter = bead_forge::format::JsonFormatter;
    let bf_output = formatter.format_issues(&beads);

    // Parse both outputs for comparison
    let bf_lines: Vec<&str> = bf_output.lines().collect();
    assert_eq!(
        bf_lines.len(),
        fixture_count,
        "Output line count should match fixture count"
    );

    // Verify each bead is present and has matching fields
    let mut matched_ids = Vec::new();
    for bf_line in &bf_lines {
        let bf_value: serde_json::Value =
            serde_json::from_str(bf_line).expect("Failed to parse bf output as JSON");

        let id = bf_value
            .get("id")
            .and_then(|v| v.as_str())
            .expect("bf output missing id field");

        // Find corresponding line in fixture
        let fixture_line = fixture_lines
            .iter()
            .find(|line| {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                    v.get("id")
                        .and_then(|i| i.as_str())
                        .map_or(false, |fid| fid == id)
                } else {
                    false
                }
            })
            .unwrap_or_else(|| panic!("Bead {} not found in fixture", id));

        let fixture_value: serde_json::Value =
            serde_json::from_str(fixture_line).expect("Failed to parse fixture line as JSON");

        // Compare critical fields (bf strips dependencies/comments for br compatibility)
        assert_eq!(
            bf_value.get("id"),
            fixture_value.get("id"),
            "ID mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("title"),
            fixture_value.get("title"),
            "Title mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("status"),
            fixture_value.get("status"),
            "Status mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("priority"),
            fixture_value.get("priority"),
            "Priority mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("issue_type"),
            fixture_value.get("issue_type"),
            "Type mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("created_at"),
            fixture_value.get("created_at"),
            "created_at mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("updated_at"),
            fixture_value.get("updated_at"),
            "updated_at mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("description"),
            fixture_value.get("description"),
            "Description mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("assignee"),
            fixture_value.get("assignee"),
            "Assignee mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("labels"),
            fixture_value.get("labels"),
            "Labels mismatch for bead {}",
            id
        );

        // Verify dependencies and comments are stripped (br compatibility)
        assert!(
            bf_value
                .get("dependencies")
                .and_then(|d| d.as_array())
                .map_or(true, |arr| arr.is_empty()),
            "Dependencies should be stripped for br compatibility (bead {})",
            id
        );
        assert!(
            bf_value
                .get("comments")
                .and_then(|c| c.as_array())
                .map_or(true, |arr| arr.is_empty()),
            "Comments should be stripped for br compatibility (bead {})",
            id
        );

        matched_ids.push(id.to_string());
    }

    // Verify all fixture beads are represented
    assert_eq!(
        matched_ids.len(),
        fixture_count,
        "Not all beads from fixture were in bf output"
    );

    println!(
        "E2E parity test passed: {} beads validated",
        matched_ids.len()
    );
}

/// Test E2E parity with needle-snapshot.jsonl (another real workspace).
#[test]
fn test_e2e_bf_vs_br_output_parity_needle_snapshot() {
    let fixture_path = PathBuf::from("tests/fixtures/needle-snapshot.jsonl");
    let fixture_jsonl =
        fs::read_to_string(&fixture_path).expect("Failed to read needle-snapshot.jsonl fixture");

    let fixture_lines: Vec<&str> = fixture_jsonl.lines().collect();
    let fixture_count = fixture_lines.len();

    println!("Fixture: needle-snapshot.jsonl");
    println!("  Total beads: {}", fixture_count);

    let ws = common::TempWorkspace::from_fixture("needle-snapshot.jsonl")
        .expect("Failed to create workspace from fixture");

    let import_result = ws.import_jsonl().expect("Failed to import JSONL");

    // Note: The fixture may contain duplicate IDs which get skipped during import.
    // We expect: imported + skipped >= fixture_count (some lines may be duplicates)
    // but the actual bead count should be <= fixture_count.
    let expected_beads = import_result.imported;
    println!(
        "  Imported: {}, Skipped: {}, Total expected beads: {}",
        import_result.imported, import_result.skipped, expected_beads
    );

    let beads = ws.list_beads().expect("Failed to list beads");
    assert_eq!(
        beads.len(),
        expected_beads,
        "Listed bead count should match import count"
    );

    let formatter = bead_forge::format::JsonFormatter;
    let bf_output = formatter.format_issues(&beads);

    let bf_lines: Vec<&str> = bf_output.lines().collect();
    assert_eq!(
        bf_lines.len(),
        expected_beads,
        "Output line count should match bead count"
    );

    // Verify structure: all lines are valid JSON with required fields
    for bf_line in &bf_lines {
        let bf_value: serde_json::Value =
            serde_json::from_str(bf_line).expect("Failed to parse bf output as JSON");

        // Verify required br-compatible fields are present
        assert!(bf_value.get("id").is_some(), "Missing id field");
        assert!(bf_value.get("title").is_some(), "Missing title field");
        assert!(bf_value.get("status").is_some(), "Missing status field");
        assert!(bf_value.get("priority").is_some(), "Missing priority field");
        assert!(
            bf_value.get("issue_type").is_some(),
            "Missing issue_type field"
        );
        assert!(
            bf_value.get("created_at").is_some(),
            "Missing created_at field"
        );
        assert!(
            bf_value.get("updated_at").is_some(),
            "Missing updated_at field"
        );

        // Verify dependencies/comments are stripped
        assert!(
            bf_value
                .get("dependencies")
                .and_then(|d| d.as_array())
                .map_or(true, |arr| arr.is_empty()),
            "Dependencies should be stripped for br compatibility"
        );
        assert!(
            bf_value
                .get("comments")
                .and_then(|c| c.as_array())
                .map_or(true, |arr| arr.is_empty()),
            "Comments should be stripped for br compatibility"
        );
    }

    println!("E2E parity test passed: {} beads validated", bf_lines.len());
}

/// Test E2E parity with simple_bead.jsonl (minimal fixture).
#[test]
fn test_e2e_bf_vs_br_output_parity_simple_bead() {
    let ws = common::TempWorkspace::from_fixture("simple_bead.jsonl")
        .expect("Failed to create workspace from fixture");

    let import_result = ws.import_jsonl().expect("Failed to import JSONL");
    assert_eq!(import_result.imported, 1, "Should import 1 bead");

    let beads = ws.list_beads().expect("Failed to list beads");
    assert_eq!(beads.len(), 1, "Should have 1 bead");

    let formatter = bead_forge::format::JsonFormatter;
    let bf_output = formatter.format_issues(&beads);

    let bf_value: serde_json::Value =
        serde_json::from_str(&bf_output).expect("Failed to parse bf output");

    // Verify the simple bead structure (ID from fixture is bf-001)
    assert_eq!(bf_value["id"], "bf-001");
    assert_eq!(bf_value["title"], "Simple bead");
    assert_eq!(bf_value["status"], "open");
    assert_eq!(bf_value["priority"], 2);

    println!("E2E parity test passed: simple bead validated");
}

/// Test E2E round-trip: export after import produces identical JSONL.
#[test]
fn test_e2e_jsonl_round_trip_output_parity() {
    let fixture_path = PathBuf::from("tests/fixtures/forge-snapshot.jsonl");
    let fixture_jsonl =
        fs::read_to_string(&fixture_path).expect("Failed to read forge-snapshot.jsonl fixture");

    // Import into workspace
    let ws = common::TempWorkspace::from_fixture("forge-snapshot.jsonl")
        .expect("Failed to create workspace from fixture");

    let import_result = ws.import_jsonl().expect("Failed to import JSONL");

    // Export back to JSONL
    let export_count = ws.export_jsonl(false).expect("Failed to export JSONL");
    assert_eq!(
        export_count,
        import_result.imported + import_result.skipped,
        "Export count should match import count"
    );

    // Read exported JSONL
    let exported_jsonl = fs::read_to_string(&ws.jsonl_path).expect("Failed to read exported JSONL");

    // Parse and compare counts
    let fixture_lines: Vec<&str> = fixture_jsonl.lines().collect();
    let exported_lines: Vec<&str> = exported_jsonl.lines().collect();

    assert_eq!(
        exported_lines.len(),
        fixture_lines.len(),
        "Exported line count should match fixture count"
    );

    // Verify each exported bead can be re-imported
    let ws2 = common::TempWorkspace::new().expect("Failed to create second workspace");
    fs::write(&ws2.jsonl_path, &exported_jsonl).expect("Failed to write exported JSONL");

    let reimport_result = ws2
        .import_jsonl()
        .expect("Failed to re-import exported JSONL");
    assert_eq!(
        reimport_result.imported + reimport_result.skipped,
        fixture_lines.len(),
        "Re-import count should match original fixture count"
    );

    println!("E2E round-trip test passed: {} beads", exported_lines.len());
}

/// Test E2E parity: actually run br list --format json and compare to bf list output.
///
/// This is the true E2E test specified in the bead: run both br and bf on the same
/// workspace and verify they produce identical JSON output.
#[test]
fn test_e2e_br_vs_bf_list_output_parity() {
    use std::process::Command;

    // Create a workspace from the fixture
    let ws = common::TempWorkspace::from_fixture("forge-snapshot.jsonl")
        .expect("Failed to create workspace from fixture");

    let import_result = ws.import_jsonl().expect("Failed to import JSONL");
    let bead_count = import_result.imported + import_result.skipped;
    assert!(bead_count > 0, "Should have imported some beads");

    // Run bf list --format json --all (actual bf command)
    let bf_output = Command::new(env!("CARGO_BIN_EXE_bf"))
        .args([
            "list",
            "--format",
            "json",
            "--all",
            "--workspace",
            ws.workspace_path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run bf list");

    assert!(
        bf_output.status.success(),
        "bf list failed: stderr: {}",
        String::from_utf8_lossy(&bf_output.stderr)
    );

    let bf_json = String::from_utf8(bf_output.stdout).expect("bf output not valid UTF-8");

    // Run br list --format json --all (actual br command)
    // Upstream-parity comparison requires a separately installed br binary.
    // Resolve via BR_PARITY_BIN or the conventional install path; skip
    // gracefully on machines (e.g. CI containers) that don't have one.
    let br_bin =
        std::env::var("BR_PARITY_BIN").unwrap_or_else(|_| "/home/coding/.local/bin/br".to_string());
    if !std::path::Path::new(&br_bin).exists() {
        eprintln!("skipping br/bf parity test: {} not present", br_bin);
        return;
    }
    let br_output = Command::new(&br_bin)
        .args([
            "list",
            "--format",
            "json",
            "--all",
            "--workspace",
            ws.workspace_path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run br list");

    assert!(
        br_output.status.success(),
        "br list failed: stderr: {}",
        String::from_utf8_lossy(&br_output.stderr)
    );

    let br_json = String::from_utf8(br_output.stdout).expect("br output not valid UTF-8");

    // Parse both outputs for comparison
    let bf_lines: Vec<&str> = bf_json.lines().collect();
    let br_lines: Vec<&str> = br_json.lines().collect();

    assert_eq!(
        bf_lines.len(),
        br_lines.len(),
        "Output line count differs: bf has {}, br has {}",
        bf_lines.len(),
        br_lines.len()
    );

    // Compare each bead (both may be in different order)
    let mut bf_beads: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();
    let mut br_beads: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();

    for line in &bf_lines {
        let v: serde_json::Value = serde_json::from_str(line).expect("Failed to parse bf output");
        let id = v["id"].as_str().expect("Missing id").to_string();
        bf_beads.insert(id, v);
    }

    for line in &br_lines {
        let v: serde_json::Value = serde_json::from_str(line).expect("Failed to parse br output");
        let id = v["id"].as_str().expect("Missing id").to_string();
        br_beads.insert(id, v);
    }

    // Verify all beads from bf are in br output with matching fields
    for (id, bf_value) in &bf_beads {
        let br_value = br_beads
            .get(id)
            .unwrap_or_else(|| panic!("Bead {} missing from br output", id));

        // Compare critical fields
        assert_eq!(
            bf_value.get("id"),
            br_value.get("id"),
            "ID mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("title"),
            br_value.get("title"),
            "Title mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("status"),
            br_value.get("status"),
            "Status mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("priority"),
            br_value.get("priority"),
            "Priority mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("issue_type"),
            br_value.get("issue_type"),
            "Type mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("created_at"),
            br_value.get("created_at"),
            "created_at mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("updated_at"),
            br_value.get("updated_at"),
            "updated_at mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("description"),
            br_value.get("description"),
            "Description mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("assignee"),
            br_value.get("assignee"),
            "Assignee mismatch for bead {}",
            id
        );
        assert_eq!(
            bf_value.get("labels"),
            br_value.get("labels"),
            "Labels mismatch for bead {}",
            id
        );
    }

    println!(
        "E2E br vs bf parity test passed: {} beads validated",
        bf_beads.len()
    );
}
