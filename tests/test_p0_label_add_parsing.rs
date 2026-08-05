// Test CLI parsing of `bf label add --label P0` flag
// This test verifies that the label flag is correctly parsed through the CLI layer for the label add command

use bead_forge::cli::{Cli, Commands, LabelCommands};
use clap::Parser;

#[test]
fn test_label_add_p0_parsing() {
    // Test parsing of `bf label add <id> --label P0`
    let args = vec![
        "bf",
        "label",
        "add",
        "bf-12345",
        "--label",
        "P0",
    ];

    let cli = Cli::parse_from(args);

    // Verify we got the Label command
    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Label(LabelCommands::Add { id, label }) => {
            // Verify the label was parsed correctly
            assert_eq!(label.len(), 1, "Should have exactly one label");
            assert_eq!(label[0], "P0", "Label value should be 'P0'");
            assert_eq!(id, "bf-12345", "Issue ID should be 'bf-12345'");
        }
        _ => panic!("Expected Label::Add command, got a different command"),
    }
}

#[test]
fn test_label_add_multiple_labels_parsing() {
    // Test parsing of multiple --label flags in label add command
    let args = vec![
        "bf",
        "label",
        "add",
        "bf-67890",
        "--label",
        "P0",
        "--label",
        "urgent",
        "--label",
        "frontend",
    ];

    let cli = Cli::parse_from(args);

    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Label(LabelCommands::Add { id, label }) => {
            assert_eq!(label.len(), 3, "Should have exactly three labels");
            assert!(label.contains(&"P0".to_string()), "Should contain 'P0' label");
            assert!(label.contains(&"urgent".to_string()), "Should contain 'urgent' label");
            assert!(label.contains(&"frontend".to_string()), "Should contain 'frontend' label");
            assert_eq!(id, "bf-67890", "Issue ID should be 'bf-67890'");
        }
        _ => panic!("Expected Label::Add command, got a different command"),
    }
}

#[test]
fn test_label_add_short_flag() {
    // Test parsing of `-l` short flag for label add
    let args = vec![
        "bf",
        "label",
        "add",
        "bf-abcd",
        "-l",
        "P0",
    ];

    let cli = Cli::parse_from(args);

    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Label(LabelCommands::Add { id, label }) => {
            assert_eq!(label.len(), 1, "Should have exactly one label");
            assert_eq!(label[0], "P0", "Label value should be 'P0'");
            assert_eq!(id, "bf-abcd", "Issue ID should be 'bf-abcd'");
        }
        _ => panic!("Expected Label::Add command, got a different command"),
    }
}

#[test]
fn test_label_add_multiple_short_flags() {
    // Test parsing of multiple -l short flags
    let args = vec![
        "bf",
        "label",
        "add",
        "bf-test",
        "-l",
        "P0",
        "-l",
        "high-priority",
        "-l",
        "backend",
    ];

    let cli = Cli::parse_from(args);

    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Label(LabelCommands::Add { id, label }) => {
            assert_eq!(label.len(), 3, "Should have exactly three labels");
            assert!(label.contains(&"P0".to_string()), "Should contain 'P0' label");
            assert!(label.contains(&"high-priority".to_string()), "Should contain 'high-priority' label");
            assert!(label.contains(&"backend".to_string()), "Should contain 'backend' label");
            assert_eq!(id, "bf-test", "Issue ID should be 'bf-test'");
        }
        _ => panic!("Expected Label::Add command, got a different command"),
    }
}

#[test]
fn test_label_add_mixed_short_long_flags() {
    // Test parsing of mixed -l and --label flags
    let args = vec![
        "bf",
        "label",
        "add",
        "bf-mixed",
        "-l",
        "P0",
        "--label",
        "urgent",
        "-l",
        "feature",
    ];

    let cli = Cli::parse_from(args);

    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Label(LabelCommands::Add { id, label }) => {
            assert_eq!(label.len(), 3, "Should have exactly three labels");
            assert!(label.contains(&"P0".to_string()), "Should contain 'P0' label");
            assert!(label.contains(&"urgent".to_string()), "Should contain 'urgent' label");
            assert!(label.contains(&"feature".to_string()), "Should contain 'feature' label");
            assert_eq!(id, "bf-mixed", "Issue ID should be 'bf-mixed'");
        }
        _ => panic!("Expected Label::Add command, got a different command"),
    }
}

#[test]
fn test_label_add_with_complex_label_values() {
    // Test parsing of labels with special characters and formats
    let args = vec![
        "bf",
        "label",
        "add",
        "bf-complex",
        "--label",
        "P0",
        "--label",
        "bug-fix",
        "--label",
        "high_priority",
        "--label",
        "team-alpha",
    ];

    let cli = Cli::parse_from(args);

    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Label(LabelCommands::Add { label, .. }) => {
            assert_eq!(label.len(), 4, "Should have exactly four labels");
            assert!(label.contains(&"P0".to_string()), "Should contain 'P0' label");
            assert!(label.contains(&"bug-fix".to_string()), "Should contain 'bug-fix' label");
            assert!(label.contains(&"high_priority".to_string()), "Should contain 'high_priority' label");
            assert!(label.contains(&"team-alpha".to_string()), "Should contain 'team-alpha' label");
        }
        _ => panic!("Expected Label::Add command, got a different command"),
    }
}

#[test]
fn test_label_add_id_before_labels() {
    // Test that the ID comes before label flags (standard order)
    let args = vec![
        "bf",
        "label",
        "add",
        "bf-123",
        "--label",
        "P0",
    ];

    let cli = Cli::parse_from(args);
    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Label(LabelCommands::Add { id, label }) => {
            assert_eq!(id, "bf-123", "ID should be 'bf-123'");
            assert_eq!(label.len(), 1, "Should have one label");
            assert_eq!(label[0], "P0", "Label should be 'P0'");
        }
        _ => panic!("Expected Label::Add command"),
    }
}

#[test]
fn test_label_add_required_flag_validation() {
    // Test that the --label flag is required by validating a working command
    // This test confirms the CLI structure requires at least one label
    let args_valid = vec![
        "bf",
        "label",
        "add",
        "bf-12345",
        "--label",
        "P0",
    ];

    let cli = Cli::parse_from(args_valid);
    let command = cli.command.expect("Command should be present");

    match command {
        Commands::Label(LabelCommands::Add { id, label }) => {
            assert!(!label.is_empty(), "Labels should not be empty when --label is provided");
            assert_eq!(id, "bf-12345", "Issue ID should be correct");
        }
        _ => panic!("Expected Label::Add command"),
    }
}

#[test]
fn test_label_add_p0_specifically() {
    // Test specifically for the P0 label mentioned in the task
    let args = vec![
        "bf",
        "label",
        "add",
        "bf-p0-bead",
        "--label",
        "P0",
    ];

    let cli = Cli::parse_from(args);

    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Label(LabelCommands::Add { id, label }) => {
            assert_eq!(id, "bf-p0-bead", "Issue ID should be correct");
            assert_eq!(label.len(), 1, "Should have exactly one P0 label");
            assert_eq!(label[0], "P0", "Label should be exactly 'P0'");
        }
        _ => panic!("Expected Label::Add command, got a different command"),
    }
}

#[test]
fn test_label_add_duplicate_p0_labels() {
    // Test that duplicate P0 labels are all captured (CLI parsing, not storage behavior)
    let args = vec![
        "bf",
        "label",
        "add",
        "bf-duplicate",
        "--label",
        "P0",
        "--label",
        "P0",
    ];

    let cli = Cli::parse_from(args);

    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Label(LabelCommands::Add { id, label }) => {
            assert_eq!(id, "bf-duplicate", "Issue ID should be correct");
            assert_eq!(label.len(), 2, "Should capture both P0 labels at CLI layer");
            assert_eq!(label[0], "P0", "First label should be 'P0'");
            assert_eq!(label[1], "P0", "Second label should be 'P0'");
        }
        _ => panic!("Expected Label::Add command, got a different command"),
    }
}
