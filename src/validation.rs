//! Validation functions for bead fields
//!
//! This module provides validation functions for various bead fields
//! to ensure data integrity and provide clear error messages.

use anyhow::{bail, Result};

/// Validate an assignee field value
///
/// Rules:
/// - If `None`, the field is optional (valid)
/// - If `Some`, the string must be non-empty after trimming whitespace
///
/// # Examples
/// ```
/// use bead_forge::validation::validate_assignee;
///
/// // None is valid (assignee is optional)
/// assert!(validate_assignee(None).is_ok());
///
/// // Valid assignee
/// assert!(validate_assignee(Some("alice")).is_ok());
/// assert!(validate_assignee(Some("alice@example.com")).is_ok());
///
/// // Empty string is invalid
/// assert!(validate_assignee(Some("")).is_err());
///
/// // Whitespace-only string is invalid
/// assert!(validate_assignee(Some("   ")).is_err());
/// ```
pub fn validate_assignee(assignee: Option<&str>) -> Result<()> {
    if let Some(value) = assignee {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            bail!("Assignee cannot be empty or whitespace-only");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_assignee_none_is_valid() {
        assert!(validate_assignee(None).is_ok());
    }

    #[test]
    fn test_validate_assignee_valid_string() {
        assert!(validate_assignee(Some("alice")).is_ok());
        assert!(validate_assignee(Some("alice@example.com")).is_ok());
        assert!(validate_assignee(Some("Alice Smith")).is_ok());
        assert!(validate_assignee(Some("alice-worker-1")).is_ok());
    }

    #[test]
    fn test_validate_assignee_empty_string() {
        let result = validate_assignee(Some(""));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Assignee cannot be empty or whitespace-only"
        );
    }

    #[test]
    fn test_validate_assignee_whitespace_only() {
        let result = validate_assignee(Some("   "));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Assignee cannot be empty or whitespace-only"
        );
    }

    #[test]
    fn test_validate_assignee_whitespace_with_content() {
        // Strings with content after trimming whitespace should be valid
        assert!(validate_assignee(Some("  alice  ")).is_ok());
        assert!(validate_assignee(Some("\t alice\t")).is_ok());
    }
}
