//! Update bead command with single-field update semantics.
//!
//! This module implements the core update functionality, ensuring that
//! exactly one field is updated at a time, preserving all other fields.

use crate::error::{BeadForgeError, Result};
use crate::model::{Priority, Status};
use crate::storage::Storage;

/// Update a single field on a bead.
///
/// This function ensures that exactly one field is being updated and calls
/// the appropriate storage method. All other fields are preserved unchanged.
///
/// # Arguments
///
/// * `storage` - The storage backend
/// * `id` - The bead ID to update
/// * `title` - Optional new title (if Some, all other fields must be None)
/// * `status` - Optional new status (if Some, all other fields must be None)
/// * `priority` - Optional new priority (if Some, all other fields must be None)
///
/// # Returns
///
/// * `Ok(())` if the update succeeded
/// * `Err(BeadForgeError::Validation)` - Multiple fields provided or none provided
/// * `Err(BeadForgeError::NotFound)` - Bead does not exist
/// * `Err(BeadForgeError::Database)` - Storage operation failed
pub fn update(
    storage: &Storage,
    id: &str,
    title: Option<&str>,
    status: Option<Status>,
    priority: Option<Priority>,
) -> Result<()> {
    // Count how many fields are being updated
    let fields_count = [title.is_some(), status.is_some(), priority.is_some()]
        .iter()
        .filter(|&&x| x)
        .count();

    // Validate that exactly one field is provided
    if fields_count == 0 {
        return Err(BeadForgeError::validation(
            "At least one field must be provided for update (title, status, or priority)",
        ));
    }

    if fields_count > 1 {
        return Err(BeadForgeError::validation(
            "Only one field can be updated at a time (title, status, or priority)",
        ));
    }

    // Call the appropriate update method based on which field is set
    if let Some(new_title) = title {
        storage.update_title(id, new_title)?;
    } else if let Some(new_status) = status {
        storage.update_status(id, new_status)?;
    } else if let Some(new_priority) = priority {
        storage.update_priority(id, new_priority)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Issue;

    fn setup_test_storage() -> (tempfile::NamedTempFile, Storage, String) {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a test bead
        let issue = Issue::new(
            "bf-test".to_string(),
            "Test bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&issue).unwrap();

        (temp_file, storage, issue.id)
    }

    #[test]
    fn test_update_requires_exactly_one_field() {
        let (_temp, storage, id) = setup_test_storage();

        // No fields provided
        let result = update(&storage, &id, None, None, None);
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadForgeError::Validation { .. } => {}
            _ => panic!("Expected validation error"),
        }

        // Multiple fields provided
        let result = update(
            &storage,
            &id,
            Some("New title"),
            Some(Status::InProgress),
            None,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadForgeError::Validation { .. } => {}
            _ => panic!("Expected validation error"),
        }
    }

    #[test]
    fn test_update_title() {
        let (_temp, storage, id) = setup_test_storage();

        let result = update(&storage, &id, Some("Updated title"), None, None);
        assert!(result.is_ok());

        let updated = storage.get_issue(&id).unwrap().unwrap();
        assert_eq!(updated.title, "Updated title");
    }

    #[test]
    fn test_update_status() {
        let (_temp, storage, id) = setup_test_storage();

        let result = update(&storage, &id, None, Some(Status::InProgress), None);
        assert!(result.is_ok());

        let updated = storage.get_issue(&id).unwrap().unwrap();
        assert_eq!(updated.status, Status::InProgress);
    }

    #[test]
    fn test_update_priority() {
        let (_temp, storage, id) = setup_test_storage();

        let result = update(&storage, &id, None, None, Some(Priority::CRITICAL));
        assert!(result.is_ok());

        let updated = storage.get_issue(&id).unwrap().unwrap();
        assert_eq!(updated.priority, Priority::CRITICAL);
    }

    #[test]
    fn test_update_nonexistent_bead() {
        let (_temp, storage, _) = setup_test_storage();

        let result = update(
            &storage,
            "bf-nonexistent",
            Some("New title"),
            None,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_update_custom_status() {
        let (_temp, storage, id) = setup_test_storage();

        let result = update(&storage, &id, None, Some(Status::Custom("in-review".to_string())), None);
        assert!(result.is_ok());

        let updated = storage.get_issue(&id).unwrap().unwrap();
        assert_eq!(updated.status, Status::Custom("in-review".to_string()));
    }

    #[test]
    fn test_update_preserves_other_fields() {
        let (_temp, storage, _id) = setup_test_storage();

        // Create a bead with specific values
        let mut issue = Issue::new("bf-preserve".to_string(), "Original title".to_string(), ".".to_string());
        issue.status = Status::Blocked;
        issue.priority = Priority::CRITICAL;
        storage.create_issue(&issue).unwrap();

        // Update only title
        let result = update(&storage, "bf-preserve", Some("Updated title"), None, None);
        assert!(result.is_ok());

        // Verify all other fields are preserved
        let updated = storage.get_issue("bf-preserve").unwrap().unwrap();
        assert_eq!(updated.title, "Updated title");
        assert_eq!(updated.status, Status::Blocked); // Preserved
        assert_eq!(updated.priority, Priority::CRITICAL); // Preserved
    }
}
