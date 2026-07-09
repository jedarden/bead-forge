//! Test infrastructure for bf update --description functionality.
//!
//! This module provides:
//! - Test database setup/teardown utilities
//! - Helper functions to create test beads
//! - Helper functions to read bead descriptions directly from SQLite
//! - Test scaffolding for automated verification

use bead_forge::model::{Issue, IssueChanges};
use bead_forge::storage::Storage;
use chrono::Utc;
use rusqlite::Connection;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test database context that manages setup and teardown.
pub struct TestDatabase {
    /// Temporary directory containing the database
    pub temp_dir: TempDir,
    /// Path to the database file
    pub db_path: PathBuf,
    /// Storage interface
    pub storage: Storage,
}

impl TestDatabase {
    /// Create a new test database with all tables initialized.
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::open(&db_path).expect("Failed to open storage");

        Self {
            temp_dir,
            db_path,
            storage,
        }
    }

    /// Get a direct SQLite connection to the database (for low-level queries).
    pub fn connection(&self) -> Connection {
        Connection::open(&self.db_path).expect("Failed to open direct connection")
    }
}

/// Create a test bead with optional custom description.
pub fn create_test_bead_with_description(
    storage: &Storage,
    title: &str,
    description: Option<&str>,
) -> Issue {
    let mut issue = Issue::new(
        format!("test-{}", title.replace(' ', "-").to_lowercase()),
        title.to_string(),
        ".".to_string(),
    );
    issue.description = description.map(|d| d.to_string());
    issue.created_at = Utc::now();
    storage.create_issue(&issue).expect("Failed to create test bead");
    issue
}

/// Create a test bead with a default description.
pub fn create_test_bead(storage: &Storage, title: &str) -> Issue {
    create_test_bead_with_description(storage, title, Some("Initial description"))
}

/// Read a bead's description directly from SQLite (bypasses storage layer for verification).
///
/// This is useful for testing that the database was actually updated,
/// independent of the storage layer's caching or logic.
pub fn read_description_from_db(db_path: &PathBuf, bead_id: &str) -> Option<String> {
    let conn = Connection::open(db_path).expect("Failed to open connection");
    let mut stmt = conn
        .prepare("SELECT description FROM issues WHERE id = ?1")
        .expect("Failed to prepare statement");

    let result: Result<Option<String>, rusqlite::Error> = stmt.query_row([bead_id], |row| {
        let desc: String = row.get(0)?;
        Ok(if desc.is_empty() { None } else { Some(desc) })
    });

    match result {
        Ok(desc) => desc,
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => panic!("Failed to read description: {}", e),
    }
}

/// Read all fields of a bead directly from SQLite as a tuple.
///
/// Returns: (id, title, description, design, acceptance_criteria, notes)
pub fn read_bead_fields_from_db(db_path: &PathBuf, bead_id: &str) -> Option<(String, String, String, String, String, String)> {
    let conn = Connection::open(db_path).expect("Failed to open connection");
    let mut stmt = conn
        .prepare("SELECT id, title, description, design, acceptance_criteria, notes FROM issues WHERE id = ?1")
        .expect("Failed to prepare statement");

    let result: Result<(String, String, String, String, String, String), rusqlite::Error> =
        stmt.query_row([bead_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        });

    match result {
        Ok(fields) => Some(fields),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => panic!("Failed to read bead fields: {}", e),
    }
}

/// Update bead description via storage layer.
pub fn update_bead_description(storage: &Storage, bead_id: &str, new_description: &str) {
    let changes = IssueChanges {
        description: Some(new_description.to_string()),
        ..Default::default()
    };
    storage
        .update_issue(bead_id, &changes)
        .expect("Failed to update description");
}

/// Clear bead description (set to empty string/None).
pub fn clear_bead_description(storage: &Storage, bead_id: &str) {
    let changes = IssueChanges {
        description: Some(String::new()),
        ..Default::default()
    };
    storage
        .update_issue(bead_id, &changes)
        .expect("Failed to clear description");
}

/// Test helper to verify description update end-to-end.
///
/// This function:
/// 1. Creates a test bead with initial description
/// 2. Updates the description via storage layer
/// 3. Verifies the change was persisted to SQLite directly
/// 4. Returns the bead ID and updated bead for further testing
pub fn test_description_update_cycle(
    test_db: &TestDatabase,
    title: &str,
    initial_desc: &str,
    updated_desc: &str,
) -> (String, Issue) {
    // Create bead
    let bead = create_test_bead_with_description(&test_db.storage, title, Some(initial_desc));
    let bead_id = bead.id.clone();

    // Verify initial state via direct DB read
    let db_desc = read_description_from_db(&test_db.db_path, &bead_id);
    assert_eq!(
        db_desc, Some(initial_desc.to_string()),
        "Initial description not persisted to DB"
    );

    // Update description
    update_bead_description(&test_db.storage, &bead_id, updated_desc);

    // Verify update via direct DB read
    let db_desc_after = read_description_from_db(&test_db.db_path, &bead_id);
    assert_eq!(
        db_desc_after, Some(updated_desc.to_string()),
        "Updated description not persisted to DB"
    );

    // Verify via storage layer too
    let updated_bead = test_db
        .storage
        .get_issue(&bead_id)
        .expect("Failed to get updated bead")
        .expect("Bead not found");
    assert_eq!(
        updated_bead.description,
        Some(updated_desc.to_string()),
        "Storage layer returns incorrect description"
    );

    (bead_id, updated_bead)
}

/// Test helper to verify that updating description preserves other fields.
///
/// This function:
/// 1. Creates a bead with all fields set
/// 2. Updates only the description
/// 3. Verifies that all other fields remain unchanged
pub fn test_description_update_preserves_fields(
    test_db: &TestDatabase,
    title: &str,
) -> (String, Issue) {
    use bead_forge::model::{Priority, Status};

    // Create bead with all fields set
    let mut bead = Issue::new(
        format!("test-{}", title.replace(' ', "-").to_lowercase()),
        title.to_string(),
        ".".to_string(),
    );
    bead.description = Some("Original description".to_string());
    bead.design = Some("Original design".to_string());
    bead.acceptance_criteria = Some("Original AC".to_string());
    bead.notes = Some("Original notes".to_string());
    bead.status = Status::Open;
    bead.priority = Priority(2);
    bead.issue_type = bead_forge::model::IssueType::Task;

    test_db.storage.create_issue(&bead).expect("Failed to create bead");
    let bead_id = bead.id.clone();

    // Update only description
    update_bead_description(&test_db.storage, &bead_id, "Updated description only");

    // Read all fields from DB
    let fields = read_bead_fields_from_db(&test_db.db_path, &bead_id).expect("Bead not found");

    // Verify description changed
    assert_eq!(
        fields.2, "Updated description only",
        "Description not updated correctly"
    );

    // Verify other fields preserved
    assert_eq!(
        fields.3, "Original design",
        "Design field was modified"
    );
    assert_eq!(
        fields.4, "Original AC",
        "Acceptance criteria was modified"
    );
    assert_eq!(fields.5, "Original notes", "Notes field was modified");

    // Get via storage layer for return value
    let updated_bead = test_db
        .storage
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .expect("Bead not found");

    (bead_id, updated_bead)
}

// ============================================================================
// Unit Tests
// ============================================================================

#[test]
fn test_infrastructure_create_test_bead() {
    let test_db = TestDatabase::new();
    let bead = create_test_bead(&test_db.storage, "infrastructure test");

    // Verify bead was created
    let retrieved = test_db
        .storage
        .get_issue(&bead.id)
        .expect("Failed to get bead")
        .expect("Bead not found");
    assert_eq!(retrieved.title, "infrastructure test");
    assert_eq!(retrieved.description, Some("Initial description".to_string()));
}

#[test]
fn test_infrastructure_read_description_directly() {
    let test_db = TestDatabase::new();
    let bead = create_test_bead_with_description(
        &test_db.storage,
        "direct read test",
        Some("Custom description"),
    );

    let desc = read_description_from_db(&test_db.db_path, &bead.id);
    assert_eq!(desc, Some("Custom description".to_string()));
}

#[test]
fn test_infrastructure_update_via_storage_verify_via_db() {
    let test_db = TestDatabase::new();
    let bead = create_test_bead(&test_db.storage, "update verify test");

    // Update via storage
    update_bead_description(&test_db.storage, &bead.id, "New description");

    // Verify via direct DB read
    let desc = read_description_from_db(&test_db.db_path, &bead.id);
    assert_eq!(desc, Some("New description".to_string()));
}

#[test]
fn test_infrastructure_description_update_cycle() {
    let test_db = TestDatabase::new();
    let (_bead_id, _bead) = test_description_update_cycle(
        &test_db,
        "cycle test",
        "Initial",
        "Updated",
    );
}

#[test]
fn test_infrastructure_preserves_other_fields() {
    let test_db = TestDatabase::new();
    let (_bead_id, _bead) = test_description_update_preserves_fields(&test_db, "preserve test");
}

#[test]
fn test_infrastructure_clear_description() {
    let test_db = TestDatabase::new();
    let bead = create_test_bead(&test_db.storage, "clear test");

    // Clear description
    clear_bead_description(&test_db.storage, &bead.id);

    // Verify it's cleared (empty string in DB)
    let desc = read_description_from_db(&test_db.db_path, &bead.id);
    // Empty string reads as None via our helper
    assert_eq!(desc, None, "Description was not cleared");
}

#[test]
fn test_infrastructure_multiline_description() {
    let test_db = TestDatabase::new();
    let bead = create_test_bead(&test_db.storage, "multiline test");

    let multiline = "Line 1\nLine 2\nLine 3";
    update_bead_description(&test_db.storage, &bead.id, multiline);

    let desc = read_description_from_db(&test_db.db_path, &bead.id);
    assert_eq!(desc, Some(multiline.to_string()));
}

#[test]
fn test_infrastructure_unicode_description() {
    let test_db = TestDatabase::new();
    let bead = create_test_bead(&test_db.storage, "unicode test");

    let unicode = "Description with émojis 🎉 and spëcial çharacters";
    update_bead_description(&test_db.storage, &bead.id, unicode);

    let desc = read_description_from_db(&test_db.db_path, &bead.id);
    assert_eq!(desc, Some(unicode.to_string()));
}

#[test]
fn test_infrastructure_read_all_fields() {
    let test_db = TestDatabase::new();
    let bead = create_test_bead_with_description(
        &test_db.storage,
        "fields test",
        Some("Test description"),
    );

    let fields = read_bead_fields_from_db(&test_db.db_path, &bead.id).expect("Bead not found");

    assert_eq!(fields.0, bead.id);
    assert_eq!(fields.1, "fields test");
    assert_eq!(fields.2, "Test description");
    assert_eq!(fields.3, ""); // design (empty default)
    assert_eq!(fields.4, ""); // acceptance_criteria (empty default)
    assert_eq!(fields.5, ""); // notes (empty default)
}
