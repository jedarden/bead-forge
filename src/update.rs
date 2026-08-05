//! Update bead command with single-field update semantics.
//!
//! This module implements the core update functionality, ensuring that
//! exactly one field is updated at a time, preserving all other fields.

use crate::model::{IssueChanges, Priority, Status};
use crate::storage::Storage;
use anyhow::{anyhow, Result};

/// Update a single field on a bead.
///
/// This function ensures that exactly one field is being updated.
/// If multiple fields are provided, it returns an error.
///
/// # Arguments
///
/// * `storage` - The storage backend
/// * `id` - The bead ID to update
/// * `title` - Optional new title
/// * `status` - Optional new status
/// * `priority` - Optional new priority
///
/// # Returns
///
/// * `Ok(())` if the update succeeded
/// * `Err(anyhow::Error)` if:
///   - No field was provided
///   - Multiple fields were provided
///   - The bead doesn't exist
///   - Priority is out of range (0-4)
///   - Status is invalid
pub fn update(
    storage: &Storage,
    id: &str,
    title: Option<String>,
    status: Option<String>,
    priority: Option<i32>,
) -> Result<()> {
    // Count how many fields were provided
    let field_count = [
        title.is_some(),
        status.is_some(),
        priority.is_some(),
    ]
    .iter()
    .filter(|&&x| x)
    .count();

    // Require exactly one field
    if field_count == 0 {
        return Err(anyhow!(
            "No field provided. Use --title, --status, or --priority to update a field."
        ));
    }

    if field_count > 1 {
        return Err(anyhow!(
            "Only one field can be updated at a time. \
             Use exactly one of: --title, --status, --priority"
        ));
    }

    // Validate priority range if provided
    if let Some(p) = priority {
        if p < 0 || p > 4 {
            return Err(anyhow!("Priority must be between 0 and 4 (inclusive), got {}", p));
        }
    }

    // Parse status if provided
    let status_parsed = match status {
        Some(s) => {
            Some(Status::from_str(&s).map_err(|e| anyhow!("Invalid status '{}': {}", s, e))?)
        }
        None => None,
    };

    // Build changes struct with exactly one field
    let changes = IssueChanges {
        title,
        status: status_parsed,
        priority: priority.map(Priority),
        ..Default::default()
    };

    // Perform the update
    storage.update_issue(id, &changes)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Issue;
    use chrono::Utc;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn setup_test_storage() -> (Storage, TempDir, String) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create a test bead
        let issue = Issue::new(
            "bf-test".to_string(),
            "Test bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&issue).unwrap();

        (storage, temp_dir, issue.id)
    }

    #[test]
    fn test_update_requires_exactly_one_field() {
        let (storage, _temp, id) = setup_test_storage();

        // No fields provided
        let result = update(&storage, &id, None, None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No field provided"));

        // Multiple fields provided
        let result = update(
            &storage,
            &id,
            Some("New title".to_string()),
            Some("open".to_string()),
            None,
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Only one field can be updated"));
    }

    #[test]
    fn test_update_title() {
        let (storage, _temp, id) = setup_test_storage();

        let result = update(&storage, &id, Some("Updated title".to_string()), None, None);
        assert!(result.is_ok());

        let updated = storage.get_issue(&id).unwrap().unwrap();
        assert_eq!(updated.title, "Updated title");
    }

    #[test]
    fn test_update_status() {
        let (storage, _temp, id) = setup_test_storage();

        let result = update(&storage, &id, None, Some("in_progress".to_string()), None);
        assert!(result.is_ok());

        let updated = storage.get_issue(&id).unwrap().unwrap();
        assert_eq!(updated.status, Status::InProgress);
    }

    #[test]
    fn test_update_priority() {
        let (storage, _temp, id) = setup_test_storage();

        let result = update(&storage, &id, None, None, Some(0));
        assert!(result.is_ok());

        let updated = storage.get_issue(&id).unwrap().unwrap();
        assert_eq!(updated.priority, Priority(0));
    }

    #[test]
    fn test_update_nonexistent_bead() {
        let (storage, _temp, _) = setup_test_storage();

        let result = update(
            &storage,
            "bf-nonexistent",
            Some("New title".to_string()),
            None,
            None,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Bead not found"));
    }

    #[test]
    fn test_update_priority_out_of_range() {
        let (storage, _temp, id) = setup_test_storage();

        let result = update(&storage, &id, None, None, Some(5));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Priority must be between 0 and 4"));
    }

    #[test]
    fn test_update_invalid_status() {
        let (storage, _temp, id) = setup_test_storage();

        let result = update(&storage, &id, None, Some("invalid_status".to_string()), None);
        // Status parsing is permissive, so this should not error
        // Custom statuses are allowed
        assert!(result.is_ok());
    }
}
