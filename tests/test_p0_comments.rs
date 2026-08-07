//! P0 Comment Test Suite
//!
//! Comprehensive smoke tests for comment functionality.
//! Tests core comment operations: add, list, persistence, edge cases, and CLI parsing.

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

    (temp_dir, beads_dir)
}

/// Get the path to the bf binary
fn get_bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Extract bead ID from command output
fn extract_bead_id(output: &str) -> String {
    output
        .lines()
        .find(|line| line.contains("bf-"))
        .and_then(|line| line.split("bf-").nth(1))
        .map(|id| format!("bf-{}", id.trim().split_whitespace().next().unwrap_or(id)))
        .expect("Could not extract bead ID from output")
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

// ============================================================================
// Test 1: Basic comment add and list round-trip
// ============================================================================

#[test]
fn test_p0_comment_add_and_list_basic() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create a bead
    let (stdout, stderr, success) =
        run_bf_command(workspace, &["create", "--title", "P0 Comment Basic Test"]);
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Add a comment
    let (comment_out, comment_err, comment_ok) =
        run_bf_command(workspace, &["comments", "add", &bead_id, "Test comment"]);
    assert!(comment_ok, "comment add failed: {}", comment_err);
    assert!(comment_out.contains("Added comment"), "add did not confirm: {}", comment_out);

    // List comments
    let (list_out, list_err, list_ok) = run_bf_command(workspace, &["comments", "list", &bead_id]);
    assert!(list_ok, "comments list failed: {}", list_err);
    assert!(
        list_out.contains("Test comment"),
        "comment body missing from list: {}",
        list_out
    );
}

// ============================================================================
// Test 2: Multiple comments preserve insertion order
// ============================================================================

#[test]
fn test_p0_comments_preserve_insertion_order() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let (stdout, _, success) = run_bf_command(workspace, &["create", "--title", "Multi-comment test"]);
    assert!(success);
    let bead_id = extract_bead_id(&stdout);

    let comments = ["First", "Second", "Third"];
    for comment in &comments {
        let (_, err, ok) = run_bf_command(workspace, &["comments", "add", &bead_id, comment]);
        assert!(ok, "Failed to add comment '{}': {}", comment, err);
    }

    let (list_out, err, ok) = run_bf_command(workspace, &["comments", "list", &bead_id]);
    assert!(ok, "comments list failed: {}", err);

    // Verify order is preserved
    let first = list_out.find("First").expect("First comment missing");
    let second = list_out.find("Second").expect("Second comment missing");
    let third = list_out.find("Third").expect("Third comment missing");
    assert!(first < second && second < third, "Order not preserved: {}", list_out);
}

// ============================================================================
// Test 3: Empty comment list shows appropriate message
// ============================================================================

#[test]
fn test_p0_empty_comment_list_message() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let (stdout, _, success) = run_bf_command(workspace, &["create", "--title", "No comments bead"]);
    assert!(success);
    let bead_id = extract_bead_id(&stdout);

    let (list_out, err, ok) = run_bf_command(workspace, &["comments", "list", &bead_id]);
    assert!(ok, "comments list failed: {}", err);
    assert!(
        list_out.contains("No comments") || list_out.contains("no comments"),
        "Expected 'No comments' message, got: {}",
        list_out
    );
}

// ============================================================================
// Test 4: Comment with multiple words (arg joining)
// ============================================================================

#[test]
fn test_p0_comment_multiple_words() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let (stdout, _, success) = run_bf_command(workspace, &["create", "--title", "Multi-word comment test"]);
    assert!(success);
    let bead_id = extract_bead_id(&stdout);

    // Add comment with multiple args
    let (_, err, ok) = run_bf_command(
        workspace,
        &["comments", "add", &bead_id, "multi", "word", "comment", "here"],
    );
    assert!(ok, "comment add failed: {}", err);

    let (list_out, err, ok) = run_bf_command(workspace, &["comments", "list", &bead_id]);
    assert!(ok, "comments list failed: {}", err);
    assert!(
        list_out.contains("multi word comment here"),
        "joined text not round-tripped: {}",
        list_out
    );
}

// ============================================================================
// Test 5: Comment persistence across operations
// ============================================================================

#[test]
fn test_p0_comment_persistence() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let (stdout, _, success) = run_bf_command(workspace, &["create", "--title", "Comment persistence test"]);
    assert!(success);
    let bead_id = extract_bead_id(&stdout);

    // Add comment
    let (_, err, ok) = run_bf_command(workspace, &["comments", "add", &bead_id, "persistent comment"]);
    assert!(ok, "comment add failed: {}", err);

    // Update bead to trigger storage operation
    let (_, err, ok) = run_bf_command(workspace, &["update", &bead_id, "--status", "blocked"]);
    assert!(ok, "update failed: {}", err);

    // Verify comment persists after update
    let (list_out, err, ok) = run_bf_command(workspace, &["comments", "list", &bead_id]);
    assert!(ok, "comments list failed: {}", err);
    assert!(
        list_out.contains("persistent comment"),
        "comment not persisted after update: {}",
        list_out
    );
}

// ============================================================================
// Test 6: Comment persistence after multiple operations
// ============================================================================

#[test]
fn test_p0_comment_persistence_after_updates() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let (stdout, _, success) = run_bf_command(workspace, &["create", "--title", "Comment persistence test"]);
    assert!(success);
    let bead_id = extract_bead_id(&stdout);

    // Add comment
    let (_, err, ok) = run_bf_command(workspace, &["comments", "add", &bead_id, "persistent comment"]);
    assert!(ok, "comment add failed: {}", err);

    // Update bead multiple times
    let (_, err, ok) = run_bf_command(workspace, &["update", &bead_id, "--status", "blocked"]);
    assert!(ok, "update failed: {}", err);

    let (_, err, ok) = run_bf_command(workspace, &["update", &bead_id, "--priority", "0"]);
    assert!(ok, "update failed: {}", err);

    // Verify comment persists after multiple updates
    let (list_out, err, ok) = run_bf_command(workspace, &["comments", "list", &bead_id]);
    assert!(ok, "comments list failed: {}", err);
    assert!(
        list_out.contains("persistent comment"),
        "comment not persisted after updates: {}",
        list_out
    );
}

// ============================================================================
// Test 7: Multiple beads with comments (isolation)
// ============================================================================

#[test]
fn test_p0_comments_isolated_between_beads() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create two beads
    let (out1, _, success) = run_bf_command(workspace, &["create", "--title", "Bead 1"]);
    assert!(success);
    let id1 = extract_bead_id(&out1);

    let (out2, _, success) = run_bf_command(workspace, &["create", "--title", "Bead 2"]);
    assert!(success);
    let id2 = extract_bead_id(&out2);

    // Add different comments to each
    let (_, err, ok) = run_bf_command(workspace, &["comments", "add", &id1, "Comment for bead 1"]);
    assert!(ok, "comment add to bead 1 failed: {}", err);

    let (_, err, ok) = run_bf_command(workspace, &["comments", "add", &id2, "Comment for bead 2"]);
    assert!(ok, "comment add to bead 2 failed: {}", err);

    // Verify isolation
    let (list1, err, ok) = run_bf_command(workspace, &["comments", "list", &id1]);
    assert!(ok, "comments list for bead 1 failed: {}", err);
    assert!(
        list1.contains("Comment for bead 1"),
        "bead 1 missing its comment: {}",
        list1
    );
    assert!(
        !list1.contains("Comment for bead 2"),
        "bead 1 has bead 2's comment: {}",
        list1
    );

    let (list2, err, ok) = run_bf_command(workspace, &["comments", "list", &id2]);
    assert!(ok, "comments list for bead 2 failed: {}", err);
    assert!(
        list2.contains("Comment for bead 2"),
        "bead 2 missing its comment: {}",
        list2
    );
    assert!(
        !list2.contains("Comment for bead 1"),
        "bead 2 has bead 1's comment: {}",
        list2
    );
}

// ============================================================================
// Test 8: Comment with special characters
// ============================================================================

#[test]
fn test_p0_comment_special_characters() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let (stdout, _, success) = run_bf_command(workspace, &["create", "--title", "Special chars comment test"]);
    assert!(success);
    let bead_id = extract_bead_id(&stdout);

    let special_comments = [
        "Comment with @mention",
        "Comment with #hash",
        "Comment with $ymbol",
        "Comment with &amp",
        "Comment with emoji 🐛",
    ];

    for comment in &special_comments {
        let (_, err, ok) = run_bf_command(workspace, &["comments", "add", &bead_id, comment]);
        assert!(ok, "Failed to add special comment '{}': {}", comment, err);
    }

    // Verify all special comments are stored
    let (list_out, err, ok) = run_bf_command(workspace, &["comments", "list", &bead_id]);
    assert!(ok, "comments list failed: {}", err);
    for comment in &special_comments {
        assert!(
            list_out.contains(comment),
            "Special comment '{}' not found in list: {}",
            comment,
            list_out
        );
    }
}

// ============================================================================
// Test 9: Comment with unicode characters
// ============================================================================

#[test]
fn test_p0_comment_unicode() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let (stdout, _, success) = run_bf_command(workspace, &["create", "--title", "Unicode comment test"]);
    assert!(success);
    let bead_id = extract_bead_id(&stdout);

    let unicode_comments = [
        "评论测试",
        "テストコメント",
        "Комментарий",
        "تحقيق",
        "🔥 P0 issue 🚨",
    ];

    for comment in &unicode_comments {
        let (_, err, ok) = run_bf_command(workspace, &["comments", "add", &bead_id, comment]);
        assert!(ok, "Failed to add unicode comment '{}': {}", comment, err);
    }

    let (list_out, err, ok) = run_bf_command(workspace, &["comments", "list", &bead_id]);
    assert!(ok, "comments list failed: {}", err);
    for comment in &unicode_comments {
        assert!(
            list_out.contains(comment),
            "Unicode comment '{}' not found: {}",
            comment,
            list_out
        );
    }
}

// ============================================================================
// Test 10: Long comment text
// ============================================================================

#[test]
fn test_p0_long_comment() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let (stdout, _, success) = run_bf_command(workspace, &["create", "--title", "Long comment test"]);
    assert!(success);
    let bead_id = extract_bead_id(&stdout);

    let long_comment = "This is a very long comment that contains multiple sentences and detailed information about the issue. It should be stored completely and retrieved without truncation. Testing that long comments work correctly is important for real-world usage where users might leave detailed feedback or explanations.";

    let (_, err, ok) = run_bf_command(workspace, &["comments", "add", &bead_id, long_comment]);
    assert!(ok, "Failed to add long comment: {}", err);

    let (list_out, err, ok) = run_bf_command(workspace, &["comments", "list", &bead_id]);
    assert!(ok, "comments list failed: {}", err);
    assert!(
        list_out.contains(long_comment),
        "Long comment was truncated: {}",
        list_out
    );
}

// ============================================================================
// Test 11: Comment on different bead types
// ============================================================================

#[test]
fn test_p0_comments_on_different_bead_types() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let types = [
        ("task", "task"),
        ("bug", "bug"),
        ("feature", "feature"),
        ("epic", "epic"),
    ];

    for (bead_type, type_arg) in types {
        let (stdout, _, success) = run_bf_command(
            workspace,
            &["create", "--title", &format!("{} comment test", bead_type), "--type", type_arg],
        );
        assert!(success, "Failed to create {} bead", bead_type);
        let bead_id = extract_bead_id(&stdout);

        let (_, err, ok) = run_bf_command(
            workspace,
            &["comments", "add", &bead_id, &format!("Comment on {}", bead_type)],
        );
        assert!(ok, "Failed to add comment to {} bead: {}", bead_type, err);

        let (list_out, err, ok) = run_bf_command(workspace, &["comments", "list", &bead_id]);
        assert!(ok, "Failed to list comments for {} bead: {}", bead_type, err);
        assert!(
            list_out.contains(&format!("Comment on {}", bead_type)),
            "Comment not found for {} bead: {}",
            bead_type,
            list_out
        );
    }
}

// ============================================================================
// Test 12: Error handling - comment on non-existent bead
// ============================================================================

#[test]
fn test_p0_comment_nonexistent_bead() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let (_, stderr, success) =
        run_bf_command(workspace, &["comments", "add", "bf-nonexistent", "Test comment"]);
    assert!(!success, "Adding comment to non-existent bead should fail");
    assert!(
        stderr.contains("not found") || stderr.contains("does not exist") || stderr.contains("No bead"),
        "Expected error message for non-existent bead, got: {}",
        stderr
    );
}

// ============================================================================
// Test 13: Error handling - empty comment
// ============================================================================

#[test]
fn test_p0_empty_comment() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let (stdout, _, success) = run_bf_command(workspace, &["create", "--title", "Empty comment test"]);
    assert!(success);
    let bead_id = extract_bead_id(&stdout);

    // Try to add empty comment (should fail or be handled gracefully)
    let (_, _stderr, _result) = run_bf_command(workspace, &["comments", "add", &bead_id, ""]);

    // Empty comment should either fail or add an empty string
    // We just verify it doesn't crash
    let (list_out, err, ok) = run_bf_command(workspace, &["comments", "list", &bead_id]);
    assert!(ok, "comments list failed after empty comment: {}", err);
    // Output should be valid even if empty
    assert!(!list_out.contains("panic"), "Should not panic on empty comment");
}

// ============================================================================
// Test 14: Comment count aggregation
// ============================================================================

#[test]
fn test_p0_comment_count_multiple_beads() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create beads with varying comment counts
    let beads_data = vec![
        ("bead-0-comments", 0),
        ("bead-1-comment", 1),
        ("bead-3-comments", 3),
        ("bead-5-comments", 5),
    ];

    for (title, count) in &beads_data {
        let (stdout, _, success) = run_bf_command(workspace, &["create", "--title", title]);
        assert!(success);
        let bead_id = extract_bead_id(&stdout);

        for i in 0..*count {
            let comment = format!("Comment {} on {}", i + 1, title);
            let (_, err, ok) = run_bf_command(workspace, &["comments", "add", &bead_id, &comment]);
            assert!(ok, "Failed to add comment '{}' to {}: {}", comment, title, err);
        }
    }

    // Verify each bead has the correct number of comments
    for (title, expected_count) in &beads_data {
        // Re-fetch the bead ID since we didn't store them
        let (list_out, err, ok) = run_bf_command(workspace, &["list", "--json"]);
        assert!(ok, "list failed: {}", err);

        // Parse JSONL (one JSON object per line) to find bead IDs
        let mut bead_id = String::new();
        for line in list_out.lines() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                if json["title"].as_str() == Some(*title) {
                    bead_id = json["id"].as_str().unwrap().to_string();
                    break;
                }
            }
        }
        assert!(!bead_id.is_empty(), "Bead with title '{}' not found in list", title);

        let (comments_out, err, ok) = run_bf_command(workspace, &["comments", "list", &bead_id]);
        assert!(ok, "comments list failed for {}: {}", title, err);

        // Count occurrences of "Comment" in output
        let actual_count = comments_out.matches("Comment").count();
        assert_eq!(
            actual_count, *expected_count,
            "Bead {} should have {} comments, found {}: {}",
            title, expected_count, actual_count, comments_out
        );
    }
}

// ============================================================================
// Test 15: Comment show integration
// ============================================================================

#[test]
fn test_p0_comment_appears_in_show() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let (stdout, _, success) = run_bf_command(workspace, &["create", "--title", "Show comment test"]);
    assert!(success);
    let bead_id = extract_bead_id(&stdout);

    let (_, err, ok) = run_bf_command(workspace, &["comments", "add", &bead_id, "Visible in show"]);
    assert!(ok, "comment add failed: {}", err);

    // Show bead with comments
    let (show_out, err, ok) = run_bf_command(workspace, &["show", &bead_id]);
    assert!(ok, "show failed: {}", err);
    assert!(
        show_out.contains("Visible in show") || show_out.contains("Comments:"),
        "Comment or comments section should appear in show output: {}",
        show_out
    );
}
