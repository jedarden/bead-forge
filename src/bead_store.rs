//! NEEDLE bead_store integration module.
//!
//! This module provides a Rust API for NEEDLE to use bead-forge's atomic
//! claiming functionality, replacing the race-prone `br ready` → `claim`
//! pattern with a single atomic operation.
//!
//! # Migration from br's ready+claim
//!
//! **OLD (racy - TOCTOU race condition):**
//! ```text
//! // Step 1: Get candidate list (snapshot)
//! let candidates = br_ready().await?;
//! // ← Race window: another worker can claim the top candidate here
//!
//! // Step 2: Pick and claim (may fail if already claimed)
//! let bead_id = candidates.first().ok_or(Error::NoBeads)?;
//! br_claim(bead_id, worker_id).await?;
//! ```
//!
//! **NEW (atomic - no race condition):**
//! ```text
//! // Single atomic operation: scoring and claiming in one transaction
//! let result = bead_store::claim_bead(&workspace, worker_id, metadata).await?;
//! ```
//!
//! # Why this is safe
//!
//! The entire read-score-update sequence runs inside a single SQLite
//! `BEGIN IMMEDIATE` transaction. SQLite acquires the write lock before
//! any reads, so no two workers can observe the same candidate state.
//!
//! Worker 1: BEGIN IMMEDIATE → SELECT candidates → score → UPDATE winner → COMMIT
//! Worker 2: BEGIN IMMEDIATE → (blocked until Worker 1 commits) → SELECT → UPDATE → COMMIT
//!
//! No flock, no server, no daemon. SQLite's own locking serializes claims.

use crate::claim::{claim, claim_any, WorkerMetadata};
use crate::config::{find_beads_dir, load_config, load_metadata};
use crate::storage::Storage;
use anyhow::{anyhow, Result};
use chrono::Utc;
use std::path::{Path, PathBuf};

/// Configuration for bead claiming operations.
#[derive(Debug, Clone)]
pub struct ClaimConfig {
    /// Worker ID (e.g., "worker-01", "claude-sonnet-4-6-01")
    pub worker_id: String,

    /// Model name (e.g., "claude-sonnet-4-6", "claude-opus-4-7")
    pub model: Option<String>,

    /// Harness name (e.g., "needle", "custom")
    pub harness: Option<String>,

    /// Harness version (e.g., "0.5.2")
    pub harness_version: Option<String>,

    /// Claim TTL in minutes (default: 30)
    pub claim_ttl_minutes: Option<i64>,

    /// Search all workspaces instead of just the current one
    pub any_workspace: bool,

    /// Additional workspace paths to search (only used with any_workspace)
    pub workspace_paths: Vec<PathBuf>,
}

impl ClaimConfig {
    /// Create a new claim config with minimal required fields.
    pub fn new(worker_id: String) -> Self {
        Self {
            worker_id,
            model: None,
            harness: None,
            harness_version: None,
            claim_ttl_minutes: None,
            any_workspace: false,
            workspace_paths: Vec::new(),
        }
    }

    /// Set the model name.
    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    /// Set the harness name.
    pub fn with_harness(mut self, harness: String) -> Self {
        self.harness = Some(harness);
        self
    }

    /// Set the harness version.
    pub fn with_harness_version(mut self, version: String) -> Self {
        self.harness_version = Some(version);
        self
    }

    /// Enable multi-workspace claiming.
    pub fn any_workspace(mut self, any: bool) -> Self {
        self.any_workspace = any;
        self
    }

    /// Set additional workspace paths to search.
    pub fn workspace_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.workspace_paths = paths;
        self
    }

    /// Set the claim TTL.
    pub fn claim_ttl_minutes(mut self, ttl: i64) -> Self {
        self.claim_ttl_minutes = Some(ttl);
        self
    }
}

/// Result of a successful bead claim.
#[derive(Debug, Clone)]
pub struct ClaimedBead {
    /// The bead ID that was claimed
    pub bead_id: String,

    /// Number of stale claims that were reclaimed
    pub reclaimed_count: usize,

    /// Workspace path where the bead was claimed (for multi-workspace claims)
    pub workspace_path: Option<PathBuf>,
}

/// Atomically claim a bead from a workspace.
///
/// This function performs the following in a single IMMEDIATE transaction:
/// 1. Reclaim stale in_progress beads (older than claim_ttl) back to open
/// 2. Select candidates with downstream_impact + critical_float scoring
/// 3. Update the winner to in_progress with assignee=worker
/// 4. Insert an event
/// 5. Mark the bead as dirty
/// 6. Commit
///
/// # Arguments
/// * `workspace` - Path to the workspace directory (containing .beads/)
/// * `config` - Claim configuration
///
/// # Returns
/// * `Ok(Some(claimed))` - A bead was claimed
/// * `Ok(None)` - No beads available to claim
/// * `Err(e)` - Transaction error
///
/// # Example
/// ```ignore
/// use bead_forge::bead_store::{claim_bead, ClaimConfig};
///
/// let config = ClaimConfig::new("worker-01".to_string())
///     .with_model("claude-sonnet-4-6".to_string())
///     .with_harness("needle".to_string())
///     .with_harness_version("0.5.2".to_string());
///
/// match claim_bead(&PathBuf::from("."), config).await? {
///     Some(claimed) => println!("Claimed bead: {}", claimed.bead_id),
///     None => println!("No beads available"),
/// }
/// ```
pub fn claim_bead(workspace: &Path, config: ClaimConfig) -> Result<Option<ClaimedBead>> {
    let beads_dir = find_beads_dir(workspace)
        .ok_or_else(|| anyhow!("No .beads directory found in {:?}", workspace))?;

    let cfg = load_config(&beads_dir)?;
    let claim_ttl = config.claim_ttl_minutes.unwrap_or(cfg.claim_ttl_minutes);

    let worker_metadata = WorkerMetadata {
        worker_id: config.worker_id.clone(),
        model: config.model,
        harness: config.harness,
        harness_version: config.harness_version,
    };

    if config.any_workspace {
        // Multi-workspace claiming
        let workspace_paths = if config.workspace_paths.is_empty() {
            crate::claim::find_workspaces(workspace)?
        } else {
            config.workspace_paths
        };

        let result = claim_any(&workspace_paths, &config.worker_id, claim_ttl, Some(&worker_metadata))?;

        Ok(result.map(|r| ClaimedBead {
            bead_id: r.bead_id,
            reclaimed_count: r.reclaimed,
            workspace_path: r.workspace_path,
        }))
    } else {
        // Single workspace claiming
        let metadata = load_metadata(&beads_dir)?;
        let db_path = beads_dir.join(&metadata.database);
        let storage = Storage::open(&db_path)?;

        let result = storage.with_immediate_transaction(|tx| {
            claim(tx, &config.worker_id, claim_ttl, Utc::now(), Some(&worker_metadata))
        })?;

        Ok(result.map(|r| ClaimedBead {
            bead_id: r.bead_id,
            reclaimed_count: r.reclaimed,
            workspace_path: None,
        }))
    }
}

/// Get ready candidates without claiming (for display/debugging).
///
/// This returns a list of beads that would be considered for claiming,
/// ordered by the same scoring formula used by claim_bead(). This is
/// useful for displaying what beads are available, but should NOT be
/// used for actual claiming (use claim_bead() instead to avoid races).
///
/// # Arguments
/// * `workspace` - Path to the workspace directory
/// * `limit` - Maximum number of candidates to return
///
/// # Returns
/// * `Ok(Vec<ScoredBead>)` - List of scored bead candidates
pub fn get_ready(workspace: &Path, limit: usize) -> Result<Vec<crate::claim::ScoredBead>> {
    let beads_dir = find_beads_dir(workspace)
        .ok_or_else(|| anyhow!("No .beads directory found in {:?}", workspace))?;

    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    storage.with_immediate_transaction(|tx| crate::claim::get_ready_candidates(tx, limit))
}

/// Check if a bead is ready to be claimed (not blocked).
///
/// This is a convenience function for NEEDLE to verify that a bead
/// can be claimed without actually claiming it.
///
/// # Arguments
/// * `workspace` - Path to the workspace directory
/// * `bead_id` - The bead ID to check
///
/// # Returns
/// * `Ok(true)` - Bead is open and unblocked
/// * `Ok(false)` - Bead is blocked or not open
/// * `Err(e)` - Database error
pub fn is_bead_ready(workspace: &Path, bead_id: &str) -> Result<bool> {
    let beads_dir = find_beads_dir(workspace)
        .ok_or_else(|| anyhow!("No .beads directory found in {:?}", workspace))?;

    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    let issue = storage.get_issue(bead_id)?;

    match issue {
        None => Ok(false),
        Some(issue) => {
            if issue.status.to_string() != "open" {
                return Ok(false);
            }

            // Check if blocked
            let deps = storage.get_dependencies(bead_id)?;
            for dep in deps {
                if dep.dep_type.to_string() == "blocks" {
                    // Check if the blocking bead is still open/in_progress
                    if let Some(blocker) = storage.get_issue(&dep.depends_on_id)? {
                        if blocker.status.to_string() == "open" || blocker.status.to_string() == "in_progress" {
                            return Ok(false);
                        }
                    }
                }
            }

            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Issue, Priority};
    use tempfile::TempDir;

    fn setup_test_workspace() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        // Initialize workspace
        let beads_dir = workspace.join(".beads");
        std::fs::create_dir(&beads_dir).unwrap();

        let config = r#"issue_prefixes: [bf]
default_priority: 2
default_type: task
claim_ttl_minutes: 30
"#;
        std::fs::write(beads_dir.join("config.yaml"), config).unwrap();

        let metadata = r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#;
        std::fs::write(beads_dir.join("metadata.json"), metadata).unwrap();

        (temp_dir, workspace)
    }

    #[test]
    fn test_claim_bead_basic() {
        let (_temp, workspace) = setup_test_workspace();

        // Create some test beads
        let beads_dir = workspace.join(".beads");
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);
        let storage = Storage::open(&db_path).unwrap();

        for i in 1..=3 {
            let mut issue = Issue::new(format!("bf-{:0>4}", i), format!("Test bead {}", i), ".".to_string());
            issue.priority = Priority(i);
            storage.create_issue(&issue).unwrap();
        }

        // Claim a bead
        let config = ClaimConfig::new("test-worker".to_string());
        let result = claim_bead(&workspace, config).unwrap();

        assert!(result.is_some());
        let claimed = result.unwrap();
        assert_eq!(claimed.reclaimed_count, 0);

        // Verify the bead is now in_progress
        let issue = storage.get_issue(&claimed.bead_id).unwrap().unwrap();
        assert_eq!(issue.status.to_string(), "in_progress");
        assert_eq!(issue.assignee.as_ref().unwrap(), "test-worker");
    }

    #[test]
    fn test_claim_bead_priority_ordering() {
        let (_temp, workspace) = setup_test_workspace();

        let beads_dir = workspace.join(".beads");
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);
        let storage = Storage::open(&db_path).unwrap();

        // Create beads with different priorities (0 = highest)
        let priorities = [2, 0, 1, 4, 3];
        for (i, &priority) in priorities.iter().enumerate() {
            let mut issue = Issue::new(format!("bf-{:0>4}", i), format!("Test {}", i), ".".to_string());
            issue.priority = Priority(priority);
            storage.create_issue(&issue).unwrap();
        }

        // Claim should get priority 0 first
        let config = ClaimConfig::new("test-worker".to_string());
        let result = claim_bead(&workspace, config).unwrap();

        assert!(result.is_some());
        let claimed = result.unwrap();
        let issue = storage.get_issue(&claimed.bead_id).unwrap().unwrap();
        assert_eq!(issue.priority.0, 0);
    }

    #[test]
    fn test_claim_bead_empty_workspace() {
        let (_temp, workspace) = setup_test_workspace();

        let config = ClaimConfig::new("test-worker".to_string());
        let result = claim_bead(&workspace, config).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_get_ready() {
        let (_temp, workspace) = setup_test_workspace();

        let beads_dir = workspace.join(".beads");
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);
        let storage = Storage::open(&db_path).unwrap();

        // Create test beads
        for i in 1..=3 {
            let issue = Issue::new(format!("bf-{:0>4}", i), format!("Test {}", i), ".".to_string());
            storage.create_issue(&issue).unwrap();
        }

        let candidates = get_ready(&workspace, 10).unwrap();
        assert_eq!(candidates.len(), 3);
    }

    #[test]
    fn test_is_bead_ready() {
        let (_temp, workspace) = setup_test_workspace();

        let beads_dir = workspace.join(".beads");
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);
        let storage = Storage::open(&db_path).unwrap();

        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        storage.create_issue(&issue).unwrap();

        // Open bead should be ready
        assert!(is_bead_ready(&workspace, "bf-test").unwrap());
    }

    #[test]
    fn test_is_bead_ready_blocked() {
        let (_temp, workspace) = setup_test_workspace();

        let beads_dir = workspace.join(".beads");
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);
        let storage = Storage::open(&db_path).unwrap();

        let blocker = Issue::new("bf-blocker".to_string(), "Blocker".to_string(), ".".to_string());
        storage.create_issue(&blocker).unwrap();

        let blocked = Issue::new("bf-blocked".to_string(), "Blocked".to_string(), ".".to_string());
        storage.create_issue(&blocked).unwrap();

        // Add blocking dependency
        storage.add_dependency("bf-blocked", "bf-blocker", &crate::model::DependencyType::Blocks, "test").unwrap();

        // Blocked bead should not be ready
        assert!(!is_bead_ready(&workspace, "bf-blocked").unwrap());
    }
}
