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
}
