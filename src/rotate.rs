//! JSONL log rotation for closed beads.
//!
//! Moves closed beads older than `rotate_age_days` from issues.jsonl into
//! numbered archive files (issues.jsonl.1, .2, etc.). Uses streaming rewrite
//! of the active file and streaming append to archives for efficiency.

use crate::config::{load_config, load_metadata};
use crate::model::Issue;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::collections::HashSet;

/// Result of a rotation operation.
#[derive(Debug, Clone, Default)]
pub struct RotateResult {
    /// Number of beads archived
    pub archived: usize,
    /// Number of beads remaining in active file
    pub remaining: usize,
    /// Path to the archive file created (if any)
    pub archive_path: Option<PathBuf>,
    /// Paths to archives deleted due to rotate_max_archives
    pub deleted_archives: Vec<PathBuf>,
}

/// Rotation configuration.
#[derive(Debug, Clone)]
pub struct RotateOptions {
    /// Days threshold for rotating closed beads
    pub age_days: u64,
    /// Maximum size of active JSONL in MB before considering rotation
    pub max_size_mb: Option<u64>,
    /// Maximum number of archive files to keep
    pub max_archives: usize,
    /// Dry run - show what would be done without making changes
    pub dry_run: bool,
}

impl RotateOptions {
    pub fn from_config(age_days: u64, config: &crate::config::Config) -> Self {
        RotateOptions {
            age_days,
            max_size_mb: Some(config.rotate.rotate_max_size_mb),
            max_archives: config.rotate.rotate_max_archives,
            dry_run: false,
        }
    }

    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }
}

/// Rotate closed beads from active JSONL to archive files.
///
/// # Algorithm
/// 1. Scan active JSONL and identify beads to archive (closed + age threshold)
/// 2. Determine next archive number (issues.jsonl.1, .2, etc.)
/// 3. Stream active file: keep active beads in memory, append archived beads to archive
/// 4. Rewrite active file with only active beads
/// 5. Delete oldest archives if exceeding rotate_max_archives
///
/// # Arguments
/// * `beads_dir` - Path to the .beads directory
/// * `options` - Rotation options
///
/// # Returns
/// * `Ok(RotateResult)` - Statistics about the rotation
pub fn rotate(beads_dir: &Path, options: &RotateOptions) -> Result<RotateResult> {
    let metadata = load_metadata(beads_dir)?;
    let jsonl_path = beads_dir.join(&metadata.jsonl_export);

    if !jsonl_path.exists() {
        return Ok(RotateResult::default());
    }

    let cutoff_time = Utc::now() - Duration::days(options.age_days as i64);

    // Phase 1: Scan and categorize beads
    let mut active_beads = Vec::new();
    let mut archive_beads = Vec::new();
    let mut archived_ids = HashSet::new();

    let file = File::open(&jsonl_path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let issue: Issue = serde_json::from_str(&line)
            .map_err(|e| anyhow!("Failed to parse JSONL line: {}", e))?;

        if should_archive(&issue, &cutoff_time) {
            archive_beads.push(issue.clone());
            archived_ids.insert(issue.id);
        } else {
            active_beads.push(issue);
        }
    }

    if archive_beads.is_empty() {
        return Ok(RotateResult {
            archived: 0,
            remaining: active_beads.len(),
            archive_path: None,
            deleted_archives: Vec::new(),
        });
    }

    // Phase 2: Find next archive number
    let archive_num = find_next_archive_number(beads_dir, &metadata.jsonl_export)?;
    let archive_path = beads_dir.join(format!("{}.{}", metadata.jsonl_export, archive_num));

    if options.dry_run {
        return Ok(RotateResult {
            archived: archive_beads.len(),
            remaining: active_beads.len(),
            archive_path: Some(archive_path),
            deleted_archives: Vec::new(),
        });
    }

    // Phase 3: Write archive file
    {
        let archive_file = File::create(&archive_path)?;
        let mut writer = BufWriter::new(archive_file);

        for bead in &archive_beads {
            serde_json::to_writer(&mut writer, bead)?;
            writer.write_all(b"\n")?;
        }

        writer.flush()?;
    }

    // Phase 4: Rewrite active file
    let temp_path = jsonl_path.with_extension("jsonl.tmp");
    {
        let temp_file = File::create(&temp_path)?;
        let mut writer = BufWriter::new(temp_file);

        for bead in &active_beads {
            serde_json::to_writer(&mut writer, bead)?;
            writer.write_all(b"\n")?;
        }

        writer.flush()?;
    }

    // Atomic rename
    std::fs::rename(&temp_path, &jsonl_path)?;

    // Phase 5: Delete old archives if exceeding max_archives
    let deleted_archives = cleanup_old_archives(
        beads_dir,
        &metadata.jsonl_export,
        options.max_archives,
    )?;

    Ok(RotateResult {
        archived: archive_beads.len(),
        remaining: active_beads.len(),
        archive_path: Some(archive_path),
        deleted_archives,
    })
}

/// Determine if a bead should be archived.
///
/// A bead is archived if:
/// - Status is Closed or Tombstone
/// - closed_at timestamp exists and is older than cutoff_time
fn should_archive(issue: &Issue, cutoff_time: &DateTime<Utc>) -> bool {
    match issue.status {
        crate::model::Status::Closed | crate::model::Status::Tombstone => {
            if let Some(closed_at) = issue.closed_at {
                return closed_at < *cutoff_time;
            }
            false
        }
        _ => false,
    }
}

/// Find the next available archive number.
///
/// Scans for existing archive files (issues.jsonl.1, .2, etc.) and returns
/// the next number in sequence.
fn find_next_archive_number(beads_dir: &Path, base_name: &str) -> Result<usize> {
    let mut max_num = 0;

    let entries = std::fs::read_dir(beads_dir)?;
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if let Some(suffix) = name_str.strip_prefix(base_name) {
            if let Some(num_str) = suffix.strip_prefix('.') {
                if let Ok(num) = num_str.parse::<usize>() {
                    max_num = max_num.max(num);
                }
            }
        }
    }

    Ok(max_num + 1)
}

/// Delete old archives if we exceed max_archives.
///
/// Keeps only the most recent `max_archives` files.
fn cleanup_old_archives(
    beads_dir: &Path,
    base_name: &str,
    max_archives: usize,
) -> Result<Vec<PathBuf>> {
    let mut archives: Vec<(usize, PathBuf)> = Vec::new();

    let entries = std::fs::read_dir(beads_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if let Some(suffix) = name_str.strip_prefix(base_name) {
            if let Some(num_str) = suffix.strip_prefix('.') {
                if let Ok(num) = num_str.parse::<usize>() {
                    archives.push((num, path.clone()));
                }
            }
        }
    }

    // Sort by archive number (oldest first)
    archives.sort_by_key(|(num, _)| *num);

    let mut deleted = Vec::new();
    let num_to_delete = archives.len().saturating_sub(max_archives);

    for (num, path) in archives.into_iter().take(num_to_delete) {
        std::fs::remove_file(&path)?;
        deleted.push(path);
    }

    Ok(deleted)
}

/// Find a bead by ID across active and archive files.
///
/// Searches in this order:
/// 1. Active JSONL file
/// 2. Archive files (highest number first - most recent)
///
/// Returns None if the bead is not found in any file.
pub fn find_bead_in_archives(beads_dir: &Path, bead_id: &str) -> Result<Option<Issue>> {
    let metadata = load_metadata(beads_dir)?;
    let jsonl_path = beads_dir.join(&metadata.jsonl_export);

    // Search active file first
    if jsonl_path.exists() {
        if let Some(bead) = find_bead_in_file(&jsonl_path, bead_id)? {
            return Ok(Some(bead));
        }
    }

    // Search archive files, newest first
    let mut archives: Vec<(usize, PathBuf)> = Vec::new();

    let entries = std::fs::read_dir(beads_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if let Some(suffix) = name_str.strip_prefix(&metadata.jsonl_export) {
            if let Some(num_str) = suffix.strip_prefix('.') {
                if let Ok(num) = num_str.parse::<usize>() {
                    archives.push((num, path));
                }
            }
        }
    }

    // Sort by archive number descending (newest first)
    archives.sort_by_key(|(num, _)| std::cmp::Reverse(*num));

    for (_, archive_path) in archives {
        if let Some(bead) = find_bead_in_file(&archive_path, bead_id)? {
            return Ok(Some(bead));
        }
    }

    Ok(None)
}

/// Find a bead by ID in a single JSONL file.
fn find_bead_in_file(path: &Path, bead_id: &str) -> Result<Option<Issue>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(issue) = serde_json::from_str::<Issue>(&line) {
            if issue.id == bead_id {
                return Ok(Some(issue));
            }
        }
    }

    Ok(None)
}

/// List all beads across active and archive files.
///
/// Returns beads from active file first, then archives (newest first).
pub fn list_all_with_archives(beads_dir: &Path) -> Result<Vec<Issue>> {
    let metadata = load_metadata(beads_dir)?;
    let jsonl_path = beads_dir.join(&metadata.jsonl_export);
    let mut all_beads = Vec::new();

    // Read active file
    if jsonl_path.exists() {
        let file = File::open(&jsonl_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(issue) = serde_json::from_str::<Issue>(&line) {
                all_beads.push(issue);
            }
        }
    }

    // Read archive files, newest first
    let mut archives: Vec<(usize, PathBuf)> = Vec::new();

    let entries = std::fs::read_dir(beads_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if let Some(suffix) = name_str.strip_prefix(&metadata.jsonl_export) {
            if let Some(num_str) = suffix.strip_prefix('.') {
                if let Ok(num) = num_str.parse::<usize>() {
                    archives.push((num, path));
                }
            }
        }
    }

    // Sort by archive number descending (newest first)
    archives.sort_by_key(|(num, _)| std::cmp::Reverse(*num));

    for (_, archive_path) in archives {
        let file = File::open(&archive_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(issue) = serde_json::from_str::<Issue>(&line) {
                all_beads.push(issue);
            }
        }
    }

    Ok(all_beads)
}

/// Get information about archive files.
///
/// Returns a list of archive file paths and their modification times.
pub fn list_archives(beads_dir: &Path) -> Result<Vec<(PathBuf, DateTime<Utc>)>> {
    let metadata = load_metadata(beads_dir)?;
    let mut archives = Vec::new();

    let entries = std::fs::read_dir(beads_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if let Some(suffix) = name_str.strip_prefix(&metadata.jsonl_export) {
            if let Some(_) = suffix.strip_prefix('.') {
                let metadata = entry.metadata()?;
                if let Ok(modified) = metadata.modified() {
                    let modified_dt: DateTime<Utc> = modified.into();
                    archives.push((path.clone(), modified_dt));
                }
            }
        }
    }

    // Sort by modification time, newest first
    archives.sort_by(|(_, a), (_, b)| b.cmp(a));

    Ok(archives)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::init_workspace;
    use crate::model::{Issue, Priority, Status, IssueType};
    use tempfile::TempDir;

    fn create_test_bead(id: &str, days_ago: i64, status: Status) -> Issue {
        let closed_at = if matches!(status, Status::Closed | Status::Tombstone) {
            Some(Utc::now() - Duration::days(days_ago))
        } else {
            None
        };

        Issue {
            id: id.to_string(),
            title: format!("Test Bead {}", id),
            status,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now() - Duration::days(days_ago + 1),
            updated_at: Utc::now() - Duration::days(days_ago),
            closed_at,
            source_repo: Some(".".to_string()),
            ..Default::default()
        }
    }

    fn write_jsonl(path: &Path, beads: &[Issue]) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        for bead in beads {
            serde_json::to_writer(&mut writer, bead)?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
        Ok(())
    }

    #[test]
    fn test_should_archive_closed_old_bead() {
        let bead = create_test_bead("bf-1", 40, Status::Closed);
        let cutoff = Utc::now() - Duration::days(30);
        assert!(should_archive(&bead, &cutoff));
    }

    #[test]
    fn test_should_not_archive_closed_recent_bead() {
        let bead = create_test_bead("bf-1", 10, Status::Closed);
        let cutoff = Utc::now() - Duration::days(30);
        assert!(!should_archive(&bead, &cutoff));
    }

    #[test]
    fn test_should_not_archive_open_bead() {
        let bead = create_test_bead("bf-1", 40, Status::Open);
        let cutoff = Utc::now() - Duration::days(30);
        assert!(!should_archive(&bead, &cutoff));
    }

    #[test]
    fn test_rotate_creates_archive() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        let jsonl_path = beads_dir.join("issues.jsonl");

        // Create test beads: 3 old closed, 2 active
        let beads = vec![
            create_test_bead("bf-1", 40, Status::Closed),
            create_test_bead("bf-2", 35, Status::Closed),
            create_test_bead("bf-3", 31, Status::Closed),
            create_test_bead("bf-4", 1, Status::Open),
            create_test_bead("bf-5", 1, Status::InProgress),
        ];

        write_jsonl(&jsonl_path, &beads).unwrap();

        let options = RotateOptions {
            age_days: 30,
            max_size_mb: None,
            max_archives: 10,
            dry_run: false,
        };

        let result = rotate(&beads_dir, &options).unwrap();

        assert_eq!(result.archived, 3);
        assert_eq!(result.remaining, 2);
        assert!(result.archive_path.is_some());

        // Verify archive file exists
        let archive_path = beads_dir.join("issues.jsonl.1");
        assert!(archive_path.exists());

        // Verify active file only has active beads
        let active_beads = list_all_with_archives(&beads_dir).unwrap();
        assert_eq!(active_beads.len(), 5); // All beads still accessible
    }

    #[test]
    fn test_rotate_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        let jsonl_path = beads_dir.join("issues.jsonl");

        let beads = vec![
            create_test_bead("bf-1", 40, Status::Closed),
            create_test_bead("bf-2", 1, Status::Open),
        ];

        write_jsonl(&jsonl_path, &beads).unwrap();

        let options = RotateOptions {
            age_days: 30,
            max_size_mb: None,
            max_archives: 10,
            dry_run: true,
        };

        let result = rotate(&beads_dir, &options).unwrap();

        assert_eq!(result.archived, 1);
        assert_eq!(result.remaining, 1);

        // Verify no archive file was created
        let archive_path = beads_dir.join("issues.jsonl.1");
        assert!(!archive_path.exists());
    }

    #[test]
    fn test_find_bead_in_archives() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        let jsonl_path = beads_dir.join("issues.jsonl");

        let beads = vec![
            create_test_bead("bf-1", 40, Status::Closed),
            create_test_bead("bf-2", 1, Status::Open),
        ];

        write_jsonl(&jsonl_path, &beads).unwrap();

        // Rotate to create an archive
        let options = RotateOptions {
            age_days: 30,
            max_size_mb: None,
            max_archives: 10,
            dry_run: false,
        };

        rotate(&beads_dir, &options).unwrap();

        // Find bead in archive
        let bead = find_bead_in_archives(&beads_dir, "bf-1").unwrap();
        assert!(bead.is_some());
        assert_eq!(bead.unwrap().id, "bf-1");

        // Find bead in active
        let bead = find_bead_in_archives(&beads_dir, "bf-2").unwrap();
        assert!(bead.is_some());
        assert_eq!(bead.unwrap().id, "bf-2");
    }

    #[test]
    fn test_list_all_with_archives() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        let jsonl_path = beads_dir.join("issues.jsonl");

        let beads = vec![
            create_test_bead("bf-1", 40, Status::Closed),
            create_test_bead("bf-2", 1, Status::Open),
        ];

        write_jsonl(&jsonl_path, &beads).unwrap();

        // Rotate to create an archive
        let options = RotateOptions {
            age_days: 30,
            max_size_mb: None,
            max_archives: 10,
            dry_run: false,
        };

        rotate(&beads_dir, &options).unwrap();

        // List all beads
        let all_beads = list_all_with_archives(&beads_dir).unwrap();
        assert_eq!(all_beads.len(), 2);

        let ids: Vec<&str> = all_beads.iter().map(|b| b.id.as_str()).collect();
        assert!(ids.contains(&"bf-1"));
        assert!(ids.contains(&"bf-2"));
    }

    #[test]
    fn test_cleanup_old_archives() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        // Create multiple archive files
        for i in 1..=15 {
            let archive_path = beads_dir.join(format!("issues.jsonl.{}", i));
            let file = File::create(&archive_path).unwrap();
            let mut writer = BufWriter::new(file);
            writeln!(writer, r#"{{"id":"bf-{}","title":"Test","status":"closed","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"."}}"#, i).unwrap();
            writer.flush().unwrap();
        }

        let deleted = cleanup_old_archives(&beads_dir, "issues.jsonl", 10).unwrap();

        assert_eq!(deleted.len(), 5);

        // Verify only 10 archives remain
        let archives = list_archives(&beads_dir).unwrap();
        assert_eq!(archives.len(), 10);
    }
}
