//! Auto-flush plumbing (Phase 7.1, child 1/5).
//!
//! This is the PLUMBING layer that later Phase 7.1 children consume. It does
//! not change any command's behavior on its own — it only provides:
//!
//! * [`enabled`] — the effective-enablement resolution
//!   (`config.sync.auto_flush && !--no-auto-flush`).
//! * [`after_mutation`] / [`after_mutation_with_config`] — run the dirty flush
//!   and report the outcome so callers can bridge a failure into a user-facing
//!   warning (see [`crate::format::with_warning`] / [`crate::format::warn_stderr`]).
//!
//! The actual SQLite → JSONL export reuses [`crate::sync::flush_dirty`], so
//! auto-flush and the explicit `bf sync --flush-only` path share one code path.

use crate::config::Config;
use crate::sync::flush_dirty;
use std::path::Path;

/// Resolve whether auto-flush is effectively enabled for this invocation.
///
/// Two switches gate it: the persistent `sync.auto_flush` config master switch
/// (default `true`) AND the absence of the per-invocation `--no-auto-flush`
/// CLI override. Either one being off disables auto-flush.
pub fn enabled(config: &Config, no_auto_flush_flag: bool) -> bool {
    config.sync.auto_flush && !no_auto_flush_flag
}

/// Outcome of an auto-flush attempt, ready to bridge into the warning channel.
///
/// Only [`FlushOutcome::Failed`] surfaces a warning — a disabled or successful
/// flush is silent (`warning()` returns `None`).
#[derive(Debug)]
pub enum FlushOutcome {
    /// Auto-flush was disabled (config off or `--no-auto-flush`); nothing ran.
    Disabled,
    /// Flush ran successfully; carries the number of beads written to JSONL.
    Flushed(usize),
    /// Flush was attempted but failed; carries the human-readable warning.
    Failed(String),
}

impl FlushOutcome {
    /// The warning string to surface (JSON `warning` key / stderr), if any.
    ///
    /// `Disabled` and `Flushed` are silent; only `Failed` yields `Some`.
    pub fn warning(&self) -> Option<&str> {
        match self {
            FlushOutcome::Failed(msg) => Some(msg.as_str()),
            FlushOutcome::Disabled | FlushOutcome::Flushed(_) => None,
        }
    }

    /// Number of beads flushed, or `0` when disabled/failed.
    pub fn flushed_count(&self) -> usize {
        match self {
            FlushOutcome::Flushed(n) => *n,
            FlushOutcome::Disabled | FlushOutcome::Failed(_) => 0,
        }
    }

    /// Whether the flush was attempted and failed.
    pub fn is_failure(&self) -> bool {
        matches!(self, FlushOutcome::Failed(_))
    }
}

/// Perform the dirty-only flush of `workspace_dir`'s beads to JSONL.
///
/// Thin wrapper over [`crate::sync::flush_dirty`]; returns the number of beads
/// written (`0` when nothing was dirty).
pub fn run(workspace_dir: &Path) -> anyhow::Result<usize> {
    flush_dirty(workspace_dir)
}

/// Run auto-flush after a mutation, honoring a pre-resolved `enabled` decision,
/// and return an outcome whose [`FlushOutcome::warning`] bridges into the
/// JSON/stderr warning channel.
///
/// A failed flush is intentionally non-fatal: the mutation already succeeded,
/// so the caller degrades to a warning rather than an error.
pub fn after_mutation(workspace_dir: &Path, enabled: bool) -> FlushOutcome {
    if !enabled {
        return FlushOutcome::Disabled;
    }
    match run(workspace_dir) {
        Ok(count) => FlushOutcome::Flushed(count),
        Err(e) => FlushOutcome::Failed(format!("auto-flush to JSONL failed: {e}")),
    }
}

/// Convenience wrapper that resolves enablement from `config` + the CLI flag
/// via [`enabled`] before delegating to [`after_mutation`].
pub fn after_mutation_with_config(
    workspace_dir: &Path,
    config: &Config,
    no_auto_flush_flag: bool,
) -> FlushOutcome {
    after_mutation(workspace_dir, enabled(config, no_auto_flush_flag))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{init_workspace, Config};
    use crate::storage::Storage;
    use tempfile::TempDir;

    fn config_with_auto_flush(value: bool) -> Config {
        let mut cfg = Config::default();
        cfg.sync.auto_flush = value;
        cfg
    }

    #[test]
    fn enabled_true_only_when_config_on_and_flag_off() {
        assert!(enabled(&config_with_auto_flush(true), false));
        // CLI override wins over an enabled config.
        assert!(!enabled(&config_with_auto_flush(true), true));
        // Config off disables regardless of the flag.
        assert!(!enabled(&config_with_auto_flush(false), false));
        assert!(!enabled(&config_with_auto_flush(false), true));
    }

    #[test]
    fn disabled_outcome_when_not_enabled() {
        let tmp = TempDir::new().unwrap();
        let outcome = after_mutation(tmp.path(), false);
        assert!(matches!(outcome, FlushOutcome::Disabled));
        assert_eq!(outcome.warning(), None, "disabled flush must be silent");
        assert!(!outcome.is_failure());
    }

    #[test]
    fn success_yields_no_warning() {
        let tmp = TempDir::new().unwrap();
        let beads_dir = tmp.path().join(".beads");
        init_workspace(&beads_dir, "bf").unwrap();
        let storage = Storage::open(&beads_dir.join("beads.db")).unwrap();

        // A mutation marks the bead dirty, so the flush has something to write.
        let issue = crate::model::Issue::new("bf-1".into(), "T".into(), ".".into());
        storage.create_issue(&issue).unwrap();
        storage.mark_dirty("bf-1").unwrap();

        let outcome = after_mutation(tmp.path(), true);
        assert!(
            matches!(outcome, FlushOutcome::Flushed(1)),
            "expected Flushed(1), got {outcome:?}"
        );
        assert_eq!(outcome.warning(), None, "successful flush must be silent");
        assert!(tmp.path().join(".beads/issues.jsonl").exists());
    }

    #[test]
    fn forced_failure_yields_populated_warning() {
        // Real workspace with a dirty bead, but the JSONL export target is a
        // directory — the atomic temp+rename can't overwrite it, so the flush
        // fails and the outcome carries a warning.
        let tmp = TempDir::new().unwrap();
        let beads_dir = tmp.path().join(".beads");
        init_workspace(&beads_dir, "bf").unwrap();
        let storage = Storage::open(&beads_dir.join("beads.db")).unwrap();
        let issue = crate::model::Issue::new("bf-1".into(), "T".into(), ".".into());
        storage.create_issue(&issue).unwrap();
        storage.mark_dirty("bf-1").unwrap();
        // Wedge the flush: make issues.jsonl an (undeletable-by-rename) directory.
        std::fs::create_dir(beads_dir.join("issues.jsonl")).unwrap();

        let outcome = after_mutation(tmp.path(), true);
        assert!(outcome.is_failure(), "expected failure, got {outcome:?}");
        let warning = outcome.warning().expect("failure must populate a warning");
        assert!(
            warning.contains("auto-flush to JSONL failed"),
            "warning must describe the auto-flush failure, got: {warning}"
        );
    }

    #[test]
    fn with_config_resolves_before_running() {
        // Config disables → treated as Disabled even though the flag is off and
        // a real workspace exists.
        let tmp = TempDir::new().unwrap();
        let beads_dir = tmp.path().join(".beads");
        init_workspace(&beads_dir, "bf").unwrap();
        let _ = Storage::open(&beads_dir.join("beads.db")).unwrap();

        let outcome = after_mutation_with_config(tmp.path(), &config_with_auto_flush(false), false);
        assert!(matches!(outcome, FlushOutcome::Disabled));
        assert_eq!(outcome.warning(), None);
    }
}
