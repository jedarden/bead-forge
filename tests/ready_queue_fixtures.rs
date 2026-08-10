//! Test fixtures and helper functions for ready queue testing.
//!
//! Provides:
//! - `ReadyQueueTestWorkspace` for isolated ready queue testing environments
//! - Helper functions for creating test beads in different states
//! - Ready-candidate-specific assertion helpers
//! - Builder patterns for ready queue test data
//! - Environment cleanup/teardown (automatic via TempDir)
//!
//! # Example
//!
//! ```rust
//! let ws = ReadyQueueTestWorkspace::new().unwrap();
//! ws.create_open_bead("bf-open-1", "Open task", Priority::HIGH).unwrap();
//! ws.create_closed_bead("bf-closed-1", "Closed task").unwrap();
//! let candidates = ws.get_ready_candidates(10).unwrap();
//! assert_eq!(candidates.len(), 1);
//! ```

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use bead_forge::model::{Issue, Priority, Status, IssueType, DependencyType};
use bead_forge::storage::Storage;
use bead_forge::claim::{get_ready_candidates, ScoredBead};
use chrono::Utc;

/// Isolated test workspace for ready queue operations.
///
/// Automatically cleaned up when dropped (TempDir).
pub struct ReadyQueueTestWorkspace {
    /// Temp directory holding the workspace (auto-cleaned on drop)
    pub temp_dir: TempDir,
    /// Path to the .beads directory
    pub beads_dir: PathBuf,
    /// Path to the database file
    pub db_path: PathBuf,
    /// Path to the JSONL file
    pub jsonl_path: PathBuf,
}

impl ReadyQueueTestWorkspace {
    /// Create a new isolated test workspace for ready queue operations.
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
    /// let ws = ReadyQueueTestWorkspace::new().unwrap();
    /// assert!(ws.db_path.exists());
    /// assert!(ws.beads_dir.exists());
    /// ```
    pub fn new() -> anyhow::Result<Self> {
        let temp_dir = TempDir::new()?;
        let workspace_dir = temp_dir.path().join("ready-queue-test-workspace");
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
        let _storage = Storage::open(&db_path)?;

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
    pub fn storage(&self) -> anyhow::Result<Storage> {
        Ok(Storage::open(&self.db_path)?)
    }

    /// Create a test bead with the given ID and title.
    ///
    /// Creates a basic task-type bead with default priority and open status.
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
        let bead = Issue::new(id.to_string(), title.to_string(), ".".to_string());
        Ok(storage.create_issue(&bead)?)
    }

    /// Create an open bead with custom priority.
    ///
    /// # Arguments
    ///
    /// * `id` - Bead ID
    /// * `title` - Bead title
    /// * `priority` - Priority level
    ///
    /// # Returns
    ///
    /// Result indicating success or error.
    pub fn create_open_bead(
        &self,
        id: &str,
        title: &str,
        priority: Priority,
    ) -> anyhow::Result<()> {
        let storage = self.storage()?;
        let bead = Issue {
            id: id.to_string(),
            title: title.to_string(),
            priority,
            status: Status::Open,
            ..Default::default()
        };
        Ok(storage.create_issue(&bead)?)
    }

    /// Create a closed bead.
    ///
    /// # Arguments
    ///
    /// * `id` - Bead ID
    /// * `title` - Bead title
    ///
    /// # Returns
    ///
    /// Result indicating success or error.
    pub fn create_closed_bead(&self, id: &str, title: &str) -> anyhow::Result<()> {
        let storage = self.storage()?;
        let mut bead = Issue::new(id.to_string(), title.to_string(), ".".to_string());
        bead.status = Status::Closed;
        bead.closed_at = Some(Utc::now());
        Ok(storage.create_issue(&bead)?)
    }

    /// Create an in-progress bead.
    ///
    /// # Arguments
    ///
    /// * `id` - Bead ID
    /// * `title` - Bead title
    /// * `assignee` - Assignee name
    ///
    /// # Returns
    ///
    /// Result indicating success or error.
    pub fn create_in_progress_bead(
        &self,
        id: &str,
        title: &str,
        assignee: &str,
    ) -> anyhow::Result<()> {
        let storage = self.storage()?;
        let bead = Issue {
            id: id.to_string(),
            title: title.to_string(),
            status: Status::InProgress,
            assignee: Some(assignee.to_string()),
            ..Default::default()
        };
        Ok(storage.create_issue(&bead)?)
    }

    /// Create a blocked bead with dependencies.
    ///
    /// Creates a bead with open dependencies, making it effectively blocked.
    ///
    /// # Arguments
    ///
    /// * `id` - Bead ID
    /// * `title` - Bead title
    /// * `dependency_ids` - Array of bead IDs this bead depends on
    ///
    /// # Returns
    ///
    /// Result indicating success or error.
    ///
    /// # Example
    ///
    /// ```rust
    /// let ws = ReadyQueueTestWorkspace::new().unwrap();
    /// ws.create_open_bead("bf-blocker", "Blocker", Priority::MEDIUM).unwrap();
    /// ws.create_blocked_bead("bf-dependent", "Dependent task", &["bf-blocker"]).unwrap();
    /// ```
    pub fn create_blocked_bead(
        &self,
        id: &str,
        title: &str,
        dependency_ids: &[&str],
    ) -> anyhow::Result<()> {
        let storage = self.storage()?;
        let bead = Issue {
            id: id.to_string(),
            title: title.to_string(),
            status: Status::Open,
            ..Default::default()
        };
        storage.create_issue(&bead)?;

        for dep_id in dependency_ids {
            storage.add_dependency(id, dep_id, &DependencyType::Blocks, "test")?;
        }

        Ok(())
    }

    /// Create a bead that should be ready (open, no blockers, not assigned).
    ///
    /// # Arguments
    ///
    /// * `id` - Bead ID
    /// * `title` - Bead title
    /// * `priority` - Priority level
    ///
    /// # Returns
    ///
    /// Result indicating success or error.
    pub fn create_ready_bead(
        &self,
        id: &str,
        title: &str,
        priority: Priority,
    ) -> anyhow::Result<()> {
        self.create_open_bead(id, title, priority)
    }

    /// Create a pinned bead (should not appear in ready queue).
    ///
    /// # Arguments
    ///
    /// * `id` - Bead ID
    /// * `title` - Bead title
    ///
    /// # Returns
    ///
    /// Result indicating success or error.
    pub fn create_pinned_bead(&self, id: &str, title: &str) -> anyhow::Result<()> {
        let storage = self.storage()?;
        let bead = Issue {
            id: id.to_string(),
            title: title.to_string(),
            pinned: true,
            ..Default::default()
        };
        Ok(storage.create_issue(&bead)?)
    }

    /// Create an ephemeral bead (should not appear in ready queue).
    ///
    /// # Arguments
    ///
    /// * `id` - Bead ID
    /// * `title` - Bead title
    ///
    /// # Returns
    ///
    /// Result indicating success or error.
    pub fn create_ephemeral_bead(&self, id: &str, title: &str) -> anyhow::Result<()> {
        let storage = self.storage()?;
        let bead = Issue {
            id: id.to_string(),
            title: title.to_string(),
            ephemeral: true,
            ..Default::default()
        };
        Ok(storage.create_issue(&bead)?)
    }

    /// Create a template bead (should not appear in ready queue).
    ///
    /// # Arguments
    ///
    /// * `id` - Bead ID
    /// * `title` - Bead title
    ///
    /// # Returns
    ///
    /// Result indicating success or error.
    pub fn create_template_bead(&self, id: &str, title: &str) -> anyhow::Result<()> {
        let storage = self.storage()?;
        let bead = Issue {
            id: id.to_string(),
            title: title.to_string(),
            is_template: true,
            ..Default::default()
        };
        Ok(storage.create_issue(&bead)?)
    }

    /// Get ready candidates from the queue.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of candidates to return
    ///
    /// # Returns
    ///
    /// Result containing vector of ready candidates.
    pub fn get_ready_candidates(&self, limit: usize) -> anyhow::Result<Vec<ScoredBead>> {
        let storage = self.storage()?;
        Ok(storage.with_immediate_transaction(|tx| {
            Ok(get_ready_candidates(tx, limit, None, None)?)
        })?)
    }

    /// Seed workspace with multiple beads in different states.
    ///
    /// Creates a comprehensive set of test beads covering various states.
    ///
    /// # Returns
    ///
    /// Result containing vector of all created bead IDs.
    ///
    /// # Example
    ///
    /// ```rust
    /// let ws = ReadyQueueTestWorkspace::new().unwrap();
    /// let ids = ws.seed_mixed_state_beads().unwrap();
    /// assert_eq!(ids.len(), 8); // Creates 8 beads in various states
    /// ```
    pub fn seed_mixed_state_beads(&self) -> anyhow::Result<Vec<String>> {
        let mut bead_ids = Vec::new();

        // Open, ready beads (different priorities)
        self.create_ready_bead("bf-ready-p0", "P0 Ready task", Priority::CRITICAL)?;
        bead_ids.push("bf-ready-p0".to_string());

        self.create_ready_bead("bf-ready-p1", "P1 Ready task", Priority::HIGH)?;
        bead_ids.push("bf-ready-p1".to_string());

        self.create_ready_bead("bf-ready-p2", "P2 Ready task", Priority::MEDIUM)?;
        bead_ids.push("bf-ready-p2".to_string());

        // Closed bead
        self.create_closed_bead("bf-closed", "Closed task")?;
        bead_ids.push("bf-closed".to_string());

        // In-progress bead
        self.create_in_progress_bead("bf-in-progress", "In-progress task", "worker1")?;
        bead_ids.push("bf-in-progress".to_string());

        // Blocked bead (create blocker first)
        self.create_open_bead("bf-blocker", "Blocker task", Priority::MEDIUM)?;
        bead_ids.push("bf-blocker".to_string());

        self.create_blocked_bead("bf-blocked", "Blocked task", &["bf-blocker"])?;
        bead_ids.push("bf-blocked".to_string());

        // Pinned bead
        self.create_pinned_bead("bf-pinned", "Pinned task")?;
        bead_ids.push("bf-pinned".to_string());

        Ok(bead_ids)
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
    pub fn get_bead(&self, id: &str) -> anyhow::Result<Option<Issue>> {
        let storage = self.storage()?;
        Ok(storage.get_issue(id)?)
    }

    /// Count total beads in the workspace.
    ///
    /// # Returns
    ///
    /// Result containing the total bead count.
    pub fn count_beads(&self) -> anyhow::Result<usize> {
        let storage = self.storage()?;
        let beads = storage.list_issues(&Default::default())?;
        Ok(beads.len())
    }

    /// Count beads by status.
    ///
    /// # Arguments
    ///
    /// * `status` - Status to filter by
    ///
    /// # Returns
    ///
    /// Result containing the count of beads with the given status.
    pub fn count_beads_by_status(&self, status: Status) -> anyhow::Result<usize> {
        let storage = self.storage()?;
        let mut filter = bead_forge::model::IssueFilter::default();
        filter.status = Some(status);
        let beads = storage.list_issues(&filter)?;
        Ok(beads.len())
    }
}

/// Builder pattern for creating test beads with various states.
///
/// Provides fluent API for building test bead configurations.
///
/// # Example
///
/// ```rust
/// let bead = ReadyQueueTestBeadBuilder::new("bf-builder-001", "Builder Test")
///     .with_priority(Priority::CRITICAL)
///     .with_status(Status::InProgress)
///     .with_assignee("worker1")
///     .build();
/// ```
pub struct ReadyQueueTestBeadBuilder {
    id: String,
    title: String,
    priority: Priority,
    status: Status,
    issue_type: IssueType,
    assignee: Option<String>,
    pinned: bool,
    ephemeral: bool,
    is_template: bool,
    dependency_ids: Vec<String>,
}

impl ReadyQueueTestBeadBuilder {
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
            priority: Priority::MEDIUM,
            status: Status::Open,
            issue_type: IssueType::Task,
            assignee: None,
            pinned: false,
            ephemeral: false,
            is_template: false,
            dependency_ids: Vec::new(),
        }
    }

    /// Set the priority.
    ///
    /// # Arguments
    ///
    /// * `priority` - Priority value
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the status.
    ///
    /// # Arguments
    ///
    /// * `status` - Status value
    pub fn with_status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    /// Set the issue type.
    ///
    /// # Arguments
    ///
    /// * `issue_type` - Issue type (Task, Epic, etc.)
    pub fn with_type(mut self, issue_type: IssueType) -> Self {
        self.issue_type = issue_type;
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

    /// Mark as pinned.
    pub fn with_pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    /// Mark as ephemeral.
    pub fn with_ephemeral(mut self, ephemeral: bool) -> Self {
        self.ephemeral = ephemeral;
        self
    }

    /// Mark as template.
    pub fn with_template(mut self, is_template: bool) -> Self {
        self.is_template = is_template;
        self
    }

    /// Add dependency IDs.
    ///
    /// # Arguments
    ///
    /// * `dep_ids` - Array of bead IDs this bead depends on
    pub fn with_dependencies(mut self, dep_ids: &[&str]) -> Self {
        self.dependency_ids = dep_ids.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Build the test bead.
    ///
    /// Returns a fully-configured Issue ready for storage.
    pub fn build(self) -> Issue {
        let mut bead = Issue {
            id: self.id,
            title: self.title,
            priority: self.priority,
            status: self.status,
            issue_type: self.issue_type,
            assignee: self.assignee,
            pinned: self.pinned,
            ephemeral: self.ephemeral,
            is_template: self.is_template,
            ..Default::default()
        };

        // Add dependencies to the bead's dependency list
        // Note: These still need to be created in the database via storage.add_dependency()
        for dep_id in &self.dependency_ids {
            bead.dependencies.push(bead_forge::model::Dependency {
                issue_id: bead.id.clone(),
                depends_on_id: dep_id.clone(),
                dep_type: DependencyType::Blocks,
                created_at: Utc::now(),
                created_by: Some("test".to_string()),
                ..Default::default()
            });
        }

        bead
    }
}

/// Assert that a bead is ready (appears in ready queue).
///
/// # Arguments
///
/// * `issue_id` - Bead ID to check
/// * `workspace` - ReadyQueueTestWorkspace instance
///
/// # Example
///
/// ```rust
/// let ws = ReadyQueueTestWorkspace::new().unwrap();
/// ws.create_ready_bead("bf-ready", "Ready task", Priority::MEDIUM).unwrap();
/// assert_is_ready("bf-ready", &ws).unwrap();
/// ```
pub fn assert_is_ready(issue_id: &str, workspace: &ReadyQueueTestWorkspace) -> anyhow::Result<()> {
    let candidates = workspace.get_ready_candidates(100)?;
    let found = candidates.iter().any(|c| c.id == issue_id);

    if !found {
        anyhow::bail!(
            "Bead {} should be ready but does not appear in ready queue",
            issue_id
        );
    }

    Ok(())
}

/// Assert that a bead is NOT ready (does not appear in ready queue).
///
/// # Arguments
///
/// * `issue_id` - Bead ID to check
/// * `workspace` - ReadyQueueTestWorkspace instance
pub fn assert_not_ready(issue_id: &str, workspace: &ReadyQueueTestWorkspace) -> anyhow::Result<()> {
    let candidates = workspace.get_ready_candidates(100)?;
    let found = candidates.iter().any(|c| c.id == issue_id);

    if found {
        anyhow::bail!(
            "Bead {} should NOT be ready but appears in ready queue",
            issue_id
        );
    }

    Ok(())
}

/// Assert that the ready queue has exactly N candidates.
///
/// # Arguments
///
/// * `expected_count` - Expected number of ready candidates
/// * `workspace` - ReadyQueueTestWorkspace instance
pub fn assert_ready_count(expected_count: usize, workspace: &ReadyQueueTestWorkspace) -> anyhow::Result<()> {
    let candidates = workspace.get_ready_candidates(100)?;
    let actual_count = candidates.len();

    if actual_count != expected_count {
        anyhow::bail!(
            "Ready queue should have {} candidates but has {}",
            expected_count,
            actual_count
        );
    }

    Ok(())
}

/// Assert that ready candidates are ordered by priority (P0 first).
///
/// Checks that candidates appear in priority order (lower number = higher priority).
///
/// # Arguments
///
/// * `workspace` - ReadyQueueTestWorkspace instance
pub fn assert_priority_ordering(workspace: &ReadyQueueTestWorkspace) -> anyhow::Result<()> {
    let candidates = workspace.get_ready_candidates(100)?;

    for i in 1..candidates.len() {
        let prev_priority = candidates[i - 1].priority;
        let curr_priority = candidates[i].priority;

        // Previous should have lower or equal priority number (higher or equal priority)
        if prev_priority > curr_priority {
            anyhow::bail!(
                "Priority ordering violated: candidate {} has priority {} but candidate {} has priority {}",
                i - 1,
                prev_priority,
                i,
                curr_priority
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ready_queue_test_workspace_creation() {
        let ws = ReadyQueueTestWorkspace::new().unwrap();
        assert!(ws.beads_dir.exists());
        assert!(ws.db_path.exists());
    }

    #[test]
    fn test_create_open_bead() {
        let ws = ReadyQueueTestWorkspace::new().unwrap();
        ws.create_open_bead("bf-test", "Test bead", Priority::HIGH)
            .unwrap();

        let bead = ws.get_bead("bf-test").unwrap().unwrap();
        assert_eq!(bead.status, Status::Open);
        assert_eq!(bead.priority, Priority::HIGH);
    }

    #[test]
    fn test_create_closed_bead() {
        let ws = ReadyQueueTestWorkspace::new().unwrap();
        ws.create_closed_bead("bf-closed", "Closed bead").unwrap();

        let bead = ws.get_bead("bf-closed").unwrap().unwrap();
        assert_eq!(bead.status, Status::Closed);
        assert!(bead.closed_at.is_some());
    }

    #[test]
    fn test_create_in_progress_bead() {
        let ws = ReadyQueueTestWorkspace::new().unwrap();
        ws.create_in_progress_bead("bf-progress", "In-progress bead", "worker1")
            .unwrap();

        let bead = ws.get_bead("bf-progress").unwrap().unwrap();
        assert_eq!(bead.status, Status::InProgress);
        assert_eq!(bead.assignee.as_deref(), Some("worker1"));
    }

    #[test]
    fn test_create_blocked_bead() {
        let ws = ReadyQueueTestWorkspace::new().unwrap();
        ws.create_open_bead("bf-blocker", "Blocker", Priority::MEDIUM)
            .unwrap();
        ws.create_blocked_bead("bf-dependent", "Dependent", &["bf-blocker"])
            .unwrap();

        let bead = ws.get_bead("bf-dependent").unwrap().unwrap();
        assert_eq!(bead.status, Status::Open);
        assert_eq!(bead.dependencies.len(), 1);
    }

    #[test]
    fn test_create_pinned_bead() {
        let ws = ReadyQueueTestWorkspace::new().unwrap();
        ws.create_pinned_bead("bf-pinned", "Pinned bead").unwrap();

        let bead = ws.get_bead("bf-pinned").unwrap().unwrap();
        assert!(bead.pinned);
    }

    #[test]
    fn test_create_ephemeral_bead() {
        let ws = ReadyQueueTestWorkspace::new().unwrap();
        ws.create_ephemeral_bead("bf-ephemeral", "Ephemeral bead")
            .unwrap();

        let bead = ws.get_bead("bf-ephemeral").unwrap().unwrap();
        assert!(bead.ephemeral);
    }

    #[test]
    fn test_create_template_bead() {
        let ws = ReadyQueueTestWorkspace::new().unwrap();
        ws.create_template_bead("bf-template", "Template bead").unwrap();

        let bead = ws.get_bead("bf-template").unwrap().unwrap();
        assert!(bead.is_template);
    }

    #[test]
    fn test_seed_mixed_state_beads() {
        let ws = ReadyQueueTestWorkspace::new().unwrap();
        let ids = ws.seed_mixed_state_beads().unwrap();

        assert_eq!(ids.len(), 8);

        // Verify all beads exist
        for id in &ids {
            assert!(ws.get_bead(id).unwrap().is_some());
        }
    }

    #[test]
    fn test_get_ready_candidates_filters_non_ready() {
        let ws = ReadyQueueTestWorkspace::new().unwrap();
        ws.seed_mixed_state_beads().unwrap();

        let candidates = ws.get_ready_candidates(100).unwrap();

        // Only the 3 ready beads should appear
        assert_eq!(candidates.len(), 3);

        // All should be open status
        for candidate in &candidates {
            assert_eq!(candidate.status, "open");
        }
    }

    #[test]
    fn test_ready_queue_test_bead_builder() {
        let bead = ReadyQueueTestBeadBuilder::new("bf-builder", "Builder Test")
            .with_priority(Priority::CRITICAL)
            .with_status(Status::InProgress)
            .with_assignee("worker1")
            .with_pinned(true)
            .build();

        assert_eq!(bead.id, "bf-builder");
        assert_eq!(bead.title, "Builder Test");
        assert_eq!(bead.priority, Priority::CRITICAL);
        assert_eq!(bead.status, Status::InProgress);
        assert_eq!(bead.assignee.as_deref(), Some("worker1"));
        assert!(bead.pinned);
    }

    #[test]
    fn test_assert_is_ready() {
        let ws = ReadyQueueTestWorkspace::new().unwrap();
        ws.create_ready_bead("bf-ready", "Ready task", Priority::MEDIUM)
            .unwrap();

        assert_is_ready("bf-ready", &ws).unwrap();
    }

    #[test]
    fn test_assert_not_ready() {
        let ws = ReadyQueueTestWorkspace::new().unwrap();
        ws.create_closed_bead("bf-closed", "Closed task").unwrap();

        assert_not_ready("bf-closed", &ws).unwrap();
    }

    #[test]
    fn test_assert_ready_count() {
        let ws = ReadyQueueTestWorkspace::new().unwrap();
        ws.create_ready_bead("bf-ready-1", "Ready 1", Priority::MEDIUM)
            .unwrap();
        ws.create_ready_bead("bf-ready-2", "Ready 2", Priority::HIGH)
            .unwrap();

        assert_ready_count(2, &ws).unwrap();
    }

    #[test]
    fn test_assert_priority_ordering() {
        let ws = ReadyQueueTestWorkspace::new().unwrap();
        ws.create_ready_bead("bf-p2", "P2 task", Priority::MEDIUM)
            .unwrap();
        ws.create_ready_bead("bf-p0", "P0 task", Priority::CRITICAL)
            .unwrap();
        ws.create_ready_bead("bf-p1", "P1 task", Priority::HIGH)
            .unwrap();

        assert_priority_ordering(&ws).unwrap();
    }

    #[test]
    fn test_count_beads() {
        let ws = ReadyQueueTestWorkspace::new().unwrap();
        assert_eq!(ws.count_beads().unwrap(), 0);

        ws.create_ready_bead("bf-1", "Bead 1", Priority::MEDIUM)
            .unwrap();
        ws.create_closed_bead("bf-2", "Bead 2").unwrap();

        assert_eq!(ws.count_beads().unwrap(), 2);
    }

    #[test]
    fn test_count_beads_by_status() {
        let ws = ReadyQueueTestWorkspace::new().unwrap();
        ws.create_ready_bead("bf-open-1", "Open 1", Priority::MEDIUM)
            .unwrap();
        ws.create_closed_bead("bf-closed", "Closed").unwrap();

        assert_eq!(ws.count_beads_by_status(Status::Open).unwrap(), 1);
        assert_eq!(ws.count_beads_by_status(Status::Closed).unwrap(), 1);
    }
}
