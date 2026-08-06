//! Test fixtures and helper functions for label operations.
//!
//! Provides:
//! - `LabelTestWorkspace` for isolated label testing environments
//! - Helper functions for creating test beads with labels
//! - Label-specific assertion helpers
//! - Builder patterns for label test data
//! - Environment cleanup/teardown (automatic via TempDir)
//!
//! # Example
//!
//! ```rust
//! let ws = LabelTestWorkspace::new().unwrap();
//! let bead_id = ws.create_bead_with_labels("bf-test", "Test", &["label1", "label2"]).unwrap();
//! let labels = ws.get_labels(&bead_id).unwrap();
//! assert_eq!(labels.len(), 2);
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Isolated test workspace for label operations.
///
/// Automatically cleaned up when dropped (TempDir).
pub struct LabelTestWorkspace {
    /// Temp directory holding the workspace (auto-cleaned on drop)
    pub temp_dir: TempDir,
    /// Path to the .beads directory
    pub beads_dir: PathBuf,
    /// Path to the database file
    pub db_path: PathBuf,
    /// Path to the JSONL file
    pub jsonl_path: PathBuf,
}

impl LabelTestWorkspace {
    /// Create a new isolated test workspace for label operations.
    ///
    /// Initializes:
    /// - Temporary directory
    /// - .beads/ subdirectory with config
    /// - SQLite database with full schema
    /// - Empty JSONL file
    ///
    /// # Returns
    ///
    /// Result containing the workspace or an error.
    ///
    /// # Example
    ///
    /// ```rust
    /// let ws = LabelTestWorkspace::new().unwrap();
    /// assert!(ws.db_path.exists());
    /// assert!(ws.beads_dir.exists());
    /// ```
    pub fn new() -> anyhow::Result<Self> {
        let temp_dir = TempDir::new()?;
        let workspace_dir = temp_dir.path().join("label-test-workspace");
        fs::create_dir_all(&workspace_dir)?;

        let beads_dir = workspace_dir.join(".beads");
        fs::create_dir_all(&beads_dir)?;

        // Initialize bf config
        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            r#"issue_prefixes: [bf]
default_priority: 2
default_type: task
claim_ttl_minutes: 30
"#,
        )?;

        // Initialize metadata
        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )?;

        let db_path = beads_dir.join("beads.db");
        let jsonl_path = beads_dir.join("issues.jsonl");

        // Initialize database with schema
        let _storage = bead_forge::storage::Storage::open(&db_path)?;

        Ok(Self {
            temp_dir,
            beads_dir,
            db_path,
            jsonl_path,
        })
    }

    /// Open the storage backend for this workspace.
    ///
    /// # Returns
    ///
    /// Result containing the storage instance or an error.
    pub fn storage(&self) -> anyhow::Result<bead_forge::storage::Storage> {
        bead_forge::storage::Storage::open(&self.db_path)
    }

    /// Create a test bead with the given ID and title.
    ///
    /// Creates a basic task-type bead with default priority and no labels.
    ///
    /// # Arguments
    ///
    /// * `id` - Bead ID (e.g., "bf-test-001")
    /// * `title` - Bead title
    ///
    /// # Returns
    ///
    /// Result indicating success or error.
    pub fn create_bead(&self, id: &str, title: &str) -> anyhow::Result<()> {
        let storage = self.storage()?;
        let bead = bead_forge::Issue::new(id.to_string(), title.to_string(), ".".to_string());
        storage.create_issue(&bead)
    }

    /// Create a test bead with custom labels.
    ///
    /// Creates a task-type bead with the specified labels attached.
    ///
    /// # Arguments
    ///
    /// * `id` - Bead ID
    /// * `title` - Bead title
    /// * `labels` - Array of label strings to attach
    ///
    /// # Returns
    ///
    /// Result containing the created bead ID or an error.
    ///
    /// # Example
    ///
    /// ```rust
    /// let ws = LabelTestWorkspace::new().unwrap();
    /// ws.create_bead_with_labels("bf-test", "Test bead", &["bug", "critical"]).unwrap();
    /// let labels = ws.get_labels("bf-test").unwrap();
    /// assert_eq!(labels.len(), 2);
    /// ```
    pub fn create_bead_with_labels(
        &self,
        id: &str,
        title: &str,
        labels: &[&str],
    ) -> anyhow::Result<()> {
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
    ///
    /// # Arguments
    ///
    /// * `id` - Bead ID to retrieve
    ///
    /// # Returns
    ///
    /// Result containing Option with the bead if found, or None.
    pub fn get_bead(&self, id: &str) -> anyhow::Result<Option<bead_forge::Issue>> {
        let storage = self.storage()?;
        storage.get_issue(id)
    }

    /// Add a single label to a bead.
    ///
    /// # Arguments
    ///
    /// * `issue_id` - Bead ID
    /// * `label` - Label string to add
    ///
    /// # Returns
    ///
    /// Result indicating success or error.
    pub fn add_label(&self, issue_id: &str, label: &str) -> anyhow::Result<()> {
        let storage = self.storage()?;
        storage.add_label(issue_id, label)
    }

    /// Add multiple labels to a bead.
    ///
    /// # Arguments
    ///
    /// * `issue_id` - Bead ID
    /// * `labels` - Array of label strings to add
    ///
    /// # Returns
    ///
    /// Result indicating success or error.
    ///
    /// # Example
    ///
    /// ```rust
    /// let ws = LabelTestWorkspace::new().unwrap();
    /// ws.create_bead("bf-test", "Test").unwrap();
    /// ws.add_labels("bf-test", &["label1", "label2"]).unwrap();
    /// ```
    pub fn add_labels(&self, issue_id: &str, labels: &[&str]) -> anyhow::Result<()> {
        let storage = self.storage()?;
        for label in labels {
            storage.add_label(issue_id, label)?;
        }
        Ok(())
    }

    /// Remove a label from a bead.
    ///
    /// # Arguments
    ///
    /// * `issue_id` - Bead ID
    /// * `label` - Label string to remove
    ///
    /// # Returns
    ///
    /// Result indicating success or error.
    pub fn remove_label(&self, issue_id: &str, label: &str) -> anyhow::Result<()> {
        let storage = self.storage()?;
        storage.remove_label(issue_id, label)
    }

    /// Get all labels for a bead.
    ///
    /// # Arguments
    ///
    /// * `issue_id` - Bead ID
    ///
    /// # Returns
    ///
    /// Result containing vector of label strings.
    ///
    /// # Example
    ///
    /// ```rust
    /// let ws = LabelTestWorkspace::new().unwrap();
    /// ws.create_bead_with_labels("bf-test", "Test", &["a", "b"]).unwrap();
    /// let labels = ws.get_labels("bf-test").unwrap();
    /// assert_eq!(labels, vec!["a".to_string(), "b".to_string()]);
    /// ```
    pub fn get_labels(&self, issue_id: &str) -> anyhow::Result<Vec<String>> {
        let storage = self.storage()?;
        storage.get_labels(issue_id)
    }

    /// List all labels across all beads in the workspace.
    ///
    /// Returns a vector of (label, count) tuples sorted by label name.
    ///
    /// # Returns
    ///
    /// Result containing vector of (label, count) tuples.
    pub fn list_all_labels(&self) -> anyhow::Result<Vec<(String, i64)>> {
        let storage = self.storage()?;
        storage.list_all_labels()
    }

    /// Count labels for a specific bead.
    ///
    /// # Arguments
    ///
    /// * `issue_id` - Bead ID
    ///
    /// # Returns
    ///
    /// Result containing the number of labels.
    pub fn count_labels(&self, issue_id: &str) -> anyhow::Result<usize> {
        let labels = self.get_labels(issue_id)?;
        Ok(labels.len())
    }

    /// Check if a bead has a specific label.
    ///
    /// # Arguments
    ///
    /// * `issue_id` - Bead ID
    /// * `label` - Label string to check
    ///
    /// # Returns
    ///
    /// Result containing true if the label exists, false otherwise.
    pub fn has_label(&self, issue_id: &str, label: &str) -> anyhow::Result<bool> {
        let labels = self.get_labels(issue_id)?;
        Ok(labels.iter().any(|l| l == label))
    }

    /// Clear all labels from a bead.
    ///
    /// Removes all labels from the specified bead.
    ///
    /// # Arguments
    ///
    /// * `issue_id` - Bead ID
    ///
    /// # Returns
    ///
    /// Result indicating success or error.
    pub fn clear_labels(&self, issue_id: &str) -> anyhow::Result<()> {
        let storage = self.storage()?;
        let labels = storage.get_labels(issue_id)?;
        for label in labels {
            storage.remove_label(issue_id, &label)?;
        }
        Ok(())
    }

    /// Create a P0 epic with labels for testing.
    ///
    /// Creates a priority-critical epic with the specified labels.
    ///
    /// # Arguments
    ///
    /// * `id` - Epic ID
    /// * `title` - Epic title
    /// * `labels` - Array of label strings
    ///
    /// # Returns
    ///
    /// Result indicating success or error.
    pub fn create_p0_epic_with_labels(
        &self,
        id: &str,
        title: &str,
        labels: &[&str],
    ) -> anyhow::Result<()> {
        let storage = self.storage()?;
        let epic = bead_forge::Issue {
            id: id.to_string(),
            title: title.to_string(),
            labels: labels.iter().map(|s| s.to_string()).collect(),
            issue_type: bead_forge::IssueType::Epic,
            priority: bead_forge::Priority::CRITICAL,
            ..Default::default()
        };
        storage.create_issue(&epic)
    }

    /// Seed workspace with multiple beads having different labels.
    ///
    /// Creates a set of test beads with various label combinations.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of beads to create
    ///
    /// # Returns
    ///
    /// Result containing vector of created bead IDs.
    ///
    /// # Example
    ///
    /// ```rust
    /// let ws = LabelTestWorkspace::new().unwrap();
    /// let ids = ws.seed_labeled_beads(5).unwrap();
    /// assert_eq!(ids.len(), 5);
    /// ```
    pub fn seed_labeled_beads(&self, count: usize) -> anyhow::Result<Vec<String>> {
        let mut bead_ids = Vec::new();
        let label_sets = vec![
            vec!["bug", "critical"],
            vec!["feature", "enhancement"],
            vec!["documentation"],
            vec!["bug", "minor"],
            vec!["feature", "backend", "database"],
        ];

        for i in 0..count {
            let id = format!("bf-label-test-{:03}", i);
            let title = format!("Label Test Bead #{}", i);
            let labels = label_sets[i % label_sets.len()]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();

            let string_labels: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            self.create_bead_with_labels(&id, &title, &string_labels)?;
            bead_ids.push(id);
        }

        Ok(bead_ids)
    }

    /// Export beads to JSONL.
    ///
    /// # Arguments
    ///
    /// * `dirty_only` - If true, only export dirty beads
    ///
    /// # Returns
    ///
    /// Result containing the number of exported beads.
    pub fn export_jsonl(&self, dirty_only: bool) -> anyhow::Result<usize> {
        let storage = self.storage()?;
        storage.sync_to_jsonl(&self.jsonl_path, dirty_only)
    }

    /// Import beads from JSONL.
    ///
    /// # Returns
    ///
    /// Result containing import result with counts.
    pub fn import_jsonl(&self) -> anyhow::Result<bead_forge::jsonl::ImportResult> {
        let storage = self.storage()?;
        storage.sync_from_jsonl(&self.jsonl_path)
    }
}

/// Builder pattern for creating test beads with labels.
///
/// Provides fluent API for building test bead configurations.
///
/// # Example
///
/// ```rust
/// let bead = LabelTestBeadBuilder::new("bf-builder-001", "Builder Test")
///     .with_labels(&["bug", "critical"])
///     .with_priority(bead_forge::Priority::CRITICAL)
///     .with_description("Test bead description")
///     .build();
/// ```
pub struct LabelTestBeadBuilder {
    id: String,
    title: String,
    labels: Vec<String>,
    priority: bead_forge::Priority,
    issue_type: bead_forge::IssueType,
    description: Option<String>,
    assignee: Option<String>,
}

impl LabelTestBeadBuilder {
    /// Create a new test bead builder.
    ///
    /// # Arguments
    ///
    /// * `id` - Bead ID
    /// * `title` - Bead title
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            labels: Vec::new(),
            priority: bead_forge::Priority::MEDIUM,
            issue_type: bead_forge::IssueType::Task,
            description: None,
            assignee: None,
        }
    }

    /// Add labels to the bead.
    ///
    /// # Arguments
    ///
    /// * `labels` - Array of label strings
    pub fn with_labels(mut self, labels: &[&str]) -> Self {
        self.labels = labels.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set the priority.
    ///
    /// # Arguments
    ///
    /// * `priority` - Priority value
    pub fn with_priority(mut self, priority: bead_forge::Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the issue type.
    ///
    /// # Arguments
    ///
    /// * `issue_type` - Issue type (Task, Epic, etc.)
    pub fn with_type(mut self, issue_type: bead_forge::IssueType) -> Self {
        self.issue_type = issue_type;
        self
    }

    /// Add a description.
    ///
    /// # Arguments
    ///
    /// * `description` - Description text
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set an assignee.
    ///
    /// # Arguments
    ///
    /// * `assignee` - Assignee identifier
    pub fn with_assignee(mut self, assignee: impl Into<String>) -> Self {
        self.assignee = Some(assignee.into());
        self
    }

    /// Build the test bead.
    ///
    /// Returns a fully-configured Issue ready for storage.
    pub fn build(self) -> bead_forge::Issue {
        bead_forge::Issue {
            id: self.id,
            title: self.title,
            labels: self.labels,
            priority: self.priority,
            issue_type: self.issue_type,
            description: self.description,
            assignee: self.assignee,
            ..Default::default()
        }
    }
}

/// Assert that a bead has exactly the specified labels.
///
/// Fails if the bead's labels don't match exactly (order-independent).
///
/// # Arguments
///
/// * `issue_id` - Bead ID to check
/// * `expected_labels` - Array of expected label strings
/// * `workspace` - LabelTestWorkspace instance
///
/// # Example
///
/// ```rust
/// let ws = LabelTestWorkspace::new().unwrap();
/// ws.create_bead_with_labels("bf-test", "Test", &["a", "b"]).unwrap();
/// assert_labels_eq("bf-test", &["a", "b"], &ws).unwrap();
/// ```
pub fn assert_labels_eq(
    issue_id: &str,
    expected_labels: &[&str],
    workspace: &LabelTestWorkspace,
) -> anyhow::Result<()> {
    let actual_labels = workspace.get_labels(issue_id)?;
    let expected_set: std::collections::HashSet<&str> =
        expected_labels.iter().cloned().collect();
    let actual_set: std::collections::HashSet<String> =
        actual_labels.into_iter().collect();

    if expected_set.iter().map(|s| s.to_string()).collect::<std::collections::HashSet<_>>()
        != actual_set
    {
        anyhow::bail!(
            "Label mismatch for bead {}: expected {:?}, got {:?}",
            issue_id,
            expected_labels,
            actual_set
        );
    }

    Ok(())
}

/// Assert that a bead contains a specific label.
///
/// # Arguments
///
/// * `issue_id` - Bead ID to check
/// * `label` - Label string that must exist
/// * `workspace` - LabelTestWorkspace instance
pub fn assert_has_label(
    issue_id: &str,
    label: &str,
    workspace: &LabelTestWorkspace,
) -> anyhow::Result<()> {
    if !workspace.has_label(issue_id, label)? {
        anyhow::bail!(
            "Bead {} should have label '{}' but does not",
            issue_id,
            label
        );
    }
    Ok(())
}

/// Assert that a bead does NOT contain a specific label.
///
/// # Arguments
///
/// * `issue_id` - Bead ID to check
/// * `label` - Label string that must NOT exist
/// * `workspace` - LabelTestWorkspace instance
pub fn assert_not_has_label(
    issue_id: &str,
    label: &str,
    workspace: &LabelTestWorkspace,
) -> anyhow::Result<()> {
    if workspace.has_label(issue_id, label)? {
        anyhow::bail!(
            "Bead {} should NOT have label '{}' but does",
            issue_id,
            label
        );
    }
    Ok(())
}

/// Assert that a bead has exactly N labels.
///
/// # Arguments
///
/// * `issue_id` - Bead ID to check
/// * `count` - Expected number of labels
/// * `workspace` - LabelTestWorkspace instance
pub fn assert_label_count(
    issue_id: &str,
    count: usize,
    workspace: &LabelTestWorkspace,
) -> anyhow::Result<()> {
    let actual_count = workspace.count_labels(issue_id)?;
    if actual_count != count {
        anyhow::bail!(
            "Bead {} should have {} labels but has {}",
            issue_id,
            count,
            actual_count
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_test_workspace_creation() {
        let ws = LabelTestWorkspace::new().unwrap();
        assert!(ws.beads_dir.exists());
        assert!(ws.db_path.exists());
    }

    #[test]
    fn test_create_bead_with_labels() {
        let ws = LabelTestWorkspace::new().unwrap();
        ws.create_bead_with_labels("bf-test", "Test bead", &["label1", "label2"])
            .unwrap();

        let labels = ws.get_labels("bf-test").unwrap();
        assert_eq!(labels.len(), 2);
        assert!(labels.contains(&"label1".to_string()));
        assert!(labels.contains(&"label2".to_string()));
    }

    #[test]
    fn test_add_single_label() {
        let ws = LabelTestWorkspace::new().unwrap();
        ws.create_bead("bf-test", "Test bead").unwrap();

        ws.add_label("bf-test", "new-label").unwrap();

        let labels = ws.get_labels("bf-test").unwrap();
        assert_eq!(labels, vec!["new-label".to_string()]);
    }

    #[test]
    fn test_add_multiple_labels() {
        let ws = LabelTestWorkspace::new().unwrap();
        ws.create_bead("bf-test", "Test bead").unwrap();

        ws.add_labels("bf-test", &["a", "b", "c"]).unwrap();

        let labels = ws.get_labels("bf-test").unwrap();
        assert_eq!(labels.len(), 3);
    }

    #[test]
    fn test_remove_label() {
        let ws = LabelTestWorkspace::new().unwrap();
        ws.create_bead_with_labels("bf-test", "Test", &["a", "b", "c"])
            .unwrap();

        ws.remove_label("bf-test", "b").unwrap();

        let labels = ws.get_labels("bf-test").unwrap();
        assert_eq!(labels.len(), 2);
        assert!(!labels.contains(&"b".to_string()));
    }

    #[test]
    fn test_has_label() {
        let ws = LabelTestWorkspace::new().unwrap();
        ws.create_bead_with_labels("bf-test", "Test", &["existing"])
            .unwrap();

        assert!(ws.has_label("bf-test", "existing").unwrap());
        assert!(!ws.has_label("bf-test", "nonexistent").unwrap());
    }

    #[test]
    fn test_clear_labels() {
        let ws = LabelTestWorkspace::new().unwrap();
        ws.create_bead_with_labels("bf-test", "Test", &["a", "b", "c"])
            .unwrap();

        ws.clear_labels("bf-test").unwrap();

        let labels = ws.get_labels("bf-test").unwrap();
        assert!(labels.is_empty());
    }

    #[test]
    fn test_create_p0_epic_with_labels() {
        let ws = LabelTestWorkspace::new().unwrap();
        ws.create_p0_epic_with_labels("bf-epic-001", "Critical Epic", &["p0", "urgent"])
            .unwrap();

        let epic = ws.get_bead("bf-epic-001").unwrap().unwrap();
        assert_eq!(epic.issue_type, bead_forge::IssueType::Epic);
        assert_eq!(epic.priority, bead_forge::Priority::CRITICAL);
        assert_eq!(epic.labels.len(), 2);
    }

    #[test]
    fn test_seed_labeled_beads() {
        let ws = LabelTestWorkspace::new().unwrap();
        let ids = ws.seed_labeled_beads(5).unwrap();

        assert_eq!(ids.len(), 5);

        for id in &ids {
            let bead = ws.get_bead(id).unwrap().unwrap();
            assert!(!bead.labels.is_empty());
        }
    }

    #[test]
    fn test_label_test_bead_builder() {
        let bead = LabelTestBeadBuilder::new("bf-builder", "Builder Test")
            .with_labels(&["bug", "critical"])
            .with_priority(bead_forge::Priority::CRITICAL)
            .with_description("Test description")
            .build();

        assert_eq!(bead.id, "bf-builder");
        assert_eq!(bead.title, "Builder Test");
        assert_eq!(bead.labels.len(), 2);
        assert_eq!(bead.priority, bead_forge::Priority::CRITICAL);
        assert_eq!(bead.description, Some("Test description".to_string()));
    }

    #[test]
    fn test_assert_labels_eq() {
        let ws = LabelTestWorkspace::new().unwrap();
        ws.create_bead_with_labels("bf-test", "Test", &["a", "b", "c"])
            .unwrap();

        assert_labels_eq("bf-test", &["a", "b", "c"], &ws).unwrap();
        assert_labels_eq("bf-test", &["c", "b", "a"], &ws).unwrap(); // Order-independent
    }

    #[test]
    fn test_assert_has_label() {
        let ws = LabelTestWorkspace::new().unwrap();
        ws.create_bead_with_labels("bf-test", "Test", &["existing"])
            .unwrap();

        assert_has_label("bf-test", "existing", &ws).unwrap();
    }

    #[test]
    fn test_assert_not_has_label() {
        let ws = LabelTestWorkspace::new().unwrap();
        ws.create_bead_with_labels("bf-test", "Test", &["existing"])
            .unwrap();

        assert_not_has_label("bf-test", "nonexistent", &ws).unwrap();
    }

    #[test]
    fn test_assert_label_count() {
        let ws = LabelTestWorkspace::new().unwrap();
        ws.create_bead_with_labels("bf-test", "Test", &["a", "b", "c"])
            .unwrap();

        assert_label_count("bf-test", 3, &ws).unwrap();
    }

    #[test]
    fn test_list_all_labels() {
        let ws = LabelTestWorkspace::new().unwrap();
        ws.create_bead_with_labels("bf-1", "Test 1", &["label1", "label2"])
            .unwrap();
        ws.create_bead_with_labels("bf-2", "Test 2", &["label2", "label3"])
            .unwrap();

        let all_labels = ws.list_all_labels().unwrap();
        assert!(all_labels.len() >= 3);
    }

    #[test]
    fn test_label_persistence_through_jsonl_roundtrip() {
        let ws = LabelTestWorkspace::new().unwrap();
        ws.create_bead_with_labels("bf-test", "Test", &["persist1", "persist2"])
            .unwrap();

        // Export to JSONL
        ws.export_jsonl(false).unwrap();

        // Clear database
        std::fs::remove_file(&ws.db_path).unwrap();
        let _storage = bead_forge::storage::Storage::open(&ws.db_path).unwrap();

        // Import from JSONL
        ws.import_jsonl().unwrap();

        // Verify labels persisted
        let labels = ws.get_labels("bf-test").unwrap();
        assert_eq!(labels.len(), 2);
        assert!(labels.contains(&"persist1".to_string()));
        assert!(labels.contains(&"persist2".to_string()));
    }
}
