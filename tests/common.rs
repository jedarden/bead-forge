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

#[cfg(test)]
mod tests {
    use super::*;

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
}
