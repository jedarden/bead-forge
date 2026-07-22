// Regression tests for `bf --help` (bead bf-2l7).
//
// The --help flag is provided by clap's `#[derive(Parser)]` on `Cli`. These
// tests pin the acceptance criteria for the bead: the flag exits 0, prints a
// usage block, and lists usage information for *all* top-level commands (not
// just a subset). They also cover the `-h` short form and per-subcommand help
// (`bf <cmd> --help`), so a future refactor that drops a command from the CLI
// surface fails loudly here.

use std::process::Command;

fn bf_binary() -> std::path::PathBuf {
    // cargo test injects CARGO_BIN_EXE_bf pointing at the freshly built binary.
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_bf"))
}

/// Every top-level command clap registers. Kept in sync with the `Commands`
/// enum in `src/cli/mod.rs`; a command added there without updating this list
/// (or vice versa) fails `test_help_lists_all_commands`.
const TOP_LEVEL_COMMANDS: &[&str] = &[
    "create",
    "list",
    "show",
    "update",
    "close",
    "reopen",
    "delete",
    "ready",
    "claim",
    "init",
    "sync",
    "doctor",
    "merge-jsonl",
    "commit-check",
    "count",
    "batch",
    "mitosis",
    "dep",
    "label",
    "labels",
    "comments",
    "search",
    "stats",
    "schema",
    "config",
    "velocity",
    "annotate",
    "log",
    "critical-path",
    "rotate",
    "migrate",
    "recent",
    "help",
];

#[test]
fn test_help_exits_zero() {
    let output = Command::new(bf_binary())
        .arg("--help")
        .output()
        .expect("Failed to run 'bf --help'");

    assert!(
        output.status.success(),
        "'bf --help' should exit with success. status: {:?}",
        output.status
    );
}

#[test]
fn test_help_shows_usage_block() {
    let output = Command::new(bf_binary())
        .arg("--help")
        .output()
        .expect("Failed to run 'bf --help'");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Usage:"),
        "Help should contain a 'Usage:' line. Got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("Commands:"),
        "Help should contain a 'Commands:' section. Got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("bead-forge"),
        "Help should mention bead-forge. Got:\n{}",
        stdout
    );
}

#[test]
fn test_help_lists_all_commands() {
    let output = Command::new(bf_binary())
        .arg("--help")
        .output()
        .expect("Failed to run 'bf --help'");

    let stdout = String::from_utf8_lossy(&output.stdout);

    for cmd in TOP_LEVEL_COMMANDS {
        assert!(
            stdout.contains(cmd),
            "'bf --help' should list the '{}' command. Full output:\n{}",
            cmd,
            stdout
        );
    }
}

#[test]
fn test_short_help_flag() {
    // `-h` is clap's short alias and must behave like `--help`.
    let output = Command::new(bf_binary())
        .arg("-h")
        .output()
        .expect("Failed to run 'bf -h'");

    assert!(output.status.success(), "'bf -h' should exit with success");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Usage:"),
        "'bf -h' should print usage. Got:\n{}",
        stdout
    );
}

#[test]
fn test_subcommand_help() {
    // Per-subcommand help must also work: `bf create --help`.
    let output = Command::new(bf_binary())
        .args(["create", "--help"])
        .output()
        .expect("Failed to run 'bf create --help'");

    assert!(
        output.status.success(),
        "'bf create --help' should exit with success"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--title"),
        "'bf create --help' should document the --title flag. Got:\n{}",
        stdout
    );
}
