use crate::secrets::SecretProtectionConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_issue_prefixes")]
    pub issue_prefixes: Vec<String>,
    #[serde(default = "default_default_priority")]
    pub default_priority: i32,
    #[serde(default = "default_default_type")]
    pub default_type: String,
    #[serde(default)]
    pub scoring: ScoringConfig,
    #[serde(default)]
    pub claim_ttl_minutes: i64,
    #[serde(default)]
    pub rotate: RotateConfig,
    #[serde(default)]
    pub secret_protection: SecretProtectionConfig,
    #[serde(default)]
    pub checkpoint: CheckpointConfig,
    #[serde(default)]
    pub history: HistoryConfig,
    #[serde(default)]
    pub sync: SyncConfig,
}

/// Automatic SQLite → JSONL flush behavior (Phase 7.1).
///
/// When `auto_flush` is enabled, mutating commands flush the dirty beads to
/// `issues.jsonl` right after the mutation so the on-disk artifact never lags
/// the database. It is the master switch resolved against the per-invocation
/// `--no-auto-flush` CLI override (see `crate::autoflush::enabled`). Enabled by
/// default so a fresh workspace keeps JSONL current without extra flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Master switch. `true` (default) flushes dirty beads to JSONL after each
    /// mutation; set `false` to leave flushing to explicit `bf sync`/checkpoint.
    #[serde(default = "default_auto_flush")]
    pub auto_flush: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        SyncConfig {
            auto_flush: default_auto_flush(),
        }
    }
}

/// Pre-export JSONL history backups (Phase 7.9).
///
/// Before every full flush overwrites `issues.jsonl`, the previous version is
/// copied into `.bf_history/` as one more recovery layer under the artifact.
/// Local-only insurance: `.bf_history/` is git-ignored. Enabled by default and
/// bounded to `max_backups` snapshots so it can never grow without limit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    /// Master switch. `true` (default) backs up `issues.jsonl` before each full
    /// flush; set `false` to disable snapshotting entirely.
    #[serde(default = "default_history_enabled")]
    pub enabled: bool,
    /// Maximum number of snapshots to retain in `.bf_history/`. The oldest are
    /// pruned once this cap is exceeded. `0` disables pruning (unbounded).
    #[serde(default = "default_history_max_backups")]
    pub max_backups: usize,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        HistoryConfig {
            enabled: default_history_enabled(),
            max_backups: default_history_max_backups(),
        }
    }
}

/// Periodic git checkpointing of `.beads/` state (ADR-1).
///
/// Out-of-band only: the timer-driven `bf-checkpoint.sh` flushes SQLite → JSONL
/// (`bf sync --flush-only`) and commits `.beads/issues.jsonl` to git. This is
/// never invoked from the claim/close hot path — it runs solely on the systemd
/// timer. Defaults are opt-out safe: `enabled` and `push` are both `false`, so
/// deploying the timer does nothing until a workspace maintainer opts in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointConfig {
    /// Master switch. `false` (default) makes `bf-checkpoint.sh` a no-op even
    /// when the timer is deployed — new rollouts stay inert until a maintainer
    /// enables checkpointing for that workspace.
    #[serde(default)]
    pub enabled: bool,
    /// Minimum minutes between commits. The script self-throttles to this gap
    /// regardless of how often the timer fires, so `interval_minutes` is the
    /// source of truth for cadence (default 60).
    #[serde(default = "default_checkpoint_interval_minutes")]
    pub interval_minutes: u64,
    /// Persistently opt into `git push` after each commit. Off by default; the
    /// `--push` flag to `bf-checkpoint.sh` enables push for a single run.
    #[serde(default)]
    pub push: bool,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        CheckpointConfig {
            enabled: false,
            interval_minutes: default_checkpoint_interval_minutes(),
            push: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotateConfig {
    /// Days threshold for rotating closed beads (default 30)
    #[serde(default = "default_rotate_age_days")]
    pub rotate_age_days: u64,
    /// Maximum size of active JSONL in MB before rotation is considered (default 100)
    #[serde(default = "default_rotate_max_size_mb")]
    pub rotate_max_size_mb: u64,
    /// Maximum number of archive files to keep (default 10)
    #[serde(default = "default_rotate_max_archives")]
    pub rotate_max_archives: usize,
}

impl Default for RotateConfig {
    fn default() -> Self {
        RotateConfig {
            rotate_age_days: default_rotate_age_days(),
            rotate_max_size_mb: default_rotate_max_size_mb(),
            rotate_max_archives: default_rotate_max_archives(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringConfig {
    #[serde(default = "default_priority_weight")]
    pub priority_weight: f64,
    #[serde(default = "default_blockers_weight")]
    pub blockers_weight: f64,
    #[serde(default = "default_age_weight")]
    pub age_weight: f64,
    #[serde(default = "default_labels_weight")]
    pub labels_weight: f64,
    #[serde(default = "default_max_age_hours")]
    pub max_age_hours: i64,
    #[serde(default = "default_max_blockers")]
    pub max_blockers: i32,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        ScoringConfig {
            priority_weight: default_priority_weight(),
            blockers_weight: default_blockers_weight(),
            age_weight: default_age_weight(),
            labels_weight: default_labels_weight(),
            max_age_hours: default_max_age_hours(),
            max_blockers: default_max_blockers(),
        }
    }
}

fn default_issue_prefixes() -> Vec<String> {
    vec!["bf".to_string()]
}

fn default_default_priority() -> i32 {
    2
}

fn default_default_type() -> String {
    "task".to_string()
}

fn default_priority_weight() -> f64 {
    0.4
}

fn default_blockers_weight() -> f64 {
    0.3
}

fn default_age_weight() -> f64 {
    0.2
}

fn default_labels_weight() -> f64 {
    0.1
}

fn default_max_age_hours() -> i64 {
    20
}

fn default_max_blockers() -> i32 {
    3
}

fn default_rotate_age_days() -> u64 {
    30
}

fn default_rotate_max_size_mb() -> u64 {
    100
}

fn default_rotate_max_archives() -> usize {
    10
}

fn default_checkpoint_interval_minutes() -> u64 {
    60
}

fn default_history_enabled() -> bool {
    true
}

fn default_history_max_backups() -> usize {
    20
}

fn default_auto_flush() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Config {
            issue_prefixes: default_issue_prefixes(),
            default_priority: default_default_priority(),
            default_type: default_default_type(),
            scoring: ScoringConfig::default(),
            claim_ttl_minutes: 30,
            rotate: RotateConfig::default(),
            secret_protection: SecretProtectionConfig::default(),
            checkpoint: CheckpointConfig::default(),
            history: HistoryConfig::default(),
            sync: SyncConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub database: String,
    #[serde(rename = "jsonl_export")]
    pub jsonl_export: String,
}

impl Default for Metadata {
    fn default() -> Self {
        Metadata {
            database: "beads.db".to_string(),
            jsonl_export: "issues.jsonl".to_string(),
        }
    }
}

pub fn find_beads_dir(start_dir: &Path) -> Option<PathBuf> {
    let mut current = Some(start_dir);
    while let Some(dir) = current {
        let beads_dir = dir.join(".beads");
        if beads_dir.is_dir() {
            return Some(beads_dir);
        }
        current = dir.parent();
    }
    None
}

pub fn load_config(beads_dir: &Path) -> Result<Config> {
    let config_path = beads_dir.join("config.yaml");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    } else {
        Ok(Config::default())
    }
}

pub fn load_metadata(beads_dir: &Path) -> Result<Metadata> {
    let metadata_path = beads_dir.join("metadata.json");
    if metadata_path.exists() {
        let content = std::fs::read_to_string(&metadata_path)?;
        let metadata: Metadata = serde_json::from_str(&content)?;
        Ok(metadata)
    } else {
        Ok(Metadata::default())
    }
}

pub fn save_config(beads_dir: &Path, config: &Config) -> Result<()> {
    let config_path = beads_dir.join("config.yaml");
    let config_yaml = serde_yaml::to_string(config)?;
    std::fs::write(&config_path, config_yaml)?;
    Ok(())
}

pub fn get_default_prefix(config: &Config) -> &str {
    config
        .issue_prefixes
        .first()
        .map(|s| s.as_str())
        .unwrap_or("bf")
}

/// Initialize a new workspace directory with default config and metadata.
///
/// Creates the .beads directory with default configuration files.
/// Used primarily for testing.
pub fn init_workspace(beads_dir: &Path, prefix: &str) -> Result<()> {
    std::fs::create_dir_all(beads_dir)?;

    // Write default config.yaml
    let config = Config {
        issue_prefixes: vec![prefix.to_string()],
        ..Default::default()
    };
    let config_yaml = serde_yaml::to_string(&config)?;
    std::fs::write(beads_dir.join("config.yaml"), config_yaml)?;

    // Write default metadata.json
    let metadata = Metadata::default();
    let metadata_json = serde_json::to_string_pretty(&metadata)?;
    std::fs::write(beads_dir.join("metadata.json"), metadata_json)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_config_defaults() {
        let cfg = CheckpointConfig::default();
        assert!(!cfg.enabled, "enabled must default to false");
        assert_eq!(
            cfg.interval_minutes, 60,
            "interval_minutes must default to 60"
        );
        assert!(!cfg.push, "push must default to false");
    }

    #[test]
    fn test_checkpoint_config_via_config_default() {
        // The top-level Config wires `checkpoint` in with `#[serde(default)]`,
        // so `Config::default()` carries the same opt-out-safe defaults that
        // `bf config get checkpoint.*` reports in an uninitialized workspace.
        let cfg = Config::default();
        assert!(!cfg.checkpoint.enabled);
        assert_eq!(cfg.checkpoint.interval_minutes, 60);
        assert!(!cfg.checkpoint.push);
    }

    #[test]
    fn test_checkpoint_config_parses_populated_block() {
        let yaml = "\
checkpoint:
  enabled: true
  interval_minutes: 15
  push: true
";
        let cfg: Config =
            serde_yaml::from_str(yaml).expect("populated checkpoint block must parse");
        assert!(cfg.checkpoint.enabled);
        assert_eq!(cfg.checkpoint.interval_minutes, 15);
        assert!(cfg.checkpoint.push);
    }

    #[test]
    fn test_checkpoint_config_omitted_block_uses_defaults() {
        // A config.yaml with no `checkpoint:` block must still deserialize and
        // report the safe defaults — this is what `bf config get checkpoint.*`
        // relies on in workspaces that haven't opted in.
        let yaml = "issue_prefixes:\n- bf\n";
        let cfg: Config =
            serde_yaml::from_str(yaml).expect("config without checkpoint block must parse");
        assert!(!cfg.checkpoint.enabled);
        assert_eq!(cfg.checkpoint.interval_minutes, 60);
        assert!(!cfg.checkpoint.push);
    }

    #[test]
    fn test_sync_config_default_auto_flush_true() {
        assert!(
            SyncConfig::default().auto_flush,
            "auto_flush must default to true"
        );
        assert!(
            Config::default().sync.auto_flush,
            "Config::default() must carry auto_flush=true"
        );
    }

    #[test]
    fn test_sync_config_omitted_block_uses_defaults() {
        // A config.yaml with no `sync:` block must still deserialize and report
        // auto_flush=true, matching the compiled default.
        let yaml = "issue_prefixes:\n- bf\n";
        let cfg: Config = serde_yaml::from_str(yaml).expect("config without sync block must parse");
        assert!(cfg.sync.auto_flush);
    }

    #[test]
    fn test_sync_config_parses_auto_flush_false() {
        let yaml = "sync:\n  auto_flush: false\n";
        let cfg: Config = serde_yaml::from_str(yaml).expect("populated sync block must parse");
        assert!(!cfg.sync.auto_flush, "auto_flush: false must disable");
    }

    #[test]
    fn test_checkpoint_config_partial_block_uses_defaults() {
        // Partial block: only `enabled: true`. The other two fields must fall
        // back to their serde defaults rather than zero values.
        let yaml = "checkpoint:\n  enabled: true\n";
        let cfg: Config = serde_yaml::from_str(yaml).expect("partial checkpoint block must parse");
        assert!(cfg.checkpoint.enabled);
        assert_eq!(cfg.checkpoint.interval_minutes, 60);
        assert!(!cfg.checkpoint.push);
    }
}
