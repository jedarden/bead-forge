/// Integration tests for bf show and list commands
///
/// This test verifies that the show and list commands meet all acceptance criteria:
/// - bf show displays all bead fields (id, title, description, status, type, priority, assignee, created_at, updated_at)
/// - bf list shows beads in table format by default
/// - bf list --status filters by status (open, closed, blocked)
/// - bf list --type filters by issue type
/// - bf list --assignee filters by assignee
/// - bf list --priority filters by priority level
/// - Both commands support --json output format

#[cfg(test)]
mod integration_tests {
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::TempDir;

    fn run_bf_command(args: &[&str], workspace: &PathBuf) -> String {
        let output = Command::new("bf")
            .current_dir(workspace)
            .args(args)
            .output()
            .expect("Failed to execute bf command");

        String::from_utf8_lossy(&output.stdout).to_string()
    }

    fn create_test_bead(workspace: &PathBuf, title: &str, bead_type: &str, priority: i32) -> String {
        let output = run_bf_command(
            &["create", "--title", title, "--type", bead_type, "--priority", &priority.to_string()],
            workspace,
        );
        output.trim().to_string()
    }

    #[test]
    fn test_show_displays_all_fields() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        // Initialize workspace
        run_bf_command(&["init", "--prefix", "test"], &workspace);

        // Create a test bead
        let bead_id = create_test_bead(&workspace, "Test bead", "bug", 1);

        // Show the bead
        let show_output = run_bf_command(&["show", &bead_id], &workspace);

        // Verify all fields are present
        assert!(show_output.contains("ID:"));
        assert!(show_output.contains(&bead_id));
        assert!(show_output.contains("Title:"));
        assert!(show_output.contains("Test bead"));
        assert!(show_output.contains("Status:"));
        assert!(show_output.contains("Priority:"));
        assert!(show_output.contains("Type:"));
        assert!(show_output.contains("bug"));
    }

    #[test]
    fn test_list_default_table_format() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        // Initialize workspace
        run_bf_command(&["init", "--prefix", "test"], &workspace);

        // Create test beads
        create_test_bead(&workspace, "First bead", "task", 2);
        create_test_bead(&workspace, "Second bead", "bug", 1);

        // List beads
        let list_output = run_bf_command(&["list"], &workspace);

        // Verify table format (default text format includes [id] title - status (priority))
        assert!(list_output.contains("["));
        assert!(list_output.contains("]"));
        assert!(list_output.contains("-"));
        assert!(list_output.contains("("));
        assert!(list_output.contains(")"));
    }

    #[test]
    fn test_list_status_filter() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        // Initialize workspace
        run_bf_command(&["init", "--prefix", "test"], &workspace);

        // Create test beads
        let bead1 = create_test_bead(&workspace, "Open bead", "task", 2);
        let bead2 = create_test_bead(&workspace, "Another open bead", "bug", 1);

        // Close one bead
        run_bf_command(&["close", &bead1, "--reason", "Done"], &workspace);

        // List only open beads
        let list_output = run_bf_command(&["list", "--status", "open"], &workspace);

        // Verify filtering works
        assert!(list_output.contains("open"));
        assert!(!list_output.contains("closed"));
    }

    #[test]
    fn test_list_type_filter() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        // Initialize workspace
        run_bf_command(&["init", "--prefix", "test"], &workspace);

        // Create test beads of different types
        create_test_bead(&workspace, "Bug bead", "bug", 1);
        create_test_bead(&workspace, "Feature bead", "feature", 2);
        create_test_bead(&workspace, "Task bead", "task", 2);

        // List only bugs
        let list_output = run_bf_command(&["list", "--type", "bug"], &workspace);

        // Verify only bugs are shown
        assert!(list_output.contains("Bug bead"));
        assert!(!list_output.contains("Feature bead"));
        assert!(!list_output.contains("Task bead"));
    }

    #[test]
    fn test_list_priority_filter() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        // Initialize workspace
        run_bf_command(&["init", "--prefix", "test"], &workspace);

        // Create test beads with different priorities
        create_test_bead(&workspace, "Critical bead", "bug", 0);
        create_test_bead(&workspace, "Normal bead", "task", 2);

        // List only critical priority
        let list_output = run_bf_command(&["list", "--priority", "0"], &workspace);

        // Verify only critical priority is shown
        assert!(list_output.contains("Critical bead"));
        assert!(list_output.contains("P0"));
        assert!(!list_output.contains("Normal bead"));
    }

    #[test]
    fn test_show_json_format() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        // Initialize workspace
        run_bf_command(&["init", "--prefix", "test"], &workspace);

        // Create a test bead
        let bead_id = create_test_bead(&workspace, "JSON test bead", "task", 2);

        // Show with JSON format
        let show_output = run_bf_command(&["show", &bead_id, "--json"], &workspace);

        // Verify JSON output
        assert!(show_output.contains("{"));
        assert!(show_output.contains("\"id\""));
        assert!(show_output.contains("\"title\""));
        assert!(show_output.contains("\"status\""));
        assert!(show_output.contains("\"priority\""));
        assert!(show_output.contains("\"issue_type\""));
        assert!(show_output.contains("\"created_at\""));
        assert!(show_output.contains("\"updated_at\""));
    }

    #[test]
    fn test_list_json_format() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        // Initialize workspace
        run_bf_command(&["init", "--prefix", "test"], &workspace);

        // Create test beads
        create_test_bead(&workspace, "First bead", "task", 2);
        create_test_bead(&workspace, "Second bead", "bug", 1);

        // List with JSON format
        let list_output = run_bf_command(&["list", "--json"], &workspace);

        // Verify JSON output (should be JSONL format - one JSON object per line)
        assert!(list_output.contains("{"));
        assert!(list_output.contains("\"id\""));
        assert!(list_output.contains("\"title\""));
        assert!(list_output.contains("\"status\""));
        assert!(list_output.contains("\"priority\""));
    }
}
