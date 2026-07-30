use crate::batch::{execute_batch, mitosis_ex, parse_stdin, BatchOp, MitosisChild};
use crate::claim::{
    claim, claim_any, find_workspaces, get_ready_candidates, ClaimResult, WorkerMetadata,
};
use crate::close::close_bead;
use crate::commit_check::{format_scan_results, scan_staged_beads};
use crate::config::{find_beads_dir, get_default_prefix, load_config, load_metadata, Config};
use crate::critical_path::compute_epic_critical_path;
use crate::format::{get_formatter, ClaimResultOutput, OutputFormat, StatsOutput};
use crate::model::{Issue, IssueChanges, IssueFilter, IssueType, Priority, Status};
use serde_json::Value;
use crate::rotate::{find_bead_in_archives, list_all_with_archives, rotate, RotateOptions};
use crate::storage::Storage;
use crate::validation::normalize_assignee;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Version of bead-forge, read from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "bf")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(disable_version_flag = true)]
#[command(about = "bead-forge - Drop-in replacement for beads_rust (br)", long_about = None)]
pub struct Cli {
    /// Workspace directory (defaults to current directory's .beads/)
    #[arg(short, long, global = true)]
    pub workspace: Option<PathBuf>,

    /// Disable the automatic SQLite→JSONL flush after mutating commands for
    /// this invocation. Overrides `sync.auto_flush: true` in config; a no-op
    /// when auto-flush is already off. See `crate::autoflush::enabled`.
    #[arg(long, global = true)]
    pub no_auto_flush: bool,

    /// Wrap --json output in a standard envelope: {version, kind, data, warning?}.
    /// This provides stable structure for programmatic consumers. See `bf robot-docs`.
    #[arg(long, global = true)]
    pub envelope: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new bead
    ///
    /// Generates a unique short ID and prints it. Type defaults to "task" and
    /// priority to 2 (Normal); 0 is Critical, 4 is Backlog. Pass --label
    /// repeatedly to attach multiple labels.
    Create {
        /// Bead title
        #[arg(long)]
        title: String,

        /// Bead type
        #[arg(long, default_value = "task")]
        type_: String,

        /// Priority (0=Critical, 4=Backlog)
        #[arg(long, default_value = "2")]
        priority: i32,

        /// Description
        #[arg(long)]
        description: Option<String>,

        /// Assignee
        #[arg(long)]
        assignee: Option<String>,

        /// Labels
        #[arg(long)]
        label: Vec<String>,

        /// Output JSON ({"id": "..."} plus a "warning" key if auto-flush fails)
        #[arg(long)]
        json: bool,
    },

    /// List beads
    ///
    /// Lists beads in the workspace, optionally filtered by status, type,
    /// assignee, priority, or annotation (key=value). Use --all to also include
    /// beads that have been rotated to archive files. Output formats are text
    /// (default), json, and toon; --limit 0 means unlimited.
    List {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Filter by type
        #[arg(long)]
        type_: Option<String>,

        /// Filter by assignee
        #[arg(long)]
        assignee: Option<String>,

        /// Filter by priority
        #[arg(long)]
        priority: Option<i32>,

        /// Filter by annotation (key=value)
        #[arg(long)]
        annotation: Option<String>,

        /// Limit results (0 = unlimited)
        #[arg(long)]
        limit: Option<usize>,

        /// Include archived beads from archive files
        #[arg(long)]
        all: bool,

        /// Output format (text, json, toon)
        #[arg(long, default_value = "text")]
        format: String,

        /// Output JSON (alias for --format json)
        #[arg(long)]
        json: bool,
    },

    /// Show bead details
    ///
    /// Prints full details for a single bead: title, status, priority, type,
    /// description, assignee, labels, and dependencies. If the ID is not in the
    /// active database, archive files are searched as a fallback.
    Show {
        /// Bead ID
        id: String,

        /// Output format (text, json, toon)
        #[arg(long, default_value = "text")]
        format: String,

        /// Output JSON (alias for --format json)
        #[arg(long)]
        json: bool,
    },

    /// Update a bead
    ///
    /// Changes only the fields you pass. `--description` and
    /// `--acceptance-criteria` edit those fields directly (closing the old
    /// "add a comment instead" gap). For long/multiline descriptions pass
    /// `--description-file <path>` instead — it reads the file's contents and
    /// sets the description, and conflicts with `--description`.
    /// --due-at expects an RFC3339 timestamp (e.g. 2025-01-01T00:00:00Z).
    Update {
        /// Bead ID
        id: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New status
        #[arg(long)]
        status: Option<String>,

        /// New priority
        #[arg(long)]
        priority: Option<i32>,

        /// New assignee
        #[arg(long)]
        assignee: Option<String>,

        /// Clear the assignee (set to unassigned). Equivalent to --assignee ""
        /// but more discoverable; useful for freeing an open bead that still
        /// carries a stale assignee from a dead worker. Conflicts with
        /// --assignee.
        #[arg(long, conflicts_with = "assignee")]
        clear_assignee: bool,

        /// New description
        #[arg(long)]
        description: Option<String>,

        /// Read the new description from a file. Useful for long or multiline
        /// bodies that are awkward to pass on the shell. Conflicts with
        /// --description (which wins for short inline text).
        #[arg(long, conflicts_with = "description")]
        description_file: Option<PathBuf>,

        /// New acceptance criteria
        #[arg(long)]
        acceptance_criteria: Option<String>,

        /// New notes
        #[arg(long)]
        notes: Option<String>,

        /// New design
        #[arg(long)]
        design: Option<String>,

        /// New due date (RFC3339 format, e.g., 2025-01-01T00:00:00Z)
        #[arg(long)]
        due_at: Option<String>,
    },

    /// Close a bead
    ///
    /// Marks a bead as closed, recording a close event with the given reason
    /// (default "Completed") in the event log.
    Close {
        /// Bead ID
        id: String,

        /// Close reason
        #[arg(long, default_value = "Completed")]
        reason: String,
    },

    /// Reopen a bead
    ///
    /// Resets a closed bead back to open and clears any stale assignee left
    /// over from before it was closed.
    Reopen {
        /// Bead ID
        id: String,
    },

    /// Delete a bead
    ///
    /// Permanently removes a bead from the database. Unlike close, this is
    /// destructive and cannot be undone.
    Delete {
        /// Bead ID
        id: String,
    },

    /// Show ready (unblocked) beads
    ///
    /// Lists beads that are open and unblocked, ranked by downstream impact,
    /// priority, and age — the best candidates to claim next. --limit 0 means
    /// unlimited (default 10).
    Ready {
        /// Limit results (0 = unlimited)
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Output format (text, json, toon)
        #[arg(long, default_value = "text")]
        format: String,

        /// Output JSON (alias for --format json)
        #[arg(long)]
        json: bool,
    },

    /// Claim a bead (atomic)
    ///
    /// Atomically assigns an unblocked bead to a worker and sets it
    /// in_progress. The claim runs under BEGIN IMMEDIATE, so concurrent workers
    /// never claim the same bead. With --any, it searches all discoverable
    /// workspaces; --fallback any tries the current workspace first and only
    /// fans out if nothing is available. --dry-run previews without claiming.
    Claim {
        /// Assignee (worker ID)
        #[arg(long)]
        assignee: String,

        /// Model
        #[arg(long)]
        model: Option<String>,

        /// Harness
        #[arg(long)]
        harness: Option<String>,

        /// Harness version
        #[arg(long)]
        harness_version: Option<String>,

        /// Claim from any workspace (searches all .beads/ directories)
        #[arg(long)]
        any: bool,

        /// Fallback mode: try current workspace first, fall back to any if no beads available
        #[arg(long, value_name = "MODE")]
        fallback: Option<String>,

        /// Workspace paths to search (only used with --any)
        #[arg(long)]
        workspace_paths: Vec<PathBuf>,

        /// Dry run (show what would be claimed without claiming)
        #[arg(long)]
        dry_run: bool,

        /// Output format (text, json, toon)
        #[arg(long, default_value = "text")]
        format: String,

        /// Output JSON (alias for --format json)
        #[arg(long)]
        json: bool,
    },

    /// Initialize a new workspace
    ///
    /// Creates a .beads/ directory with a config.yaml, metadata.json, SQLite
    /// database, and .gitignore. The issue prefix seeds the short IDs on
    /// generated beads (default "bf").
    Init {
        /// Issue prefix
        #[arg(long, default_value = "bf")]
        prefix: String,
    },

    /// Sync (flush to JSONL or import from JSONL)
    ///
    /// With no flags, bidirectionally syncs between SQLite and issues.jsonl.
    /// --flush-only checkpoints the database out to JSONL; --import-only
    /// rebuilds the database from JSONL.
    Sync {
        /// Flush only (SQLite -> JSONL)
        #[arg(long)]
        flush_only: bool,

        /// Import only (JSONL -> SQLite)
        #[arg(long)]
        import_only: bool,
    },

    /// Doctor - check and repair
    ///
    /// Checks database integrity, JSONL validity, and drift between the two.
    /// --repair rebuilds SQLite from JSONL (flush unflushed beads first with
    /// --flush-first to avoid losing them). --reclaim-stale resets beads stuck
    /// in_progress past the claim TTL back to open. --reconcile backfills rows
    /// that predate a forward-only fix (stale blocked status, empty assignees).
    Doctor {
        /// Repair database
        #[arg(long)]
        repair: bool,

        /// Flush unflushed beads to JSONL before repair (protects against data loss)
        #[arg(long)]
        flush_first: bool,

        /// Force repair even with unflushed beads (WARNING: unflushed beads will be lost)
        #[arg(long)]
        force: bool,

        /// Reclaim stale in_progress beads (reset to open without claiming)
        #[arg(long)]
        reclaim_stale: bool,

        /// TTL in minutes for stale bead detection (overrides config claim_ttl_minutes)
        #[arg(long)]
        ttl: Option<i64>,

        /// Repair NULL values in NOT NULL columns in place (non-destructive; does
        /// not rebuild from JSONL). Fixes the NULL-datetime crash class.
        #[arg(long)]
        fix_schema: bool,

        /// Backfill rows that predate a forward-only fix (non-destructive, in place):
        /// flips beads stuck at 'blocked' whose blockers are all closed back to 'open',
        /// and rewrites empty-string assignees to NULL.
        #[arg(long)]
        reconcile: bool,

        /// Proceed with --repair even if a prior rebuild failed post-verification
        /// (clears the repeat-failure gate; see the doctor safety stack).
        #[arg(long)]
        allow_repeated_repair: bool,

        /// List verified pre-rebuild recovery runs (hash-checked DB backups).
        #[arg(long)]
        runs: bool,

        /// Restore the DB family from a recovery run: a run id or "latest".
        #[arg(long, value_name = "RUN_ID")]
        restore: Option<String>,
    },

    /// Three-way merge of JSONL bead files (usable as a git merge driver)
    ///
    /// Resolves divergent `issues.jsonl` files per-bead instead of per-line,
    /// so no bead is ever dropped or corrupted by a text merge. Configure as a
    /// git merge driver:
    ///
    ///   git config merge.beads.name "bead-forge 3-way JSONL merge"
    ///   git config merge.beads.driver "bf merge-jsonl --base %O --ours %A --theirs %B --output %A"
    ///   echo '.beads/issues.jsonl merge=beads' >> .gitattributes
    MergeJsonl {
        /// Common-ancestor snapshot (git %O). Defaults to `.beads/beads.base.jsonl`.
        #[arg(long)]
        base: Option<PathBuf>,

        /// Our version of the file (git %A).
        #[arg(long)]
        ours: PathBuf,

        /// Their version of the file (git %B).
        #[arg(long)]
        theirs: PathBuf,

        /// Where to write the merged result. Defaults to `--ours` (git driver
        /// convention: git reads the resolved artifact back from %A).
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Commit check - scan staged .beads/ changes for secrets (git pre-commit hook)
    ///
    /// Intended for use as a git pre-commit hook. Scans staged .beads/ changes
    /// for secrets and prints nothing on success (exit 0); if secrets are
    /// found it prints a report and exits 1.
    CommitCheck,

    /// Count beads
    ///
    /// Prints the number of beads. Pass --status to count only beads in a
    /// given status.
    Count {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,
    },

    /// Batch operations (atomic)
    ///
    /// Operations:
    ///
    /// 1. create: Create a new bead
    ///    {
    ///      "op": "create",
    ///      "title": "<string>",           // required
    ///      "type": "<string>",             // optional, default "task"
    ///      "priority": <int>,              // optional, default 2 (0=Critical, 4=Backlog)
    ///      "description": "<string>",      // optional
    ///      "assignee": "<string>",        // optional
    ///      "labels": ["<string>", ...]    // optional
    ///    }
    ///
    /// 2. dep_add_blocker: Add a blocking dependency
    ///    {
    ///      "op": "dep_add_blocker",
    ///      "id": "<string>",               // required: bead being blocked
    ///      "blocker": "<string>"          // required: bead that blocks id (must close before id)
    ///    }
    ///    Direction: blocker blocks id (blocker must close before id can close)
    ///    Aliases: parent -> blocker, child -> id (deprecated but supported)
    ///
    /// 3. close: Close a bead
    ///    {
    ///      "op": "close",
    ///      "id": "<string>",               // required: bead ID to close
    ///      "reason": "<string>"            // optional, default "Completed"
    ///    }
    ///
    /// Placeholder references: Use @0, @1, @2... to reference beads created earlier in the batch
    ///
    /// Examples:
    ///   echo '{"op":"create","title":"Fix bug"}' | bf batch --stdin
    ///   echo '[{"op":"dep_add_blocker","id":"bf-task","blocker":"bf-blocker"}]' | bf batch --stdin
    Batch {
        /// JSON file containing operations
        #[arg(long)]
        file: Option<PathBuf>,

        /// JSON string containing operations
        #[arg(long)]
        json: Option<String>,

        /// Read from stdin
        #[arg(long, default_value = "false")]
        stdin: bool,

        /// Output format (text, json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Mitosis: split a bead into children atomically
    ///
    /// Splits a parent bead into the children defined by --children (a JSON
    /// array of {title, type, priority, ...}), then closes the parent and wires
    /// the new children to block it — all within a single atomic transaction.
    Mitosis {
        /// Parent bead ID to split
        id: String,

        /// Child bead definitions (JSON array of {title, type, priority})
        #[arg(long)]
        children: String,

        /// Close reason for parent bead
        #[arg(long, default_value = "Split into children")]
        reason: String,

        /// Output format (text, json, toon)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Manage dependencies
    ///
    /// Subcommands to add, remove, list, or tree dependencies between beads.
    /// A "blocks" dependency marks the blocked bead as Blocked until its
    /// blocker closes.
    #[command(subcommand)]
    Dep(DepCommands),

    /// Manage labels
    ///
    /// Subcommands to add, remove, or list labels on beads. Labels are
    /// free-form strings used for grouping and filtering.
    #[command(subcommand)]
    Label(LabelCommands),

    /// List labels for beads
    ///
    /// With a bead ID, lists that bead's labels (one per line). Without an ID,
    /// lists all beads with their labels in a formatted table showing ID, title,
    /// and comma-separated labels.
    Labels {
        /// Bead ID (optional - if omitted, shows all beads with their labels)
        #[arg(required = false)]
        id: Option<String>,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Output JSON (alias for --format json)
        #[arg(long)]
        json: bool,
    },

    /// Manage comments
    ///
    /// Subcommands to add or list comments on a bead. Comments are appended to
    /// the bead's history and shown by `bf show`.
    #[command(subcommand)]
    Comments(CommentsCommands),

    /// Search beads
    ///
    /// Full-text search over bead titles and descriptions with filters for
    /// status, type, assignee, label, and a priority range. Multiple values for
    /// --status, --type, and --label are OR-combined.
    Search {
        /// Search query
        query: Option<String>,

        /// Filter by status
        #[arg(short, long)]
        status: Vec<String>,

        /// Filter by type
        #[arg(short, long)]
        type_: Vec<String>,

        /// Filter by assignee
        #[arg(long)]
        assignee: Option<String>,

        /// Filter by label
        #[arg(short, long)]
        label: Vec<String>,

        /// Filter by minimum priority
        #[arg(long, value_name = "MIN")]
        priority_min: Option<i32>,

        /// Filter by maximum priority
        #[arg(long, value_name = "MAX")]
        priority_max: Option<i32>,

        /// Limit results
        #[arg(long, default_value = "50")]
        limit: usize,

        /// Output format (text, json, toon)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Show statistics
    ///
    /// Summarizes bead counts by status. Add --by-type, --by-priority,
    /// --by-assignee, or --by-label for the corresponding breakdown.
    Stats {
        /// Show breakdown by type
        #[arg(long)]
        by_type: bool,

        /// Show breakdown by priority
        #[arg(long)]
        by_priority: bool,

        /// Show breakdown by assignee
        #[arg(long)]
        by_assignee: bool,

        /// Show breakdown by label
        #[arg(long)]
        by_label: bool,

        /// Output format (text, json, toon)
        #[arg(long, default_value = "text")]
        format: String,

        /// Wrap output in a JSON envelope (requires --format json)
        #[arg(long)]
        envelope: bool,
    },

    /// Emit JSON Schema
    ///
    /// With "all" (default) prints the SQLite DDL for every bf table. With a
    /// bead ID instead, prints that bead's full JSON representation including
    /// its annotations.
    Schema {
        /// Schema target
        #[arg(default_value = "all")]
        target: String,

        /// Output format (text, json)
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Configuration management
    ///
    /// Subcommands to list, get, set, or locate configuration values. Set and
    /// Get support dot notation for nested keys (e.g. scoring.priority_weight).
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Show velocity stats (bead-forge specific)
    ///
    /// Reports per-model/harness/type throughput (P50, P90, and average
    /// seconds) reconstructed from claim-to-close events. Filter with --model
    /// or --harness. Velocity data accumulates as beads are claimed and closed.
    Velocity {
        /// Model
        #[arg(long)]
        model: Option<String>,

        /// Harness
        #[arg(long)]
        harness: Option<String>,

        /// Output format (text, json, toon)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Manage annotations
    ///
    /// Subcommands to set, get, remove, list, or clear annotations — arbitrary
    /// key/value metadata — on a bead. Annotations live in the
    /// bead_annotations table, never as a column on issues.
    #[command(subcommand)]
    Annotate(AnnotateCommands),

    /// Show event log for a bead
    ///
    /// Shows the event history for one bead (omit the ID for all events).
    /// Filter by --actor or --since; --status-changes shows only status
    /// transitions; --git merges in events reconstructed from the JSONL git
    /// history; --diff prints the field-level change for each event.
    Log {
        /// Bead ID (omit to show all events)
        id: Option<String>,

        /// Limit number of entries
        #[arg(long)]
        limit: Option<usize>,

        /// Show events since this date (RFC3339 format, e.g., 2026-05-01T00:00:00Z)
        #[arg(long)]
        since: Option<String>,

        /// Filter by actor (worker name)
        #[arg(long)]
        actor: Option<String>,

        /// Show only status change events
        #[arg(long)]
        status_changes: bool,

        /// Show field-level diff between old and new values
        #[arg(long)]
        diff: bool,

        /// Include git history from .beads/issues.jsonl
        #[arg(long)]
        git: bool,

        /// Output format (text, json, toon)
        #[arg(long, default_value = "text")]
        format: String,

        /// Output JSON (alias for --format json)
        #[arg(long)]
        json: bool,
    },

    /// Show critical path (longest chain of blocking dependencies)
    ///
    /// Computes the critical path through an epic: the longest chain of
    /// blocking dependencies plus the float (slack) of every bead. Beads with
    /// zero float lie on the critical path; the minimum remaining time is the
    /// length of that longest chain.
    CriticalPath {
        /// Root bead ID
        id: String,

        /// Maximum depth
        #[arg(long, default_value = "20")]
        max_depth: usize,

        /// Output format (text, json, toon)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Rotate (archive) closed beads older than threshold
    ///
    /// Moves closed beads older than --days (default 30) into a timestamped
    /// archive JSONL file, keeping the active issues.jsonl lean. --dry-run
    /// previews what would be archived without writing.
    Rotate {
        /// Days threshold (archive beads closed this many days ago)
        #[arg(long, default_value = "30")]
        days: u64,

        /// Dry run (show what would be rotated)
        #[arg(long)]
        dry_run: bool,
    },

    /// Migrate workspace from br to bf
    ///
    /// Migrates a beads_rust (br) workspace to bead-forge format, with backup
    /// and verification. --from-jsonl reimports from JSONL for corrupted or
    /// missing databases; --seed-velocity reconstructs velocity stats from
    /// events; --dry-run previews without writing.
    Migrate {
        /// Workspace path to migrate (defaults to current directory)
        #[arg(short, long)]
        workspace: Option<PathBuf>,

        /// Reimport from JSONL (for corrupted/missing databases)
        #[arg(long)]
        from_jsonl: bool,

        /// Seed velocity stats from reconstructed events
        #[arg(long)]
        seed_velocity: bool,

        /// Dry run (show what would be done without making changes)
        #[arg(long)]
        dry_run: bool,

        /// Skip verification steps
        #[arg(long)]
        skip_verify: bool,
    },

    /// Show recently modified beads
    ///
    /// Lists beads ordered by last-updated time. Filter by status, type,
    /// assignee, or priority, and by time using --time-period (e.g. 24h, 7d,
    /// 4w) or explicit --since/--before RFC3339 timestamps. -n/--limit caps the
    /// result count (0 means unlimited).
    Recent {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Filter by type
        #[arg(long)]
        type_: Option<String>,

        /// Filter by assignee
        #[arg(long)]
        assignee: Option<String>,

        /// Filter by priority
        #[arg(long)]
        priority: Option<i32>,

        /// Show beads updated since this date (RFC3339 format, e.g., 2026-07-01T00:00:00Z)
        #[arg(long)]
        since: Option<String>,

        /// Show beads updated before this date (RFC3339 format, e.g., 2026-07-01T00:00:00Z)
        #[arg(long)]
        before: Option<String>,

        /// Show beads modified in the last time period (e.g., 1h, 24h, 7d, 4w)
        #[arg(short, long)]
        time_period: Option<String>,

        /// Limit results (0 = unlimited)
        #[arg(short = 'n', long)]
        limit: Option<usize>,

        /// Output format (text, json, toon)
        #[arg(long, default_value = "text")]
        format: String,

        /// Output JSON (alias for --format json)
        #[arg(long)]
        json: bool,
    },

    /// Robot docs - machine-readable command contract documentation
    ///
    /// Outputs a JSON schema describing every command's --json output contract,
    /// enabling agent consumers to parse responses programmatically without
    /// hardcoding shapes.
    RobotDocs {
        /// Output format (text, json)
        #[arg(long, default_value = "json")]
        format: String,
    },
}

#[derive(Subcommand)]
pub enum DepCommands {
    /// Add a dependency
    ///
    /// Records that --blocks depends on --blocker. For a "blocks" dependency
    /// (the default) the blocked bead is marked Blocked until its blocker
    /// closes.
    Add {
        /// Bead that is blocked (depends on the blocker)
        #[arg(long)]
        blocks: Option<String>,

        /// Bead that blocks (the bead being depended on)
        blocker: String,

        /// Dependency type (e.g., blocks, relates_to)
        #[arg(short = 't', long, default_value = "blocks")]
        type_: String,
    },

    /// Remove a dependency
    ///
    /// Removes the dependency recorded from <issue> onto <depends-on>.
    Remove {
        /// Issue ID
        issue: String,

        /// Target issue ID to remove dependency to
        depends_on: String,
    },

    /// List dependencies of an issue
    ///
    /// Lists the direct dependencies recorded for a single bead.
    List {
        /// Issue ID
        id: String,
    },

    /// Show dependency tree rooted at issue
    ///
    /// Prints the dependency tree from a root bead. --direction controls
    /// traversal: down (default) shows what this bead depends on, up shows what
    /// depends on it, both shows each separately.
    Tree {
        /// Issue ID (root of tree)
        id: String,

        /// Tree direction (down, up, both)
        #[arg(short, long, default_value = "down")]
        direction: String,

        /// Maximum depth
        #[arg(long, default_value = "10")]
        max_depth: usize,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Output JSON (alias for --format json)
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum LabelCommands {
    /// Add label(s) to an issue
    ///
    /// Adds one or more labels (-l repeatable) to a bead. Labels already
    /// present are left as-is.
    Add {
        /// Label(s) to add (multiple labels supported)
        #[arg(short, long, required = true, num_args = 1..)]
        label: Vec<String>,

        /// Issue ID
        id: String,
    },

    /// Remove label(s) from an issue
    ///
    /// Removes one or more labels (-l repeatable) from a bead.
    Remove {
        /// Label(s) to remove (multiple labels supported)
        #[arg(short, long, required = true, num_args = 1..)]
        label: Vec<String>,

        /// Issue ID
        id: String,
    },

    /// List labels for an issue or all unique labels
    ///
    /// With a bead ID, lists that bead's labels. Without one, lists every
    /// unique label across the workspace with usage counts.
    List {
        /// Issue ID (optional - if omitted, lists all unique labels)
        id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum CommentsCommands {
    /// Add a comment
    ///
    /// Adds a comment to a bead. Multiple text arguments are joined with
    /// spaces, so quoting is optional.
    Add {
        /// Issue ID
        id: String,

        /// Comment text
        #[arg(required = true, num_args = 1..)]
        text: Vec<String>,
    },

    /// List comments for an issue
    ///
    /// Lists comments on a bead in the order they were added.
    List {
        /// Issue ID
        id: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// List all config values
    ///
    /// Prints the current resolved configuration for the workspace.
    List,

    /// Get a specific config value
    ///
    /// Prints a single config value by key, supporting dot notation for nested
    /// keys (e.g. scoring.priority_weight).
    Get {
        /// Config key
        key: String,
    },

    /// Set a config value
    ///
    /// Sets a config value and persists it to config.yaml. Supports dot
    /// notation for nested keys (e.g. scoring.priority_weight).
    Set {
        /// Config key (supports dot notation for nested values, e.g., scoring.priority_weight)
        key: String,

        /// Config value
        value: String,
    },

    /// Show config file path
    ///
    /// Prints the path to the workspace's config.yaml.
    Path,
}

#[derive(Subcommand)]
pub enum AnnotateCommands {
    /// Set an annotation
    ///
    /// Sets a key/value annotation on a bead, overwriting any existing value
    /// for that key.
    Set {
        /// Issue ID
        id: String,

        /// Annotation key
        key: String,

        /// Annotation value
        value: String,
    },

    /// Get an annotation
    ///
    /// Prints the value of a single annotation key on a bead.
    Get {
        /// Issue ID
        id: String,

        /// Annotation key
        key: String,
    },

    /// Remove an annotation
    ///
    /// Removes a single annotation key from a bead.
    Remove {
        /// Issue ID
        id: String,

        /// Annotation key
        key: String,
    },

    /// List all annotations for an issue
    ///
    /// Lists every key/value annotation on a bead.
    List {
        /// Issue ID
        id: String,
    },

    /// Clear all annotations for an issue
    ///
    /// Removes every annotation from a bead at once.
    Clear {
        /// Issue ID
        id: String,
    },
}

/// Wrap data in a JSON envelope for --json output.
///
/// This helper creates the standard envelope shape that all bf commands emit:
/// { version: 1, kind: "<command>", data: <data>, warning?: "<msg>" }
fn wrap_envelope(kind: &str, data: serde_json::Value, warning: Option<&str>) -> String {
    let envelope = crate::format::JsonEnvelope::new(kind, data);
    let envelope = if let Some(w) = warning {
        envelope.with_warning(w)
    } else {
        envelope
    };
    envelope.to_json().unwrap_or_else(|_| "{}".to_string())
}

pub fn run_cli() -> Result<Cli> {
    Ok(Cli::parse())
}

pub fn run(cli: Cli) -> Result<()> {
    // Captured before `cli.command` is moved out below. This is the
    // per-invocation half of the effective auto-flush switch; combined with
    // `config.sync.auto_flush` inside each handler via
    // `autoflush::after_mutation_with_config` (see child 1, bf-37xjd).
    let no_auto_flush = cli.no_auto_flush;
    let workspace = cli.workspace.unwrap_or_else(|| PathBuf::from("."));

    // Enable envelope wrapping for this process if --envelope flag is set.
    // This is a process-wide setting that affects all JSON formatting.
    if cli.envelope {
        crate::format::json::JsonFormatter::with_envelope_enabled();
    }

    // Handle case where no subcommand is provided
    let command = match cli.command {
        None => {
            // clap handles --help automatically, exiting before this point
            // If we reach here, it means no valid flag was provided
            return Err(anyhow!(
                "No command provided. Use 'bf --help' for usage information."
            ));
        }
        Some(cmd) => cmd,
    };

    // Handle Init command specially (doesn't require existing .beads directory)
    if let Commands::Init { prefix } = &command {
        let beads_dir = workspace.join(".beads");
        return cmd_init(&beads_dir, prefix);
    }

    // Handle MergeJsonl specially: it operates on explicit file paths (as a git
    // merge driver would) and must not require a discoverable .beads directory.
    if let Commands::MergeJsonl {
        base,
        ours,
        theirs,
        output,
    } = &command
    {
        return cmd_merge_jsonl(&workspace, base.as_deref(), ours, theirs, output.as_deref());
    }

    // All other commands require existing .beads directory
    let beads_dir = find_beads_dir(&workspace)
        .ok_or_else(|| anyhow!("No .beads directory found in {:?}", workspace))?;

    match command {
        Commands::Create {
            title,
            type_,
            priority,
            description,
            assignee,
            label,
            json,
        } => cmd_create(
            &beads_dir,
            title,
            type_,
            priority,
            description,
            assignee,
            label,
            json,
            no_auto_flush,
        ),
        Commands::List {
            status,
            type_,
            assignee,
            priority,
            annotation,
            limit,
            all,
            format,
            json,
        } => {
            let format = if json { "json".to_string() } else { format };
            cmd_list(
                &beads_dir, status, type_, assignee, priority, annotation, limit, all, &format, cli.envelope,
            )
        }
        Commands::Show { id, format, json } => {
            let format = if json { "json".to_string() } else { format };
            cmd_show(&beads_dir, &id, &format, cli.envelope)
        }
        Commands::Update {
            id,
            title,
            status,
            priority,
            assignee,
            clear_assignee,
            description,
            description_file,
            acceptance_criteria,
            notes,
            design,
            due_at,
        } => {
            // --clear-assignee is sugar for --assignee "": both flow the
            // empty-string "clear to NULL" signal into update_issue. clap
            // guarantees the two flags are mutually exclusive.
            let assignee = if clear_assignee {
                Some(String::new())
            } else {
                assignee
            };
            // --description-file resolves into `description` here (the REAL
            // update path — cmd_update -> update_issue writes the column).
            // clap's conflicts_with("description") guarantees only one is set.
            let description = match description_file {
                Some(path) => Some(
                    std::fs::read_to_string(&path)
                        .map_err(|e| anyhow!("Failed to read --description-file {}: {}", path.display(), e))?,
                ),
                None => description,
            };
            cmd_update(
                &beads_dir,
                &id,
                title,
                status,
                priority,
                assignee,
                description,
                acceptance_criteria,
                notes,
                design,
                due_at,
                no_auto_flush,
            )
        }
        Commands::Close { id, reason } => cmd_close(&beads_dir, &id, &reason, no_auto_flush),
        Commands::Reopen { id } => cmd_reopen(&beads_dir, &id, no_auto_flush),
        Commands::Delete { id } => cmd_delete(&beads_dir, &id, no_auto_flush),
        Commands::Ready {
            limit,
            format,
            json,
        } => {
            let format = if json { "json".to_string() } else { format };
            cmd_ready(&beads_dir, limit, &format, cli.envelope)
        }
        Commands::Claim {
            assignee,
            model,
            harness,
            harness_version,
            any,
            fallback,
            workspace_paths,
            dry_run,
            format,
            json,
        } => {
            let format = if json { "json".to_string() } else { format };
            cmd_claim(
                &beads_dir,
                &assignee,
                model,
                harness,
                harness_version,
                any,
                fallback.as_deref(),
                &workspace_paths,
                dry_run,
                &format,
                no_auto_flush,
            )
        }
        Commands::Sync {
            flush_only,
            import_only,
        } => cmd_sync(&beads_dir, flush_only, import_only),
        // Handled specially before the .beads-directory requirement above.
        Commands::MergeJsonl { .. } => unreachable!("MergeJsonl handled earlier"),
        Commands::Doctor {
            repair,
            flush_first,
            force,
            reclaim_stale,
            ttl,
            fix_schema,
            reconcile,
            allow_repeated_repair,
            runs,
            restore,
        } => cmd_doctor(
            &beads_dir,
            repair,
            flush_first,
            force,
            reclaim_stale,
            ttl,
            fix_schema,
            reconcile,
            allow_repeated_repair,
            runs,
            restore,
        ),
        Commands::CommitCheck => cmd_commit_check(&beads_dir),
        Commands::Count { status } => cmd_count(&beads_dir, status),
        Commands::Batch { file, json, stdin, format } => {
            cmd_batch(&beads_dir, file, json, stdin, &format, no_auto_flush)
        }
        Commands::Mitosis {
            id,
            children,
            reason,
            format,
        } => cmd_mitosis(&beads_dir, &id, &children, &reason, &format, no_auto_flush),
        Commands::Dep(dep) => cmd_dep(&beads_dir, dep, no_auto_flush),
        Commands::Label(label) => cmd_label(&beads_dir, label, no_auto_flush),
        Commands::Comments(comments) => cmd_comments(&beads_dir, comments, no_auto_flush),
        Commands::Search {
            query,
            status,
            type_,
            assignee,
            label,
            priority_min,
            priority_max,
            limit,
            format,
        } => cmd_search(
            &beads_dir,
            query,
            status,
            type_,
            assignee,
            label,
            priority_min,
            priority_max,
            limit,
            &format,
            cli.envelope,
        ),
        Commands::Stats {
            by_type,
            by_priority,
            by_assignee,
            by_label,
            format,
            envelope,
        } => cmd_stats(
            &beads_dir,
            by_type,
            by_priority,
            by_assignee,
            by_label,
            &format,
            envelope,
        ),
        Commands::Schema { target, format } => cmd_schema(&target, &format),
        Commands::Config(config) => cmd_config(&beads_dir, config),
        Commands::Velocity {
            model,
            harness,
            format,
        } => cmd_velocity(&beads_dir, model, harness, &format),
        Commands::Labels { id, format, json } => {
            let format = if json { "json".to_string() } else { format };
            cmd_labels(&beads_dir, id.as_deref(), &format)
        }
        Commands::Annotate(annotate) => cmd_annotate(&beads_dir, annotate, no_auto_flush),
        Commands::Log {
            id,
            limit,
            since,
            actor,
            status_changes,
            diff,
            git,
            format,
            json,
        } => {
            let format = if json { "json".to_string() } else { format };
            cmd_log(
                &beads_dir,
                id,
                limit,
                since,
                actor,
                status_changes,
                diff,
                git,
                &format,
            )
        }
        Commands::CriticalPath {
            id,
            max_depth,
            format,
        } => cmd_critical_path(&beads_dir, &id, max_depth, &format),
        Commands::Rotate { days, dry_run } => cmd_rotate(&beads_dir, days, dry_run),
        Commands::Migrate {
            workspace,
            from_jsonl,
            seed_velocity,
            dry_run,
            skip_verify,
        } => cmd_migrate(
            &beads_dir,
            workspace,
            from_jsonl,
            seed_velocity,
            dry_run,
            skip_verify,
        ),
        Commands::Recent {
            status,
            type_,
            assignee,
            priority,
            since,
            before,
            time_period,
            limit,
            format,
            json,
        } => {
            let format = if json { "json".to_string() } else { format };
            cmd_recent(
                &beads_dir,
                status,
                type_,
                assignee,
                priority,
                since,
                before,
                time_period,
                limit,
                &format,
            )
        }
        Commands::RobotDocs { .. } => {
            eprintln!("Error: robot-docs command is not yet implemented");
            std::process::exit(1);
        }
        Commands::Init { .. } => unreachable!("Init command handled earlier"),
    }
}

fn cmd_init(beads_dir: &PathBuf, prefix: &str) -> Result<()> {
    std::fs::create_dir_all(beads_dir)?;

    let config_path = beads_dir.join("config.yaml");
    if !config_path.exists() {
        let config = format!(
            r#"issue_prefixes: [{}]
default_priority: 2
default_type: task
claim_ttl_minutes: 30
"#,
            prefix
        );
        std::fs::write(&config_path, config)?;
    }

    let metadata_path = beads_dir.join("metadata.json");
    if !metadata_path.exists() {
        let metadata = r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#;
        std::fs::write(&metadata_path, metadata)?;
    }

    let db_path = beads_dir.join("beads.db");
    let _storage = Storage::open(&db_path)?;

    let gitignore_path = beads_dir.join(".gitignore");
    if !gitignore_path.exists() {
        std::fs::write(&gitignore_path, "beads.db\nbeads.db-shm\nbeads.db-wal\n")?;
    }

    println!("Initialized bead-forge workspace in {:?}", beads_dir);
    Ok(())
}

/// Best-effort SQLite→JSONL flush after a SUCCESSFUL single-issue mutation.
///
/// This is the shared wiring for Phase 7.1 child 2/5 (bf-3iosi): every
/// single-issue mutation handler calls this once its storage write has
/// committed. It honors the effective auto-flush switch
/// (`config.sync.auto_flush && !--no-auto-flush`) via child 1's
/// [`crate::autoflush::after_mutation_with_config`].
///
/// A flush failure NEVER fails the mutation — the write already succeeded and
/// the flush layer retains the `dirty_issues` marks so the next flush (or
/// `bf sync --flush-only`) recovers. On failure we emit a stderr warning and
/// return the warning text so a `--json` caller can also fold it into its
/// output envelope via [`crate::format::with_warning`]. Returns `None` when
/// auto-flush is disabled or the flush succeeded.
fn autoflush_after_mutation(
    beads_dir: &Path,
    config: &Config,
    no_auto_flush: bool,
) -> Option<String> {
    let outcome = crate::autoflush::after_mutation_with_config(beads_dir, config, no_auto_flush);
    surface_flush_outcome(&outcome)
}

/// Like [`autoflush_after_mutation`] but for a hard delete: prunes the deleted
/// beads' now-stale lines from JSONL via [`crate::autoflush::after_delete`]
/// (the FK cascade drops their `dirty_issues` rows, so an ordinary dirty flush
/// could never remove them). Best-effort — a flush failure never fails the
/// delete; it degrades to a stderr warning and the returned text.
fn autoflush_after_delete(
    beads_dir: &Path,
    config: &Config,
    no_auto_flush: bool,
    removed_ids: &[String],
) -> Option<String> {
    let enabled = crate::autoflush::enabled(config, no_auto_flush);
    let outcome = crate::autoflush::after_delete(beads_dir, enabled, removed_ids);
    surface_flush_outcome(&outcome)
}

/// Bridge a [`crate::autoflush::FlushOutcome`] into the user-facing warning
/// channel: emit a stderr `warning:` line on failure and return the text so a
/// `--json` caller can fold it into its envelope. `None` when silent
/// (disabled/succeeded).
fn surface_flush_outcome(outcome: &crate::autoflush::FlushOutcome) -> Option<String> {
    match outcome.warning() {
        Some(w) => {
            crate::format::warn_stderr(w);
            Some(w.to_string())
        }
        None => None,
    }
}

fn cmd_create(
    beads_dir: &PathBuf,
    title: String,
    type_: String,
    priority: i32,
    description: Option<String>,
    assignee: Option<String>,
    labels: Vec<String>,
    json: bool,
    no_auto_flush: bool,
) -> Result<()> {
    let config = load_config(beads_dir)?;
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open_with_config(&db_path, &config)?;

    let count = storage.count_issues()?;
    let prefix = get_default_prefix(&config);

    let mut issue = Issue::new(String::new(), title, ".".to_string());
    issue.issue_type = IssueType::from_str(type_.as_str()).map_err(|e| anyhow::anyhow!(e))?;
    issue.priority = Priority(priority);
    issue.description = description;
    // Normalize empty/whitespace-only to None so `bf create --assignee ''`
    // creates a bead with no assignee instead of a literal empty string.
    issue.assignee = normalize_assignee(assignee.as_deref());
    issue.labels = labels;

    // Short IDs are sized for ~1% collision probability by design
    // (id::optimal_hash_length), so a colliding INSERT is an expected
    // event: re-roll the ID instead of failing the create.
    let mut id = String::new();
    let mut created = false;
    let mut last_err = None;
    for _ in 0..5 {
        id = crate::id::generate_id(prefix, count);
        issue.id = id.clone();
        match storage.create_issue(&issue) {
            Ok(()) => {
                created = true;
                break;
            }
            Err(e)
                if e.to_string()
                    .contains("UNIQUE constraint failed: issues.id") =>
            {
                last_err = Some(e);
            }
            Err(e) => return Err(e),
        }
    }
    if !created {
        return Err(last_err.unwrap_or_else(|| anyhow::anyhow!("ID collision retries exhausted")));
    }

    // Incremental flush of the just-created bead (best effort; never fatal).
    let warning = autoflush_after_mutation(beads_dir, &config, no_auto_flush);

    if json {
        let formatter = get_formatter(OutputFormat::Json);
        let data = serde_json::json!({ "id": id });
        let json_str = serde_json::to_string(&data)?;
        println!("{}", formatter.format_with_envelope_and_warning("create", &json_str, warning.as_deref()));
    } else {
        println!("{}", id);
    }
    Ok(())
}

fn cmd_list(
    beads_dir: &PathBuf,
    status: Option<String>,
    type_: Option<String>,
    assignee: Option<String>,
    priority: Option<i32>,
    annotation: Option<String>,
    limit: Option<usize>,
    all: bool,
    format: &str,
    envelope: bool,
) -> Result<()> {
    // Parse annotation filter (key=value format)
    let annotation_filter = match annotation {
        Some(ref ann) => {
            let parts: Vec<&str> = ann.splitn(2, '=').collect();
            if parts.len() != 2 {
                return Err(anyhow!("Invalid annotation format. Use key=value"));
            }
            Some((parts[0].to_string(), parts[1].to_string()))
        }
        None => None,
    };

    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    let mut issues = if all {
        // Include archived beads
        list_all_with_archives(beads_dir)?
    } else {
        // Only from database
        let mut filter = IssueFilter::default();
        if let Some(ref s) = status {
            filter.status = Some(Status::from_str(s.as_str()).map_err(|e| anyhow::anyhow!(e))?);
        }
        if let Some(ref t) = type_ {
            filter.issue_type =
                Some(IssueType::from_str(t.as_str()).map_err(|e| anyhow::anyhow!(e))?);
        }
        filter.assignee = assignee.clone();
        filter.priority = priority;
        filter.annotation = annotation_filter.clone();
        // --limit 0 means unlimited
        filter.limit = limit.and_then(|l| if l == 0 { None } else { Some(l) });
        storage.list_issues(&filter)?
    };

    // Apply additional filters for --all mode (since we're not using DB filters)
    if all {
        if let Some(ref s) = status {
            let status_filter = Status::from_str(s.as_str()).map_err(|e| anyhow::anyhow!(e))?;
            issues.retain(|i| i.status == status_filter);
        }
        if let Some(ref t) = type_ {
            let type_filter = IssueType::from_str(t.as_str()).map_err(|e| anyhow::anyhow!(e))?;
            issues.retain(|i| i.issue_type == type_filter);
        }
        if let Some(ref assignee_val) = assignee {
            issues.retain(|i| i.assignee.as_deref() == Some(assignee_val));
        }
        if let Some(p) = priority {
            issues.retain(|i| i.priority.0 == p);
        }
        if let Some((ref key, ref value)) = annotation_filter {
            issues.retain(|i| i.annotations.get(key).map_or(false, |v| v == value));
        }
        // Apply limit
        if let Some(l) = limit {
            if l != 0 {
                issues.truncate(l);
            }
        }
    }

    let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
    let formatter = get_formatter(output_format);

    match output_format {
        OutputFormat::Json => {
            let jsonl = formatter.format_issues(&issues);
            if envelope {
                // Wrap in envelope with kind="list"
                // Convert JSONL to JSON array for envelope wrapping
                let data = if jsonl.is_empty() {
                    "[]".to_string()
                } else {
                    // Convert JSONL (one object per line) to JSON array
                    let objects: Vec<serde_json::Value> = jsonl
                        .lines()
                        .filter_map(|line| serde_json::from_str(line).ok())
                        .collect();
                    serde_json::to_string(&objects).unwrap_or_else(|_| "[]".to_string())
                };
                println!("{}", formatter.format_with_envelope("list", &data));
            } else {
                // Raw JSONL output; empty list prints nothing (unlike ready, which prints [])
                if !jsonl.is_empty() {
                    println!("{}", jsonl);
                }
            }
        }
        _ => {
            print!("{}", formatter.format_issues(&issues));
        }
    }

    Ok(())
}

fn cmd_show(beads_dir: &PathBuf, id: &str, format: &str, envelope: bool) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    let issue = match storage.get_issue(id)? {
        Some(i) => i,
        None => {
            // Search archives
            find_bead_in_archives(beads_dir, id)?
                .ok_or_else(|| anyhow!("Bead not found: {}", id))?
        }
    };

    match format {
        "json" => {
            // Strip dependencies/comments before serializing: NEEDLE's BrDependency
            // format ({id, title, status, priority, dependency_type}) differs from
            // bead-forge's Dependency format ({issue_id, depends_on_id, type, ...}).
            // NEEDLE has #[serde(default)] on the deps field so empty is fine.
            let mut out = issue;
            out.dependencies = vec![];
            out.comments = vec![];
            // Serialize to JSON
            let formatter = get_formatter(OutputFormat::Json);
            let json_str = formatter.format_issue(&out);
            if envelope {
                // Wrap in envelope with kind="show"
                println!("{}", formatter.format_with_envelope("show", &json_str));
            } else {
                // Raw JSON array output (NEEDLE contract: single-element array)
                println!("[{}]", json_str);
            }
        }
        "toon" => {
            println!("ID: {}", issue.id);
            println!("Title: {}", issue.title);
            println!("Status: {}", issue.status);
            println!("Priority: {}", issue.priority);
            println!("Type: {}", issue.issue_type);
            if let Some(desc) = &issue.description {
                println!("Description: {}", desc);
            }
            if let Some(assignee) = &issue.assignee {
                println!("Assignee: {}", assignee);
            }
            if let Some(reason) = &issue.close_reason {
                println!("Close reason: {}", reason);
            }
            if !issue.labels.is_empty() {
                println!("Labels: {}", issue.labels.join(", "));
            }
            if !issue.dependencies.is_empty() {
                println!("Dependencies:");
                for dep in &issue.dependencies {
                    println!("  -> {} ({})", dep.depends_on_id, dep.dep_type);
                }
            }
        }
        _ => {
            println!("ID: {}", issue.id);
            println!("Title: {}", issue.title);
            println!("Status: {}", issue.status);
            println!("Priority: {}", issue.priority);
            println!("Type: {}", issue.issue_type);
            if let Some(desc) = &issue.description {
                println!("Description: {}", desc);
            }
            if let Some(assignee) = &issue.assignee {
                println!("Assignee: {}", assignee);
            }
            if let Some(reason) = &issue.close_reason {
                println!("Close reason: {}", reason);
            }
            if !issue.labels.is_empty() {
                println!("Labels: {}", issue.labels.join(", "));
            }
            if !issue.dependencies.is_empty() {
                println!("Dependencies:");
                for dep in &issue.dependencies {
                    println!("  -> {} ({})", dep.depends_on_id, dep.dep_type);
                }
            }
        }
    }

    Ok(())
}

fn cmd_update(
    beads_dir: &PathBuf,
    id: &str,
    title: Option<String>,
    status: Option<String>,
    priority: Option<i32>,
    assignee: Option<String>,
    description: Option<String>,
    acceptance_criteria: Option<String>,
    notes: Option<String>,
    design: Option<String>,
    due_at: Option<String>,
    no_auto_flush: bool,
) -> Result<()> {
    let config = load_config(beads_dir)?;
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open_with_config(&db_path, &config)?;

    // Note: an empty/whitespace `--assignee` is intentionally NOT rejected here.
    // It flows through to update_issue, whose storage layer maps it to
    // `assignee = NULL` (clearing the assignee). Normalizing to None at this
    // layer would erase the "clear" intent (None means "leave unchanged").
    // Parse due_at if provided
    let due_at_parsed = match due_at {
        Some(date_str) => {
            let dt = DateTime::parse_from_rfc3339(&date_str).map_err(|_| {
                anyhow!("Invalid --due-at format. Use RFC3339 format, e.g., 2025-01-01T00:00:00Z")
            })?;
            Some(dt.with_timezone(&Utc))
        }
        None => None,
    };

    let changes = IssueChanges {
        title,
        status: status.map(|s| Status::from_str(&s).ok()).flatten(),
        priority,
        assignee,
        description,
        acceptance_criteria,
        notes,
        design,
        due_at: due_at_parsed,
        ..Default::default()
    };

    storage.update_issue(id, &changes)?;
    autoflush_after_mutation(beads_dir, &config, no_auto_flush);
    println!("Updated bead {}", id);

    Ok(())
}

fn cmd_close(beads_dir: &PathBuf, id: &str, reason: &str, no_auto_flush: bool) -> Result<()> {
    let config = load_config(beads_dir)?;
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);

    close_bead(&db_path, id, reason, "cli")?;
    autoflush_after_mutation(beads_dir, &config, no_auto_flush);
    println!("Closed bead {}", id);

    Ok(())
}

fn cmd_reopen(beads_dir: &PathBuf, id: &str, no_auto_flush: bool) -> Result<()> {
    let config = load_config(beads_dir)?;
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    // Reopening a bead logically makes it unclaimed again, so clear any stale
    // assignee left over from before it was closed/tombstoned. An empty
    // `assignee` is the three-valued "clear to NULL" signal: update_issue's
    // storage layer maps it to `assignee = NULL` (it never persists a literal
    // empty string, which would read back as "assigned" and hide the bead from
    // claiming). Defaulting `assignee` to None here would mean "leave
    // unchanged", leaving a foreign assignee on a now-open bead.
    let changes = IssueChanges {
        status: Some(Status::Open),
        assignee: Some(String::new()),
        ..Default::default()
    };

    storage.update_issue(id, &changes)?;
    autoflush_after_mutation(beads_dir, &config, no_auto_flush);
    println!("Reopened bead {}", id);

    Ok(())
}

fn cmd_delete(beads_dir: &PathBuf, id: &str, no_auto_flush: bool) -> Result<()> {
    let config = load_config(beads_dir)?;
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    storage.with_immediate_transaction(|tx| {
        tx.execute("DELETE FROM issues WHERE id = ?", [&id])?;
        Ok(())
    })?;

    // The hard DELETE cascades away the bead's dirty_issues row, so a normal
    // dirty flush can never remove its stale JSONL line. Prune it explicitly
    // (best effort; a flush failure never fails the delete).
    autoflush_after_delete(beads_dir, &config, no_auto_flush, &[id.to_string()]);

    println!("Deleted bead {}", id);

    Ok(())
}

fn cmd_ready(beads_dir: &PathBuf, limit: usize, format: &str, envelope: bool) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    // --limit 0 means unlimited (get_ready_candidates omits LIMIT clause when limit == 0)
    let candidates =
        storage.with_immediate_transaction(|tx| get_ready_candidates(tx, limit, None, None))?;

    match format {
        "json" => {
            // Use the shared formatter for consistency with `list`/`search`.
            // Resolve each scored candidate to its full Issue record so
            // the formatter has every field; empty result prints `[]`.
            let formatter = get_formatter(OutputFormat::Json);
            let issues: Vec<Issue> = candidates
                .iter()
                .filter_map(|c| storage.get_issue(&c.id).ok().flatten())
                .collect();

            let jsonl = formatter.format_issues(&issues);
            if envelope {
                // Wrap in envelope with kind="ready"
                // Convert JSONL to JSON array for the envelope data field
                let data = if jsonl.is_empty() {
                    "[]".to_string()
                } else {
                    let objects: Vec<Value> = jsonl
                        .lines()
                        .filter_map(|line| serde_json::from_str(line).ok())
                        .collect();
                    serde_json::to_string(&objects).unwrap_or_else(|_| "[]".to_string())
                };
                println!("{}", formatter.format_with_envelope("ready", &data));
            } else {
                // Raw JSONL output; empty ready prints `[]` as a special case
                if jsonl.is_empty() {
                    println!("[]");
                } else {
                    println!("{}", jsonl);
                }
            }
        }
        "toon" => {
            for candidate in candidates {
                println!(
                    "{}",
                    crate::format::toon::format_ready_bead(
                        &candidate.id,
                        &candidate.title,
                        candidate.priority,
                        candidate.downstream_impact,
                        candidate.critical_float,
                    )
                );
            }
        }
        _ => {
            for candidate in candidates {
                println!(
                    "[{}] {} (priority={}, impact={}, float={})",
                    candidate.id,
                    candidate.title,
                    candidate.priority,
                    candidate.downstream_impact,
                    candidate.critical_float
                );
            }
        }
    }

    Ok(())
}

fn cmd_claim(
    beads_dir: &PathBuf,
    assignee: &str,
    model: Option<String>,
    harness: Option<String>,
    harness_version: Option<String>,
    any: bool,
    fallback: Option<&str>,
    workspace_paths: &[PathBuf],
    dry_run: bool,
    format: &str,
    no_auto_flush: bool,
) -> Result<()> {
    let config = load_config(beads_dir)?;
    let claim_ttl = config.claim_ttl_minutes;

    // Pre-resolved auto-flush switch; each successful (non-dry-run) claim flushes
    // the workspace whose bead it mutated. `flush_claim` targets that workspace
    // (which may differ from `beads_dir` under `--any`/fallback) and surfaces a
    // flush failure as a warning without failing the claim.
    let flush_enabled = crate::autoflush::enabled(&config, no_auto_flush);
    let flush_claim = |ws: &Path| {
        let outcome = crate::autoflush::after_mutation(ws, flush_enabled);
        surface_flush_outcome(&outcome);
    };

    let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
    let formatter = get_formatter(output_format);

    // Build worker metadata
    let worker_metadata = WorkerMetadata {
        worker_id: assignee.to_string(),
        model: model.clone(),
        harness: harness.clone(),
        harness_version: harness_version.clone(),
    };

    if dry_run {
        // Dry run mode - show what would be claimed
        let candidates: Vec<(PathBuf, crate::claim::ScoredBead)> = if any || fallback == Some("any")
        {
            // Multi-workspace dry run
            let paths = if workspace_paths.is_empty() {
                // Auto-discover workspaces from current directory
                find_workspaces(&std::env::current_dir()?)?
            } else {
                workspace_paths.to_vec()
            };

            let mut all_candidates = Vec::new();
            for path in &paths {
                let local_beads_dir = path.join(".beads");
                if local_beads_dir.exists() {
                    let local_metadata = match load_metadata(&local_beads_dir) {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    let local_db_path = local_beads_dir.join(&local_metadata.database);
                    if let Ok(local_storage) = Storage::open(&local_db_path) {
                        if let Ok(local_candidates) =
                            local_storage.with_immediate_transaction(|tx| {
                                get_ready_candidates(tx, 1, None, None)
                            })
                        {
                            for c in local_candidates {
                                all_candidates.push((path.clone(), c));
                            }
                        }
                    }
                }
            }

            // Sort by score and take top 1
            all_candidates.sort_by(|a, b| {
                let score_a = (b.1.downstream_impact, b.1.priority, b.1.created_at.clone());
                let score_b = (a.1.downstream_impact, a.1.priority, a.1.created_at.clone());
                score_a.cmp(&score_b)
            });
            all_candidates.into_iter().take(1).collect()
        } else {
            // Single workspace dry run
            let metadata = load_metadata(beads_dir)?;
            let db_path = beads_dir.join(&metadata.database);
            let storage = Storage::open(&db_path)?;
            let candidates =
                storage.with_immediate_transaction(|tx| get_ready_candidates(tx, 1, None, None))?;
            candidates
                .into_iter()
                .map(|c| (beads_dir.parent().unwrap_or(beads_dir).to_path_buf(), c))
                .collect()
        };

        if let Some((path, candidate)) = candidates.first() {
            let mut out = ClaimResultOutput::new(&candidate.id, assignee);
            out.title = Some(candidate.title.clone());
            out.priority = Some(candidate.priority);
            out.downstream_impact = Some(candidate.downstream_impact);
            out.workspace = Some(path.display().to_string());
            out.dry_run = Some(true);
            println!("{}", formatter.format_claim_result(&out));
        } else {
            println!("{}", formatter.format_no_claim());
        }
    } else if any {
        // Claim from any workspace
        let paths = if workspace_paths.is_empty() {
            // Auto-discover workspaces from current directory
            find_workspaces(&std::env::current_dir()?)?
        } else {
            workspace_paths.to_vec()
        };

        let result = claim_any(&paths, assignee, claim_ttl, Some(&worker_metadata))?;

        match result {
            Some(ClaimResult {
                bead_id,
                reclaimed,
                workspace_path,
            }) => {
                // Flush the workspace that actually got claimed (may differ
                // from the invoking `beads_dir` under --any).
                flush_claim(workspace_path.as_deref().unwrap_or(beads_dir.as_path()));
                let mut out = ClaimResultOutput::new(&bead_id, assignee);
                out.reclaimed = Some(reclaimed);
                out.workspace = workspace_path.map(|p| p.display().to_string());
                println!("{}", formatter.format_claim_result(&out));
            }
            None => {
                println!("{}", formatter.format_no_claim());
            }
        }
    } else if fallback == Some("any") {
        // Fallback mode: try current workspace first, then any
        let metadata = load_metadata(beads_dir)?;
        let db_path = beads_dir.join(&metadata.database);
        let storage = Storage::open(&db_path)?;

        let result = storage.with_immediate_transaction(|tx| {
            claim(tx, assignee, claim_ttl, Utc::now(), Some(&worker_metadata))
        })?;

        match result {
            Some(ClaimResult {
                bead_id, reclaimed, ..
            }) => {
                // Claimed in the current workspace.
                flush_claim(beads_dir);
                let mut out = ClaimResultOutput::new(&bead_id, assignee);
                out.reclaimed = Some(reclaimed);
                println!("{}", formatter.format_claim_result(&out));
            }
            None => {
                // Fallback to any workspace
                let paths = if workspace_paths.is_empty() {
                    find_workspaces(&std::env::current_dir()?)?
                } else {
                    workspace_paths.to_vec()
                };

                let result = claim_any(&paths, assignee, claim_ttl, Some(&worker_metadata))?;

                match result {
                    Some(ClaimResult {
                        bead_id,
                        reclaimed,
                        workspace_path,
                    }) => {
                        flush_claim(workspace_path.as_deref().unwrap_or(beads_dir.as_path()));
                        let mut out = ClaimResultOutput::new(&bead_id, assignee);
                        out.reclaimed = Some(reclaimed);
                        out.workspace = workspace_path.map(|p| p.display().to_string());
                        println!("{}", formatter.format_claim_result(&out));
                    }
                    None => {
                        println!("{}", formatter.format_no_claim());
                    }
                }
            }
        }
    } else {
        // Normal single-workspace claim
        let metadata = load_metadata(beads_dir)?;
        let db_path = beads_dir.join(&metadata.database);
        let storage = Storage::open(&db_path)?;

        let result = storage.with_immediate_transaction(|tx| {
            claim(tx, assignee, claim_ttl, Utc::now(), Some(&worker_metadata))
        })?;

        match result {
            Some(ClaimResult {
                bead_id, reclaimed, ..
            }) => {
                flush_claim(beads_dir);
                let mut out = ClaimResultOutput::new(&bead_id, assignee);
                out.reclaimed = Some(reclaimed);
                println!("{}", formatter.format_claim_result(&out));
            }
            None => {
                println!("{}", formatter.format_no_claim());
            }
        }
    }

    Ok(())
}

fn cmd_merge_jsonl(
    workspace: &Path,
    base: Option<&Path>,
    ours: &Path,
    theirs: &Path,
    output: Option<&Path>,
) -> Result<()> {
    // Resolve the base: explicit path wins; otherwise fall back to the merge
    // anchor in the discovered .beads directory. A missing base is not fatal —
    // merge_jsonl_files degrades to a safe union.
    let base_path = match base {
        Some(p) => p.to_path_buf(),
        None => {
            let anchor = find_beads_dir(workspace)
                .map(|bd| crate::merge::base_anchor_path(&bd))
                .unwrap_or_else(|| PathBuf::from(crate::merge::BASE_ANCHOR));
            anchor
        }
    };

    // Git driver convention: write the resolved artifact back over "ours" (%A).
    let out_path = output.unwrap_or(ours);

    let report = crate::merge::merge_jsonl_files(&base_path, ours, theirs, out_path)?;

    eprintln!(
        "Merged {} bead(s): {} added, {} updated, {} deleted, {} conflict(s) auto-resolved",
        report.total, report.added, report.updated, report.deleted, report.conflicts
    );

    // Exit 0 even with auto-resolved conflicts: as a git merge driver, a
    // zero exit means "clean merge, use the result". We never emit markers, so
    // there is nothing for a human to resolve.
    Ok(())
}

fn cmd_sync(beads_dir: &PathBuf, flush_only: bool, import_only: bool) -> Result<()> {
    let workspace_dir = beads_dir.parent().unwrap_or(beads_dir);

    if import_only {
        let result = crate::sync::import(workspace_dir)?;
        println!("Imported {} beads", result.imported);
        if result.updated > 0 {
            println!("Updated {} beads", result.updated);
        }
        if result.skipped > 0 {
            println!("Skipped {} unchanged beads", result.skipped);
        }
    } else if flush_only {
        let count = crate::sync::flush(workspace_dir)?;
        println!("Flushed {} beads to JSONL", count);
    } else {
        let result = crate::sync::sync(workspace_dir)?;
        println!(
            "Synced {} beads from JSONL and flushed {} to JSONL",
            result.imported + result.updated,
            result.exported
        );
        if result.updated > 0 {
            println!("Updated {} beads", result.updated);
        }
        if result.skipped > 0 {
            println!("Skipped {} unchanged beads", result.skipped);
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_doctor(
    beads_dir: &PathBuf,
    repair: bool,
    flush_first: bool,
    force: bool,
    reclaim_stale: bool,
    ttl: Option<i64>,
    fix_schema: bool,
    reconcile: bool,
    allow_repeated_repair: bool,
    runs: bool,
    restore: Option<String>,
) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);

    if runs {
        // List verified pre-rebuild recovery runs (doctor safety stack, layer 3).
        let all = crate::recovery::list_runs(beads_dir)?;
        if all.is_empty() {
            println!("No recovery runs found");
        } else {
            println!("Recovery runs (newest first):");
            for m in &all {
                let total: u64 = m.files.iter().map(|f| f.bytes).sum();
                println!(
                    "  {}  {}  {} file(s), {} bytes  [{}]",
                    m.run_id,
                    m.created_at,
                    m.files.len(),
                    total,
                    m.reason
                );
                for f in &m.files {
                    println!("      {}  sha256:{}…  {} bytes", f.name, &f.sha256[..12.min(f.sha256.len())], f.bytes);
                }
            }
            println!();
            println!("Restore one with: bf doctor --restore <run-id|latest>");
        }
    } else if let Some(run_ref) = restore {
        // Restore the DB family from a hash-verified recovery run (layer 3).
        let manifest = crate::recovery::restore_run(beads_dir, &run_ref)?;
        println!(
            "✓ Restored {} file(s) from recovery run {} (all hashes verified)",
            manifest.files.len(),
            manifest.run_id
        );
        for f in &manifest.files {
            println!("    {}", f.name);
        }
    } else if fix_schema {
        let workspace_dir = beads_dir.parent().unwrap_or(beads_dir);
        let fixed = crate::doctor::fix_null_not_null(workspace_dir)?;
        if fixed == 0 {
            println!("✓ No NULL values in NOT NULL columns");
        } else {
            println!(
                "Repaired {} NULL value(s) in NOT NULL column(s) in place",
                fixed
            );
        }
    } else if reconcile {
        // Backfill rows left behind by forward-only fixes (bf-29wxxl).
        let workspace_dir = beads_dir.parent().unwrap_or(beads_dir);
        let report = crate::doctor::reconcile(workspace_dir)?;
        if report.is_clean() {
            println!("✓ Nothing to reconcile — no stale blocked beads, no empty assignees");
        } else {
            if report.unblocked.is_empty() {
                println!("✓ No beads stuck at 'blocked' with all blockers closed");
            } else {
                println!(
                    "Reopened {} bead(s) stuck at 'blocked' with all blockers closed:",
                    report.unblocked.len()
                );
                for id in &report.unblocked {
                    println!("    - {}", id);
                }
            }
            if report.normalized_assignees.is_empty() {
                println!("✓ No empty-string assignees");
            } else {
                println!(
                    "Normalized {} empty-string assignee(s) to NULL:",
                    report.normalized_assignees.len()
                );
                for id in &report.normalized_assignees {
                    println!("    - {}", id);
                }
            }
        }
        if !report.blocked_without_dependencies.is_empty() {
            println!(
                "⚠ {} bead(s) are 'blocked' with no blocking dependency — set by hand, \
                 so they were left alone. Review and `bf update <id> --status open` if stale:",
                report.blocked_without_dependencies.len()
            );
            for id in &report.blocked_without_dependencies {
                println!("    - {}", id);
            }
        }
    } else if repair {
        let workspace_dir = beads_dir.parent().unwrap_or(beads_dir);
        let opts = crate::doctor::RepairOptions {
            flush_first,
            force,
            allow_repeated_repair,
        };
        let report = crate::doctor::repair_stack(workspace_dir, &opts)?;
        if report.rebuilt {
            println!(
                "Repaired database: rebuilt from JSONL ({} imported, {} unflushed bead(s) preserved)",
                report.imported, report.preserved_dirty
            );
            if let Some(run_id) = &report.backup_run_id {
                println!("  Verified pre-rebuild backup: recovery run {}", run_id);
            }
        } else {
            println!("✓ Workspace healthy — no JSONL rebuild needed");
            if !report.local_fixes.is_empty() {
                println!("  Applied local fixers: {}", report.local_fixes.join(", "));
            }
        }
        for msg in &report.messages {
            println!("  {}", msg);
        }
    } else if reclaim_stale {
        let workspace_dir = beads_dir.parent().unwrap_or(beads_dir);
        let config = load_config(beads_dir)?;
        let ttl_minutes = ttl.unwrap_or(config.claim_ttl_minutes);
        let reclaimed = crate::doctor::reclaim_stale(workspace_dir, ttl_minutes)?;
        println!("Reclaimed {} stale bead(s)", reclaimed);
    } else {
        // Run health check
        let workspace_dir = beads_dir.parent().unwrap_or(beads_dir);
        let result = crate::doctor::check(workspace_dir)?;

        // Report results
        if result.db_ok && result.jsonl_ok {
            println!("✓ Database integrity: OK");
            println!("✓ JSONL validity: OK");
            println!("  Database beads: {}", result.db_issue_count);
            println!("  JSONL beads: {}", result.jsonl_line_count);

            let total_drift = result.missing_in_jsonl.len()
                + result.missing_in_sqlite.len()
                + result.hash_mismatch.len();

            if total_drift == 0 {
                println!("✓ Consistency: No drift detected");
            } else {
                println!("⚠ Consistency: Drift detected");
                if !result.missing_in_jsonl.is_empty() {
                    println!("  Missing in JSONL ({}):", result.missing_in_jsonl.len());
                    for id in result.missing_in_jsonl.iter().take(10) {
                        println!("    - {}", id);
                    }
                    if result.missing_in_jsonl.len() > 10 {
                        println!("    ... and {} more", result.missing_in_jsonl.len() - 10);
                    }
                }
                if !result.missing_in_sqlite.is_empty() {
                    println!("  Missing in SQLite ({}):", result.missing_in_sqlite.len());
                    for id in result.missing_in_sqlite.iter().take(10) {
                        println!("    - {}", id);
                    }
                    if result.missing_in_sqlite.len() > 10 {
                        println!("    ... and {} more", result.missing_in_sqlite.len() - 10);
                    }
                }
                if !result.hash_mismatch.is_empty() {
                    println!("  Hash mismatch ({}):", result.hash_mismatch.len());
                    for id in result.hash_mismatch.iter().take(10) {
                        println!("    - {}", id);
                    }
                    if result.hash_mismatch.len() > 10 {
                        println!("    ... and {} more", result.hash_mismatch.len() - 10);
                    }
                }
                println!();
                println!("Run 'bf doctor --repair' to rebuild SQLite from JSONL");
            }

            // Report unflushed beads (if any)
            if result.unflushed_count > 0 {
                println!("⚠ Unflushed beads: {}", result.unflushed_count);
                println!("  Run 'bf sync --flush-only' before repair to avoid data loss");
            }
        } else {
            if !result.db_ok {
                println!("✗ Database integrity: FAILED");
            }
            if !result.jsonl_ok {
                println!("✗ JSONL validity: FAILED");
            }
            for issue in &result.issues {
                eprintln!("  {}", issue);
            }
            if !result.jsonl_ok || !result.db_ok {
                println!();
                println!("Run 'bf doctor --repair' to rebuild SQLite from JSONL");
            }
        }

        // Report NULL-in-NOT-NULL corruption regardless of db_ok/jsonl_ok. This is
        // the crash class from bf-3hm5h (e.g. a NULL created_at/updated_at) and is
        // repaired in place, not by rebuilding from JSONL.
        if !result.null_not_null.is_empty() {
            let total: usize = result.null_not_null.iter().map(|v| v.count).sum();
            println!("⚠ NULL in NOT NULL column(s): {}", total);
            for v in &result.null_not_null {
                println!(
                    "    - {}.{} ({}): {} row(s)",
                    v.table, v.column, v.decl_type, v.count
                );
            }
            println!("  Run 'bf doctor --fix-schema' to repair these rows in place");
        }

        // Report rows left behind by forward-only fixes (bf-29wxxl). Like the NULL
        // check above these are independent of db_ok/jsonl_ok, and both starve
        // `bf ready`: a stale blocked bead is never claimable, and an empty-string
        // assignee reads back as already-claimed.
        if !result.stale_blocked_ids.is_empty() || !result.empty_assignee_ids.is_empty() {
            if !result.stale_blocked_ids.is_empty() {
                println!(
                    "⚠ Stale 'blocked' beads (all blockers closed): {}",
                    result.stale_blocked_ids.len()
                );
                for id in result.stale_blocked_ids.iter().take(10) {
                    println!("    - {}", id);
                }
                if result.stale_blocked_ids.len() > 10 {
                    println!("    ... and {} more", result.stale_blocked_ids.len() - 10);
                }
            }
            if !result.empty_assignee_ids.is_empty() {
                println!(
                    "⚠ Empty-string assignees (should be NULL): {}",
                    result.empty_assignee_ids.len()
                );
                for id in result.empty_assignee_ids.iter().take(10) {
                    println!("    - {}", id);
                }
                if result.empty_assignee_ids.len() > 10 {
                    println!("    ... and {} more", result.empty_assignee_ids.len() - 10);
                }
            }
            println!("  Run 'bf doctor --reconcile' to backfill these rows in place");
        }
    }

    Ok(())
}

fn cmd_commit_check(beads_dir: &PathBuf) -> Result<()> {
    let result = scan_staged_beads(beads_dir)?;

    if result.secrets_found.is_empty() {
        // Clean - no output on success (standard for pre-commit hooks)
        std::process::exit(0);
    }

    // Secrets found - print details and exit 1
    eprintln!("{}", format_scan_results(&result));
    std::process::exit(1);
}

fn cmd_count(beads_dir: &PathBuf, status: Option<String>) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    let count = if let Some(s) = status {
        let filter = IssueFilter {
            status: Some(Status::from_str(&s).map_err(|e| anyhow::anyhow!(e))?),
            ..Default::default()
        };
        storage.list_issues(&filter)?.len()
    } else {
        storage.count_issues()?
    };

    println!("{}", count);
    Ok(())
}

fn cmd_batch(
    beads_dir: &PathBuf,
    file: Option<PathBuf>,
    json: Option<String>,
    stdin: bool,
    format: &str,
    no_auto_flush: bool,
) -> Result<()> {
    let config = load_config(beads_dir)?;
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open_with_config(&db_path, &config)?;

    let ops: Vec<BatchOp> = if let Some(json_str) = json {
        serde_json::from_str(&json_str)?
    } else if let Some(file_path) = file {
        let content = std::fs::read_to_string(&file_path)?;
        serde_json::from_str(&content)?
    } else if stdin {
        parse_stdin()?
    } else {
        return Err(anyhow!("Must provide --file, --json, or --stdin"));
    };

    let results = execute_batch(&storage, ops, beads_dir, no_auto_flush)?;

    // Check if we should output JSON
    let output_format = crate::format::OutputFormat::from_str(format).unwrap_or(crate::format::OutputFormat::Text);
    match output_format {
        crate::format::OutputFormat::Json => {
            let formatter = get_formatter(output_format);
            // Convert Vec<BatchResult> to JSONL (newline-separated JSON objects)
            let jsonl = results
                .iter()
                .map(|r| serde_json::to_string(r))
                .collect::<Result<Vec<_>, _>>()
                .unwrap_or_default()
                .join("\n");
            println!("{}", formatter.format_with_envelope("batch", &jsonl));
        }
        _ => {
            // Print results in human-readable format
            for result in results {
                if result.status == "ok" {
                    if let Some(id) = result.id {
                        println!("[op {}] ok: {}", result.op, id);
                    } else {
                        println!("[op {}] ok", result.op);
                    }
                } else {
                    eprintln!(
                        "[op {}] error: {}",
                        result.op,
                        result.error.unwrap_or_default()
                    );
                }
            }
        }
    }

    Ok(())
}

fn cmd_mitosis(
    beads_dir: &PathBuf,
    id: &str,
    children: &str,
    reason: &str,
    format: &str,
    no_auto_flush: bool,
) -> Result<()> {
    let config = load_config(beads_dir)?;
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open_with_config(&db_path, &config)?;

    // Parse children as JSON array of {title, type, priority} or {title, type, priority, description, assignee, labels}
    let children_defs: Vec<MitosisChild> = serde_json::from_str(children)?;

    // Build the batch operations
    let ops = mitosis_ex(id, children_defs, Some(reason.to_string()))?;

    // Execute atomically
    let results = execute_batch(&storage, ops, beads_dir, false /* enable auto-flush */)?;

    // One surgical flush after the whole mitosis transaction commits (parent
    // close + all children + dep edges were marked dirty together).
    autoflush_after_mutation(beads_dir, &config, no_auto_flush);

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        _ => {
            // Print child IDs that were created
            for result in &results {
                if let Some(child_id) = &result.id {
                    println!("Created child: {}", child_id);
                }
            }
            println!(
                "Parent bead {} closed with {} children",
                id,
                results.len() - 2
            ); // -2 for close + last dep
        }
    }

    Ok(())
}

/// Format and print a dependency tree.
fn print_dep_tree(nodes: &[crate::storage::DepTreeNode], _storage: &Storage) -> Result<()> {
    if nodes.is_empty() {
        println!("  (no dependencies)");
        return Ok(());
    }

    // Status indicators: ●=open, ◐=in_progress, ○=closed/blocked/deferred, ⊘=tombstone
    let status_symbol = |status: &str| -> char {
        match status {
            "open" => '●',
            "in_progress" => '◐',
            "closed" => '○',
            "blocked" => '◌',
            "deferred" => '○',
            "tombstone" => '⊘',
            _ => '○',
        }
    };

    for (i, node) in nodes.iter().enumerate() {
        let is_cycle = node.path.contains("[CYCLE]");

        // Build tree prefix with proper branching
        let mut prefix = String::new();
        if node.depth > 0 {
            for d in 0..node.depth as usize {
                if d < node.depth as usize - 1 {
                    prefix.push_str("│   ");
                } else {
                    // Check if this is the last node at this depth
                    let is_last = nodes
                        .iter()
                        .skip(i + 1)
                        .all(|n| n.depth < node.depth || (n.depth == node.depth && n.id > node.id));
                    if is_last {
                        prefix.push_str("└── ");
                    } else {
                        prefix.push_str("├── ");
                    }
                }
            }
        }

        // Truncate title if too long
        let title = if node.title.len() > 60 {
            format!("{}...", &node.title[..57])
        } else {
            node.title.clone()
        };

        let cycle_mark = if is_cycle { " [CYCLE]" } else { "" };
        let dep_type_str = node.dep_type.as_deref().unwrap_or("blocks");

        println!(
            "{}[{}] {} {} (P{}, {}){}",
            prefix,
            node.id,
            status_symbol(&node.status),
            title,
            node.priority,
            dep_type_str,
            cycle_mark
        );
    }

    Ok(())
}

fn cmd_dep(beads_dir: &PathBuf, dep: DepCommands, no_auto_flush: bool) -> Result<()> {
    match dep {
        DepCommands::Add {
            blocks,
            blocker,
            type_,
        } => {
            let blocks = blocks.ok_or_else(|| {
                anyhow!("Missing --blocks argument. Usage: bf dep add <blocker> --blocks <blocked>")
            })?;

            let config = load_config(beads_dir)?;
            let metadata = load_metadata(beads_dir)?;
            let db_path = beads_dir.join(&metadata.database);
            let storage = Storage::open(&db_path)?;
            let dep_type =
                crate::model::DependencyType::from_str(&type_).map_err(|e| anyhow::anyhow!(e))?;

            // Add the dependency: blocks depends on blocker
            storage.add_dependency(&blocks, &blocker, &dep_type, "cli")?;

            // If this is a blocker dependency, update status to 'blocked'
            if matches!(dep_type, crate::model::DependencyType::Blocks) {
                let changes = IssueChanges {
                    status: Some(Status::Blocked),
                    ..Default::default()
                };
                storage.update_issue(&blocks, &changes)?;
            }

            autoflush_after_mutation(beads_dir, &config, no_auto_flush);
            println!(
                "Added dependency: {} depends on {} ({})",
                blocks, blocker, type_
            );
        }
        DepCommands::Remove { issue, depends_on } => {
            let config = load_config(beads_dir)?;
            let metadata = load_metadata(beads_dir)?;
            let db_path = beads_dir.join(&metadata.database);
            let storage = Storage::open(&db_path)?;
            storage.remove_dependency(&issue, &depends_on)?;
            autoflush_after_mutation(beads_dir, &config, no_auto_flush);
            println!("Removed dependency: {} -> {}", issue, depends_on);
        }
        DepCommands::List { id } => {
            let metadata = load_metadata(beads_dir)?;
            let db_path = beads_dir.join(&metadata.database);
            let storage = Storage::open(&db_path)?;
            let deps = storage.get_dependencies(&id)?;
            if deps.is_empty() {
                println!("No dependencies found for {}", id);
            } else {
                for dep in deps {
                    println!(
                        "  {} depends on {} ({})",
                        dep.issue_id, dep.depends_on_id, dep.dep_type
                    );
                }
            }
        }
        DepCommands::Tree {
            id,
            direction,
            max_depth,
            format,
            json,
        } => {
            let metadata = load_metadata(beads_dir)?;
            let db_path = beads_dir.join(&metadata.database);
            let storage = Storage::open(&db_path)?;
            let format = if json { "json".to_string() } else { format };

            // Validate direction
            let direction = match direction.as_str() {
                "down" | "up" | "both" => direction.as_str(),
                _ => {
                    return Err(anyhow!(
                        "Invalid direction: {}. Use 'down', 'up', or 'both'",
                        direction
                    ))
                }
            };

            if format == "json" {
                // JSON output format
                if direction == "both" {
                    let down_nodes = storage.get_dep_tree(&id, "down", max_depth)?;
                    let up_nodes = storage.get_dep_tree(&id, "up", max_depth)?;
                    let output = serde_json::json!({
                        "root_id": id,
                        "direction": direction,
                        "max_depth": max_depth,
                        "downward": down_nodes,
                        "upward": up_nodes
                    });
                    println!("{}", serde_json::to_string_pretty(&output)?);
                } else {
                    let nodes = storage.get_dep_tree(&id, direction, max_depth)?;
                    let output = serde_json::json!({
                        "root_id": id,
                        "direction": direction,
                        "max_depth": max_depth,
                        "nodes": nodes
                    });
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
            } else {
                // Text output format
                if direction == "both" {
                    // Show both directions separately
                    println!(
                        "Dependency tree for {} (downward - what this depends on):\n",
                        id
                    );
                    let down_nodes = storage.get_dep_tree(&id, "down", max_depth)?;
                    print_dep_tree(&down_nodes, &storage)?;

                    if !down_nodes.is_empty() {
                        println!();
                    }

                    println!(
                        "Reverse dependency tree for {} (upward - what depends on this):\n",
                        id
                    );
                    let up_nodes = storage.get_dep_tree(&id, "up", max_depth)?;
                    print_dep_tree(&up_nodes, &storage)?;
                } else {
                    let nodes = storage.get_dep_tree(&id, direction, max_depth)?;
                    print_dep_tree(&nodes, &storage)?;
                }
            }
        }
    }
    Ok(())
}

fn cmd_label(beads_dir: &PathBuf, label: LabelCommands, no_auto_flush: bool) -> Result<()> {
    match label {
        LabelCommands::Add { id, label } => {
            let config = load_config(beads_dir)?;
            let metadata = load_metadata(beads_dir)?;
            let db_path = beads_dir.join(&metadata.database);
            let storage = Storage::open(&db_path)?;
            for l in label {
                storage.add_label(&id, &l)?;
                println!("Added label '{}' to {}", l, id);
            }
            // One flush after all labels are applied (all mark the same bead).
            autoflush_after_mutation(beads_dir, &config, no_auto_flush);
        }
        LabelCommands::Remove { id, label } => {
            let config = load_config(beads_dir)?;
            let metadata = load_metadata(beads_dir)?;
            let db_path = beads_dir.join(&metadata.database);
            let storage = Storage::open(&db_path)?;
            for l in label {
                storage.remove_label(&id, &l)?;
                println!("Removed label '{}' from {}", l, id);
            }
            autoflush_after_mutation(beads_dir, &config, no_auto_flush);
        }
        LabelCommands::List { id } => {
            let metadata = load_metadata(beads_dir)?;
            let db_path = beads_dir.join(&metadata.database);
            let storage = Storage::open(&db_path)?;
            if let Some(id) = id {
                let labels = storage.get_labels(&id)?;
                println!("Labels for {}:", id);
                for label in labels {
                    println!("  {}", label);
                }
            } else {
                let labels = storage.list_all_labels()?;
                println!("All labels:");
                for (label, count) in labels {
                    println!("  {} ({})", label, count);
                }
            }
        }
    }
    Ok(())
}

fn cmd_labels(beads_dir: &PathBuf, id: Option<&str>, format: &str) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    if let Some(issue_id) = id {
        // Single bead mode - show labels for one bead
        let labels = storage.get_labels(issue_id)?;
        if format == "json" {
            // Output labels array as compact JSON
            println!("{}", serde_json::to_string(&labels)?);
        } else {
            for label in &labels {
                println!("{}", label);
            }
        }
    } else {
        // All beads mode - show all beads with their labels
        let filter = IssueFilter::default();
        let mut issues = storage.list_issues(&filter)?;

        // Sort by bead ID
        issues.sort_by(|a, b| a.id.cmp(&b.id));

        if format == "json" {
            // Output JSONL (one {id, title, labels} object per line)
            // Empty bead set prints [] (matches list/ready convention)
            if issues.is_empty() {
                println!("[]");
            } else {
                for issue in &issues {
                    let obj = serde_json::json!({
                        "id": issue.id,
                        "title": issue.title,
                        "labels": issue.labels
                    });
                    println!("{}", serde_json::to_string(&obj)?);
                }
            }
        } else {
            // Text format - display in a clean table
            for issue in &issues {
                let labels_str = if issue.labels.is_empty() {
                    "(no labels)".to_string()
                } else {
                    issue.labels.join(", ")
                };
                println!("{} {} | {}", issue.id, issue.title, labels_str);
            }
        }
    }

    Ok(())
}

fn cmd_comments(beads_dir: &PathBuf, comments: CommentsCommands, no_auto_flush: bool) -> Result<()> {
    match comments {
        CommentsCommands::Add { id, text } => {
            let config = load_config(beads_dir)?;
            let metadata = load_metadata(beads_dir)?;
            let db_path = beads_dir.join(&metadata.database);
            let storage = Storage::open(&db_path)?;
            let comment_text = text.join(" ");
            let comment_id = storage.add_comment(&id, "cli", &comment_text)?;
            autoflush_after_mutation(beads_dir, &config, no_auto_flush);
            println!("Added comment {} to {}", comment_id, id);
        }
        CommentsCommands::List { id } => {
            let metadata = load_metadata(beads_dir)?;
            let db_path = beads_dir.join(&metadata.database);
            let storage = Storage::open(&db_path)?;
            let comments = storage.list_comments(&id)?;
            if comments.is_empty() {
                println!("No comments for {}", id);
            } else {
                for comment in comments {
                    println!("  [{}] {}: {}", comment.id, comment.author, comment.body);
                }
            }
        }
    }
    Ok(())
}

fn cmd_search(
    beads_dir: &PathBuf,
    query: Option<String>,
    status: Vec<String>,
    type_: Vec<String>,
    assignee: Option<String>,
    label: Vec<String>,
    priority_min: Option<i32>,
    priority_max: Option<i32>,
    limit: usize,
    format: &str,
    envelope: bool,
) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    let statuses: Vec<Status> = status
        .iter()
        .filter_map(|s| Status::from_str(s).ok())
        .collect();
    let types: Vec<IssueType> = type_
        .iter()
        .filter_map(|t| IssueType::from_str(t).ok())
        .collect();

    let issues = storage.search_issues(
        query.as_deref(),
        &statuses,
        &types,
        assignee.as_deref(),
        &label,
        priority_min,
        priority_max,
        limit,
    )?;

    let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
    let formatter = get_formatter(output_format);
    match output_format {
        OutputFormat::Json => {
            let jsonl = formatter.format_issues(&issues);
            // Empty search results produce no output (JSONL: 0 lines)
            if !jsonl.is_empty() {
                println!("{}", jsonl);
            }
        }
        _ => {
            print!("{}", formatter.format_issues(&issues));
        }
    }

    Ok(())
}

fn cmd_stats(
    beads_dir: &PathBuf,
    by_type: bool,
    by_priority: bool,
    by_assignee: bool,
    by_label: bool,
    format: &str,
    envelope: bool,
) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;
    let stats = storage.get_stats()?;

    // Build the serializable projection. Breakdowns are fetched only when
    // requested and folded into the object so that `--format json --by-*`
    // yields one valid JSON document (the prior impl appended plain text
    // after the JSON object). Priority/assignee keys are stringified to
    // satisfy JSON's string-key rule; unassigned buckets use "None" to match
    // the text view.
    let mut output = StatsOutput::new(stats.total, stats.open, stats.in_progress, stats.closed);
    if by_type {
        output.by_type = Some(storage.get_stats_by_type()?.into_iter().collect());
    }
    if by_priority {
        output.by_priority = Some(
            storage
                .get_stats_by_priority()?
                .into_iter()
                .map(|(priority, count)| (priority.to_string(), count))
                .collect(),
        );
    }
    if by_assignee {
        output.by_assignee = Some(
            storage
                .get_stats_by_assignee()?
                .into_iter()
                .map(|(assignee, count)| (assignee.unwrap_or_else(|| "None".to_string()), count))
                .collect(),
        );
    }
    if by_label {
        output.by_label = Some(storage.list_all_labels()?.into_iter().collect());
    }

    let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
    let formatter = get_formatter(output_format);
    match output_format {
        OutputFormat::Json => {
            let json_str = formatter.format_stats(&output);
            if envelope {
                println!("{}", formatter.format_with_envelope("stats", &json_str));
            } else {
                println!("{}", json_str);
            }
        }
        _ => {
            print!("{}", formatter.format_stats(&output));
        }
    }

    Ok(())
}

fn cmd_schema(target: &str, format: &str) -> Result<()> {
    match target {
        "all" => {
            // Print SQLite schema DDL for all bf tables
            match format {
                "json" => {
                    let output = serde_json::json!({
                        "schema": crate::storage::schema::SCHEMA_SQL
                    });
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
                _ => {
                    println!("{}", crate::storage::schema::SCHEMA_SQL);
                }
            }
        }
        bead_id => {
            // Print that bead's full JSON representation including annotations
            let current_dir = std::env::current_dir()?;
            let beads_dir = crate::config::find_beads_dir(&current_dir)
                .ok_or_else(|| anyhow!("No .beads directory found"))?;
            let metadata = crate::config::load_metadata(&beads_dir)?;
            let db_path = beads_dir.join(&metadata.database);
            let storage = crate::storage::Storage::open(&db_path)?;

            let mut issue = match storage.get_issue(bead_id)? {
                Some(i) => i,
                None => {
                    // Search archives
                    crate::rotate::find_bead_in_archives(&beads_dir, bead_id)?
                        .ok_or_else(|| anyhow!("Bead not found: {}", bead_id))?
                }
            };

            // Load annotations for this bead
            issue.annotations = storage.get_annotations(bead_id)?;

            match format {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&issue)?);
                }
                _ => {
                    println!("{}", serde_json::to_string_pretty(&issue)?);
                }
            }
        }
    }
    Ok(())
}

fn cmd_config(beads_dir: &PathBuf, config: ConfigCommands) -> Result<()> {
    match config {
        ConfigCommands::List => {
            let cfg = load_config(beads_dir)?;
            println!("Config:");
            println!("  issue_prefixes: {:?}", cfg.issue_prefixes);
            println!("  default_priority: {}", cfg.default_priority);
            println!("  default_type: {}", cfg.default_type);
            println!("  claim_ttl_minutes: {}", cfg.claim_ttl_minutes);
        }
        ConfigCommands::Get { key } => {
            let cfg = load_config(beads_dir)?;
            // Parse key with dot notation support (e.g., scoring.priority_weight)
            let parts: Vec<&str> = key.split('.').collect();
            let value = match parts.as_slice() {
                ["issue_prefixes"] => format!("{:?}", cfg.issue_prefixes),
                ["default_priority"] => cfg.default_priority.to_string(),
                ["default_type"] => cfg.default_type.clone(),
                ["claim_ttl_minutes"] => cfg.claim_ttl_minutes.to_string(),
                ["scoring", "priority_weight"] => cfg.scoring.priority_weight.to_string(),
                ["scoring", "blockers_weight"] => cfg.scoring.blockers_weight.to_string(),
                ["scoring", "age_weight"] => cfg.scoring.age_weight.to_string(),
                ["scoring", "labels_weight"] => cfg.scoring.labels_weight.to_string(),
                ["scoring", "max_age_hours"] => cfg.scoring.max_age_hours.to_string(),
                ["scoring", "max_blockers"] => cfg.scoring.max_blockers.to_string(),
                ["rotate", "rotate_age_days"] => cfg.rotate.rotate_age_days.to_string(),
                ["rotate", "rotate_max_size_mb"] => cfg.rotate.rotate_max_size_mb.to_string(),
                ["rotate", "rotate_max_archives"] => cfg.rotate.rotate_max_archives.to_string(),
                ["secret_protection", "enabled"] => cfg.secret_protection.enabled.to_string(),
                ["checkpoint", "enabled"] => cfg.checkpoint.enabled.to_string(),
                ["checkpoint", "interval_minutes"] => cfg.checkpoint.interval_minutes.to_string(),
                ["checkpoint", "push"] => cfg.checkpoint.push.to_string(),
                _ => return Err(anyhow!("Unknown config key: {}", key)),
            };
            println!("{}", value);
        }
        ConfigCommands::Set { key, value } => {
            use crate::config::save_config;
            let mut cfg = load_config(beads_dir)?;

            // Parse key with dot notation support (e.g., scoring.priority_weight)
            let parts: Vec<&str> = key.split('.').collect();
            let result = match parts.as_slice() {
                ["issue_prefixes"] => {
                    // Parse as comma-separated list: "bf,nf,xf"
                    cfg.issue_prefixes = value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    Ok(())
                }
                ["default_priority"] => {
                    cfg.default_priority = value
                        .parse()
                        .map_err(|_| anyhow!("Invalid priority: {}. Must be 0-4", value))?;
                    Ok(())
                }
                ["default_type"] => {
                    cfg.default_type = value.clone();
                    Ok(())
                }
                ["claim_ttl_minutes"] => {
                    cfg.claim_ttl_minutes = value.parse().map_err(|_| {
                        anyhow!("Invalid TTL minutes: {}. Must be an integer", value)
                    })?;
                    Ok(())
                }
                ["scoring", "priority_weight"] => {
                    cfg.scoring.priority_weight = value.parse().map_err(|_| {
                        anyhow!("Invalid priority_weight: {}. Must be a number", value)
                    })?;
                    Ok(())
                }
                ["scoring", "blockers_weight"] => {
                    cfg.scoring.blockers_weight = value.parse().map_err(|_| {
                        anyhow!("Invalid blockers_weight: {}. Must be a number", value)
                    })?;
                    Ok(())
                }
                ["scoring", "age_weight"] => {
                    cfg.scoring.age_weight = value
                        .parse()
                        .map_err(|_| anyhow!("Invalid age_weight: {}. Must be a number", value))?;
                    Ok(())
                }
                ["scoring", "labels_weight"] => {
                    cfg.scoring.labels_weight = value.parse().map_err(|_| {
                        anyhow!("Invalid labels_weight: {}. Must be a number", value)
                    })?;
                    Ok(())
                }
                ["scoring", "max_age_hours"] => {
                    cfg.scoring.max_age_hours = value.parse().map_err(|_| {
                        anyhow!("Invalid max_age_hours: {}. Must be an integer", value)
                    })?;
                    Ok(())
                }
                ["scoring", "max_blockers"] => {
                    cfg.scoring.max_blockers = value.parse().map_err(|_| {
                        anyhow!("Invalid max_blockers: {}. Must be an integer", value)
                    })?;
                    Ok(())
                }
                ["rotate", "rotate_age_days"] => {
                    cfg.rotate.rotate_age_days = value.parse().map_err(|_| {
                        anyhow!("Invalid rotate_age_days: {}. Must be an integer", value)
                    })?;
                    Ok(())
                }
                ["rotate", "rotate_max_size_mb"] => {
                    cfg.rotate.rotate_max_size_mb = value.parse().map_err(|_| {
                        anyhow!("Invalid rotate_max_size_mb: {}. Must be an integer", value)
                    })?;
                    Ok(())
                }
                ["rotate", "rotate_max_archives"] => {
                    cfg.rotate.rotate_max_archives = value.parse().map_err(|_| {
                        anyhow!("Invalid rotate_max_archives: {}. Must be an integer", value)
                    })?;
                    Ok(())
                }
                ["secret_protection", "enabled"] => {
                    cfg.secret_protection.enabled = value
                        .parse()
                        .map_err(|_| anyhow!("Invalid enabled: {}. Must be true/false", value))?;
                    Ok(())
                }
                ["checkpoint", "enabled"] => {
                    cfg.checkpoint.enabled = value
                        .parse()
                        .map_err(|_| anyhow!("Invalid enabled: {}. Must be true/false", value))?;
                    Ok(())
                }
                ["checkpoint", "interval_minutes"] => {
                    cfg.checkpoint.interval_minutes = value.parse().map_err(|_| {
                        anyhow!("Invalid interval_minutes: {}. Must be an integer", value)
                    })?;
                    Ok(())
                }
                ["checkpoint", "push"] => {
                    cfg.checkpoint.push = value
                        .parse()
                        .map_err(|_| anyhow!("Invalid push: {}. Must be true/false", value))?;
                    Ok(())
                }
                _ => Err(anyhow!("Unknown config key: {}", key)),
            };

            result?;
            save_config(beads_dir, &cfg)?;
            println!("Set {} = {}", key, value);
        }
        ConfigCommands::Path => {
            let config_path = beads_dir.join("config.yaml");
            println!("{}", config_path.display());
        }
    }
    Ok(())
}

fn cmd_velocity(
    beads_dir: &PathBuf,
    model: Option<String>,
    harness: Option<String>,
    format: &str,
) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    let stats = storage.with_immediate_transaction(|tx| {
        crate::velocity::get_velocity_stats(tx, model.as_deref(), harness.as_deref())
    })?;

    let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
    let formatter = get_formatter(output_format);
    match output_format {
        OutputFormat::Json => {
            println!("{}", formatter.format_velocity(&stats));
        }
        _ => {
            print!("{}", formatter.format_velocity(&stats));
        }
    }

    Ok(())
}

fn cmd_annotate(beads_dir: &PathBuf, annotate: AnnotateCommands, no_auto_flush: bool) -> Result<()> {
    let config = load_config(beads_dir)?;
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    match annotate {
        AnnotateCommands::Set { id, key, value } => {
            storage.set_annotation(&id, &key, &value)?;
            autoflush_after_mutation(beads_dir, &config, no_auto_flush);
            println!("Set annotation '{}' on {}", key, id);
        }
        AnnotateCommands::Get { id, key } => {
            let annotations = storage.get_annotations(&id)?;
            if let Some(value) = annotations.get(&key) {
                println!("{}", value);
            } else {
                println!("Annotation '{}' not found on {}", key, id);
            }
        }
        AnnotateCommands::Remove { id, key } => {
            storage.remove_annotation(&id, &key)?;
            autoflush_after_mutation(beads_dir, &config, no_auto_flush);
            println!("Removed annotation '{}' from {}", key, id);
        }
        AnnotateCommands::List { id } => {
            let annotations = storage.get_annotations(&id)?;
            if annotations.is_empty() {
                println!("No annotations for {}", id);
            } else {
                println!("Annotations for {}:", id);
                for (key, value) in annotations {
                    println!("  {}: {}", key, value);
                }
            }
        }
        AnnotateCommands::Clear { id } => {
            storage.clear_annotations(&id)?;
            autoflush_after_mutation(beads_dir, &config, no_auto_flush);
            println!("Cleared all annotations from {}", id);
        }
    }
    Ok(())
}

fn cmd_log(
    beads_dir: &PathBuf,
    id: Option<String>,
    limit: Option<usize>,
    since: Option<String>,
    actor: Option<String>,
    status_changes: bool,
    diff: bool,
    git: bool,
    format: &str,
) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let jsonl_path = beads_dir.join(&metadata.jsonl_export);
    let workspace = beads_dir.parent().unwrap_or(beads_dir);

    let storage = Storage::open(&db_path)?;

    // Build filter
    let mut filter = crate::log::EventFilter::new();

    if let Some(ref issue_id) = id {
        filter = filter.with_issue_id(issue_id.clone());
    }

    if let Some(limit_val) = limit {
        filter = filter.with_limit(limit_val);
    }

    if let Some(ref actor_val) = actor {
        filter = filter.with_actor(actor_val.clone());
    }

    if status_changes {
        filter = filter.status_changes_only();
    }

    if diff {
        filter = filter.with_diff();
    }

    // Parse since date if provided
    if let Some(ref since_str) = since {
        match chrono::DateTime::parse_from_rfc3339(since_str) {
            Ok(dt) => {
                filter = filter.with_since(dt.with_timezone(&chrono::Utc));
            }
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Invalid --since date format. Use RFC3339 format, e.g., 2026-05-01T00:00:00Z"
                ));
            }
        }
    }

    // Query events from SQLite
    let sqlite_events = crate::log::query_events(&storage, &filter)?;

    // If --git flag is set, also query git history
    let events = if git {
        let git_events =
            crate::git_log::reconstruct_events_from_git(workspace, &jsonl_path, id.as_deref())?;

        // Merge git events with SQLite events
        crate::git_log::merge_events(sqlite_events, git_events)
    } else {
        sqlite_events
    };

    // Apply limit after merging (if limit was set)
    let events = if let Some(limit_val) = limit {
        if events.len() > limit_val {
            // Take the last N events (most recent)
            events.into_iter().rev().take(limit_val).rev().collect()
        } else {
            events
        }
    } else {
        events
    };

    // Apply actor filter in-memory for git events
    let events = if let Some(ref actor_val) = actor {
        events
            .into_iter()
            .filter(|e| e.actor == *actor_val)
            .collect()
    } else {
        events
    };

    // Apply status_changes_only filter in-memory
    let events = if status_changes {
        events
            .into_iter()
            .filter(|e| {
                matches!(
                    e.event_type,
                    crate::model::EventType::StatusChanged
                        | crate::model::EventType::Closed
                        | crate::model::EventType::Reopened
                        | crate::model::EventType::PriorityChanged
                        | crate::model::EventType::AssigneeChanged
                )
            })
            .collect()
    } else {
        events
    };

    match format {
        "json" => {
            println!("{}", crate::log::format_events_json(&events)?);
        }
        "toon" => {
            for event in &events {
                println!("{}", crate::log::format_event_toon(event));
            }
        }
        _ => {
            if events.is_empty() {
                println!("No events found");
            } else {
                for event in &events {
                    println!("{}", crate::log::format_event_text(event, diff));
                }
            }
        }
    }

    Ok(())
}

fn cmd_critical_path(beads_dir: &PathBuf, id: &str, _max_depth: usize, format: &str) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    // Compute critical path for the epic
    let result = storage.with_immediate_transaction(|tx| compute_epic_critical_path(tx, id))?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        _ => {
            println!(
                "Critical path for {} ({} open beads, {} on critical path):",
                id,
                result.beads.len(),
                result.beads.iter().filter(|b| b.float == 0).count()
            );
            println!();

            for bead in &result.beads {
                let float_marker = if bead.float == 0 { "★" } else { " " };
                println!(
                    "  {} float={:<3} [{}]",
                    float_marker, bead.float, bead.bead_id
                );
            }

            println!();
            if !result.longest_chain.is_empty() {
                println!("Longest chain: {}", result.longest_chain.join(" → "));
                println!(
                    "Minimum remaining time: {} bead-completions on critical path",
                    result.min_remaining
                );
            }
        }
    }

    Ok(())
}

fn cmd_rotate(beads_dir: &PathBuf, days: u64, dry_run: bool) -> Result<()> {
    let config = load_config(beads_dir)?;

    let mut options = RotateOptions::from_config(days, &config);
    if dry_run {
        options = options.dry_run();
    }

    let result = rotate(beads_dir, &options)?;

    if dry_run {
        println!("Dry run: would archive {} closed beads", result.archived);
        if let Some(ref archive_path) = result.archive_path {
            println!("Archive would be created at: {}", archive_path.display());
        }
        println!("{} beads would remain in active file", result.remaining);
    } else {
        println!("Archived {} closed beads", result.archived);
        if let Some(ref archive_path) = result.archive_path {
            println!("Created archive: {}", archive_path.display());
        }
        println!("{} beads remain in active file", result.remaining);

        if !result.deleted_archives.is_empty() {
            println!("Deleted {} old archive(s):", result.deleted_archives.len());
            for path in &result.deleted_archives {
                println!("  {}", path.display());
            }
        }
    }

    Ok(())
}

fn cmd_migrate(
    beads_dir: &PathBuf,
    workspace: Option<PathBuf>,
    from_jsonl: bool,
    seed_velocity: bool,
    dry_run: bool,
    skip_verify: bool,
) -> Result<()> {
    // Determine workspace path
    let workspace_path =
        workspace.unwrap_or_else(|| beads_dir.parent().unwrap_or(beads_dir).to_path_buf());

    if from_jsonl {
        // Migration Path C: Reimport from JSONL
        let result = crate::migrate::migrate_from_jsonl(&workspace_path, seed_velocity)?;

        if !result.verification.errors.is_empty() {
            eprintln!("Verification warnings:");
            for error in &result.verification.errors {
                eprintln!("  {}", error);
            }
        }
    } else {
        // Migration Path B: Explicit migration with backup
        let opts = crate::migrate::MigrateOptions::new(workspace_path)
            .with_dry_run(dry_run)
            .skip_verify(skip_verify);

        let result = crate::migrate::migrate(opts)?;

        if !result.verification.errors.is_empty() {
            eprintln!("Verification warnings:");
            for error in &result.verification.errors {
                eprintln!("  {}", error);
            }
        }
    }

    Ok(())
}

/// Parse a time period shorthand (e.g., "1h", "24h", "7d", "4w") into a DateTime<Utc>.
///
/// Returns the timestamp that represents the cutoff point (now minus the period).
/// Units:
/// - "s" or "sec": seconds
/// - "m" or "min": minutes
/// - "h" or "hour": hours
/// - "d" or "day": days
/// - "w" or "week": weeks
fn parse_time_period(period: &str) -> Result<DateTime<Utc>> {
    let period = period.trim().to_lowercase();

    // Parse the numeric prefix and unit suffix
    let (num_str, unit) = period.split_at(
        period
            .chars()
            .position(|c| !c.is_ascii_digit())
            .unwrap_or(period.len()),
    );

    if num_str.is_empty() {
        return Err(anyhow!(
            "Invalid time period format: '{}'. Expected format like '1h', '24h', '7d', '4w'",
            period
        ));
    }

    let value: i64 = num_str
        .parse()
        .map_err(|_| anyhow!("Invalid number in time period: '{}'", period))?;

    if value <= 0 {
        return Err(anyhow!("Time period must be positive: '{}'", period));
    }

    let duration = match unit {
        "s" | "sec" | "secs" | "second" | "seconds" => chrono::Duration::seconds(value),
        "m" | "min" | "mins" | "minute" | "minutes" => chrono::Duration::minutes(value),
        "h" | "hour" | "hours" => chrono::Duration::hours(value),
        "d" | "day" | "days" => chrono::Duration::days(value),
        "w" | "week" | "weeks" => chrono::Duration::weeks(value),
        _ => return Err(anyhow!("Unknown time unit in '{}'. Supported: s/sec/seconds, m/min/minutes, h/hours, d/days, w/weeks", period)),
    };

    Ok(Utc::now() - duration)
}

fn cmd_recent(
    beads_dir: &PathBuf,
    status: Option<String>,
    type_: Option<String>,
    assignee: Option<String>,
    priority: Option<i32>,
    since: Option<String>,
    before: Option<String>,
    time_period: Option<String>,
    limit: Option<usize>,
    format: &str,
) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    // Build the filter
    let mut filter = IssueFilter::default();

    // Parse status filter
    if let Some(ref s) = status {
        filter.status = Some(Status::from_str(s.as_str()).map_err(|e| anyhow::anyhow!(e))?);
    }

    // Parse type filter
    if let Some(ref t) = type_ {
        filter.issue_type = Some(IssueType::from_str(t.as_str()).map_err(|e| anyhow::anyhow!(e))?);
    }

    // Parse assignee filter
    filter.assignee = assignee;

    // Parse priority filter
    filter.priority = priority;

    // Parse time filters ( precedence: time_period > since/before )
    if let Some(period) = time_period {
        // Parse shorthand like "1h", "24h", "7d", "4w"
        let cutoff = parse_time_period(&period)?;
        filter.updated_since = Some(cutoff);
    } else {
        // Parse explicit --since and --before dates (RFC3339 format)
        if let Some(ref since_str) = since {
            let dt = DateTime::parse_from_rfc3339(since_str).map_err(|_| {
                anyhow!("Invalid --since format. Use RFC3339 format, e.g., 2026-07-01T00:00:00Z")
            })?;
            filter.updated_since = Some(dt.with_timezone(&Utc));
        }

        if let Some(ref before_str) = before {
            let dt = DateTime::parse_from_rfc3339(before_str).map_err(|_| {
                anyhow!("Invalid --before format. Use RFC3339 format, e.g., 2026-07-01T00:00:00Z")
            })?;
            filter.updated_before = Some(dt.with_timezone(&Utc));
        }
    }

    // Apply limit (0 means unlimited in IssueFilter)
    filter.limit = limit.and_then(|l| if l == 0 { None } else { Some(l) });

    // Query beads
    let issues = storage.list_issues(&filter)?;

    // Format output
    let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
    let formatter = get_formatter(output_format);
    match output_format {
        OutputFormat::Json => {
            let json_str = formatter.format_issues(&issues);
            println!("{}", formatter.format_with_envelope("recent", &json_str));
        }
        _ => {
            print!("{}", formatter.format_issues(&issues));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    pub mod json_output;
    pub mod list_ready_recent_json_tests;
    pub mod show_json_tests;
    pub mod search_json_tests;
    pub mod edge_case_json_tests;
    pub mod error_json_tests;
    pub mod json_schema_validation;
    pub use crate::config::init_workspace;
    pub use crate::Storage;
}

