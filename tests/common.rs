//! Common test infrastructure for integration tests.
//!
//! Provides TempWorkspace harness for creating isolated test workspaces
//! with automatic cleanup.

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// A temporary workspace for testing.
///
/// Creates a .beads directory with config and database, automatically
/// cleaned up when dropped.
pub struct TempWorkspace {
    /// Temp directory that holds the workspace (cleaned up on drop)
    pub temp_dir: TempDir,
    /// Path to the .beads directory
    pub beads_dir: PathBuf,
    /// Path to the database file
    pub db_path: PathBuf,
    /// Path to the JSONL file
    pub jsonl_path: PathBuf,
}

impl TempWorkspace {
    /// Create a new temporary workspace with bf configuration.
    pub fn new() -> anyhow::Result<Self> {
        let temp_dir = TempDir::new()?;
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir(&beads_dir)?;

        // Initialize workspace with bf config
        bead_forge::config::init_workspace(&beads_dir, "bf")?;

        let metadata = bead_forge::config::load_metadata(&beads_dir)?;
        let db_path = beads_dir.join(&metadata.database);
        let jsonl_path = beads_dir.join("issues.jsonl");

        // Open storage to create the database file and initialize schema
        let _storage = bead_forge::Storage::open(&db_path)?;

        Ok(Self {
            temp_dir,
            beads_dir,
            db_path,
            jsonl_path,
        })
    }

    /// Create a workspace with a pre-existing JSONL file.
    pub fn with_jsonl(jsonl_content: &str) -> anyhow::Result<Self> {
        let ws = Self::new()?;
        fs::write(&ws.jsonl_path, jsonl_content)?;
        Ok(ws)
    }

    /// Create a workspace from a fixture file.
    ///
    /// Loads a JSONL snapshot from tests/fixtures/ and copies it into
    /// the workspace. Use this for JSONL round-trip and schema compatibility tests.
    ///
    /// # Arguments
    ///
    /// * `fixture_name` - Name of the fixture file (e.g., "forge-snapshot.jsonl")
    ///
    /// # Example
    ///
    /// ```rust
    /// let ws = TempWorkspace::from_fixture("forge-snapshot.jsonl").unwrap();
    /// ws.import_jsonl().unwrap();
    /// assert_eq!(ws.count_beads().unwrap(), 8);
    /// ```
    pub fn from_fixture(fixture_name: &str) -> anyhow::Result<Self> {
        let ws = Self::new()?;
        let fixture_path = PathBuf::from("tests/fixtures").join(fixture_name);

        let fixture_content = fs::read_to_string(&fixture_path).map_err(|e| {
            anyhow::anyhow!("Failed to read fixture {}: {}", fixture_path.display(), e)
        })?;

        fs::write(&ws.jsonl_path, fixture_content)?;
        Ok(ws)
    }

    /// Open the storage backend for this workspace.
    pub fn storage(&self) -> anyhow::Result<bead_forge::Storage> {
        bead_forge::Storage::open(&self.db_path)
    }

    /// Get the workspace path (parent of .beads).
    pub fn workspace_path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Load the workspace config.
    pub fn config(&self) -> anyhow::Result<bead_forge::Config> {
        bead_forge::config::load_config(&self.beads_dir)
    }

    /// Load the workspace metadata.
    pub fn metadata(&self) -> anyhow::Result<bead_forge::Metadata> {
        bead_forge::config::load_metadata(&self.beads_dir)
    }

    /// Import issues from JSONL into the database.
    pub fn import_jsonl(&self) -> anyhow::Result<bead_forge::jsonl::ImportResult> {
        let storage = self.storage()?;
        storage.sync_from_jsonl(&self.jsonl_path)
    }

    /// Export issues from database to JSONL.
    pub fn export_jsonl(&self, dirty_only: bool) -> anyhow::Result<usize> {
        let storage = self.storage()?;
        storage.sync_to_jsonl(&self.jsonl_path, dirty_only)
    }

    /// Create a test bead with the given ID and title.
    pub fn create_bead(&self, id: &str, title: &str) -> anyhow::Result<()> {
        let storage = self.storage()?;
        let bead = bead_forge::Issue::new(id.to_string(), title.to_string(), ".".to_string());
        storage.create_issue(&bead)
    }

    /// Create a test bead from a fully-specified Issue, preserving issue_type,
    /// priority, description, labels, and all other fields.
    ///
    /// Use this instead of `create_bead` whenever the test asserts on any field
    /// beyond id/title — `create_bead` persists a default (task-type) Issue.
    pub fn create_issue(&self, issue: &bead_forge::Issue) -> anyhow::Result<()> {
        let storage = self.storage()?;
        storage.create_issue(issue)
    }

    /// Create a test bead with custom labels.
    ///
    /// This is a convenience helper for creating beads with labels in tests.
    /// It creates a basic task-type bead with the specified labels attached.
    ///
    /// # Arguments
    ///
    /// * `id` - Bead ID (e.g., "bf-test-001")
    /// * `title` - Bead title
    /// * `labels` - Array of label strings to attach to the bead
    ///
    /// # Example
    ///
    /// ```rust
    /// let ws = TempWorkspace::new().unwrap();
    /// ws.create_bead_with_labels("bf-labeled", "Test bead", &["bug", "critical"]).unwrap();
    /// let bead = ws.get_bead("bf-labeled").unwrap().unwrap();
    /// assert_eq!(bead.labels, vec!["bug".to_string(), "critical".to_string()]);
    /// ```
    pub fn create_bead_with_labels(&self, id: &str, title: &str, labels: &[&str]) -> anyhow::Result<()> {
        let storage = self.storage()?;
        let bead = bead_forge::Issue {
            id: id.to_string(),
            title: title.to_string(),
            labels: labels.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        storage.create_issue(&bead)
    }

    /// Get a bead by ID.
    pub fn get_bead(&self, id: &str) -> anyhow::Result<Option<bead_forge::Issue>> {
        let storage = self.storage()?;
        storage.get_issue(id)
    }

    /// List all beads in the workspace.
    pub fn list_beads(&self) -> anyhow::Result<Vec<bead_forge::Issue>> {
        let storage = self.storage()?;
        storage.list_all_issues()
    }

    /// Count beads in the workspace.
    pub fn count_beads(&self) -> anyhow::Result<usize> {
        let storage = self.storage()?;
        storage.count_issues()
    }
}

/// Create a sample JSONL line for a bead.
pub fn sample_bead_jsonl(id: &str, title: &str) -> String {
    format!(
        r#"{{"id":"{}","title":"{}","description":"","design":"","acceptance_criteria":"","notes":"","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":".","labels":[],"dependencies":[],"comments":[]}}"#,
        id, title
    )
}

/// Create a sample closed bead JSONL line.
pub fn sample_closed_bead_jsonl(id: &str, title: &str, close_reason: &str) -> String {
    format!(
        r#"{{"id":"{}","title":"{}","description":"","design":"","acceptance_criteria":"","notes":"","status":"closed","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T12:00:00Z","closed_at":"2024-01-01T12:00:00Z","close_reason":"{}","source_repo":".","labels":[],"dependencies":[],"comments":[]}}"#,
        id, title, close_reason
    )
}

/// Create a bead with dependencies JSONL line.
pub fn sample_bead_with_deps_jsonl(id: &str, title: &str, deps: &[&str]) -> String {
    let deps_json = serde_json::to_string(deps).unwrap();
    format!(
        r#"{{"id":"{}","title":"{}","description":"","design":"","acceptance_criteria":"","notes":"","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":".","labels":[],"dependencies":[{}]}}"#,
        id, title, deps_json
    )
}

/// Create a bead with labels JSONL line.
pub fn sample_bead_with_labels_jsonl(id: &str, title: &str, labels: &[&str]) -> String {
    let labels_json = serde_json::to_string(labels).unwrap();
    format!(
        r#"{{"id":"{}","title":"{}","description":"","design":"","acceptance_criteria":"","notes":"","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":".","labels":{},"dependencies":[],"comments":[]}}"#,
        id, title, labels_json
    )
}

/// Assert that two JSONL strings represent semantically equal issues.
///
/// This handles minor formatting differences like whitespace.
pub fn assert_jsonl_eq(jsonl1: &str, jsonl2: &str) -> anyhow::Result<()> {
    let issue1: serde_json::Value = serde_json::from_str(jsonl1)?;
    let issue2: serde_json::Value = serde_json::from_str(jsonl2)?;

    if issue1 != issue2 {
        anyhow::bail!(
            "JSONL mismatch:\n  Expected: {}\n  Got:      {}",
            jsonl1,
            jsonl2
        );
    }
    Ok(())
}

// ============================================================================
// P0 Epic Test Infrastructure
// ============================================================================

/// Create a sample P0 epic JSONL line with minimal fields.
///
/// P0 (Priority::CRITICAL = 0) is the highest priority level.
/// This is the minimal viable P0 epic for testing.
///
/// # Arguments
///
/// * `id` - Epic ID (e.g., "bf-epic-001")
/// * `title` - Epic title
///
/// # Example
///
/// ```rust
/// let jsonl = sample_p0_epic_jsonl("bf-epic-001", "Critical infrastructure migration");
/// let ws = TempWorkspace::with_jsonl(&jsonl).unwrap();
/// ```
pub fn sample_p0_epic_jsonl(id: &str, title: &str) -> String {
    format!(
        r#"{{"id":"{}","title":"{}","description":"","design":"","acceptance_criteria":"","notes":"","status":"open","priority":0,"issue_type":"epic","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":".","labels":[],"dependencies":[],"comments":[]}}"#,
        id, title
    )
}

/// Create a P0 epic JSONL line with description.
///
/// Use this for testing P0 epics with descriptive text.
///
/// # Arguments
///
/// * `id` - Epic ID
/// * `title` - Epic title
/// * `description` - Epic description text
///
/// # Example
///
/// ```rust
/// let jsonl = sample_p0_epic_with_description_jsonl(
///     "bf-epic-002",
///     "Security overhaul",
///     "Complete security audit and fixes"
/// );
/// ```
pub fn sample_p0_epic_with_description_jsonl(id: &str, title: &str, description: &str) -> String {
    format!(
        r#"{{"id":"{}","title":"{}","description":"{}","design":"","acceptance_criteria":"","notes":"","status":"open","priority":0,"issue_type":"epic","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":".","labels":[],"dependencies":[],"comments":[]}}"#,
        id, title, description
    )
}

/// Create a P0 epic JSONL line with labels.
///
/// Use this for testing P0 epics with categorization labels.
///
/// # Arguments
///
/// * `id` - Epic ID
/// * `title` - Epic title
/// * `labels` - Array of label strings
///
/// # Example
///
/// ```rust
/// let jsonl = sample_p0_epic_with_labels_jsonl(
///     "bf-epic-003",
///     "Database migration",
///     &["database", "migration", "critical"]
/// );
/// ```
pub fn sample_p0_epic_with_labels_jsonl(id: &str, title: &str, labels: &[&str]) -> String {
    let labels_json = serde_json::to_string(labels).unwrap();
    format!(
        r#"{{"id":"{}","title":"{}","description":"","design":"","acceptance_criteria":"","notes":"","status":"open","priority":0,"issue_type":"epic","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":".","labels":{},"dependencies":[],"comments":[]}}"#,
        id, title, labels_json
    )
}

/// Create a comprehensive P0 epic JSONL line with all metadata.
///
/// Use this for full round-trip testing of P0 epic serialization.
///
/// # Arguments
///
/// * `id` - Epic ID
/// * `title` - Epic title
/// * `description` - Epic description
/// * `assignee` - Optional assignee
/// * `labels` - Array of label strings
///
/// # Example
///
/// ```rust
/// let jsonl = sample_p0_epic_full_jsonl(
///     "bf-epic-004",
///     "API redesign",
///     "Complete REST API overhaul",
///     Some("architect-team"),
///     &["api", "backend", "p0"]
/// );
/// ```
pub fn sample_p0_epic_full_jsonl(
    id: &str,
    title: &str,
    description: &str,
    assignee: Option<&str>,
    labels: &[&str],
) -> String {
    let assignee_json = assignee
        .map(|a| serde_json::to_string(a).unwrap())
        .unwrap_or("null".to_string());
    let labels_json = serde_json::to_string(labels).unwrap();
    format!(
        r#"{{"id":"{}","title":"{}","description":"{}","design":"","acceptance_criteria":"","notes":"","status":"open","priority":0,"issue_type":"epic","assignee":{},"created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":".","labels":{},"dependencies":[],"comments":[]}}"#,
        id, title, description, assignee_json, labels_json
    )
}

/// Assert that an issue is a P0 epic.
///
/// Verifies:
/// - issue_type is Epic
/// - priority is CRITICAL (0)
/// - title is non-empty
///
/// # Arguments
///
/// * `issue` - The issue to verify
/// * `context` - Optional context string for error messages
///
/// # Example
///
/// ```rust
/// let epic = storage.get_issue("bf-epic-001").unwrap().unwrap();
/// assert_p0_epic(&epic, Some("Retrieved epic"));
/// ```
pub fn assert_p0_epic(issue: &bead_forge::Issue, context: Option<&str>) {
    let ctx = context.unwrap_or("Issue");

    assert_eq!(
        issue.issue_type,
        bead_forge::IssueType::Epic,
        "{}: must be epic type, got {:?}",
        ctx,
        issue.issue_type
    );

    assert_eq!(
        issue.priority,
        bead_forge::Priority::CRITICAL,
        "{}: must be P0 (CRITICAL), got P{}",
        ctx,
        issue.priority.0
    );

    assert_eq!(
        issue.priority.0, 0,
        "{}: priority value must be 0, got {}",
        ctx, issue.priority.0
    );

    assert!(!issue.title.is_empty(), "{}: title must not be empty", ctx);
}

/// Assert P0 epic display formatting.
///
/// Verifies that the epic's priority displays as "P0".
///
/// # Arguments
///
/// * `issue` - The issue to verify
///
/// # Example
///
/// ```rust
/// let epic = storage.get_issue("bf-epic-001").unwrap().unwrap();
/// assert_p0_epic_display(&epic);
/// ```
pub fn assert_p0_epic_display(issue: &bead_forge::Issue) {
    let display = format!("{}", issue.priority);
    assert_eq!(
        display, "P0",
        "Priority must display as 'P0', got '{}'",
        display
    );
}

/// Assert P0 epic JSON serialization.
///
/// Verifies that a P0 epic serializes to JSON with correct field values.
///
/// # Arguments
///
/// * `issue` - The issue to serialize and verify
///
/// # Example
///
/// ```rust
/// let epic = create_test_p0_epic();
/// assert_p0_epic_json_serialization(&epic);
/// ```
pub fn assert_p0_epic_json_serialization(issue: &bead_forge::Issue) {
    let json = serde_json::to_string(issue).unwrap();

    // Verify epic type
    assert!(
        json.contains(r#""issue_type":"epic""#),
        "JSON must contain epic type, got: {}",
        json
    );

    // Verify P0 priority
    assert!(
        json.contains(r#""priority":0"#),
        "JSON must contain priority 0, got: {}",
        json
    );

    // Verify no "P0" string (priority serializes as integer)
    assert!(
        !json.contains(r#""priority":"P0""#),
        "JSON must not contain string 'P0' for priority, got: {}",
        json
    );
}

/// Seed a workspace with P0 epic test data.
///
/// Creates multiple P0 epics for testing list operations and filtering.
///
/// # Arguments
///
/// * `workspace` - The TempWorkspace to seed
/// * `count` - Number of P0 epics to create
///
/// # Returns
///
/// Vector of created epic IDs
///
/// # Example
///
/// ```rust
/// let ws = TempWorkspace::new().unwrap();
/// let epic_ids = seed_p0_epics(&ws, 5);
/// assert_eq!(epic_ids.len(), 5);
/// ```
pub fn seed_p0_epics(workspace: &TempWorkspace, count: usize) -> anyhow::Result<Vec<String>> {
    let mut epic_ids = Vec::new();

    for i in 0..count {
        let id = format!("bf-p0-epic-{:03}", i);
        let title = format!("P0 Test Epic #{}", i);

        let epic = bead_forge::Issue {
            id: id.clone(),
            title,
            issue_type: bead_forge::IssueType::Epic,
            priority: bead_forge::Priority::CRITICAL,
            description: Some(format!("Test epic number {} with P0 priority", i)),
            ..Default::default()
        };

        workspace.create_issue(&epic)?;
        epic_ids.push(id);
    }

    Ok(epic_ids)
}

/// Count P0 epics in a workspace.
///
/// Queries the workspace and returns the count of issues that are:
/// - issue_type: Epic
/// - priority: CRITICAL (0)
///
/// # Arguments
///
/// * `workspace` - The TempWorkspace to query
///
/// # Returns
///
/// Number of P0 epics found
///
/// # Example
///
/// ```rust
/// let ws = TempWorkspace::new().unwrap();
/// seed_p0_epics(&ws, 3).unwrap();
/// assert_eq!(count_p0_epics(&ws).unwrap(), 3);
/// ```
pub fn count_p0_epics(workspace: &TempWorkspace) -> anyhow::Result<usize> {
    let all_issues = workspace.list_beads()?;
    let p0_epics = all_issues
        .iter()
        .filter(|i| {
            i.issue_type == bead_forge::IssueType::Epic
                && i.priority == bead_forge::Priority::CRITICAL
        })
        .count();

    Ok(p0_epics)
}

/// P0 Epic fixture builder.
///
/// Builder pattern for creating P0 epic test fixtures with varying configurations.
///
/// # Example
///
/// ```rust
/// let epic = P0EpicBuilder::new("bf-epic-001", "Test epic")
///     .with_description("Critical infrastructure update")
///     .with_labels(&["backend", "database"])
///     .with_assignee("team-lead")
///     .build();
/// ```
pub struct P0EpicBuilder {
    id: String,
    title: String,
    description: Option<String>,
    assignee: Option<String>,
    labels: Vec<String>,
    status: bead_forge::Status,
}

impl P0EpicBuilder {
    /// Create a new P0 epic builder.
    ///
    /// # Arguments
    ///
    /// * `id` - Epic ID
    /// * `title` - Epic title
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            assignee: None,
            labels: Vec::new(),
            status: bead_forge::Status::Open,
        }
    }

    /// Add a description to the epic.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add an assignee to the epic.
    pub fn with_assignee(mut self, assignee: impl Into<String>) -> Self {
        self.assignee = Some(assignee.into());
        self
    }

    /// Add labels to the epic.
    pub fn with_labels(mut self, labels: &[&str]) -> Self {
        self.labels = labels.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set the epic status.
    pub fn with_status(mut self, status: bead_forge::Status) -> Self {
        self.status = status;
        self
    }

    /// Build the P0 epic Issue.
    pub fn build(self) -> bead_forge::Issue {
        bead_forge::Issue {
            id: self.id,
            title: self.title,
            description: self.description,
            assignee: self.assignee,
            labels: self.labels,
            issue_type: bead_forge::IssueType::Epic,
            priority: bead_forge::Priority::CRITICAL,
            status: self.status,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bead_forge::{IssueType, Priority, Status};

    #[test]
    fn test_temp_workspace_creation() {
        let ws = TempWorkspace::new().unwrap();
        assert!(ws.beads_dir.exists());
        assert!(ws.db_path.exists());
        // JSONL file is created on export, not on initialization
    }

    #[test]
    fn test_temp_workspace_create_bead() {
        let ws = TempWorkspace::new().unwrap();
        ws.create_bead("bf-test", "Test bead").unwrap();

        let bead = ws.get_bead("bf-test").unwrap().unwrap();
        assert_eq!(bead.id, "bf-test");
        assert_eq!(bead.title, "Test bead");
        assert_eq!(bead.status.to_string(), "open");
    }

    #[test]
    fn test_temp_workspace_with_jsonl() {
        let jsonl = sample_bead_jsonl("bf-test", "Test bead");
        let ws = TempWorkspace::with_jsonl(&jsonl).unwrap();

        let result = ws.import_jsonl().unwrap();
        assert_eq!(result.imported, 1);

        let bead = ws.get_bead("bf-test").unwrap().unwrap();
        assert_eq!(bead.id, "bf-test");
        assert_eq!(bead.title, "Test bead");
    }

    // P0 Epic Infrastructure Tests

    #[test]
    fn test_sample_p0_epic_jsonl() {
        let jsonl = sample_p0_epic_jsonl("bf-epic-001", "Test Epic");

        let issue: bead_forge::Issue = serde_json::from_str(&jsonl).unwrap();

        assert_eq!(issue.id, "bf-epic-001");
        assert_eq!(issue.title, "Test Epic");
        assert_eq!(issue.issue_type, IssueType::Epic);
        assert_eq!(issue.priority, Priority::CRITICAL);
        assert_eq!(issue.priority.0, 0);
    }

    #[test]
    fn test_sample_p0_epic_with_description_jsonl() {
        let jsonl = sample_p0_epic_with_description_jsonl(
            "bf-epic-002",
            "Security Epic",
            "Complete security audit",
        );

        let issue: bead_forge::Issue = serde_json::from_str(&jsonl).unwrap();

        assert_eq!(issue.id, "bf-epic-002");
        assert_eq!(issue.title, "Security Epic");
        assert_eq!(
            issue.description,
            Some("Complete security audit".to_string())
        );
        assert_eq!(issue.issue_type, IssueType::Epic);
        assert_eq!(issue.priority, Priority::CRITICAL);
    }

    #[test]
    fn test_sample_p0_epic_with_labels_jsonl() {
        let jsonl = sample_p0_epic_with_labels_jsonl(
            "bf-epic-003",
            "Database Epic",
            &["database", "migration", "critical"],
        );

        let issue: bead_forge::Issue = serde_json::from_str(&jsonl).unwrap();

        assert_eq!(issue.id, "bf-epic-003");
        assert_eq!(issue.labels, vec!["database", "migration", "critical"]);
        assert_eq!(issue.issue_type, IssueType::Epic);
        assert_eq!(issue.priority, Priority::CRITICAL);
    }

    #[test]
    fn test_sample_p0_epic_full_jsonl() {
        let jsonl = sample_p0_epic_full_jsonl(
            "bf-epic-004",
            "API Redesign",
            "Complete REST API overhaul",
            Some("architect-team"),
            &["api", "backend", "p0"],
        );

        let issue: bead_forge::Issue = serde_json::from_str(&jsonl).unwrap();

        assert_eq!(issue.id, "bf-epic-004");
        assert_eq!(issue.title, "API Redesign");
        assert_eq!(
            issue.description,
            Some("Complete REST API overhaul".to_string())
        );
        assert_eq!(issue.assignee, Some("architect-team".to_string()));
        assert_eq!(issue.labels, vec!["api", "backend", "p0"]);
        assert_eq!(issue.issue_type, IssueType::Epic);
        assert_eq!(issue.priority, Priority::CRITICAL);
    }

    #[test]
    fn test_assert_p0_epic() {
        let epic = bead_forge::Issue {
            id: "bf-epic-test".to_string(),
            title: "Test Epic".to_string(),
            issue_type: IssueType::Epic,
            priority: Priority::CRITICAL,
            ..Default::default()
        };

        assert_p0_epic(&epic, Some("Test epic"));

        // Test non-epic fails
        let task = bead_forge::Issue {
            id: "bf-task".to_string(),
            title: "Task".to_string(),
            issue_type: IssueType::Task,
            priority: Priority::CRITICAL,
            ..Default::default()
        };

        let result = std::panic::catch_unwind(|| {
            assert_p0_epic(&task, Some("Should fail"));
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_assert_p0_epic_display() {
        let epic = bead_forge::Issue {
            id: "bf-epic-display".to_string(),
            title: "Display Test".to_string(),
            issue_type: IssueType::Epic,
            priority: Priority::CRITICAL,
            ..Default::default()
        };

        assert_p0_epic_display(&epic);
    }

    #[test]
    fn test_assert_p0_epic_json_serialization() {
        let epic = bead_forge::Issue {
            id: "bf-epic-json".to_string(),
            title: "JSON Test".to_string(),
            issue_type: IssueType::Epic,
            priority: Priority::CRITICAL,
            ..Default::default()
        };

        assert_p0_epic_json_serialization(&epic);
    }

    #[test]
    fn test_p0_epic_builder_minimal() {
        let epic = P0EpicBuilder::new("bf-epic-builder-001", "Builder Test Epic").build();

        assert_eq!(epic.id, "bf-epic-builder-001");
        assert_eq!(epic.title, "Builder Test Epic");
        assert_eq!(epic.issue_type, IssueType::Epic);
        assert_eq!(epic.priority, Priority::CRITICAL);
        assert_eq!(epic.description, None);
        assert!(epic.labels.is_empty());
    }

    #[test]
    fn test_p0_epic_builder_full() {
        let epic = P0EpicBuilder::new("bf-epic-builder-002", "Full Builder Epic")
            .with_description("Complete infrastructure migration")
            .with_assignee("platform-team")
            .with_labels(&["backend", "database", "p0"])
            .with_status(Status::InProgress)
            .build();

        assert_eq!(epic.id, "bf-epic-builder-002");
        assert_eq!(epic.title, "Full Builder Epic");
        assert_eq!(
            epic.description,
            Some("Complete infrastructure migration".to_string())
        );
        assert_eq!(epic.assignee, Some("platform-team".to_string()));
        assert_eq!(epic.labels, vec!["backend", "database", "p0"]);
        assert_eq!(epic.status, Status::InProgress);
        assert_eq!(epic.issue_type, IssueType::Epic);
        assert_eq!(epic.priority, Priority::CRITICAL);
    }

    #[test]
    fn test_seed_p0_epics() {
        let ws = TempWorkspace::new().unwrap();
        let epic_ids = seed_p0_epics(&ws, 5).unwrap();

        assert_eq!(epic_ids.len(), 5);

        // Verify all were created
        for id in &epic_ids {
            let epic = ws.get_bead(id).unwrap().unwrap();
            assert_eq!(epic.issue_type, IssueType::Epic);
            assert_eq!(epic.priority, Priority::CRITICAL);
        }
    }

    #[test]
    fn test_count_p0_epics() {
        let ws = TempWorkspace::new().unwrap();

        // Initially zero
        assert_eq!(count_p0_epics(&ws).unwrap(), 0);

        // Add some P0 epics
        seed_p0_epics(&ws, 3).unwrap();
        assert_eq!(count_p0_epics(&ws).unwrap(), 3);

        // Add a non-epic (should not count)
        ws.create_bead("bf-task-001", "Regular task").unwrap();
        assert_eq!(count_p0_epics(&ws).unwrap(), 3);

        // Add a P1 epic (should not count)
        let p1_epic = bead_forge::Issue {
            id: "bf-epic-p1".to_string(),
            title: "P1 Epic".to_string(),
            issue_type: IssueType::Epic,
            priority: Priority::HIGH,
            ..Default::default()
        };
        ws.create_bead(&p1_epic.id, &p1_epic.title).unwrap();
        assert_eq!(count_p0_epics(&ws).unwrap(), 3);
    }

    #[test]
    fn test_p0_epic_roundtrip_through_workspace() {
        let ws = TempWorkspace::new().unwrap();

        // Create P0 epic
        let epic = P0EpicBuilder::new("bf-epic-roundtrip", "Roundtrip Test")
            .with_description("Testing P0 epic roundtrip")
            .with_labels(&["test", "p0"])
            .build();

        ws.create_issue(&epic).unwrap();

        // Retrieve and verify
        let retrieved = ws.get_bead("bf-epic-roundtrip").unwrap().unwrap();
        assert_p0_epic(&retrieved, Some("Retrieved epic"));
        assert_p0_epic_display(&retrieved);
        assert_p0_epic_json_serialization(&retrieved);
    }
}
