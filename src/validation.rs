//! Normalization helpers for bead fields
//!
//! This module provides normalization functions for various bead fields
//! so that empty/whitespace-only input is collapsed to `None` rather than
//! persisted as a literal empty string.

/// Validate a priority field value.
///
/// Returns `Ok(())` if the priority is in the valid range (0-4), otherwise
/// returns an error with a descriptive message. Priority values correspond to:
/// 0 = Critical, 1 = High, 2 = Medium, 3 = Low, 4 = Backlog.
///
/// # Examples
/// ```
/// use bead_forge::validation::validate_priority;
///
/// // Valid priorities
/// assert!(validate_priority(0).is_ok());   // Critical
/// assert!(validate_priority(2).is_ok());   // Medium
/// assert!(validate_priority(4).is_ok());   // Backlog
///
/// // Invalid priorities
/// assert!(validate_priority(-1).is_err()); // Negative
/// assert!(validate_priority(5).is_err());  // Too high
/// ```
///
/// # Where this is used
///
/// `bf create` and `bf update` call this to validate the priority field before
/// creating or updating a bead. This prevents invalid priority values from being
/// stored in the database.
pub fn validate_priority(priority: i32) -> Result<(), String> {
    if (0..=4).contains(&priority) {
        Ok(())
    } else {
        Err(format!(
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
