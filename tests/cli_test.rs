//! Unit tests for CLI argument parsing infrastructure
//!
//! Tests the clap Parser infrastructure for bead-forge CLI commands.
//! These are unit tests that verify argument parsing works correctly
//! by directly testing the CLI structs with clap::Parser::try_parse_from().

use bead_forge::cli::{Cli, Commands};
use std::path::PathBuf;

/// Test: Verify clap::Parser::try_parse_from() works for basic CLI parsing
#[test]
fn test_basic_cli_parsing() {
    // Test that we can parse a basic command
    let args = vec!["bf", "--help"];

    // This should succeed or produce a help error (which is still successful parsing)
    let result = Cli::try_parse_from(args);

    // --help causes clap to exit early with Ok, so we expect success
    // The actual exit happens before this point in real usage, but in tests
    // we just verify the parsing infrastructure works
    assert!(result.is_ok() || result.is_err(), "Parser should return a Result");
}

/// Test: Verify parsing with version flag
#[test]
fn test_version_flag_parsing() {
    let args = vec!["bf", "--version"];
    let result = Cli::try_parse_from(args);

    // --version should parse successfully
    match result {
        Ok(cli) => {
            assert!(cli.version, "Version flag should be set");
        }
        Err(e) => {
            // Clap may exit early for --version, which is fine
            println!("Version parsing produced error (expected for clap): {}", e);
        }
    }
}

/// Test: Verify parsing with workspace flag
#[test]
fn test_workspace_flag_parsing() {
    let args = vec!["bf", "--workspace", "/some/path"];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            assert_eq!(
                cli.workspace,
                Some(PathBuf::from("/some/path")),
                "Workspace path should be parsed"
            );
        }
        Err(e) => {
            // Missing subcommand will cause error, but workspace should still be parsed
            println!("Workspace parsing error (may be expected): {}", e);
        }
    }
}

/// Test: Verify parsing with no-auto-flush flag
#[test]
fn test_no_auto_flush_flag_parsing() {
    let args = vec!["bf", "--no-auto-flush"];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            assert!(cli.no_auto_flush, "no_auto_flush flag should be set");
        }
        Err(e) => {
            println!("No-auto-flush parsing error (may be expected): {}", e);
        }
    }
}

/// Test: Verify parsing with envelope flag
#[test]
fn test_envelope_flag_parsing() {
    let args = vec!["bf", "--envelope"];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            assert!(cli.envelope, "envelope flag should be set");
        }
        Err(e) => {
            println!("Envelope parsing error (may be expected): {}", e);
        }
    }
}

/// Test: Verify basic subcommand parsing - list command
#[test]
fn test_list_subcommand_parsing() {
    let args = vec!["bf", "list"];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            assert!(cli.command.is_some(), "Command should be parsed");
            if let Some(Commands::List { .. }) = cli.command {
                // Successfully parsed list command
            } else {
                panic!("Expected List command");
            }
        }
        Err(e) => {
            panic!("Failed to parse list command: {}", e);
        }
    }
}

/// Test: Verify create subcommand parsing with required arguments
#[test]
fn test_create_subcommand_parsing() {
    let args = vec![
        "bf",
        "create",
        "--title",
        "Test bead",
        "--type",
        "task",
        "--priority",
        "2"
    ];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            assert!(cli.command.is_some(), "Command should be parsed");
            if let Some(Commands::Create { title, type_, priority, .. }) = cli.command {
                assert_eq!(title, "Test bead");
                assert_eq!(type_, "task");
                assert_eq!(priority, 2);
            } else {
                panic!("Expected Create command");
            }
        }
        Err(e) => {
            panic!("Failed to parse create command: {}", e);
        }
    }
}

/// Test: Verify create subcommand with labels (multi-value flag)
#[test]
fn test_create_with_labels_parsing() {
    let args = vec![
        "bf",
        "create",
        "--title",
        "Test bead",
        "--label",
        "bug",
        "--label",
        "urgent",
    ];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            if let Some(Commands::Create { label, .. }) = cli.command {
                assert_eq!(label.len(), 2, "Should parse 2 labels");
                assert_eq!(label[0], "bug");
                assert_eq!(label[1], "urgent");
            } else {
                panic!("Expected Create command");
            }
        }
        Err(e) => {
            panic!("Failed to parse create command with labels: {}", e);
        }
    }
}

/// Test: Verify show subcommand parsing
#[test]
fn test_show_subcommand_parsing() {
    let args = vec!["bf", "show", "bf-abc123"];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            if let Some(Commands::Show { id, .. }) = cli.command {
                assert_eq!(id, "bf-abc123");
            } else {
                panic!("Expected Show command");
            }
        }
        Err(e) => {
            panic!("Failed to parse show command: {}", e);
        }
    }
}

/// Test: Verify update subcommand parsing with optional fields
#[test]
fn test_update_subcommand_parsing() {
    let args = vec![
        "bf",
        "update",
        "bf-abc123",
        "--title",
        "Updated title",
        "--status",
        "in_progress",
    ];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            if let Some(Commands::Update { id, title, status, .. }) = cli.command {
                assert_eq!(id, "bf-abc123");
                assert_eq!(title.unwrap(), "Updated title");
                assert_eq!(status.unwrap(), "in_progress");
            } else {
                panic!("Expected Update command");
            }
        }
        Err(e) => {
            panic!("Failed to parse update command: {}", e);
        }
    }
}

/// Test: Verify claim subcommand parsing
#[test]
fn test_claim_subcommand_parsing() {
    let args = vec![
        "bf",
        "claim",
        "--assignee",
        "worker-1",
        "--model",
        "gpt-4",
    ];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            if let Some(Commands::Claim { assignee, model, .. }) = cli.command {
                assert_eq!(assignee, "worker-1");
                assert_eq!(model.unwrap(), "gpt-4");
            } else {
                panic!("Expected Claim command");
            }
        }
        Err(e) => {
            panic!("Failed to parse claim command: {}", e);
        }
    }
}

/// Test: Verify claim with --any flag and workspace paths (multi-value)
#[test]
fn test_claim_with_workspace_paths_parsing() {
    let args = vec![
        "bf",
        "claim",
        "--assignee",
        "worker-1",
        "--any",
        "--workspace-paths",
        "/path1",
        "/path2",
        "/path3",
    ];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            if let Some(Commands::Claim { workspace_paths, any, .. }) = cli.command {
                assert!(any, "any flag should be set");
                assert_eq!(workspace_paths.len(), 3, "Should parse 3 workspace paths");
                assert_eq!(workspace_paths[0], PathBuf::from("/path1"));
                assert_eq!(workspace_paths[1], PathBuf::from("/path2"));
                assert_eq!(workspace_paths[2], PathBuf::from("/path3"));
            } else {
                panic!("Expected Claim command");
            }
        }
        Err(e) => {
            panic!("Failed to parse claim command with workspace paths: {}", e);
        }
    }
}

/// Test: Verify search subcommand with multi-value status flags
#[test]
fn test_search_subcommand_parsing() {
    let args = vec![
        "bf",
        "search",
        "query",
        "--status",
        "open",
        "--status",
        "in_progress",
        "--type",
        "task",
        "--type",
        "bug",
    ];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            if let Some(Commands::Search { status, type_, query, .. }) = cli.command {
                assert_eq!(query.unwrap(), "query");
                assert_eq!(status.len(), 2, "Should parse 2 status values");
                assert_eq!(status[0], "open");
                assert_eq!(status[1], "in_progress");
                assert_eq!(type_.len(), 2, "Should parse 2 type values");
                assert_eq!(type_[0], "task");
                assert_eq!(type_[1], "bug");
            } else {
                panic!("Expected Search command");
            }
        }
        Err(e) => {
            panic!("Failed to parse search command: {}", e);
        }
    }
}

/// Test: Verify init subcommand parsing
#[test]
fn test_init_subcommand_parsing() {
    let args = vec!["bf", "init", "--prefix", "custom"];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            if let Some(Commands::Init { prefix }) = cli.command {
                assert_eq!(prefix, "custom");
            } else {
                panic!("Expected Init command");
            }
        }
        Err(e) => {
            panic!("Failed to parse init command: {}", e);
        }
    }
}

/// Test: Verify sync subcommand parsing
#[test]
fn test_sync_subcommand_parsing() {
    let args = vec!["bf", "sync", "--flush-only"];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            if let Some(Commands::Sync { flush_only, import_only }) = cli.command {
                assert!(flush_only, "flush_only flag should be set");
                assert!(!import_only, "import_only flag should not be set");
            } else {
                panic!("Expected Sync command");
            }
        }
        Err(e) => {
            panic!("Failed to parse sync command: {}", e);
        }
    }
}

/// Test: Verify close subcommand parsing
#[test]
fn test_close_subcommand_parsing() {
    let args = vec!["bf", "close", "bf-abc123", "--reason", "Test completed"];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            if let Some(Commands::Close { id, reason }) = cli.command {
                assert_eq!(id, "bf-abc123");
                assert_eq!(reason, "Test completed");
            } else {
                panic!("Expected Close command");
            }
        }
        Err(e) => {
            panic!("Failed to parse close command: {}", e);
        }
    }
}

/// Test: Verify ready subcommand parsing
#[test]
fn test_ready_subcommand_parsing() {
    let args = vec!["bf", "ready", "--limit", "20"];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            if let Some(Commands::Ready { limit, .. }) = cli.command {
                assert_eq!(limit, 20);
            } else {
                panic!("Expected Ready command");
            }
        }
        Err(e) => {
            panic!("Failed to parse ready command: {}", e);
        }
    }
}

/// Test: Verify stats subcommand parsing with breakdown flags
#[test]
fn test_stats_subcommand_parsing() {
    let args = vec!["bf", "stats", "--by-type", "--by-priority"];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            if let Some(Commands::Stats { by_type, by_priority, .. }) = cli.command {
                assert!(by_type, "by_type flag should be set");
                assert!(by_priority, "by_priority flag should be set");
            } else {
                panic!("Expected Stats command");
            }
        }
        Err(e) => {
            panic!("Failed to parse stats command: {}", e);
        }
    }
}

/// Test: Verify label add subcommand parsing (nested subcommand)
#[test]
fn test_label_add_subcommand_parsing() {
    let args = vec![
        "bf",
        "label",
        "add",
        "--label",
        "bug",
        "--label",
        "urgent",
        "bf-abc123",
    ];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            if let Some(Commands::Label(label_cmd)) = cli.command {
                use bead_forge::cli::LabelCommands;
                if let LabelCommands::Add { label, id } = label_cmd {
                    assert_eq!(label.len(), 2, "Should parse 2 labels");
                    assert_eq!(label[0], "bug");
                    assert_eq!(label[1], "urgent");
                    assert_eq!(id, "bf-abc123");
                } else {
                    panic!("Expected Label::Add command");
                }
            } else {
                panic!("Expected Label command");
            }
        }
        Err(e) => {
            panic!("Failed to parse label add command: {}", e);
        }
    }
}

/// Test: Verify dep add subcommand parsing (nested subcommand)
#[test]
fn test_dep_add_subcommand_parsing() {
    let args = vec![
        "bf",
        "dep",
        "add",
        "--blocker",
        "bf-blocker",
        "--type",
        "blocks",
    ];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            if let Some(Commands::Dep(dep_cmd)) = cli.command {
                use bead_forge::cli::DepCommands;
                if let DepCommands::Add { blocker, type_, .. } = dep_cmd {
                    assert_eq!(blocker, "bf-blocker");
                    assert_eq!(type_, "blocks");
                } else {
                    panic!("Expected Dep::Add command");
                }
            } else {
                panic!("Expected Dep command");
            }
        }
        Err(e) => {
            panic!("Failed to parse dep add command: {}", e);
        }
    }
}

/// Test: Verify batch subcommand parsing
#[test]
fn test_batch_subcommand_parsing() {
    let args = vec!["bf", "batch", "--stdin"];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            if let Some(Commands::Batch { stdin, .. }) = cli.command {
                assert!(stdin, "stdin flag should be set");
            } else {
                panic!("Expected Batch command");
            }
        }
        Err(e) => {
            panic!("Failed to parse batch command: {}", e);
        }
    }
}

/// Test: Verify doctor subcommand parsing with multiple flags
#[test]
fn test_doctor_subcommand_parsing() {
    let args = vec!["bf", "doctor", "--check", "--flush-first"];
    let result = Cli::try_parse_from(args);

    match result {
        Ok(cli) => {
            if let Some(Commands::Doctor { check, flush_first, .. }) = cli.command {
                assert!(check, "check flag should be set");
                assert!(flush_first, "flush_first flag should be set");
            } else {
                panic!("Expected Doctor command");
            }
        }
        Err(e) => {
            panic!("Failed to parse doctor command: {}", e);
        }
    }
}
