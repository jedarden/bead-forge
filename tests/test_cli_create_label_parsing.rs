// Test CLI parsing of `bf create --label P0` flag
// This test verifies that the label flag is correctly parsed through the CLI layer

use bead_forge::cli::{Cli, Commands};
use clap::Parser;

#[test]
fn test_create_label_p0_parsing() {
    // Test parsing of `bf create --label P0 --title "Test bead"`
    let args = vec![
        "bf",
        "create",
        "--label",
        "P0",
        "--title",
        "Test bead",
    ];

    let cli = Cli::parse_from(args);

    // Verify we got the Create command
    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Create { title, type_, priority, description, assignee, label, json } => {
            // Verify the label was parsed correctly
            assert_eq!(label.len(), 1, "Should have exactly one label");
            assert_eq!(label[0], "P0", "Label value should be 'P0'");
            assert_eq!(title, "Test bead", "Title should be 'Test bead'");
            assert_eq!(type_, "task", "Default type should be 'task'");
            assert_eq!(priority, 2, "Default priority should be 2");
            assert!(description.is_none(), "Description should be None");
            assert!(assignee.is_none(), "Assignee should be None");
            assert!(!json, "JSON flag should be false");
        }
        _ => panic!("Expected Create command, got a different command"),
    }
}

#[test]
fn test_create_multiple_labels_parsing() {
    // Test parsing of multiple --label flags
    let args = vec![
        "bf",
        "create",
        "--label",
        "P0",
        "--label",
        "urgent",
        "--label",
        "frontend",
        "--title",
        "Multi-label bead",
    ];

    let cli = Cli::parse_from(args);

    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Create { label, .. } => {
            assert_eq!(label.len(), 3, "Should have exactly three labels");
            assert!(label.contains(&"P0".to_string()), "Should contain 'P0' label");
            assert!(label.contains(&"urgent".to_string()), "Should contain 'urgent' label");
            assert!(label.contains(&"frontend".to_string()), "Should contain 'frontend' label");
        }
        _ => panic!("Expected Create command, got a different command"),
    }
}

#[test]
fn test_create_no_label_parsing() {
    // Test parsing when no --label flag is provided
    let args = vec![
        "bf",
        "create",
        "--title",
        "Bead without labels",
    ];

    let cli = Cli::parse_from(args);

    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Create { label, .. } => {
            assert_eq!(label.len(), 0, "Should have no labels when --label is not provided");
        }
        _ => panic!("Expected Create command, got a different command"),
    }
}

#[test]
fn test_create_label_with_other_options() {
    // Test parsing of --label with other create options
    let args = vec![
        "bf",
        "create",
        "--label",
        "P0",
        "--title",
        "Complex bead",
        "--type",
        "bug",
        "--priority",
        "0",
        "--description",
        "This is a critical bug",
        "--assignee",
        "john-doe",
        "--json",
    ];

    let cli = Cli::parse_from(args);

    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Create { title, type_, priority, description, assignee, label, json } => {
            assert_eq!(label.len(), 1, "Should have exactly one label");
            assert_eq!(label[0], "P0", "Label value should be 'P0'");
            assert_eq!(title, "Complex bead", "Title should be correct");
            assert_eq!(type_, "bug", "Type should be 'bug'");
            assert_eq!(priority, 0, "Priority should be 0");
            assert_eq!(description, Some("This is a critical bug".to_string()), "Description should be correct");
            assert_eq!(assignee, Some("john-doe".to_string()), "Assignee should be correct");
            assert!(json, "JSON flag should be true");
        }
        _ => panic!("Expected Create command, got a different command"),
    }
}

#[test]
fn test_create_empty_label_value() {
    // Test parsing of --label with empty string value (should still parse, even if invalid for storage)
    let args = vec![
        "bf",
        "create",
        "--label",
        "",
        "--title",
        "Bead with empty label",
    ];

    let cli = Cli::parse_from(args);

    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Create { label, .. } => {
            assert_eq!(label.len(), 1, "Should parse empty label");
            assert_eq!(label[0], "", "Empty label should be preserved");
        }
        _ => panic!("Expected Create command, got a different command"),
    }
}

#[test]
fn test_create_p0_label_specifically() {
    // Test specifically for the P0 label mentioned in the task
    let args = vec![
        "bf",
        "create",
        "--label",
        "P0",
        "--title",
        "P0 priority bead",
    ];

    let cli = Cli::parse_from(args);

    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Create { label, .. } => {
            assert_eq!(label.len(), 1, "Should have exactly one P0 label");
            assert_eq!(label[0], "P0", "Label should be exactly 'P0'");
        }
        _ => panic!("Expected Create command, got a different command"),
    }
}
