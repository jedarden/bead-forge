//! Standalone integration tests for None/missing exit code edge cases
//!
//! These tests verify graceful handling of absent or missing exit codes
//! across all exit code formatting and processing functions.

use bead_forge::exit_code::*;

#[test]
fn test_none_exit_code_formatting_comprehensive() {
    // Test that ExitCode::None formats correctly
    let none = ExitCode::None;
    assert_eq!(format!("{}", none), "none");

    // Test format_exit_code with ExitCode::None
    assert_eq!(format_exit_code(Some(none.clone())), "=== Exit Code: (none) ===");

    // Test format_exit_code with Option::None
    assert_eq!(format_exit_code(None), "=== Exit Code: (none) ===");

    // Verify consistency between Some(ExitCode::None) and None
    assert_eq!(
        format_exit_code(Some(ExitCode::None)),
        format_exit_code(None)
    );
}

#[test]
fn test_missing_exit_code_in_process_termination() {
    // Test ProcessTermination::from_code with None
    let term = ProcessTermination::from_code(None);
    assert_eq!(term, ProcessTermination::Unknown);

    // Verify formatting of Unknown termination
    assert_eq!(term.format(), "=== Exit Code: unknown ===");

    // Test that negative codes also produce Unknown
    let negative_term = ProcessTermination::from_code(Some(-1));
    assert_eq!(negative_term, ProcessTermination::Unknown);
    assert_eq!(negative_term.format(), "=== Exit Code: unknown ===");
}

#[test]
fn test_none_exit_code_with_empty_log() {
    // Test appending None exit code to empty log
    let empty_log = "";
    let result = append_exit_code_to_log(empty_log, None);

    assert!(result.contains("=== Exit Code: unknown ==="));
    assert_eq!(result, "\n=== Exit Code: unknown ===\n");
}

#[test]
fn test_none_exit_code_with_multiline_log() {
    // Test appending None exit code to multiline content
    let multiline_log = "Line 1\nLine 2\nLine 3";
    let result = append_exit_code_to_log(multiline_log, None);

    // Verify original content is preserved
    assert!(result.contains("Line 1"));
    assert!(result.contains("Line 2"));
    assert!(result.contains("Line 3"));

    // Verify unknown exit code is appended
    assert!(result.contains("=== Exit Code: unknown ==="));
}

#[test]
fn test_none_exit_code_formatting_consistency() {
    // Test that all None-like cases format consistently
    let cases = vec![
        format_exit_code(None),
        format_exit_code(Some(ExitCode::None)),
        ProcessTermination::Unknown.format(),
    ];

    // All should contain "(none)" or "unknown"
    for case in cases {
        assert!(case.contains("(none)") || case.contains("unknown"));
    }
}

#[test]
fn test_none_exit_code_equality_comparisons() {
    // Test ExitCode::None equality
    let none1 = ExitCode::None;
    let none2 = ExitCode::None;
    assert_eq!(none1, none2);

    // Test ProcessTermination::Unknown equality
    let unknown1 = ProcessTermination::Unknown;
    let unknown2 = ProcessTermination::Unknown;
    assert_eq!(unknown1, unknown2);

    // Test that different None representations are not equal
    assert_ne!(format_exit_code(None), format_exit_code(Some(ExitCode::Code(0))));
}

#[test]
fn test_none_exit_code_with_special_characters() {
    // Test None exit code with logs containing special characters
    let special_log = "Log with émojis 🎉 and spëcial çharacters";
    let result = append_exit_code_to_log(special_log, None);

    assert!(result.contains("émojis"));
    assert!(result.contains("🎉"));
    assert!(result.contains("=== Exit Code: unknown ==="));
}

#[test]
fn test_none_exit_code_multiple_appends() {
    // Test multiple consecutive None exit codes
    let log = "Original";
    let first = append_exit_code_to_log(log, None);
    let second = append_exit_code_to_log(&first, None);

    // Should have two "unknown" exit codes
    let count = second.matches("=== Exit Code: unknown ===").count();
    assert_eq!(count, 2);
}

#[test]
fn test_none_exit_code_between_valid_codes() {
    // Test None exit code sandwiched between valid codes
    let log = "Process output";
    let with_none = append_exit_code_to_log(log, None);
    let with_first = append_exit_code_to_log(&with_none, Some(0));
    let with_second = append_exit_code_to_log(&with_first, Some(1));

    assert!(with_second.contains("=== Exit Code: unknown ==="));
    assert!(with_second.contains("=== Exit Code: 0 ==="));
    assert!(with_second.contains("=== Exit Code: 1 ==="));
}

#[test]
fn test_none_exit_code_debug_display() {
    // Test Debug trait for ExitCode::None
    let none = ExitCode::None;
    let debug = format!("{:?}", none);
    assert!(debug.contains("None"));

    // Test Debug trait for ProcessTermination::Unknown
    let unknown = ProcessTermination::Unknown;
    let debug = format!("{:?}", unknown);
    assert!(debug.contains("Unknown"));
}

#[test]
fn test_none_exit_code_clone_behavior() {
    // Test that ExitCode::None clones correctly
    let none = ExitCode::None;
    let cloned = none.clone();
    assert_eq!(none, cloned);

    // Test that ProcessTermination::Unknown clones correctly
    let unknown = ProcessTermination::Unknown;
    let cloned_unknown = unknown.clone();
    assert_eq!(unknown, cloned_unknown);
}

#[test]
fn test_none_exit_code_with_whitespace() {
    // Test None exit code with logs containing leading/trailing whitespace
    let whitespace_log = "  \n  Line with spaces  \n  ";
    let result = append_exit_code_to_log(whitespace_log, None);

    assert!(result.contains("=== Exit Code: unknown ==="));
    assert!(result.contains("Line with spaces"));
}

#[test]
fn test_none_exit_code_format_structure() {
    // Test that None exit code formatting maintains correct structure
    let result = format_exit_code(None);

    // Should have exactly 3 equals at start and end
    assert!(result.starts_with("==="));
    assert!(result.ends_with("==="));

    // Should not have 4 equals at start or end
    assert!(!result.starts_with("===="));
    assert!(!result.ends_with("===="));

    // Should contain "(none)" substring
    assert!(result.contains("(none)"));
}

#[test]
fn test_none_vs_zero_exit_code_distinction() {
    // Test that None exit code is distinctly different from zero exit code
    let none_result = format_exit_code(None);
    let zero_result = format_exit_code(Some(ExitCode::Code(0)));

    assert_ne!(none_result, zero_result);
    assert!(none_result.contains("(none)"));
    assert!(zero_result.contains("0"));
    assert!(!zero_result.contains("(none)"));
}

#[test]
fn test_none_exit_code_graceful_handling() {
    // Test that None exit code is handled gracefully across all functions

    // format_exit_code should not panic with None
    let formatted = format_exit_code(None);
    assert!(!formatted.is_empty());

    // ProcessTermination::from_code should not panic with None
    let term = ProcessTermination::from_code(None);
    assert_eq!(term, ProcessTermination::Unknown);

    // append_exit_code_to_log should not panic with None
    let log = "Test log";
    let result = append_exit_code_to_log(log, None);
    assert!(!result.is_empty());
    assert!(result.contains(log));
}

#[test]
fn test_exit_code_none_display_format() {
    // Test Display trait implementation for ExitCode::None
    let none = ExitCode::None;
    let display = format!("{}", none);

    // Should be lowercase "none"
    assert_eq!(display, "none");
    assert!(!display.contains("None")); // Not "None" with capital N
    assert!(!display.contains("NONE")); // Not "NONE" in all caps
}
