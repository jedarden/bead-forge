use crate::batch::{execute_batch, mitosis_ex, parse_stdin, BatchOp, MitosisChild};
use crate::claim::{claim, claim_any, ClaimResult, get_ready_candidates, WorkerMetadata, find_workspaces};
use crate::config::{find_beads_dir, load_config, load_metadata, get_default_prefix};
use crate::format::{OutputFormat, get_formatter};
use crate::model::{Issue, IssueChanges, IssueFilter, IssueType, Priority, Status};
use crate::storage::Storage;
use anyhow::{anyhow, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser)]
#[command(name = "bf")]
#[command(about = "bead-forge - Drop-in replacement for beads_rust (br)", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Workspace directory (defaults to current directory's .beads/)
    #[arg(short, long, global = true)]
    pub workspace: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new bead
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
    },

    /// List beads
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

        /// Limit results (0 = unlimited)
        #[arg(long)]
        limit: Option<usize>,

        /// Output format (text, json, toon)
        #[arg(long, default_value = "text")]
        format: String,

        /// Output JSON (alias for --format json)
        #[arg(long)]
        json: bool,
    },

    /// Show bead details
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
    },

    /// Close a bead
    Close {
        /// Bead ID
        id: String,

        /// Close reason
        #[arg(long, default_value = "Completed")]
        reason: String,
    },

    /// Reopen a bead
    Reopen {
        /// Bead ID
        id: String,
    },

    /// Delete a bead
    Delete {
        /// Bead ID
        id: String,
    },

    /// Show ready (unblocked) beads
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
        #[arg(long)]
        fallback: bool,

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
    Init {
        /// Issue prefix
        #[arg(long, default_value = "bf")]
        prefix: String,
    },

    /// Sync (flush to JSONL or import from JSONL)
    Sync {
        /// Flush only (SQLite -> JSONL)
        #[arg(long)]
        flush_only: bool,

        /// Import only (JSONL -> SQLite)
        #[arg(long)]
        import_only: bool,
    },

    /// Doctor - check and repair
    Doctor {
        /// Repair database
        #[arg(long)]
        repair: bool,
    },

    /// Count beads
    Count {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,
    },

    /// Batch operations (atomic)
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
    },

    /// Mitosis: split a bead into children atomically
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
    #[command(subcommand)]
    Dep(DepCommands),

    /// Manage labels
    #[command(subcommand)]
    Label(LabelCommands),

    /// List labels for a specific issue (direct SELECT, efficient)
    Labels {
        /// Bead ID
        id: String,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Manage comments
    #[command(subcommand)]
    Comments(CommentsCommands),

    /// Search beads
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
    },

    /// Emit JSON Schema
    Schema {
        /// Schema target
        #[arg(default_value = "all")]
        target: String,

        /// Output format (text, json)
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Configuration management
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Show velocity stats (bead-forge specific)
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
    #[command(subcommand)]
    Annotate(AnnotateCommands),

    /// Show event log for a bead
    Log {
        /// Bead ID
        id: String,

        /// Limit number of entries
        #[arg(long)]
        limit: Option<usize>,

        /// Output format (text, json, toon)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Show critical path (longest chain of blocking dependencies)
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
    Rotate {
        /// Days threshold (archive beads closed this many days ago)
        #[arg(long, default_value = "30")]
        days: u64,

        /// Dry run (show what would be rotated)
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub enum DepCommands {
    /// Add a dependency
    Add {
        /// Issue ID (the one that will depend on something)
        issue: String,

        /// Target issue ID (the one being depended on)
        depends_on: String,

        /// Dependency type
        #[arg(short, long, default_value = "blocks")]
        type_: String,
    },

    /// Remove a dependency
    Remove {
        /// Issue ID
        issue: String,

        /// Target issue ID to remove dependency to
        depends_on: String,
    },

    /// List dependencies of an issue
    List {
        /// Issue ID
        id: String,
    },

    /// Show dependency tree rooted at issue
    Tree {
        /// Issue ID (root of tree)
        id: String,

        /// Tree direction (down, up, both)
        #[arg(short, long, default_value = "down")]
        direction: String,

        /// Maximum depth
        #[arg(long, default_value = "10")]
        max_depth: usize,
    },
}

#[derive(Subcommand)]
pub enum LabelCommands {
    /// Add label(s) to an issue
    Add {
        /// Label(s) to add (multiple labels supported)
        #[arg(short, long, required = true, num_args = 1..)]
        label: Vec<String>,

        /// Issue ID
        id: String,
    },

    /// Remove label(s) from an issue
    Remove {
        /// Label(s) to remove (multiple labels supported)
        #[arg(short, long, required = true, num_args = 1..)]
        label: Vec<String>,

        /// Issue ID
        id: String,
    },

    /// List labels for an issue or all unique labels
    List {
        /// Issue ID (optional - if omitted, lists all unique labels)
        id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum CommentsCommands {
    /// Add a comment
    Add {
        /// Issue ID
        id: String,

        /// Comment text
        #[arg(required = true, num_args = 1..)]
        text: Vec<String>,
    },

    /// List comments for an issue
    List {
        /// Issue ID
        id: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// List all config values
    List,

    /// Get a specific config value
    Get {
        /// Config key
        key: String,
    },

    /// Show config file path
    Path,
}

#[derive(Subcommand)]
pub enum AnnotateCommands {
    /// Set an annotation
    Set {
        /// Issue ID
        id: String,

        /// Annotation key
        key: String,

        /// Annotation value
        value: String,
    },

    /// Get an annotation
    Get {
        /// Issue ID
        id: String,

        /// Annotation key
        key: String,
    },

    /// Remove an annotation
    Remove {
        /// Issue ID
        id: String,

        /// Annotation key
        key: String,
    },

    /// List all annotations for an issue
    List {
        /// Issue ID
        id: String,
    },

    /// Clear all annotations for an issue
    Clear {
        /// Issue ID
        id: String,
    },
}

pub fn run_cli() -> Result<Cli> {
    Ok(Cli::try_parse()?)
}

pub fn run(cli: Cli) -> Result<()> {
    let workspace = cli.workspace.unwrap_or_else(|| PathBuf::from("."));

    // For init command, we allow the .beads directory to not exist yet
    match &cli.command {
        Commands::Init { .. } => {
            let beads_dir = workspace.join(".beads");
            return match cli.command {
                Commands::Init { prefix } => cmd_init(&beads_dir, &prefix),
                _ => unreachable!(),
            };
        }
        _ => {}
    }

    let beads_dir = find_beads_dir(&workspace)
        .ok_or_else(|| anyhow!("No .beads directory found in {:?}", workspace))?;

    match cli.command {
        Commands::Init { prefix } => cmd_init(&beads_dir, &prefix),
        Commands::Create { title, type_, priority, description, assignee, label } => {
            cmd_create(&beads_dir, title, type_, priority, description, assignee, label)
        }
        Commands::List { status, type_, assignee, priority, limit, format, json } => {
            let format = if json { "json".to_string() } else { format };
            cmd_list(&beads_dir, status, type_, assignee, priority, limit, &format)
        }
        Commands::Show { id, format, json } => {
            let format = if json { "json".to_string() } else { format };
            cmd_show(&beads_dir, &id, &format)
        }
        Commands::Update { id, title, status, priority, assignee } => {
            cmd_update(&beads_dir, &id, title, status, priority, assignee)
        }
        Commands::Close { id, reason } => cmd_close(&beads_dir, &id, &reason),
        Commands::Reopen { id } => cmd_reopen(&beads_dir, &id),
        Commands::Delete { id } => cmd_delete(&beads_dir, &id),
        Commands::Ready { limit, format, json } => {
            let format = if json { "json".to_string() } else { format };
            cmd_ready(&beads_dir, limit, &format)
        }
        Commands::Claim { assignee, model, harness, harness_version, any, fallback, workspace_paths, dry_run, format, json } => {
            let format = if json { "json".to_string() } else { format };
            cmd_claim(&beads_dir, &assignee, model, harness, harness_version, any, fallback, &workspace_paths, dry_run, &format)
        }
        Commands::Sync { flush_only, import_only } => cmd_sync(&beads_dir, flush_only, import_only),
        Commands::Doctor { repair } => cmd_doctor(&beads_dir, repair),
        Commands::Count { status } => cmd_count(&beads_dir, status),
        Commands::Batch { file, json, stdin } => cmd_batch(&beads_dir, file, json, stdin),
        Commands::Mitosis { id, children, reason, format } => cmd_mitosis(&beads_dir, &id, &children, &reason, &format),
        Commands::Dep(dep) => cmd_dep(&beads_dir, dep),
        Commands::Label(label) => cmd_label(&beads_dir, label),
        Commands::Comments(comments) => cmd_comments(&beads_dir, comments),
        Commands::Search { query, status, type_, assignee, label, priority_min, priority_max, limit, format } => {
            cmd_search(&beads_dir, query, status, type_, assignee, label, priority_min, priority_max, limit, &format)
        }
        Commands::Stats { by_type, by_priority, by_assignee, by_label, format } => {
            cmd_stats(&beads_dir, by_type, by_priority, by_assignee, by_label, &format)
        }
        Commands::Schema { target, format } => cmd_schema(&target, &format),
        Commands::Config(config) => cmd_config(&beads_dir, config),
        Commands::Velocity { model, harness, format } => cmd_velocity(&beads_dir, model, harness, &format),
        Commands::Labels { id, format } => cmd_labels(&beads_dir, &id, &format),
        Commands::Annotate(annotate) => cmd_annotate(&beads_dir, annotate),
        Commands::Log { id, limit, format } => cmd_log(&beads_dir, &id, limit, &format),
        Commands::CriticalPath { id, max_depth, format } => cmd_critical_path(&beads_dir, &id, max_depth, &format),
        Commands::Rotate { days, dry_run } => cmd_rotate(&beads_dir, days, dry_run),
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

fn cmd_create(
    beads_dir: &PathBuf,
    title: String,
    type_: String,
    priority: i32,
    description: Option<String>,
    assignee: Option<String>,
    labels: Vec<String>,
) -> Result<()> {
    let config = load_config(beads_dir)?;
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    let count = storage.count_issues()?;
    let prefix = get_default_prefix(&config);
    let id = crate::id::generate_id(prefix, count);

    let mut issue = Issue::new(id.clone(), title, ".".to_string());
    issue.issue_type = IssueType::from_str(type_.as_str()).map_err(|e| anyhow::anyhow!(e))?;
    issue.priority = Priority(priority);
    issue.description = description;
    issue.assignee = assignee;
    issue.labels = labels;

    storage.create_issue(&issue)?;

    println!("{}", id);
    Ok(())
}

fn cmd_list(
    beads_dir: &PathBuf,
    status: Option<String>,
    type_: Option<String>,
    assignee: Option<String>,
    priority: Option<i32>,
    limit: Option<usize>,
    format: &str,
) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    let mut filter = IssueFilter::default();
    if let Some(s) = status {
        filter.status = Some(Status::from_str(s.as_str()).map_err(|e| anyhow::anyhow!(e))?);
    }
    if let Some(t) = type_ {
        filter.issue_type = Some(IssueType::from_str(t.as_str()).map_err(|e| anyhow::anyhow!(e))?);
    }
    filter.assignee = assignee;
    filter.priority = priority;
    // --limit 0 means unlimited
    filter.limit = limit.and_then(|l| if l == 0 { None } else { Some(l) });

    let issues = storage.list_issues(&filter)?;

    let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
    let formatter = get_formatter(output_format);
    print!("{}", formatter.format_issues(&issues));

    Ok(())
}

fn cmd_show(beads_dir: &PathBuf, id: &str, format: &str) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    let issue = storage.get_issue(id)?.ok_or_else(|| anyhow!("Bead not found: {}", id))?;

    match format {
        "json" => {
            // Strip dependencies/comments before serializing: NEEDLE's BrDependency
            // format ({id, title, status, priority, dependency_type}) differs from
            // bead-forge's Dependency format ({issue_id, depends_on_id, type, ...}).
            // NEEDLE has #[serde(default)] on the deps field so empty is fine.
            let mut out = issue;
            out.dependencies = vec![];
            out.comments = vec![];
            // Wrap in array so NEEDLE's parse_single_bead (Vec<Bead> → first) works.
            println!("{}", serde_json::to_string(&vec![out])?);
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
            if !issue.labels.is_empty() {
                println!("Labels: {}", issue.labels.join(", "));
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
            if !issue.labels.is_empty() {
                println!("Labels: {}", issue.labels.join(", "));
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
) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    let changes = IssueChanges {
        title,
        status: status.map(|s| Status::from_str(&s).ok()).flatten(),
        priority,
        assignee,
        ..Default::default()
    };

    storage.update_issue(id, &changes)?;
    println!("Updated bead {}", id);
    Ok(())
}

fn cmd_close(beads_dir: &PathBuf, id: &str, reason: &str) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    storage.close_issue(id, reason, "cli")?;
    println!("Closed bead {}", id);
    Ok(())
}

fn cmd_reopen(beads_dir: &PathBuf, id: &str) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    let changes = IssueChanges {
        status: Some(Status::Open),
        ..Default::default()
    };

    storage.update_issue(id, &changes)?;
    println!("Reopened bead {}", id);
    Ok(())
}

fn cmd_delete(beads_dir: &PathBuf, id: &str) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    storage.with_immediate_transaction(|tx| {
        tx.execute("DELETE FROM issues WHERE id = ?", [&id])?;
        Ok(())
    })?;

    println!("Deleted bead {}", id);
    Ok(())
}

fn cmd_ready(beads_dir: &PathBuf, limit: usize, format: &str) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    let candidates = storage.with_immediate_transaction(|tx| {
        get_ready_candidates(tx, limit)
    })?;

    match format {
        "json" => {
            for candidate in candidates {
                println!("{}", serde_json::to_string(&candidate)?);
            }
        }
        "toon" => {
            for candidate in candidates {
                println!("{}", crate::format::toon::format_ready_bead(
                    &candidate.id,
                    &candidate.title,
                    candidate.priority,
                    candidate.downstream_impact,
                    candidate.critical_float,
                ));
            }
        }
        _ => {
            for candidate in candidates {
                println!("[{}] {} (priority={}, impact={}, float={})",
                    candidate.id, candidate.title, candidate.priority,
                    candidate.downstream_impact, candidate.critical_float);
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
    fallback: bool,
    workspace_paths: &[PathBuf],
    dry_run: bool,
    format: &str,
) -> Result<()> {
    let config = load_config(beads_dir)?;
    let claim_ttl = config.claim_ttl_minutes;

    // Build worker metadata
    let worker_metadata = WorkerMetadata {
        worker_id: assignee.to_string(),
        model: model.clone(),
        harness: harness.clone(),
        harness_version: harness_version.clone(),
    };

    if dry_run {
        // Dry run mode - show what would be claimed
        let candidates: Vec<(PathBuf, crate::claim::ScoredBead)> = if any || fallback {
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
                        if let Ok(local_candidates) = local_storage.with_immediate_transaction(|tx| {
                            get_ready_candidates(tx, 1)
                        }) {
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
            let candidates = storage.with_immediate_transaction(|tx| {
                get_ready_candidates(tx, 1)
            })?;
            candidates.into_iter().map(|c| (beads_dir.parent().unwrap_or(beads_dir).to_path_buf(), c)).collect()
        };

        if let Some((path, candidate)) = candidates.first() {
            match format {
                "json" => {
                    let output = serde_json::json!({
                        "bead_id": candidate.id,
                        "title": candidate.title,
                        "priority": candidate.priority,
                        "downstream_impact": candidate.downstream_impact,
                        "assignee": assignee,
                        "workspace": path.display().to_string(),
                        "dry_run": true
                    });
                    println!("{}", output);
                }
                _ => {
                    println!("{} (priority={}, impact={}, workspace={})",
                        candidate.id, candidate.priority, candidate.downstream_impact, path.display());
                }
            }
        } else if format == "json" {
            println!("{{}}");
        } else {
            println!("No beads available to claim");
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
            Some(ClaimResult { bead_id, reclaimed, workspace_path }) => {
                match format {
                    "json" => {
                        let output = serde_json::json!({
                            "bead_id": bead_id,
                            "reclaimed": reclaimed,
                            "assignee": assignee,
                            "workspace": workspace_path.map(|p| p.display().to_string())
                        });
                        println!("{}", output);
                    }
                    _ => {
                        if let Some(path) = workspace_path {
                            println!("{} (workspace: {})", bead_id, path.display());
                        } else {
                            println!("{}", bead_id);
                        }
                    }
                }
            }
            None => {
                if format == "json" {
                    println!("{{}}");
                } else {
                    println!("No beads available to claim");
                }
            }
        }
    } else if fallback {
        // Fallback mode: try current workspace first, then any
        let metadata = load_metadata(beads_dir)?;
        let db_path = beads_dir.join(&metadata.database);
        let storage = Storage::open(&db_path)?;

        let result = storage.with_immediate_transaction(|tx| {
            claim(tx, assignee, claim_ttl, Utc::now(), Some(&worker_metadata))
        })?;

        match result {
            Some(ClaimResult { bead_id, reclaimed, .. }) => {
                match format {
                    "json" => {
                        let output = serde_json::json!({
                            "bead_id": bead_id,
                            "reclaimed": reclaimed,
                            "assignee": assignee
                        });
                        println!("{}", output);
                    }
                    _ => {
                        println!("{}", bead_id);
                    }
                }
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
                    Some(ClaimResult { bead_id, reclaimed, workspace_path }) => {
                        match format {
                            "json" => {
                                let output = serde_json::json!({
                                    "bead_id": bead_id,
                                    "reclaimed": reclaimed,
                                    "assignee": assignee,
                                    "workspace": workspace_path.map(|p| p.display().to_string())
                                });
                                println!("{}", output);
                            }
                            _ => {
                                if let Some(path) = workspace_path {
                                    println!("{} (workspace: {})", bead_id, path.display());
                                } else {
                                    println!("{}", bead_id);
                                }
                            }
                        }
                    }
                    None => {
                        if format == "json" {
                            println!("{{}}");
                        } else {
                            println!("No beads available to claim");
                        }
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
            Some(ClaimResult { bead_id, reclaimed, .. }) => {
                match format {
                    "json" => {
                        let output = serde_json::json!({
                            "bead_id": bead_id,
                            "reclaimed": reclaimed,
                            "assignee": assignee
                        });
                        println!("{}", output);
                    }
                    _ => {
                        println!("{}", bead_id);
                    }
                }
            }
            None => {
                if format == "json" {
                    println!("{{}}");
                } else {
                    println!("No beads available to claim");
                }
            }
        }
    }

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
        println!("Synced {} beads from JSONL and flushed {} to JSONL", result.imported + result.updated, result.exported);
        if result.updated > 0 {
            println!("Updated {} beads", result.updated);
        }
        if result.skipped > 0 {
            println!("Skipped {} unchanged beads", result.skipped);
        }
    }

    Ok(())
}

fn cmd_doctor(beads_dir: &PathBuf, repair: bool) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);

    if repair {
        std::fs::remove_file(&db_path)?;
        let jsonl_path = beads_dir.join(&metadata.jsonl_export);
        let storage = Storage::open(&db_path)?;
        let result = storage.sync_from_jsonl(&jsonl_path)?;
        println!("Repaired database: imported {} beads from JSONL", result.imported);
    } else {
        let storage = Storage::open(&db_path)?;
        let count = storage.count_issues()?;
        println!("Database is healthy: {} beads", count);
    }

    Ok(())
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

fn cmd_batch(beads_dir: &PathBuf, file: Option<PathBuf>, json: Option<String>, stdin: bool) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

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

    let results = execute_batch(&storage, ops, beads_dir)?;

    // Print results
    for result in results {
        if result.status == "ok" {
            if let Some(id) = result.id {
                println!("[op {}] ok: {}", result.op, id);
            } else {
                println!("[op {}] ok", result.op);
            }
        } else {
            eprintln!("[op {}] error: {}", result.op, result.error.unwrap_or_default());
        }
    }

    Ok(())
}

fn cmd_mitosis(beads_dir: &PathBuf, id: &str, children: &str, reason: &str, format: &str) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    // Parse children as JSON array of {title, type, priority} or {title, type, priority, description, assignee, labels}
    let children_defs: Vec<MitosisChild> = serde_json::from_str(children)?;

    // Build the batch operations
    let ops = mitosis_ex(id, children_defs, Some(reason.to_string()))?;

    // Execute atomically
    let results = execute_batch(&storage, ops, beads_dir)?;

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
            println!("Parent bead {} closed with {} children", id, results.len() - 2); // -2 for close + last dep
        }
    }

    Ok(())
}

fn cmd_dep(beads_dir: &PathBuf, dep: DepCommands) -> Result<()> {
    match dep {
        DepCommands::Add { issue, depends_on, type_ } => {
            let metadata = load_metadata(beads_dir)?;
            let db_path = beads_dir.join(&metadata.database);
            let storage = Storage::open(&db_path)?;
            let dep_type = crate::model::DependencyType::from_str(&type_)
                .map_err(|e| anyhow::anyhow!(e))?;
            storage.add_dependency(&issue, &depends_on, &dep_type, "cli")?;
            println!("Added dependency: {} depends on {} ({})", issue, depends_on, type_);
        }
        DepCommands::Remove { issue, depends_on } => {
            let metadata = load_metadata(beads_dir)?;
            let db_path = beads_dir.join(&metadata.database);
            let storage = Storage::open(&db_path)?;
            storage.remove_dependency(&issue, &depends_on)?;
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
                    println!("  {} depends on {} ({})", dep.issue_id, dep.depends_on_id, dep.dep_type);
                }
            }
        }
        DepCommands::Tree { id, direction: _, max_depth: _ } => {
            println!("Dependency tree for {}", id);
            println!("(tree view not yet implemented)");
        }
    }
    Ok(())
}

fn cmd_label(beads_dir: &PathBuf, label: LabelCommands) -> Result<()> {
    match label {
        LabelCommands::Add { id, label } => {
            let metadata = load_metadata(beads_dir)?;
            let db_path = beads_dir.join(&metadata.database);
            let storage = Storage::open(&db_path)?;
            for l in label {
                storage.add_label(&id, &l)?;
                println!("Added label '{}' to {}", l, id);
            }
        }
        LabelCommands::Remove { id, label } => {
            let metadata = load_metadata(beads_dir)?;
            let db_path = beads_dir.join(&metadata.database);
            let storage = Storage::open(&db_path)?;
            for l in label {
                storage.remove_label(&id, &l)?;
                println!("Removed label '{}' from {}", l, id);
            }
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

fn cmd_labels(beads_dir: &PathBuf, id: &str, format: &str) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;
    let labels = storage.get_labels(id)?;
    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&labels)?);
    } else {
        for label in &labels {
            println!("{}", label);
        }
    }
    Ok(())
}

fn cmd_comments(beads_dir: &PathBuf, comments: CommentsCommands) -> Result<()> {
    match comments {
        CommentsCommands::Add { id, text } => {
            let metadata = load_metadata(beads_dir)?;
            let db_path = beads_dir.join(&metadata.database);
            let storage = Storage::open(&db_path)?;
            let comment_text = text.join(" ");
            let comment_id = storage.add_comment(&id, "cli", &comment_text)?;
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
) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    let statuses: Vec<Status> = status.iter()
        .filter_map(|s| Status::from_str(s).ok())
        .collect();
    let types: Vec<IssueType> = type_.iter()
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
    print!("{}", formatter.format_issues(&issues));

    Ok(())
}

fn cmd_stats(
    beads_dir: &PathBuf,
    by_type: bool,
    by_priority: bool,
    by_assignee: bool,
    by_label: bool,
    format: &str,
) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;
    let stats = storage.get_stats()?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&stats)?);
        }
        _ => {
            println!("Total beads: {}", stats.total);
            println!("  Open: {}", stats.open);
            println!("  In Progress: {}", stats.in_progress);
            println!("  Closed: {}", stats.closed);
        }
    }

    if by_type {
        println!("\nBy type: (not yet implemented)");
    }
    if by_priority {
        println!("\nBy priority: (not yet implemented)");
    }
    if by_assignee {
        println!("\nBy assignee: (not yet implemented)");
    }
    if by_label {
        let labels = storage.list_all_labels()?;
        println!("\nBy label:");
        for (label, count) in labels {
            println!("  {} ({})", label, count);
        }
    }

    Ok(())
}

fn cmd_schema(target: &str, _format: &str) -> Result<()> {
    match target {
        "all" => {
            println!("Schema for all: (use 'json' format for actual schema)");
        }
        _ => {
            println!("Schema for {}: (not yet implemented)", target);
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
            let value = match key.as_str() {
                "issue_prefixes" => format!("{:?}", cfg.issue_prefixes),
                "default_priority" => cfg.default_priority.to_string(),
                "default_type" => cfg.default_type,
                "claim_ttl_minutes" => cfg.claim_ttl_minutes.to_string(),
                _ => return Err(anyhow!("Unknown config key: {}", key)),
            };
            println!("{}", value);
        }
        ConfigCommands::Path => {
            let config_path = beads_dir.join("config.yaml");
            println!("{}", config_path.display());
        }
    }
    Ok(())
}

fn cmd_velocity(
    _beads_dir: &PathBuf,
    _model: Option<String>,
    _harness: Option<String>,
    _format: &str,
) -> Result<()> {
    println!("Velocity stats: (not yet implemented)");
    Ok(())
}

fn cmd_annotate(beads_dir: &PathBuf, annotate: AnnotateCommands) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    match annotate {
        AnnotateCommands::Set { id, key, value } => {
            storage.set_annotation(&id, &key, &value)?;
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
            println!("Cleared all annotations from {}", id);
        }
    }
    Ok(())
}

fn cmd_log(beads_dir: &PathBuf, id: &str, _limit: Option<usize>, format: &str) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let _storage = Storage::open(&db_path)?;

    // For now, we'll just return a placeholder
    match format {
        "json" => {
            println!("{}", serde_json::json!({"events": []}));
        }
        _ => {
            println!("Event log for {} (not yet implemented)", id);
        }
    }
    Ok(())
}

fn cmd_critical_path(beads_dir: &PathBuf, id: &str, _max_depth: usize, format: &str) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let _storage = Storage::open(&db_path)?;

    // For now, we'll just return a placeholder
    match format {
        "json" => {
            println!("{}", serde_json::json!({"critical_path": []}));
        }
        _ => {
            println!("Critical path for {} (not yet implemented)", id);
        }
    }
    Ok(())
}

fn cmd_rotate(beads_dir: &PathBuf, days: u64, dry_run: bool) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let _storage = Storage::open(&db_path)?;

    if dry_run {
        println!("Dry run: would rotate closed beads older than {} days", days);
    } else {
        println!("Rotate: closed beads older than {} days (not yet implemented)", days);
    }
    Ok(())
}
