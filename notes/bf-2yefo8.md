# Bead bf-2yefo8: Show Command Error Case Test for Non-Existent Bead

## Finding
The test `test_show_command_json_nonexistent_bead` already exists in `src/cli/tests/json_output.rs` at line 1579.

## Verification of Acceptance Criteria

### 1. ✅ Test validates show command returns proper error for non-existent bead ID
- Test uses `capture_failed_command()` to execute `bf show <non-existent-id> --format json`
- Asserts `!success` to verify the command fails
- Located at lines 1593-1594 in the test

### 2. ✅ Test confirms error message is informative  
- Test verifies stderr contains "not found" or "Bead not found" (lines 1596-1601)
- Manual verification shows actual error message: "Error: Bead not found: bf-nonexistent-test-12345"
- Error message includes the problematic bead ID, making it informative

### 3. ✅ Test located in src/cli/tests/json_output.rs
- Test function `test_show_command_json_nonexistent_bead` is at line 1579
- Part of the `command_json_output_tests` test module
- Properly integrated with the existing test infrastructure

### 4. ✅ Test compiles without errors
- Test compiles successfully with `cargo build`
- Test passes successfully: `cargo test test_show_command_json_nonexistent_bead` passes
- No clippy warnings for the test file
- Test is not marked with `#[ignore]` and runs by default

## Test Implementation Details

The test:
```rust
#[test]
fn test_show_command_json_nonexistent_bead() {
    require_binary();

    // Test with a bead ID that doesn't exist
    let fake_bead_id = "bf-test-nonexistent-12345";

    let (stdout, stderr, success) = capture::capture_failed_command(
        bf_command()
            .arg("show")
            .arg(fake_bead_id)
            .arg("--format")
            .arg("json")
    );

    // Command should fail
    assert!(!success, "show command should fail for non-existent bead");

    // Stderr should contain error message
    assert!(
        stderr.contains("not found") || stderr.contains("Bead not found"),
        "stderr should mention bead not found, got: {}",
        stderr
    );

    // Stdout should be empty (no JSON output for errors)
    assert!(
        stdout.trim().is_empty(),
        "stdout should be empty for non-existent bead, got: {}",
        stdout
    );
}
```

## Conclusion
All acceptance criteria for bead bf-2yefo8 are already met by the existing test implementation. No additional work is required.