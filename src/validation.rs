//! Normalization helpers for bead fields
//!
//! This module provides normalization functions for various bead fields
//! so that empty/whitespace-only input is collapsed to `None` rather than
//! persisted as a literal empty string.

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
