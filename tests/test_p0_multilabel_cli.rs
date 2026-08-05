// Integration test for multi-label P0 priority bead creation
// Tests the complete workflow of creating P0 beads with multiple labels via CLI
// and verifying they are correctly stored, displayed, and can be queried

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Create a temporary workspace for testing
fn setup_test_workspace() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-workspace");
    fs::create_dir_all(&workspace_dir).unwrap();
    let beads_dir = workspace_dir.join(".beads");
    fs::create_dir_all(&beads_dir).unwrap();

    let config_path = beads_dir.join("config.yaml");
    fs::write(
        &config_path,
        r#"issue_prefixes: [bf]
default_priority: 2
default_type: task
claim_ttl_minutes: 30
"#,
    )
    .unwrap();

    let metadata_path = beads_dir.join("metadata.json");
    fs::write(
        &metadata_path,
        r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
    )
    .unwrap();

    let db_path = beads_dir.join("beads.db");
    bead_forge::storage::Storage::open(&db_path).unwrap();

    (temp_dir, workspace_dir)
}

/// Get the path to the bf binary
fn get_bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf")
        .unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Run a bf command and return the output
fn run_bf_command(workspace: &Path, args: &[&str]) -> (String, String, bool) {
    let out = Command::new(get_bf_binary())
        .args(args)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf command");
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    let success = out.status.success();
    (stdout, stderr, success)
}

/// Parse JSON output from bf --json
fn parse_json_output(output: &str) -> serde_json::Value {
    serde_json::from_str(output).expect("Failed to parse JSON output")
}

#[test]
fn test_p0_create_with_multiple_labels_cli() {
    let (_temp, workspace) = setup_test_workspace();

    // Create P0 bead with multiple labels using CLI
    let (stdout, stderr, success) = run_bf_command(
        &workspace,
        &[
            "create",
            "--title",
            "Critical security fix with multiple labels",
            "--type",
            "bug",
            "--priority",
            "0",
            "--label",
            "critical",
            "--label",
            "security",
            "--label",
            "urgent",
            "--label",
            "hotfix",
            "--json",
        ],
    );

    assert!(success, "bf create command failed: {}", stderr);

    let json_output = parse_json_output(&stdout);

    // Verify creation response - output may be wrapped in envelope
    let data = if json_output.get("data").is_some() {
        json_output.get("data").unwrap()
    } else {
        &json_output
    };

    assert!(data.get("id").is_some());
    assert!(data.get("priority").is_some());
    assert!(data.get("labels").is_some());

    // Verify priority is P0
    let priority = data.get("priority").unwrap().as_i64().unwrap();
    assert_eq!(priority, 0, "P0 priority should be 0");

    // Verify all labels are present
    let labels = data
        .get("labels")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();

    assert_eq!(labels.len(), 4);
    assert!(labels.contains(&"critical".to_string()));
    assert!(labels.contains(&"security".to_string()));
    assert!(labels.contains(&"urgent".to_string()));
    assert!(labels.contains(&"hotfix".to_string()));

    let bead_id = data.get("id").unwrap().as_str().unwrap();

    // Verify we can retrieve the bead with show command
    let (show_stdout, _, show_success) =
        run_bf_command(&workspace, &["show", &bead_id, "--json"]);

    assert!(show_success, "bf show command failed");
    let show_json = parse_json_output(&show_stdout);
    let show_bead = &show_json[0];

    // Verify P0 priority and labels persist
    assert_eq!(show_bead.get("priority").unwrap().as_i64().unwrap(), 0);
    let show_labels = show_bead
        .get("labels")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(show_labels.len(), 4);
}

#[test]
fn test_p0_multiple_labels_text_format() {
    let (_temp, workspace) = setup_test_workspace();

    // Create P0 bead with multiple labels
    let (stdout, _, success) = run_bf_command(
        &workspace,
        &[
            "create",
            "--title",
            "P0 with labels in text format",
            "--type",
            "task",
            "--priority",
            "0",
            "--label",
            "critical",
            "--label",
            "backend",
            "--label",
            "performance",
        ],
    );

    assert!(success);
    let bead_id = stdout.trim();

    // Show in text format and verify labels are displayed
    let (show_stdout, _, _) = run_bf_command(&workspace, &["show", bead_id]);

    assert!(show_stdout.contains("critical"));
    assert!(show_stdout.contains("backend"));
    assert!(show_stdout.contains("performance"));
    assert!(show_stdout.contains("P0") || show_stdout.contains("priority=0"));
}

#[test]
fn test_p0_multiple_labels_search_and_filter() {
    let (_temp, workspace) = setup_test_workspace();

    // Create multiple P0 beads with different label combinations
    let test_cases = vec![
        (vec!["critical", "security"], "Security hotfix"),
        (vec!["critical", "performance"], "Performance fix"),
        (vec!["urgent", "backend"], "Backend issue"),
    ];

    for (labels, title) in &test_cases {
        let label_args: Vec<&str> = labels
            .iter()
            .flat_map(|l| vec!["--label", l])
            .collect();

        let args: Vec<&str> = vec![
            "create",
            "--title",
            title,
            "--type",
            "bug",
            "--priority",
            "0",
        ]
        .into_iter()
        .chain(label_args.into_iter())
        .collect();

        let (_, _, success) = run_bf_command(&workspace, &args);
        assert!(success);
    }

    // Search for P0 beads with "critical" label
    let (search_stdout, _, _) =
        run_bf_command(&workspace, &["search", "--label", "critical", "--priority", "0", "--format", "json"]);

    let search_json: Vec<serde_json::Value> = serde_json::from_str(&search_stdout).unwrap();
    assert!(!search_json.is_empty(), "Should find at least one P0 bead with 'critical' label");

    // Verify we can also filter by priority alone
    let (list_stdout, _, _) = run_bf_command(&workspace, &["list", "--priority", "0", "--json"]);
    let list_json: Vec<serde_json::Value> =
        serde_json::from_str(&list_stdout).unwrap();

    assert!(list_json.len() >= 3);
    for bead in &list_json {
        assert_eq!(bead.get("priority").unwrap().as_i64().unwrap(), 0);
    }
}

#[test]
fn test_p0_labels_add_and_remove_operations() {
    let (_temp, workspace) = setup_test_workspace();

    // Create P0 bead with initial labels
    let (create_stdout, _, _) = run_bf_command(
        &workspace,
        &[
            "create",
            "--title",
            "P0 label operations test",
            "--type",
            "task",
            "--priority",
            "0",
            "--label",
            "critical",
        ],
    );

    let bead_id = create_stdout.trim();

    // Add more labels
    let (_, _, add_success1) = run_bf_command(
        &workspace,
        &["label", "add", bead_id, "-l", "urgent", "-l", "security"],
    );
    assert!(add_success1);

    // Verify labels were added
    let (show_stdout, _, _) = run_bf_command(&workspace, &["show", bead_id, "--json"]);
    let show_json = parse_json_output(&show_stdout);
    let labels = show_json[0]
        .get("labels")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();

    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"critical".to_string()));
    assert!(labels.contains(&"urgent".to_string()));
    assert!(labels.contains(&"security".to_string()));

    // Remove a label
    let (_, _, remove_success) =
        run_bf_command(&workspace, &["label", "remove", bead_id, "-l", "urgent"]);
    assert!(remove_success);

    // Verify label was removed but P0 priority unchanged
    let (final_show_stdout, _, _) =
        run_bf_command(&workspace, &["show", bead_id, "--json"]);
    let final_json = parse_json_output(&final_show_stdout);

    assert_eq!(final_json[0].get("priority").unwrap().as_i64().unwrap(), 0);
    let final_labels = final_json[0]
        .get("labels")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();

    assert_eq!(final_labels.len(), 2);
    assert!(final_labels.contains(&"critical".to_string()));
    assert!(final_labels.contains(&"security".to_string()));
}

#[test]
fn test_p0_multiple_labels_persistence() {
    let (_temp, workspace) = setup_test_workspace();

    // Create P0 bead with multiple labels
    let (stdout, _, _) = run_bf_command(
        &workspace,
        &[
            "create",
            "--title",
            "P0 persistence test",
            "--type",
            "feature",
            "--priority",
            "0",
            "--label",
            "critical",
            "--label",
            "database",
            "--label",
            "scaling",
        ],
    );

    let bead_id = stdout.trim();

    // Force flush to JSONL
    let (_, _, sync_success) = run_bf_command(&workspace, &["sync", "--flush-only"]);
    assert!(sync_success);

    // Verify the data was written to JSONL
    let jsonl_path = workspace.join(".beads").join("issues.jsonl");
    let jsonl_content = fs::read_to_string(&jsonl_path).unwrap();

    assert!(jsonl_content.contains("\"priority\":0"));
    assert!(jsonl_content.contains("critical"));
    assert!(jsonl_content.contains("database"));
    assert!(jsonl_content.contains("scaling"));

    // Import from JSONL to verify roundtrip
    let (_, _, import_success) = run_bf_command(&workspace, &["sync", "--import-only"]);
    assert!(import_success);

    // Show the bead to verify it persisted correctly
    let (show_stdout, _, _) = run_bf_command(&workspace, &["show", bead_id, "--json"]);
    let show_json = parse_json_output(&show_stdout);

    assert_eq!(show_json[0].get("priority").unwrap().as_i64().unwrap(), 0);
    let labels = show_json[0]
        .get("labels")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();

    assert_eq!(labels.len(), 3);
}

#[test]
fn test_p0_batch_operations_with_labels() {
    let (_temp, workspace) = setup_test_workspace();

    // Create batch operations with P0 priority and labels
    let batch_json = r#"[
        {"op": "create", "title": "P0 batch test 1", "type": "task", "priority": 0, "labels": ["critical", "batch"]},
        {"op": "create", "title": "P0 batch test 2", "type": "bug", "priority": 0, "labels": ["urgent", "batch"]},
        {"op": "create", "title": "P0 batch test 3", "type": "feature", "priority": 0, "labels": ["critical", "hotfix"]}
    ]"#;

    // Write batch JSON to stdin
    let mut child = Command::new(get_bf_binary())
        .args(&["batch", "--stdin"])
        .current_dir(&workspace)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    use std::io::Write;
    let stdin = child.stdin.as_mut().unwrap();
    stdin.write_all(batch_json.as_bytes()).unwrap();
    drop(stdin);

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());

    // Verify all beads were created
    let (list_stdout, _, _) = run_bf_command(&workspace, &["list", "--priority", "0", "--json"]);
    let list_json: Vec<serde_json::Value> =
        serde_json::from_str(&list_stdout).unwrap();

    assert_eq!(list_json.len(), 3);
    for bead in &list_json {
        assert_eq!(bead.get("priority").unwrap().as_i64().unwrap(), 0);
        let labels = bead
            .get("labels")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        assert!(!labels.is_empty());
        assert!(labels.contains(&"batch".to_string()) ||
                labels.contains(&"critical".to_string()) ||
                labels.contains(&"hotfix".to_string()));
    }
}

#[test]
fn test_p0_various_label_counts() {
    let (_temp, workspace) = setup_test_workspace();

    // Test creating P0 beads with varying numbers of labels
    for count in &[1, 2, 5, 10] {
        let labels: Vec<String> = (0..*count)
            .map(|i| format!("label-{}", i))
            .collect();

        let label_args: Vec<String> = labels
            .iter()
            .flat_map(|l| vec!["--label".to_string(), l.clone()])
            .collect();

        let title = format!("P0 with {} labels", count);
        let args: Vec<String> = vec![
            "create".to_string(),
            "--title".to_string(),
            title,
            "--type".to_string(),
            "task".to_string(),
            "--priority".to_string(),
            "0".to_string(),
        ]
        .into_iter()
        .chain(label_args.into_iter())
        .collect();

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let (stdout, _, success) = run_bf_command(&workspace, &args_refs);
        assert!(success, "Failed to create P0 bead with {} labels", count);

        let bead_id = stdout.trim();

        // Verify all labels are present
        let (show_stdout, _, _) = run_bf_command(&workspace, &["show", bead_id, "--json"]);
        let show_json = parse_json_output(&show_stdout);

        assert_eq!(show_json[0].get("priority").unwrap().as_i64().unwrap(), 0);
        let show_labels = show_json[0]
            .get("labels")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect::<Vec<_>>();

        assert_eq!(show_labels.len(), *count);
    }
}

#[test]
fn test_p0_label_edge_cases() {
    let (_temp, workspace) = setup_test_workspace();

    // Test 1: P0 bead with no labels (should work)
    let (stdout, _, success) = run_bf_command(
        &workspace,
        &[
            "create",
            "--title",
            "P0 with no labels",
            "--type",
            "task",
            "--priority",
            "0",
        ],
    );
    assert!(success);
    let bead_id = stdout.trim();

    let (show_stdout, _, _) = run_bf_command(&workspace, &["show", bead_id, "--json"]);
    let show_json = parse_json_output(&show_stdout);
    assert_eq!(show_json[0].get("priority").unwrap().as_i64().unwrap(), 0);
    assert_eq!(
        show_json[0]
            .get("labels")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );

    // Test 2: Adding duplicate labels should not create duplicates
    let (_, _, add1_success) = run_bf_command(
        &workspace,
        &["label", "add", bead_id, "-l", "test"],
    );
    assert!(add1_success);

    let (_, _, add2_success) = run_bf_command(
        &workspace,
        &["label", "add", bead_id, "-l", "test"],
    );
    assert!(add2_success);

    let (final_show_stdout, _, _) =
        run_bf_command(&workspace, &["show", bead_id, "--json"]);
    let final_json = parse_json_output(&final_show_stdout);

    // Should only have one "test" label
    let labels: Vec<String> = final_json[0]
        .get("labels")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    let test_count = labels.iter().filter(|l| *l == "test").count();
    assert_eq!(test_count, 1, "Duplicate labels should not be added");
}
