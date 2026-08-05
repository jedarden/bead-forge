//! Pre-export JSONL history backups (Phase 7.9).
//!
//! One more insurance layer under `issues.jsonl`, cheap to keep. Every full
//! flush copies the *previous* `issues.jsonl` into `.bf_history/` before the
//! new one atomically replaces it, so a bad export (partial write, a merge that
//! resolved the wrong way, an accidental `doctor --repair --force` over
//! unflushed beads) is always recoverable from the last few snapshots.
//!
//! Backups are pruned to a bounded count so the directory can never grow
//! without limit. The whole feature is local-only insurance: `.bf_history/` is
//! git-ignored and never shared.

use crate::config::HistoryConfig;
use anyhow::{Context, Result};
use chrono::Utc;
use std::path::{Path, PathBuf};

/// Directory name (under `.beads/`) holding pre-export JSONL snapshots.
pub const HISTORY_DIR: &str = ".bf_history";

/// Filename prefix for snapshot files.
const SNAPSHOT_PREFIX: &str = "issues-";
const SNAPSHOT_SUFFIX: &str = ".jsonl";

/// Back up the current `jsonl_path` into `<beads_dir>/.bf_history/` then prune.
///
/// No-op when history is disabled, when `jsonl_path` does not yet exist (first
/// flush of a fresh workspace), or when the file is empty. Returns the path of
/// the snapshot written, if any. Backup failures are surfaced to the caller so
/// they can be logged, but callers generally treat this as best-effort — a
/// failed backup must never abort the flush it is protecting.
pub fn backup_before_export(
    beads_dir: &Path,
    jsonl_path: &Path,
    config: &HistoryConfig,
) -> Result<Option<PathBuf>> {
    if !config.enabled {
        return Ok(None);
    }
    let meta = match std::fs::metadata(jsonl_path) {
        Ok(m) => m,
        Err(_) => return Ok(None), // nothing to back up yet
    };
    if meta.len() == 0 {
        return Ok(None);
    }

    let history_dir = beads_dir.join(HISTORY_DIR);
    std::fs::create_dir_all(&history_dir)
        .with_context(|| format!("creating {}", history_dir.display()))?;

    // Timestamped, monotonic-ish name. A short nanosecond tail disambiguates
    // two flushes inside the same second so neither snapshot clobbers the other.
    let now = Utc::now();
    let stamp = now.format("%Y%m%dT%H%M%S");
    let tail = now.timestamp_subsec_nanos();
    let name = format!("{SNAPSHOT_PREFIX}{stamp}-{tail:09}{SNAPSHOT_SUFFIX}");
    let dest = history_dir.join(name);

    std::fs::copy(jsonl_path, &dest)
        .with_context(|| format!("copying {} -> {}", jsonl_path.display(), dest.display()))?;

    prune(&history_dir, config.max_backups)?;

    Ok(Some(dest))
}

/// List existing snapshot files, oldest first (sorted by filename, which is
/// timestamp-ordered).
pub fn list_snapshots(history_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut snapshots: Vec<PathBuf> = Vec::new();
    let entries = match std::fs::read_dir(history_dir) {
        Ok(e) => e,
        Err(_) => return Ok(snapshots),
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(SNAPSHOT_PREFIX) && name.ends_with(SNAPSHOT_SUFFIX) {
            snapshots.push(entry.path());
        }
    }
    snapshots.sort();
    Ok(snapshots)
}

/// Keep at most `max_backups` snapshots, deleting the oldest beyond that.
///
/// A `max_backups` of 0 disables pruning (unbounded); any positive value caps
/// the directory. Deletion errors on individual stale files are ignored — a
/// leftover snapshot is harmless, and we never want prune to fail a flush.
fn prune(history_dir: &Path, max_backups: usize) -> Result<()> {
    if max_backups == 0 {
        return Ok(());
    }
    let snapshots = list_snapshots(history_dir)?;
    if snapshots.len() <= max_backups {
        return Ok(());
    }
    let excess = snapshots.len() - max_backups;
    for stale in snapshots.iter().take(excess) {
        let _ = std::fs::remove_file(stale);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HistoryConfig;
    use std::io::Write;

    fn write_jsonl(path: &Path, contents: &str) {
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
    }

    #[test]
    fn disabled_config_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let beads_dir = dir.path();
        let jsonl = beads_dir.join("issues.jsonl");
        write_jsonl(&jsonl, "{\"id\":\"bf-1\"}\n");

        let cfg = HistoryConfig {
            enabled: false,
            max_backups: 5,
        };
        let result = backup_before_export(beads_dir, &jsonl, &cfg).unwrap();
        assert!(result.is_none());
        assert!(!beads_dir.join(HISTORY_DIR).exists());
    }

    #[test]
    fn missing_source_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let beads_dir = dir.path();
        let jsonl = beads_dir.join("issues.jsonl"); // does not exist
        let cfg = HistoryConfig::default();
        let result = backup_before_export(beads_dir, &jsonl, &cfg).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn empty_source_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let beads_dir = dir.path();
        let jsonl = beads_dir.join("issues.jsonl");
        write_jsonl(&jsonl, "");
        let cfg = HistoryConfig::default();
        let result = backup_before_export(beads_dir, &jsonl, &cfg).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn backup_copies_content() {
        let dir = tempfile::tempdir().unwrap();
        let beads_dir = dir.path();
        let jsonl = beads_dir.join("issues.jsonl");
        let payload = "{\"id\":\"bf-1\",\"title\":\"x\"}\n";
        write_jsonl(&jsonl, payload);

        let cfg = HistoryConfig::default();
        let snap = backup_before_export(beads_dir, &jsonl, &cfg)
            .unwrap()
            .expect("snapshot written");
        assert_eq!(std::fs::read_to_string(&snap).unwrap(), payload);
    }

    #[test]
    fn prune_keeps_only_max_backups() {
        let dir = tempfile::tempdir().unwrap();
        let history_dir = dir.path().join(HISTORY_DIR);
        std::fs::create_dir_all(&history_dir).unwrap();

        // Fabricate 10 snapshots with increasing names.
        for i in 0..10 {
            let name = format!("{SNAPSHOT_PREFIX}20260101T00000{i}-000000000{SNAPSHOT_SUFFIX}");
            write_jsonl(&history_dir.join(name), "x");
        }
        prune(&history_dir, 3).unwrap();

        let remaining = list_snapshots(&history_dir).unwrap();
        assert_eq!(remaining.len(), 3, "prune should cap to max_backups");
        // Newest three (highest names) survive.
        let names: Vec<String> = remaining
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names
            .iter()
            .all(|n| n.contains("000007") || n.contains("000008") || n.contains("000009")));
    }

    #[test]
    fn prune_zero_is_unbounded() {
        let dir = tempfile::tempdir().unwrap();
        let history_dir = dir.path().join(HISTORY_DIR);
        std::fs::create_dir_all(&history_dir).unwrap();
        for i in 0..5 {
            let name = format!("{SNAPSHOT_PREFIX}20260101T00000{i}-000000000{SNAPSHOT_SUFFIX}");
            write_jsonl(&history_dir.join(name), "x");
        }
        prune(&history_dir, 0).unwrap();
        assert_eq!(list_snapshots(&history_dir).unwrap().len(), 5);
    }

    #[test]
    fn successive_backups_accumulate_then_cap() {
        let dir = tempfile::tempdir().unwrap();
        let beads_dir = dir.path();
        let jsonl = beads_dir.join("issues.jsonl");
        let cfg = HistoryConfig {
            enabled: true,
            max_backups: 2,
        };
        // Each backup captures the file as it was before the "next export".
        for i in 0..5 {
            write_jsonl(&jsonl, &format!("{{\"id\":\"bf-{i}\"}}\n"));
            backup_before_export(beads_dir, &jsonl, &cfg).unwrap();
        }
        let remaining = list_snapshots(&beads_dir.join(HISTORY_DIR)).unwrap();
        assert!(remaining.len() <= 2, "never exceed max_backups");
    }
}
