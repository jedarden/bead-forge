// Test CLI mutual exclusion of `bf update --clear-assignee` and `--assignee` flags
//
// This test verifies that:
// 1. --clear-assignee and --assignee flags are mutually exclusive
// 2. Each flag works individually when used alone
// 3. Clear error messages when both flags are provided

use bead_forge::cli::{Cli, Commands};

#[test]
fn test_update_clear_assignee_and_assignee_mutual_exclusion() {
    // Test that `bf update --clear-assignee --assignee foo` fails with clear error
    let args = vec![
        "bf",
        "update",
        "bf-test-id",
        "--clear-assignee",
        "--assignee",
        "foo",
    ];

    let result = std::panic::catch_unwind(|| {
        Cli::parse_from(args);
    });

    // The parse should fail due to conflict
    assert!(result.is_err(), "Parsing should fail when both --clear-assignee and --assignee are provided");

    // Verify the error message mentions the conflict
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("--clear-assignee") && error_msg.contains("--assignee"),
        "Error message should mention both conflicting flags. Got: {}",
        error_msg
    );
}

#[test]
fn test_update_clear_assignee_alone() {
    // Test that --clear-assignee works when used alone
    let args = vec![
        "bf",
        "update",
        "bf-test-id",
        "--clear-assignee",
    ];

    let cli = Cli::parse_from(args);
    let command = cli.command.expect("Command should be present");

    match command {
        Commands::Update {
            id,
            title,
            status,
            priority,
            assignee,
            clear_assignee,
            description,
            description_file,
            acceptance_criteria,
            notes,
            design,
            due_at,
            json,
        } => {
            assert_eq!(id, "bf-test-id");
            assert!(clear_assignee, "--clear-assignee should be true");
            assert!(assignee.is_none(), "--assignee should be None when only --clear-assignee is set");
            assert!(title.is_none());
            assert!(status.is_none());
            assert!(priority.is_none());
            assert!(description.is_none());
            assert!(description_file.is_none());
            assert!(acceptance_criteria.is_none());
            assert!(notes.is_none());
            assert!(design.is_none());
            assert!(due_at.is_none());
            assert!(!json);
        }
        _ => panic!("Expected Update command, got a different command"),
    }
}

#[test]
fn test_update_assignee_alone() {
    // Test that --assignee works when used alone
    let args = vec![
        "bf",
        "update",
        "bf-test-id",
        "--assignee",
        "alice",
    ];

    let cli = Cli::parse_from(args);
    let command = cli.command.expect("Command should be present");

    match command {
        Commands::Update {
            id,
            assignee,
            clear_assignee,
            ..
        } => {
            assert_eq!(id, "bf-test-id");
            assert_eq!(assignee, Some("alice".to_string()), "--assignee should be 'alice'");
            assert!(!clear_assignee, "--clear-assignee should be false when only --assignee is set");
        }
        _ => panic!("Expected Update command, got a different command"),
    }
}

#[test]
fn test_update_assignee_empty_string_allowed() {
    // Test that --assignee "" is allowed (it clears the assignee, different from --clear-assignee flag)
    let args = vec![
        "bf",
        "update",
        "bf-test-id",
        "--assignee",
        "",
    ];

    let cli = Cli::parse_from(args);
    let command = cli.command.expect("Command should be present");

    match command {
        Commands::Update {
            id,
            assignee,
            clear_assignee,
            ..
        } => {
            assert_eq!(id, "bf-test-id");
            assert_eq!(assignee, Some(String::new()), "--assignee should be empty string");
            assert!(!clear_assignee, "--clear-assignee flag should not be set");
        }
        _ => panic!("Expected Update command, got a different command"),
    }
}

#[test]
fn test_update_clear_assignee_with_other_flags() {
    // Test that --clear-assignee works with other update flags
    let args = vec![
        "bf",
        "update",
        "bf-test-id",
        "--clear-assignee",
        "--title",
        "Updated title",
        "--status",
        "in_progress",
        "--priority",
        "1",
    ];

    let cli = Cli::parse_from(args);
    let command = cli.command.expect("Command should be present");

    match command {
        Commands::Update {
            id,
            title,
            status,
            priority,
            assignee,
            clear_assignee,
            ..
        } => {
            assert_eq!(id, "bf-test-id");
            assert!(clear_assignee, "--clear-assignee should be true");
            assert!(assignee.is_none(), "--assignee should be None when --clear-assignee is set");
            assert_eq!(title, Some("Updated title".to_string()));
            assert_eq!(status, Some("in_progress".to_string()));
            assert_eq!(priority, Some(1));
        }
        _ => panic!("Expected Update command, got a different command"),
    }
}

#[test]
fn test_update_assignee_with_other_flags() {
    // Test that --assignee works with other update flags
    let args = vec![
        "bf",
        "update",
        "bf-test-id",
        "--assignee",
        "bob",
        "--title",
        "Another update",
        "--description",
        "Updated description",
    ];

    let cli = Cli::parse_from(args);
    let command = cli.command.expect("Command should be present");

    match command {
        Commands::Update {
            id,
            title,
            assignee,
            description,
            clear_assignee,
            ..
        } => {
            assert_eq!(id, "bf-test-id");
            assert_eq!(assignee, Some("bob".to_string()));
            assert!(!clear_assignee, "--clear-assignee should be false");
            assert_eq!(title, Some("Another update".to_string()));
            assert_eq!(description, Some("Updated description".to_string()));
        }
        _ => panic!("Expected Update command, got a different command"),
    }
}

#[test]
fn test_update_neither_assignee_flag() {
    // Test that update works without any assignee-related flags
    let args = vec![
        "bf",
        "update",
        "bf-test-id",
        "--title",
        "Just title update",
    ];

    let cli = Cli::parse_from(args);
    let command = cli.command.expect("Command should be present");

    match command {
        Commands::Update {
            id,
            title,
            assignee,
            clear_assignee,
            ..
        } => {
            assert_eq!(id, "bf-test-id");
            assert_eq!(title, Some("Just title update".to_string()));
            assert!(assignee.is_none(), "No assignee should be set");
            assert!(!clear_assignee, "--clear-assignee should be false");
        }
        _ => panic!("Expected Update command, got a different command"),
    }
}
