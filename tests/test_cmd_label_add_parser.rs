//! Test CLI argument parsing for `bf label add` command
//!
//! Verifies the parser correctly accepts:
//! - Bead ID as positional argument
//! - One or more labels via `-l` or `--label` flags
//! - Multiple labels with repeated flag usage
//! - Required validation (at least one label must be provided)
//!
//! Test scenarios:
//! - Single label with short flag: `bf label add <id> -l bug`
//! - Multiple labels with short flags: `bf label add <id> -l bug -l urgent -l priority`
//! - Single label with long flag: `bf label add <id> --label bug`
//! - Multiple labels with long flags: `bf label add <id> --label bug --label urgent`
//! - Mixed short and long flags: `bf label add <id> -l bug --label urgent`
//! - Missing required label flag (should fail parsing)
//! - Positional argument order validation

use clap::Parser;
use bead_forge::cli::{Cli, Commands, LabelCommands};

/// Helper to parse CLI args and extract the Label::Add command with its ID and labels
fn parse_label_add_command(args: &[&str]) -> Result<(String, Vec<String>), String> {
    let cli = Cli::parse_from(args);

    match cli.command {
        Some(Commands::Label(LabelCommands::Add { id, label })) => Ok((id, label)),
        Some(other) => Err(format!("Expected Label::Add command, got: {:?}", other)),
        None => Err("No command provided".to_string()),
    }
}

#[test]
fn test_label_add_parser_single_label_short_flag() {
    // Test parsing of `bf label add bf-123 -l bug`
    // Verifies: Parser accepts bead ID as 'id' argument and single label via '-l' flag
    let args = vec![
        "bf",
        "label",
        "add",
        "bf-123",
        "-l",
        "bug",
    ];

    let (id, labels) = parse_label_add_command(&args).expect("Should parse Label::Add command");

    // Verify bead ID is correctly parsed
    assert_eq!(id, "bf-123", "Bead ID should be 'bf-123'");

    // Verify labels Vec contains exactly 1 element
    assert_eq!(labels.len(), 1, "Labels count should be 1");

    // Verify the label value is correct
    assert_eq!(labels[0], "bug", "Label value should be 'bug'");
}

#[test]
fn test_label_add_parser_multiple_labels_short_flags() {
    // Test parsing of `bf label add bf-456 -l bug -l urgent -l priority`
    // Verifies: Multiple labels can be specified with repeated -l flags
    let args = vec![
        "bf",
        "label",
        "add",
        "bf-456",
        "-l",
        "bug",
        "-l",
        "urgent",
        "-l",
        "priority",
    ];

    let (id, labels) = parse_label_add_command(&args).expect("Should parse Label::Add command");

    // Verify bead ID is correctly parsed
    assert_eq!(id, "bf-456", "Bead ID should be 'bf-456'");

    // Verify parsed labels Vec contains exactly 3 elements
    assert_eq!(labels.len(), 3, "Labels count should be 3");

    // Verify all 3 labels are present
    assert!(labels.contains(&"bug".to_string()), "Should contain 'bug' label");
    assert!(labels.contains(&"urgent".to_string()), "Should contain 'urgent' label");
    assert!(labels.contains(&"priority".to_string()), "Should contain 'priority' label");

    // Verify order is preserved (clap's Append action maintains order)
    assert_eq!(labels[0], "bug", "First label should be 'bug'");
    assert_eq!(labels[1], "urgent", "Second label should be 'urgent'");
    assert_eq!(labels[2], "priority", "Third label should be 'priority'");
}

#[test]
fn test_label_add_parser_single_label_long_flag() {
    // Test parsing of `bf label add bf-789 --label enhancement`
    // Verifies: Parser accepts labels via '--label' long flag
    let args = vec![
        "bf",
        "label",
        "add",
        "bf-789",
        "--label",
        "enhancement",
    ];

    let (id, labels) = parse_label_add_command(&args).expect("Should parse Label::Add command");

    // Verify bead ID is correctly parsed
    assert_eq!(id, "bf-789", "Bead ID should be 'bf-789'");

    // Verify labels Vec contains exactly 1 element
    assert_eq!(labels.len(), 1, "Labels count should be 1");

    // Verify the label value is correct
    assert_eq!(labels[0], "enhancement", "Label value should be 'enhancement'");
}

#[test]
fn test_label_add_parser_multiple_labels_long_flags() {
    // Test parsing of `bf label add bf-abc --label frontend --label backend --label database`
    // Verifies: Multiple labels can be specified with repeated --label flags
    let args = vec![
        "bf",
        "label",
        "add",
        "bf-abc",
        "--label",
        "frontend",
        "--label",
        "backend",
        "--label",
        "database",
    ];

    let (id, labels) = parse_label_add_command(&args).expect("Should parse Label::Add command");

    // Verify bead ID is correctly parsed
    assert_eq!(id, "bf-abc", "Bead ID should be 'bf-abc'");

    // Verify parsed labels Vec contains exactly 3 elements
    assert_eq!(labels.len(), 3, "Labels count should be 3");

    // Verify all 3 labels are present
    assert!(labels.contains(&"frontend".to_string()), "Should contain 'frontend' label");
    assert!(labels.contains(&"backend".to_string()), "Should contain 'backend' label");
    assert!(labels.contains(&"database".to_string()), "Should contain 'database' label");

    // Verify order preservation
    assert_eq!(labels[0], "frontend", "First label should be 'frontend'");
    assert_eq!(labels[1], "backend", "Second label should be 'backend'");
    assert_eq!(labels[2], "database", "Third label should be 'database'");
}

#[test]
fn test_label_add_parser_mixed_short_long_flags() {
    // Test parsing of `bf label add bf-xyz -l critical --label p0 -l security`
    // Verifies: Parser accepts mixed short (-l) and long (--label) flags
    let args = vec![
        "bf",
        "label",
        "add",
        "bf-xyz",
        "-l",
        "critical",
        "--label",
        "p0",
        "-l",
        "security",
    ];

    let (id, labels) = parse_label_add_command(&args).expect("Should parse Label::Add command");

    // Verify bead ID is correctly parsed
    assert_eq!(id, "bf-xyz", "Bead ID should be 'bf-xyz'");

    // Verify parsed labels Vec contains exactly 3 elements
    assert_eq!(labels.len(), 3, "Labels count should be 3");

    // Verify all 3 labels are present
    assert!(labels.contains(&"critical".to_string()), "Should contain 'critical' label");
    assert!(labels.contains(&"p0".to_string()), "Should contain 'p0' label");
    assert!(labels.contains(&"security".to_string()), "Should contain 'security' label");

    // Verify order preservation with mixed flags
    assert_eq!(labels[0], "critical", "First label should be 'critical'");
    assert_eq!(labels[1], "p0", "Second label should be 'p0'");
    assert_eq!(labels[2], "security", "Third label should be 'security'");
}

#[test]
fn test_label_add_parser_missing_required_label_flag() {
    // Test parsing of `bf label add bf-missing` (no -l/--label flag)
    // Verifies: Required validation enforced - at least one label must be provided
    let args = vec![
        "bf",
        "label",
        "add",
        "bf-missing",
    ];

    let result = std::panic::catch_unwind(|| {
        parse_label_add_command(&args)
    });

    // The parser should fail because the label flag is required
    // clap will exit or panic when required arguments are missing
    assert!(result.is_err() || result.unwrap().is_err(),
            "Parser should fail when required label flag is missing");
}

#[test]
fn test_label_add_parser_positional_argument_order() {
    // Test that the positional bead ID argument comes before the label flags
    // This is the standard and expected order: `bf label add <id> -l <label>`
    let args = vec![
        "bf",
        "label",
        "add",
        "bf-order-test",
        "-l",
        "test-label",
    ];

    let (id, labels) = parse_label_add_command(&args).expect("Should parse Label::Add command");

    // Verify bead ID is correctly parsed as the positional argument
    assert_eq!(id, "bf-order-test", "Bead ID should be 'bf-order-test'");

    // Verify labels are correctly parsed as flags
    assert_eq!(labels.len(), 1, "Labels count should be 1");
    assert_eq!(labels[0], "test-label", "Label value should be 'test-label'");
}

#[test]
fn test_label_add_parser_two_labels_basic() {
    // Test parsing of `bf label add bf-2labels -l bug -l urgent`
    // Verifies the exact pattern from acceptance criteria
    let args = vec![
        "bf",
        "label",
        "add",
        "bf-2labels",
        "-l",
        "bug",
        "-l",
        "urgent",
    ];

    let (id, labels) = parse_label_add_command(&args).expect("Should parse Label::Add command");

    // Verify bead ID
    assert_eq!(id, "bf-2labels", "Bead ID should be 'bf-2labels'");

    // Verify exactly 2 labels
    assert_eq!(labels.len(), 2, "Labels count should be 2");

    // Verify both labels are present in correct order
    assert_eq!(labels[0], "bug", "First label should be 'bug'");
    assert_eq!(labels[1], "urgent", "Second label should be 'urgent'");
}

#[test]
fn test_label_add_parser_comprehensive_acceptance_criteria() {
    // Comprehensive test covering all acceptance criteria:
    // 1. Parser accepts bead ID as 'id' argument ✓
    // 2. Parser accepts one or more labels via '-l' or '--label' flags ✓
    // 3. Multiple labels can be specified: -l bug -l urgent -l priority ✓
    // 4. Required validation: at least one label must be provided ✓

    // Test case 1: Single label with short flag
    let args1 = vec!["bf", "label", "add", "bf-test1", "-l", "single"];
    let (id1, labels1) = parse_label_add_command(&args1).expect("Should parse single label");
    assert_eq!(id1, "bf-test1");
    assert_eq!(labels1.len(), 1);
    assert_eq!(labels1[0], "single");

    // Test case 2: Multiple labels with short flags
    let args2 = vec!["bf", "label", "add", "bf-test2", "-l", "bug", "-l", "urgent", "-l", "priority"];
    let (id2, labels2) = parse_label_add_command(&args2).expect("Should parse multiple labels");
    assert_eq!(id2, "bf-test2");
    assert_eq!(labels2.len(), 3);
    assert!(labels2.contains(&"bug".to_string()));
    assert!(labels2.contains(&"urgent".to_string()));
    assert!(labels2.contains(&"priority".to_string()));

    // Test case 3: Labels with long flag
    let args3 = vec!["bf", "label", "add", "bf-test3", "--label", "longform"];
    let (id3, labels3) = parse_label_add_command(&args3).expect("Should parse long flag");
    assert_eq!(id3, "bf-test3");
    assert_eq!(labels3.len(), 1);
    assert_eq!(labels3[0], "longform");

    // Test case 4: Required validation (missing label fails)
    let args4 = vec!["bf", "label", "add", "bf-test4"];
    let result4 = std::panic::catch_unwind(|| {
        parse_label_add_command(&args4)
    });
    assert!(result4.is_err() || result4.unwrap().is_err(),
            "Required label flag enforcement should prevent parsing without -l/--label");
}
