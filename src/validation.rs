//! Input validation helpers for bead fields
//!
//! This module provides validation functions for bead IDs, titles, priorities,
//! and normalization functions for various bead fields so that empty/whitespace-only
//! input is collapsed to `None` rather than persisted as a literal empty string.

use std::fmt;

/// Result of a validation operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Input is valid
    Valid,
    /// Input is invalid with a descriptive reason
    Invalid(String),
}

impl ValidationResult {
    /// Returns true if the validation result is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid)
    }

    /// Returns true if the validation result is invalid.
    #[must_use]
    pub fn is_invalid(&self) -> bool {
        matches!(self, Self::Invalid(_))
    }

    /// Convert ValidationResult to Result<(), String>.
    ///
    /// Returns `Ok(())` if valid, `Err(reason)` if invalid.
    ///
    /// # Examples
    /// ```
    /// use bead_forge::validation::{ValidationResult, validate_priority};
    ///
    /// let result = validate_priority(2);
    /// assert!(result.to_result().is_ok());
    ///
    /// let result = validate_priority(99);
    /// assert!(result.to_result().is_err());
    /// ```
    #[must_use]
    pub fn to_result(&self) -> Result<(), String> {
        match self {
            Self::Valid => Ok(()),
            Self::Invalid(reason) => Err(reason.clone()),
        }
    }
}

impl fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Valid => write!(f, "Valid"),
            Self::Invalid(reason) => write!(f, "{}", reason),
        }
    }
}

/// Validate a bead ID format.
///
/// Checks that the ID follows the pattern `{prefix}-{hash}` where prefix is
/// one of: `bf`, `bd`, `nd`, or `needle`, and hash is at least one alphanumeric
/// character.
///
/// # Examples
/// ```
/// use bead_forge::validation::{validate_bead_id, ValidationResult};
///
/// // Valid IDs
/// assert_eq!(validate_bead_id("bf-abc123"), ValidationResult::Valid);
/// assert_eq!(validate_bead_id("bd-x1y2z3"), ValidationResult::Valid);
/// assert_eq!(validate_bead_id("nd-test123"), ValidationResult::Valid);
/// assert_eq!(validate_bead_id("needle-abc"), ValidationResult::Valid);
///
/// // Invalid IDs
/// assert!(validate_bead_id("invalid").is_invalid());
/// assert!(validate_bead_id("bf-").is_invalid());
/// assert!(validate_bead_id("xyz-123").is_invalid());
/// ```
pub fn validate_bead_id(id: &str) -> ValidationResult {
    let parts: Vec<&str> = id.split('-').collect();
    if parts.len() < 2 {
        return ValidationResult::Invalid(format!(
            "Invalid bead ID format: '{}'. Must be {{bf,bd,nd,needle}}-{{hash}}",
            id
        ));
    }

    let prefix = parts[0];
    let hash_part = parts[1..].join("");

    // Validate prefix
    if !matches!(prefix, "bf" | "bd" | "nd" | "needle") {
        return ValidationResult::Invalid(format!(
            "Invalid bead ID prefix: '{}'. Must be one of: bf, bd, nd, needle",
            prefix
        ));
    }

    // Validate hash part is non-empty and alphanumeric
    if hash_part.is_empty() {
        return ValidationResult::Invalid(format!(
            "Invalid bead ID: '{}'. Hash part cannot be empty",
            id
        ));
    }

    if !hash_part.chars().all(|c| c.is_ascii_alphanumeric()) {
        return ValidationResult::Invalid(format!(
            "Invalid bead ID: '{}'. Hash part must contain only alphanumeric characters",
            id
        ));
    }

    ValidationResult::Valid
}

/// Validate a bead title.
///
/// Checks that the title is non-empty and within a reasonable length (1-500 characters).
///
/// # Examples
/// ```
/// use bead_forge::validation::{validate_title, ValidationResult};
///
/// // Valid titles
/// assert_eq!(validate_title("Fix the bug"), ValidationResult::Valid);
/// assert_eq!(validate_title("A"), ValidationResult::Valid);
///
/// // Invalid titles
/// assert!(validate_title("").is_invalid());
/// assert!(validate_title("   ").is_invalid());
/// ```
pub fn validate_title(title: &str) -> ValidationResult {
    let trimmed = title.trim();

    if trimmed.is_empty() {
        return ValidationResult::Invalid(
            "Title cannot be empty or whitespace-only".to_string()
        );
    }

    if trimmed.len() > 500 {
        return ValidationResult::Invalid(format!(
            "Title too long: {} characters. Maximum is 500 characters",
            trimmed.len()
        ));
    }

    ValidationResult::Valid
}

/// Validate a priority field value.
///
/// Returns `ValidationResult::Valid` if the priority is in the valid range (0-4),
/// otherwise returns `ValidationResult::Invalid` with a descriptive message.
/// Priority values correspond to: 0 = Critical, 1 = High, 2 = Medium, 3 = Low, 4 = Backlog.
///
/// # Examples
/// ```
/// use bead_forge::validation::{validate_priority, ValidationResult};
///
/// // Valid priorities
/// assert_eq!(validate_priority(0), ValidationResult::Valid);   // Critical
/// assert_eq!(validate_priority(2), ValidationResult::Valid);   // Medium
/// assert_eq!(validate_priority(4), ValidationResult::Valid);   // Backlog
///
/// // Invalid priorities
/// assert!(validate_priority(-1).is_invalid()); // Negative
/// assert!(validate_priority(5).is_invalid());  // Too high
/// ```
///
/// # Where this is used
///
/// `bf create` and `bf update` call this to validate the priority field before
/// creating or updating a bead. This prevents invalid priority values from being
/// stored in the database.
pub fn validate_priority(priority: i32) -> ValidationResult {
    if (0..=4).contains(&priority) {
        ValidationResult::Valid
    } else {
        ValidationResult::Invalid(format!(
            "Invalid priority: {}. Must be 0-4 (0=Critical, 1=High, 2=Medium, 3=Low, 4=Backlog)",
            priority
        ))
    }
}

/// Normalize an assignee field value.
///
/// Trims surrounding whitespace and collapses empty/whitespace-only input to
/// `None`. This lets `bf create --assignee ''` create a bead with no assignee
/// instead of persisting a literal empty string — which would read back as
/// "assigned" and hide the bead from claiming.
///
/// # Examples
/// ```
/// use bead_forge::validation::normalize_assignee;
///
/// // None passes through (assignee is optional)
/// assert_eq!(normalize_assignee(None), None);
///
/// // A real value is trimmed and kept
/// assert_eq!(normalize_assignee(Some("alice")), Some("alice".to_string()));
/// assert_eq!(normalize_assignee(Some("  alice  ")), Some("alice".to_string()));
///
/// // Empty / whitespace-only collapses to None
/// assert_eq!(normalize_assignee(Some("")), None);
/// assert_eq!(normalize_assignee(Some("   ")), None);
/// ```
///
/// # Where this is used
///
/// `bf create` calls this to derive the new bead's assignee. `bf update` does
/// NOT: its `--assignee` value is three-valued (`None` = leave unchanged,
/// `Some("")` = clear to NULL, `Some(x)` = set), and collapsing empty to `None`
/// would erase the "clear" intent. `update_issue`'s storage layer performs the
/// equivalent trim-and-NULL mapping internally, so `bf update --assignee ''`
/// clears the assignee without any CLI-level normalization.
pub fn normalize_assignee(assignee: Option<&str>) -> Option<String> {
    assignee
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ValidationResult tests

    #[test]
    fn test_validation_result_is_valid() {
        assert!(ValidationResult::Valid.is_valid());
        assert!(!ValidationResult::Invalid("error".to_string()).is_valid());
    }

    #[test]
    fn test_validation_result_is_invalid() {
        assert!(ValidationResult::Invalid("error".to_string()).is_invalid());
        assert!(!ValidationResult::Valid.is_invalid());
    }

    // validate_bead_id tests

    #[test]
    fn test_validate_bead_id_valid_bf() {
        assert_eq!(validate_bead_id("bf-abc123"), ValidationResult::Valid);
        assert_eq!(validate_bead_id("bf-a1b2c3"), ValidationResult::Valid);
        assert_eq!(validate_bead_id("bf-xyz"), ValidationResult::Valid);
    }

    #[test]
    fn test_validate_bead_id_valid_bd() {
        assert_eq!(validate_bead_id("bd-abc123"), ValidationResult::Valid);
        assert_eq!(validate_bead_id("bd-x1y2z3"), ValidationResult::Valid);
    }

    #[test]
    fn test_validate_bead_id_valid_nd() {
        assert_eq!(validate_bead_id("nd-test123"), ValidationResult::Valid);
        assert_eq!(validate_bead_id("nd-abc"), ValidationResult::Valid);
    }

    #[test]
    fn test_validate_bead_id_valid_needle() {
        assert_eq!(validate_bead_id("needle-abc"), ValidationResult::Valid);
        assert_eq!(validate_bead_id("needle-123456"), ValidationResult::Valid);
    }

    #[test]
    fn test_validate_bead_id_invalid_no_dash() {
        let result = validate_bead_id("invalid");
        assert!(result.is_invalid());
        assert!(result.to_string().contains("Invalid bead ID format"));
    }

    #[test]
    fn test_validate_bead_id_invalid_empty_hash() {
        let result = validate_bead_id("bf-");
        assert!(result.is_invalid());
        assert!(result.to_string().contains("Hash part cannot be empty"));
    }

    #[test]
    fn test_validate_bead_id_invalid_prefix() {
        let result = validate_bead_id("xyz-123");
        assert!(result.is_invalid());
        assert!(result.to_string().contains("Invalid bead ID prefix"));
    }

    #[test]
    fn test_validate_bead_id_invalid_hash_characters() {
        let result = validate_bead_id("bf-abc-123!");
        assert!(result.is_invalid());
        assert!(result.to_string().contains("alphanumeric characters"));
    }

    #[test]
    fn test_validate_bead_id_multiple_dashes() {
        // Multiple dashes should be valid as long as prefix and combined hash are valid
        assert_eq!(validate_bead_id("bf-abc-123"), ValidationResult::Valid);
    }

    // validate_title tests

    #[test]
    fn test_validate_title_valid() {
        assert_eq!(validate_title("Fix the bug"), ValidationResult::Valid);
        assert_eq!(validate_title("A"), ValidationResult::Valid);
        assert_eq!(validate_title("Implement feature XYZ"), ValidationResult::Valid);
    }

    #[test]
    fn test_validate_title_empty() {
        let result = validate_title("");
        assert!(result.is_invalid());
        assert!(result.to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_title_whitespace_only() {
        let result = validate_title("   ");
        assert!(result.is_invalid());
        assert!(result.to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_title_too_long() {
        let long_title = "a".repeat(501);
        let result = validate_title(&long_title);
        assert!(result.is_invalid());
        assert!(result.to_string().contains("too long"));
    }

    #[test]
    fn test_validate_title_exactly_max_length() {
        let max_title = "a".repeat(500);
        assert_eq!(validate_title(&max_title), ValidationResult::Valid);
    }

    #[test]
    fn test_validate_title_with_padding_whitespace() {
        assert_eq!(validate_title("  Valid title  "), ValidationResult::Valid);
    }

    // validate_priority tests

    #[test]
    fn test_validate_priority_valid_all_values() {
        assert_eq!(validate_priority(0), ValidationResult::Valid);   // Critical
        assert_eq!(validate_priority(1), ValidationResult::Valid);   // High
        assert_eq!(validate_priority(2), ValidationResult::Valid);   // Medium
        assert_eq!(validate_priority(3), ValidationResult::Valid);   // Low
        assert_eq!(validate_priority(4), ValidationResult::Valid);   // Backlog
    }

    #[test]
    fn test_validate_priority_invalid_negative() {
        let result = validate_priority(-1);
        assert!(result.is_invalid());
        assert!(result.to_string().contains("Invalid priority"));
    }

    #[test]
    fn test_validate_priority_invalid_too_high() {
        let result = validate_priority(5);
        assert!(result.is_invalid());
        assert!(result.to_string().contains("Invalid priority"));
    }

    #[test]
    fn test_validate_priority_invalid_very_negative() {
        let result = validate_priority(-100);
        assert!(result.is_invalid());
    }

    #[test]
    fn test_validate_priority_invalid_very_high() {
        let result = validate_priority(100);
        assert!(result.is_invalid());
    }

    // normalize_assignee tests

    #[test]
    fn test_normalize_assignee_none() {
        assert_eq!(normalize_assignee(None), None);
    }

    #[test]
    fn test_normalize_assignee_real_value() {
        assert_eq!(normalize_assignee(Some("alice")), Some("alice".to_string()));
        assert_eq!(
            normalize_assignee(Some("alice@example.com")),
            Some("alice@example.com".to_string())
        );
        assert_eq!(
            normalize_assignee(Some("Alice Smith")),
            Some("Alice Smith".to_string())
        );
        assert_eq!(
            normalize_assignee(Some("alice-worker-1")),
            Some("alice-worker-1".to_string())
        );
    }

    #[test]
    fn test_normalize_assignee_trims_padding() {
        assert_eq!(
            normalize_assignee(Some("  alice  ")),
            Some("alice".to_string())
        );
        assert_eq!(
            normalize_assignee(Some("\t alice\t")),
            Some("alice".to_string())
        );
    }

    #[test]
    fn test_normalize_assignee_collapses_empty() {
        assert_eq!(normalize_assignee(Some("")), None);
        assert_eq!(normalize_assignee(Some("   ")), None);
        assert_eq!(normalize_assignee(Some("\t\t")), None);
    }
}
