//! Test infrastructure for separator formatting tests.
//!
//! Provides common utilities, helper functions, and test fixtures for testing
//! separator formatting across all bead-forge formatters (text, table, json, toon, exit codes).
//!
//! # Separator Types
//!
//! - **Equals separators**: `=== content ===` (exit codes, detail views)
//! - **Dash separators**: `---` (velocity stats, section dividers)
//! - **Table separators**: `-+-` (table column boundaries)
//! - **Mix separators**: `-+-+` (multi-column table separators)
//!
//! # Usage
//!
//! ```rust
//! use separator_formatting_test_infrastructure::*;
//!
//! #[test]
//! fn test_my_formatter_separator() {
//!     let output = "=== My Content ===";
//!     assert!(has_exact_equals_count(output, 3));
//!     assert!(separator_positioning_correct(output, SeparatorPosition::FullLine));
//! }
//! ```

use std::fmt::Debug;

/// Separator position within formatted output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeparatorPosition {
    /// Separator spans the full line (e.g., `=================================================================================`)
    FullLine,
    /// Separator surrounds content (e.g., `=== content ===`)
    SurroundsContent,
    /// Separator appears between columns (e.g., `-+-` in tables)
    BetweenColumns,
    /// Separator appears after header (e.g., dashes under table header)
    AfterHeader,
}

/// Separator character type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeparatorChar {
    /// Equals sign (=)
    Equals,
    /// Dash (-)
    Dash,
    /// Plus sign (+)
    Plus,
    /// Custom character
    Custom(char),
}

impl SeparatorChar {
    /// Get the actual character value
    pub fn as_char(self) -> char {
        match self {
            SeparatorChar::Equals => '=',
            SeparatorChar::Dash => '-',
            SeparatorChar::Plus => '+',
            SeparatorChar::Custom(c) => c,
        }
    }
}

/// Separator validation result
#[derive(Debug, Clone, PartialEq)]
pub enum SeparatorValidation {
    /// Separator is valid
    Valid,
    /// Invalid separator count
    InvalidCount { expected: usize, found: usize },
    /// Invalid separator position
    InvalidPosition(String),
    /// Invalid separator character
    InvalidChar { expected: char, found: char },
    /// Mixed separator characters
    MixedChars(Vec<char>),
}

/// Check if a line has exactly the specified count of separator characters at the start
///
/// # Arguments
///
/// * `line` - The line to check
/// * `char` - The separator character to count
/// * `expected_count` - The expected count of separator characters
///
/// # Returns
///
/// `true` if the line starts with exactly `expected_count` of the specified character
///
/// # Example
///
/// ```rust
/// assert!(has_exact_start_count("=== content ===", '=', 3));
/// assert!(!has_exact_start_count("==== content ===", '=', 3));
/// ```
pub fn has_exact_start_count(line: &str, char: char, expected_count: usize) -> bool {
    let start_count = line.chars().take_while(|&c| c == char).count();
    start_count == expected_count
}

/// Check if a line has exactly the specified count of separator characters at the end
///
/// # Arguments
///
/// * `line` - The line to check
/// * `char` - The separator character to count
/// * `expected_count` - The expected count of separator characters
///
/// # Returns
///
/// `true` if the line ends with exactly `expected_count` of the specified character
pub fn has_exact_end_count(line: &str, char: char, expected_count: usize) -> bool {
    let end_count = line.chars().rev().take_while(|&c| c == char).count();
    end_count == expected_count
}

/// Check if a line has exactly the specified count of separator characters at both start and end
///
/// # Arguments
///
/// * `line` - The line to check
/// * `char` - The separator character to count
/// * `expected_count` - The expected count at both start and end
///
/// # Returns
///
/// `true` if the line starts and ends with exactly `expected_count` of the specified character
pub fn has_exact_equals_count(line: &str, char: char, expected_count: usize) -> bool {
    has_exact_start_count(line, char, expected_count) && has_exact_end_count(line, char, expected_count)
}

/// Count separator characters in a line
///
/// # Arguments
///
/// * `line` - The line to count characters in
/// * `char` - The separator character to count
///
/// # Returns
///
/// The total count of the specified character in the line
pub fn count_separator_chars(line: &str, char: char) -> usize {
    line.chars().filter(|&c| c == char).count()
}

/// Check if a line consists entirely of a single separator character
///
/// # Arguments
///
/// * `line` - The line to check
/// * `char` - The expected separator character
///
/// # Returns
///
/// `true` if the line contains only the specified separator character
pub fn is_separator_only_line(line: &str, char: char) -> bool {
    !line.is_empty() && line.chars().all(|c| c == char)
}

/// Find the separator line in multi-line output
///
/// # Arguments
///
/// * `output` - The multi-line output to search
/// * `char` - The separator character to look for
///
/// # Returns
///
/// `Some(line)` if a separator-only line is found, `None` otherwise
pub fn find_separator_line(output: &str, char: char) -> Option<String> {
    output
        .lines()
        .find(|line| is_separator_only_line(line, char))
        .map(|line| line.to_string())
}

/// Extract content between surrounding separators
///
/// # Arguments
///
/// * `line` - A line with surrounding separators (e.g., `=== content ===`)
/// * `char` - The separator character
/// * `separator_count` - The count of separator characters on each side
///
/// # Returns
///
/// The content between the separators, or the original line if no separators found
///
/// # Example
///
/// ```rust
/// assert_eq!(extract_content_between("=== content ===", '=', 3), " content ");
/// ```
pub fn extract_content_between(line: &str, char: char, separator_count: usize) -> String {
    if line.len() < separator_count * 2 {
        return line.to_string();
    }

    let start_sep = line.chars().take(separator_count).all(|c| c == char);
    let end_sep = line.chars().rev().take(separator_count).all(|c| c == char);

    if start_sep && end_sep {
        line[separator_count..line.len() - separator_count].to_string()
    } else {
        line.to_string()
    }
}

/// Validate separator formatting in output
///
/// # Arguments
///
/// * `output` - The formatted output to validate
/// * `expected_char` - The expected separator character
/// * `expected_count` - The expected count of separator characters
/// * `position` - The expected separator position
///
/// # Returns
///
/// A `SeparatorValidation` result indicating whether the separator is valid
pub fn validate_separator_format(
    output: &str,
    expected_char: char,
    expected_count: usize,
    position: SeparatorPosition,
) -> SeparatorValidation {
    match position {
        SeparatorPosition::FullLine => {
            if let Some(separator_line) = find_separator_line(output, expected_char) {
                let count = count_separator_chars(&separator_line, expected_char);
                if count != expected_count {
                    return SeparatorValidation::InvalidCount { expected: expected_count, found: count };
                }
                SeparatorValidation::Valid
            } else {
                SeparatorValidation::InvalidPosition("No full-line separator found".to_string())
            }
        }
        SeparatorPosition::SurroundsContent => {
            let lines: Vec<&str> = output.lines().collect();
            for line in lines {
                if line.contains(expected_char) {
                    let start_count = line.chars().take_while(|&c| c == expected_char).count();
                    let end_count = line.chars().rev().take_while(|&c| c == expected_char).count();

                    if start_count == expected_count && end_count == expected_count {
                        return SeparatorValidation::Valid;
                    } else if start_count != 0 || end_count != 0 {
                        return SeparatorValidation::InvalidCount {
                            expected: expected_count,
                            found: start_count.max(end_count)
                        };
                    }
                }
            }
            SeparatorValidation::InvalidPosition("No surrounding separator found".to_string())
        }
        SeparatorPosition::BetweenColumns => {
            // Check for table-style separators like "-+-"
            if output.contains(&format!("{}{}{}", expected_char, expected_char, expected_char)) {
                SeparatorValidation::Valid
            } else {
                SeparatorValidation::InvalidPosition("No column separator found".to_string())
            }
        }
        SeparatorPosition::AfterHeader => {
            // Check for separator after header (dashes under text)
            let lines: Vec<&str> = output.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if i > 0 && is_separator_only_line(line, expected_char) {
                    return SeparatorValidation::Valid;
                }
            }
            SeparatorValidation::InvalidPosition("No header separator found".to_string())
        }
    }
}

/// Check if separator positioning is correct in multi-line output
///
/// # Arguments
///
/// * `output` - The multi-line output to check
/// * `position` - The expected separator position
///
/// # Returns
///
/// `true` if the separator positioning matches the expected position
pub fn separator_positioning_correct(output: &str, position: SeparatorPosition) -> bool {
    match position {
        SeparatorPosition::FullLine => {
            find_separator_line(output, '=').is_some() || find_separator_line(output, '-').is_some()
        }
        SeparatorPosition::SurroundsContent => {
            output.lines().any(|line| {
                let has_start = line.starts_with("===") || line.starts_with("---");
                let has_end = line.ends_with("===") || line.ends_with("---");
                has_start && has_end
            })
        }
        SeparatorPosition::BetweenColumns => {
            output.contains("-+-") || output.contains("=+= ")
        }
        SeparatorPosition::AfterHeader => {
            let lines: Vec<&str> = output.lines().collect();
            if lines.len() < 2 {
                return false;
            }

            // Check if second line is a separator
            is_separator_only_line(lines.get(1).unwrap_or(&""), '-')
                || is_separator_only_line(lines.get(1).unwrap_or(&""), '=')
        }
    }
}

/// Create a test fixture with common separator patterns
///
/// # Returns
///
/// A map of separator pattern names to their expected string values
pub fn separator_test_fixtures() -> std::collections::HashMap<String, String> {
    let mut fixtures = std::collections::HashMap::new();

    fixtures.insert(
        "exit_code_zero".to_string(),
        "=== Exit Code: 0 ===".to_string()
    );
    fixtures.insert(
        "exit_code_signal".to_string(),
        "=== Signal: SIGTERM ===".to_string()
    );
    fixtures.insert(
        "exit_code_none".to_string(),
        "=== Exit Code: (none) ===".to_string()
    );
    fixtures.insert(
        "table_separator".to_string(),
        "------+---------+------+------+-----+".to_string()
    );
    fixtures.insert(
        "velocity_separator".to_string(),
        "-------------------------------------------------------------------------------------".to_string()
    );
    fixtures.insert(
        "full_line_equals".to_string(),
        "=====================================================================================".to_string()
    );

    fixtures
}

/// Assert that a line has the exact separator count
///
/// # Panics
///
/// Panics if the line doesn't have the expected separator count
///
/// # Example
///
/// ```rust
/// assert_exact_separator_count("=== content ===", '=', 3);
/// ```
pub fn assert_exact_separator_count(line: &str, char: char, expected_count: usize) {
    let start_count = line.chars().take_while(|&c| c == char).count();
    let end_count = line.chars().rev().take_while(|&c| c == char).count();

    assert_eq!(
        start_count, expected_count,
        "Expected {} {} characters at start, found {} in line: {}",
        expected_count, char, start_count, line
    );

    assert_eq!(
        end_count, expected_count,
        "Expected {} {} characters at end, found {} in line: {}",
        expected_count, char, end_count, line
    );
}

/// Assert that separator width matches header width (for tables)
///
/// # Arguments
///
/// * `header_line` - The header line to match width against
/// * `separator_line` - The separator line to check
///
/// # Panics
///
/// Panics if the separator width doesn't match the header width
pub fn assert_separator_matches_header_width(header_line: &str, separator_line: &str) {
    assert_eq!(
        header_line.len(), separator_line.len(),
        "Separator width ({} chars) must match header width ({} chars)\nHeader: {}\nSeparator: {}",
        separator_line.len(), header_line.len(), header_line, separator_line
    );
}

/// Test module with basic sanity checks for the infrastructure
#[cfg(test)]
mod infrastructure_tests {
    use super::*;

    #[test]
    fn test_has_exact_start_count() {
        assert!(has_exact_start_count("=== content ===", '=', 3));
        assert!(!has_exact_start_count("==== content ===", '=', 3));
        assert!(!has_exact_start_count("== content ===", '=', 3));
        assert!(has_exact_start_count("--- content", '-', 3));
    }

    #[test]
    fn test_has_exact_end_count() {
        assert!(has_exact_end_count("=== content ===", '=', 3));
        assert!(!has_exact_end_count("=== content ====", '=', 3));
        assert!(!has_exact_end_count("=== content ==", '=', 3));
    }

    #[test]
    fn test_has_exact_equals_count() {
        assert!(has_exact_equals_count("=== content ===", '=', 3));
        assert!(!has_exact_equals_count("==== content ===", '=', 3));
        assert!(has_exact_equals_count("--- content ---", '-', 3));
    }

    #[test]
    fn test_count_separator_chars() {
        assert_eq!(count_separator_chars("=== test ===", '='), 6);
        assert_eq!(count_separator_chars("--- test ---", '-'), 6);
        assert_eq!(count_separator_chars("==- test -==", '='), 4);
    }

    #[test]
    fn test_is_separator_only_line() {
        assert!(is_separator_only_line("=====", '='));
        assert!(is_separator_only_line("-----", '-'));
        assert!(!is_separator_only_line("=== text ===", '='));
    }

    #[test]
    fn test_find_separator_line() {
        let output = "Header\n=====\nContent";
        assert_eq!(find_separator_line(output, '='), Some("=====".to_string()));

        let no_separator = "Header\nContent";
        assert_eq!(find_separator_line(no_separator, '='), None);
    }

    #[test]
    fn test_extract_content_between() {
        assert_eq!(extract_content_between("=== content ===", '=', 3), " content ");
        assert_eq!(extract_content_between("---test---", '-', 3), "test");
        assert_eq!(extract_content_between("no separators", '=', 3), "no separators");
    }

    #[test]
    fn test_validate_separator_format_full_line() {
        let output = "Header\n=====\nContent";
        assert_eq!(
            validate_separator_format(output, '=', 5, SeparatorPosition::FullLine),
            SeparatorValidation::Valid
        );
    }

    #[test]
    fn test_validate_separator_format_surrounds() {
        let output = "=== Exit Code: 0 ===";
        assert_eq!(
            validate_separator_format(output, '=', 3, SeparatorPosition::SurroundsContent),
            SeparatorValidation::Valid
        );
    }

    #[test]
    fn test_separator_positioning_correct() {
        let full_line = "Header\n=====\nContent";
        assert!(separator_positioning_correct(full_line, SeparatorPosition::FullLine));

        let surrounds = "=== content ===\nMore content";
        assert!(separator_positioning_correct(surrounds, SeparatorPosition::SurroundsContent));

        let after_header = "Header\n----\nContent";
        assert!(separator_positioning_correct(after_header, SeparatorPosition::AfterHeader));
    }

    #[test]
    fn test_separator_test_fixtures() {
        let fixtures = separator_test_fixtures();
        assert!(fixtures.contains_key("exit_code_zero"));
        assert!(fixtures.contains_key("table_separator"));
        assert!(fixtures.contains_key("velocity_separator"));
    }

    #[test]
    fn test_assert_exact_separator_count() {
        // Should not panic
        assert_exact_separator_count("=== test ===", '=', 3);
    }

    #[test]
    #[should_panic(expected = "Expected 3 = characters at start")]
    fn test_assert_exact_separator_count_panics() {
        assert_exact_separator_count("== test ===", '=', 3);
    }

    #[test]
    fn test_assert_separator_matches_header_width() {
        // Should not panic - widths match
        assert_separator_matches_header_width("ID   | Title  ", "-----|--------");
    }

    #[test]
    #[should_panic(expected = "Separator width")]
    fn test_assert_separator_matches_header_width_panics() {
        assert_separator_matches_header_width("ID   | Title  ", "-----|-------"); // Wrong width
    }

    #[test]
    fn test_separator_char_enum() {
        assert_eq!(SeparatorChar::Equals.as_char(), '=');
        assert_eq!(SeparatorChar::Dash.as_char(), '-');
        assert_eq!(SeparatorChar::Plus.as_char(), '+');
        assert_eq!(SeparatorChar::Custom('x').as_char(), 'x');
    }

    // ==================== Exact Equals Count Verification Tests ====================

    #[test]
    fn test_exact_equals_count_verification_positive_cases() {
        // Test case 1: Standard 3-equal separator with content
        let line1 = "=== Exit Code: 0 ===";
        assert!(has_exact_equals_count(line1, '=', 3),
               "Line with '=== content ===' should have exactly 3 equals at both ends");

        // Test case 2: Standard 3-equal separator with different content
        let line2 = "=== Signal: SIGTERM ===";
        assert!(has_exact_equals_count(line2, '=', 3),
               "Line with '=== content ===' should have exactly 3 equals at both ends");

        // Test case 3: Single equal separator
        let line3 = "= test =";
        assert!(has_exact_equals_count(line3, '=', 1),
               "Line with '= test =' should have exactly 1 equal at both ends");

        // Test case 4: Five equal separator (common in headers)
        let line4 = "===== Header =====";
        assert!(has_exact_equals_count(line4, '=', 5),
               "Line with '===== content =====' should have exactly 5 equals at both ends");

        // Test case 5: Specification requirement: 80 equals (full-line separator)
        let line5 = "=".repeat(80);
        let start_count = line5.chars().take_while(|&c| c == '=').count();
        let end_count = line5.chars().rev().take_while(|&c| c == '=').count();
        assert_eq!(start_count, 80, "Full-line separator should have 80 equals at start");
        assert_eq!(end_count, 80, "Full-line separator should have 80 equals at end");
    }

    #[test]
    fn test_exact_equals_count_verification_negative_cases() {
        // Negative case 1: Too many equals at start (4 instead of 3)
        let line1 = "==== content ===";
        assert!(!has_exact_equals_count(line1, '=', 3),
               "Line with '==== content ===' should NOT validate with expected count of 3");

        // Negative case 2: Too few equals at start (2 instead of 3)
        let line2 = "== content ===";
        assert!(!has_exact_equals_count(line2, '=', 3),
               "Line with '== content ===' should NOT validate with expected count of 3");

        // Negative case 3: Too many equals at end (4 instead of 3)
        let line3 = "=== content ====";
        assert!(!has_exact_equals_count(line3, '=', 3),
               "Line with '=== content ====' should NOT validate with expected count of 3");

        // Negative case 4: Too few equals at end (2 instead of 3)
        let line4 = "=== content ==";
        assert!(!has_exact_equals_count(line4, '=', 3),
               "Line with '=== content ==' should NOT validate with expected count of 3");

        // Negative case 5: Asymmetric separators (3 at start, 2 at end)
        let line5 = "=== content ==";
        assert!(!has_exact_equals_count(line5, '=', 3),
               "Asymmetric line '=== content ==' should NOT validate");

        // Negative case 6: Asymmetric separators (2 at start, 3 at end)
        let line6 = "== content ===";
        assert!(!has_exact_equals_count(line6, '=', 3),
               "Asymmetric line '== content ===' should NOT validate");

        // Negative case 7: Wrong expected count (5 instead of 3)
        let line7 = "=== content ===";
        assert!(!has_exact_equals_count(line7, '=', 5),
               "Line with 3 equals should NOT validate with expected count of 5");

        // Negative case 8: No equals at all
        let line8 = "content only";
        assert!(!has_exact_equals_count(line8, '=', 3),
               "Line with no equals should NOT validate");

        // Negative case 9: Empty line
        let line9 = "";
        assert!(!has_exact_equals_count(line9, '=', 3),
               "Empty line should NOT validate");

        // Negative case 10: Full-line separator with wrong count (79 instead of 80)
        let line10 = "=".repeat(79);
        assert!(!has_exact_equals_count(&line10, '=', 80),
               "Line with 79 equals should NOT validate with expected count of 80");

        // Negative case 11: Full-line separator with wrong count (81 instead of 80)
        let line11 = "=".repeat(81);
        assert!(!has_exact_equals_count(&line11, '=', 80),
               "Line with 81 equals should NOT validate with expected count of 80");
    }

    #[test]
    fn test_exact_equals_count_specification_validation() {
        // Specification requirement: Detail view separators must be exactly 80 equals
        let spec_width = 80;

        // Positive: Exact specification match
        let correct_line = "=".repeat(spec_width);
        let equals_count = count_separator_chars(&correct_line, '=');
        assert_eq!(equals_count, spec_width,
                  "Specification requires exactly {} equals, found {}", spec_width, equals_count);

        // Verify it's a pure separator line
        assert!(is_separator_only_line(&correct_line, '='),
               "Specification requires separator-only line (no other characters)");

        // Negative: Below specification
        let below_spec = "=".repeat(spec_width - 1);
        let below_count = count_separator_chars(&below_spec, '=');
        assert_ne!(below_count, spec_width,
                  "Separator with {} equals does NOT meet specification of {}", below_count, spec_width);

        // Negative: Above specification
        let above_spec = "=".repeat(spec_width + 1);
        let above_count = count_separator_chars(&above_spec, '=');
        assert_ne!(above_count, spec_width,
                  "Separator with {} equals does NOT meet specification of {}", above_count, spec_width);

        // Validate with full positioning
        let output = format!("Header\n{}\nContent", correct_line);
        assert_eq!(validate_separator_format(&output, '=', spec_width, SeparatorPosition::FullLine),
                   SeparatorValidation::Valid,
                   "80-equal separator must validate as FullLine per specification");

        // Validate that wrong count fails
        let wrong_output = format!("Header\n{}\nContent", below_spec);
        match validate_separator_format(&wrong_output, '=', spec_width, SeparatorPosition::FullLine) {
            SeparatorValidation::InvalidCount { expected, found } => {
                assert_eq!(expected, spec_width, "Expected count should be specification width");
                assert_eq!(found, spec_width - 1, "Found count should be below specification");
            }
            _ => panic!("Should return InvalidCount for separator below specification"),
        }
    }

    #[test]
    fn test_exact_equals_count_with_content_variations() {
        // Test various content patterns with exact equals count

        // Pattern 1: Content with spaces
        let line1 = "=== Exit Code: 0 ===";
        assert_exact_separator_count(line1, '=', 3);

        // Pattern 2: Content without spaces
        let line2 = "===Exit Code: 0===";
        assert_exact_separator_count(line2, '=', 3);

        // Pattern 3: Multiple words
        let line3 = "=== Signal: SIGTERM (15) ===";
        assert_exact_separator_count(line3, '=', 3);

        // Pattern 4: Content with numbers
        let line4 = "=== Count: 12345 ===";
        assert_exact_separator_count(line4, '=', 3);

        // Pattern 5: Content with special characters
        let line5 = "=== Status: [OPEN] (P0) ===";
        assert_exact_separator_count(line5, '=', 3);

        // Pattern 6: Content with equals signs in the middle
        let line6 = "=== Formula: x=y+z ===";
        let total_equals = count_separator_chars(line6, '=');
        assert_eq!(total_equals, 7, "Should count ALL equals, including those in content (3+1+3)");
        // But start and end should still be exactly 3
        assert!(has_exact_equals_count(line6, '=', 3), "Start and end should have exactly 3 equals");
    }

    #[test]
    fn test_exact_equals_count_edge_cases() {
        // Edge case 1: Line that is ONLY equals (no content)
        let line1 = "=========="; // 10 equals
        assert!(has_exact_equals_count(line1, '=', 10),
               "Pure equals line should validate with full count");
        assert!(!has_exact_equals_count(line1, '=', 5),
               "Pure equals line should NOT validate with partial count");

        // Edge case 2: Single equals with no content
        let line2 = "=";
        assert!(has_exact_equals_count(line2, '=', 1),
               "Single equals should validate with count of 1");

        // Edge case 3: Very long separator (200 equals)
        let line3 = "=".repeat(200);
        assert!(has_exact_equals_count(&line3, '=', 200),
               "Very long separator should validate with exact count");

        // Edge case 4: Mixed content types
        let line4 = "=== 123 ===";
        assert!(has_exact_equals_count(line4, '=', 3),
               "Mixed alphanumeric content should validate");

        // Edge case 5: Unicode content with equals
        let line5 = "=== Unicode: café ===";
        assert!(has_exact_equals_count(line5, '=', 3),
               "Unicode content should not affect equals counting");

        // Edge case 6: Tabs and newlines in content (shouldn't happen in practice but test anyway)
        let line6 = "=== tab\there ===";
        assert!(has_exact_equals_count(line6, '=', 3),
               "Tabs in content should not affect equals counting");
    }

    #[test]
    fn test_exact_equals_count_multi_context_validation() {
        // Test equals count verification across different formatting contexts

        // Context 1: Exit code display
        let exit_code_output = "=== Exit Code: 0 ===";
        assert!(has_exact_equals_count(exit_code_output, '=', 3),
               "Exit code display must use exactly 3 equals");
        assert_eq!(extract_content_between(exit_code_output, '=', 3), " Exit Code: 0 ",
                  "Should extract content correctly");

        // Context 2: Signal display
        let signal_output = "=== Signal: SIGTERM ===";
        assert!(has_exact_equals_count(signal_output, '=', 3),
               "Signal display must use exactly 3 equals");

        // Context 3: Status headers
        let status_header = "===== Status =====";
        assert!(has_exact_equals_count(status_header, '=', 5),
               "Status headers must use exactly 5 equals");

        // Context 4: Full-line separator (specification: 80 equals)
        let full_separator = "=".repeat(80);
        assert!(is_separator_only_line(&full_separator, '='),
               "Full-line separator must contain only equals");
        assert_eq!(count_separator_chars(&full_separator, '='), 80,
               "Full-line separator must have exactly 80 equals per specification");

        // Context 5: Table detail view separator
        let detail_output = format!("bf-test: Test Issue\n{}\nStatus:", full_separator);
        if let Some(found_separator) = find_separator_line(&detail_output, '=') {
            assert_eq!(count_separator_chars(&found_separator, '='), 80,
                      "Detail view separator must meet specification of 80 equals");
        } else {
            panic!("Should find separator line in detail output");
        }
    }
}
