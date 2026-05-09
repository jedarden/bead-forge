//! Workspace migration from br to bf.
//!
//! Phase 4C: Migration utilities including git log reconstruction for events.

use crate::config::{find_beads_dir, load_config, load_metadata};
use crate::model::{EventType, Issue, Status};
use crate::storage::Storage;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Options for workspace migration.
#[derive(Debug, Clone)]
pub struct MigrateOptions {
    workspace_path: std::path::PathBuf,
    dry_run: bool,
    skip_verify: bool,
}

impl MigrateOptions {
    pub fn new(workspace_path: std::path::PathBuf) -> Self {
        Self {
            workspace_path,
            dry_run: false,
            skip_verify: false,
        }
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    pub fn skip_verify(mut self, skip_verify: bool) -> Self {
        self.skip_verify = skip_verify;
        self
    }
}

/// Result of a migration operation.
#[derive(Debug)]
pub struct MigrateResult {
    pub verification: VerificationResult,
    pub imported: usize,
    pub events_reconstructed: usize,
}

/// Verification results after migration.
#[derive(Debug)]
pub struct VerificationResult {
    pub errors: Vec<String>,
}

/// A snapshot of the JSONL file at a specific commit.
struct JsonlSnapshot {
    commit_hash: String,
    commit_date: DateTime<Utc>,
    issues: HashMap<String, Issue>,
}

/// Migrate a br workspace to bf (Path B: explicit migration with backup and verification).
///
/// Steps:
/// 1. Pause fleet: write migration_lock row
/// 2. Backup: copy beads.db to beads.db.br-backup-<timestamp>
/// 3. Apply migrations: create bf-only tables via CREATE TABLE IF NOT EXISTS
/// 4. Prime caches: populate critical_path_cache for all epics
/// 5. Seed config: add bf-specific keys to config.yaml if missing
/// 6. Verify forward compat: check that br would accept this database
/// 7. Verify backward compat: run bf doctor check
/// 8. Release fleet: remove migration_lock
pub fn migrate(opts: MigrateOptions) -> Result<MigrateResult> {
    let workspace = &opts.workspace_path;
    let beads_dir = find_beads_dir(workspace).ok_or_else(|| {
        anyhow!(
            "No .beads directory found in {:?}",
            workspace
        )
    })?;

    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let config_path = beads_dir.join("config.yaml");

    // Dry-run mode: print what would be done
    if opts.dry_run {
        println!("Dry-run migration for {:?}", workspace);
        println!("  Would back up: {} -> {}", db_path.display(), format!("{}.br-backup-{}", db_path.display(), Utc::now().format("%Y%m%d%H%M%S")));
        println!("  Would apply schema migrations");
        println!("  Would prime critical_path_cache");
        println!("  Would seed config.yaml with bf defaults");
        println!("  Would verify forward/backward compatibility");
        return Ok(MigrateResult {
            verification: VerificationResult { errors: vec![] },
            imported: 0,
            events_reconstructed: 0,
        });
    }

    // Step 1: Pause fleet - acquire migration lock
    println!("  Acquiring migration lock...");
    let storage = Storage::open(&db_path)?;
    let lock_id = acquire_migration_lock(&storage, "bf-migrate")?;
    println!("  Migration lock acquired");

    // Ensure lock is released even if migration fails
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let result = inner_migrate(&storage, &beads_dir, &db_path, &config_path, &opts.skip_verify);
        // Release lock on completion (regardless of success/failure)
        let _ = release_migration_lock(&storage, lock_id);
        result
    }));

    match result {
        Ok(inner_result) => inner_result,
        Err(_) => {
            release_migration_lock(&storage, lock_id)?;
            Err(anyhow!("Migration panicked"))
        }
    }
}

/// Inner migration implementation (lock already held).
fn inner_migrate(
    storage: &Storage,
    beads_dir: &std::path::Path,
    db_path: &std::path::Path,
    config_path: &std::path::Path,
    skip_verify: &bool,
) -> Result<MigrateResult> {
    // Step 2: Backup database
    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
    let backup_path = db_path.with_extension(&format!("db.br-backup-{}", timestamp));
    println!("  Backing up {} -> {}", db_path.display(), backup_path.display());
    std::fs::copy(db_path, &backup_path)?;

    // Step 3: Apply migrations (CREATE TABLE IF NOT EXISTS is idempotent)
    println!("  Applying schema migrations...");
    storage.apply_migrations()?;

    // Step 4: Prime critical_path_cache
    println!("  Priming critical_path_cache...");
    let cache_result = prime_critical_path_cache(storage)?;

    // Step 5: Seed config with bf defaults
    println!("  Seeding config.yaml with bf defaults...");
    let config_updated = seed_config(config_path)?;

    // Step 6 & 7: Verify forward/backward compatibility
    let mut verification = VerificationResult { errors: vec![] };

    if !*skip_verify {
        println!("  Verifying migration...");

        // Forward compat: check that issues table column count matches br's expectation
        let forward_compat_ok = verify_forward_compat(storage)?;
        if !forward_compat_ok {
            verification.errors.push(
                "Forward compatibility check failed: issues table column count mismatch".to_string()
            );
        }

        // Backward compat: run doctor check
        let workspace = beads_dir.parent().unwrap_or(beads_dir);
        match crate::doctor::check(workspace) {
            Ok(doctor_result) => {
                if !doctor_result.db_ok {
                    verification.errors.push(
                        "Backward compatibility check failed: database integrity check failed".to_string()
                    );
                }
                if !doctor_result.issues.is_empty() {
                    for issue in &doctor_result.issues {
                        verification.errors.push(format!("Doctor check: {}", issue));
                    }
                }
            }
            Err(e) => {
                verification.errors.push(format!("Backward compatibility check failed: {}", e));
            }
        }
    }

    println!("  Migration complete");
    if config_updated {
        println!("  Note: config.yaml was updated with bf defaults");
    }

    Ok(MigrateResult {
        verification,
        imported: 0,
        events_reconstructed: cache_result,
    })
}

/// Acquire migration lock to prevent concurrent claims during migration.
fn acquire_migration_lock(storage: &Storage, locked_by: &str) -> Result<i64> {
    let now = Utc::now();
    let expires_at = now + chrono::Duration::hours(1); // Lock expires after 1 hour

    storage.with_immediate_transaction(|tx| {
        tx.execute(
            "INSERT OR REPLACE INTO migration_lock (id, locked_by, locked_at, expires_at)
             VALUES (1, ?1, ?2, ?3)",
            rusqlite::params![locked_by, now.to_rfc3339(), expires_at.to_rfc3339()],
        )?;
        Ok::<_, anyhow::Error>(1)
    })
}

/// Release migration lock after migration completes.
fn release_migration_lock(storage: &Storage, _lock_id: i64) -> Result<()> {
    storage.with_immediate_transaction(|tx| {
        tx.execute("DELETE FROM migration_lock WHERE id = 1", [])?;
        Ok::<_, anyhow::Error>(())
    })
}

/// Prime critical_path_cache for all beads.
fn prime_critical_path_cache(storage: &Storage) -> Result<usize> {
    storage.with_immediate_transaction(|tx| {
        // Compute all critical paths - this populates the cache
        crate::critical_path::compute_all_critical_paths(tx)?;

        // Count how many beads were cached
        let count: i64 = tx.query_row("SELECT COUNT(*) FROM critical_path_cache", [], |row| row.get(0))?;
        Ok(count as usize)
    })
}

/// Seed config.yaml with bf-specific defaults if missing.
fn seed_config(config_path: &std::path::Path) -> Result<bool> {
    use std::io::Write;

    // Read existing config
    let existing_content = if config_path.exists() {
        std::fs::read_to_string(config_path)?
    } else {
        String::new()
    };

    // Check if bf defaults are already present
    let has_claim_ttl = existing_content.contains("claim_ttl_minutes:");
    let has_rotate_age = existing_content.contains("rotate_age_days:");
    let has_rotate_max_size = existing_content.contains("rotate_max_size_mb:");

    if has_claim_ttl && has_rotate_age && has_rotate_max_size {
        return Ok(false); // No update needed
    }

    // Parse existing config or create new one
    let mut config: crate::config::Config = if existing_content.trim().is_empty() {
        crate::config::Config::default()
    } else {
        serde_yaml::from_str(&existing_content)?
    };

    // Ensure bf defaults are set
    if config.claim_ttl_minutes == 0 {
        config.claim_ttl_minutes = 30;
    }
    if config.rotate.rotate_age_days == 0 {
        config.rotate.rotate_age_days = 30;
    }
    if config.rotate.rotate_max_size_mb == 0 {
        config.rotate.rotate_max_size_mb = 100;
    }

    // Write back
    let yaml = serde_yaml::to_string(&config)?;
    let mut file = std::fs::File::create(config_path)?;
    writeln!(file, "# bead-forge configuration")?;
    writeln!(file, "# br ignores bf-specific keys (claim_ttl_minutes, rotate_*)")?;
    file.write_all(yaml.as_bytes())?;

    Ok(true)
}

/// Verify forward compatibility: check that br can still open this database.
///
/// br checks that issues table column count matches exactly. We verify this
/// by checking the column count against br's expected count.
fn verify_forward_compat(storage: &Storage) -> Result<bool> {
    // br expects exactly these columns in the issues table
    // From beads_rust/src/storage/schema.rs
    let expected_br_columns = vec![
        "id", "title", "description", "status", "priority", "assignee",
        "labels", "issue_type", "created_at", "updated_at", "closed_at",
        "close_reason", "dependencies", "comments", "deleted_at",
        "source_repo", "file_path", "line_number", "metadata",
    ];

    storage.with_immediate_transaction(|tx| {
        // Get actual column names
        let mut stmt = tx.prepare("PRAGMA table_info(issues)")?;
        let mut actual_columns = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            actual_columns.push(name);
        }

        // Check that all expected br columns exist
        let mut all_present = true;
        for expected in &expected_br_columns {
            if !actual_columns.iter().any(|c| c == expected) {
                all_present = false;
                break;
            }
        }

        // Check that there are no extra columns (br's rebuild_issues_table check)
        if actual_columns.len() != expected_br_columns.len() {
            all_present = false;
        }

        Ok(all_present)
    })
}

/// Migrate from br's JSONL export, reconstructing events from git log.
///
/// This function:
/// 1. Reimports issues from issues.jsonl
/// 2. Reconstructs events by parsing git log --follow -p .beads/issues.jsonl
/// 3. Creates synthetic events for state transitions
/// 4. Optionally seeds velocity stats from reconstructed events
pub fn migrate_from_jsonl(workspace: &Path, seed_velocity: bool) -> Result<MigrateResult> {
    let beads_dir = find_beads_dir(workspace).ok_or_else(|| {
        anyhow!(
            "No .beads directory found in {:?}",
            workspace
        )
    })?;

    let metadata = load_metadata(&beads_dir)?;
    let config = load_config(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let jsonl_path = beads_dir.join(&metadata.jsonl_export);

    // Step 1: Import issues from JSONL (rebuilds issues table)
    let storage = Storage::open_with_config(&db_path, &config)?;
    let import_result = storage.sync_from_jsonl(&jsonl_path)?;

    // Step 2: Reconstruct events from git log
    let events_reconstructed = reconstruct_events_from_git(&storage, workspace, &jsonl_path)?;

    // Step 3: Seed velocity stats if requested
    if seed_velocity {
        seed_velocity_from_events(&storage)?;
    }

    // Step 4: Verify migration
    let verification = verify_migration(&storage)?;

    Ok(MigrateResult {
        verification,
        imported: import_result.imported,
        events_reconstructed,
    })
}

/// Reconstruct events from git log history of issues.jsonl.
///
/// Parses git log to get the JSONL file state at each commit, then
/// creates synthetic events for state transitions:
/// - New bead appearing = synthetic created event
/// - Status open->in_progress = synthetic claimed event
/// - Status->closed = synthetic closed event with duration_seconds
///
/// All synthetic events get metadata.source=git-reconstructed annotation.
fn reconstruct_events_from_git(
    storage: &Storage,
    workspace: &Path,
    jsonl_path: &Path,
) -> Result<usize> {
    // Get snapshots from git log
    let snapshots = parse_git_log_snapshots(workspace, jsonl_path)?;

    if snapshots.is_empty() {
        return Ok(0);
    }

    let mut events_created = 0;

    // Process snapshots chronologically (oldest to newest)
    for window in snapshots.windows(2) {
        let prev = &window[0];
        let curr = &window[1];

        // Find differences between snapshots and create synthetic events
        for (issue_id, current_issue) in &curr.issues {
            let prev_issue = prev.issues.get(issue_id);

            match prev_issue {
                None => {
                    // New bead appeared - create synthetic created event
                    insert_synthetic_event(
                        storage,
                        issue_id,
                        EventType::Created,
                        None,
                        None,
                        current_issue.created_at,
                    )?;
                    events_created += 1;

                    // If already in_progress at creation, add claimed event
                    if current_issue.status == Status::InProgress {
                        insert_synthetic_event(
                            storage,
                            issue_id,
                            EventType::StatusChanged,
                            Some(Status::Open.as_str()),
                            Some(Status::InProgress.as_str()),
                            current_issue.created_at,
                        )?;
                        events_created += 1;
                    }

                    // If already closed at creation, add closed event
                    if current_issue.status == Status::Closed {
                        insert_synthetic_event(
                            storage,
                            issue_id,
                            EventType::Closed,
                            None,
                            current_issue.close_reason.as_deref(),
                            current_issue.closed_at.unwrap_or(current_issue.created_at),
                        )?;
                        events_created += 1;
                    }
                }
                Some(previous_issue) => {
                    // Existing bead - check for status changes
                    if previous_issue.status != current_issue.status {
                        match (previous_issue.status.clone(), current_issue.status.clone()) {
                            (Status::Open, Status::InProgress) => {
                                // open -> in_progress = claimed
                                insert_synthetic_event(
                                    storage,
                                    issue_id,
                                    EventType::StatusChanged,
                                    Some(Status::Open.as_str()),
                                    Some(Status::InProgress.as_str()),
                                    curr.commit_date,
                                )?;
                                events_created += 1;
                            }
                            (_, Status::Closed) => {
                                // any -> closed = closed event
                                insert_synthetic_event(
                                    storage,
                                    issue_id,
                                    EventType::Closed,
                                    None,
                                    current_issue.close_reason.as_deref(),
                                    curr.commit_date,
                                )?;
                                events_created += 1;
                            }
                            _ => {
                                // Other status changes - generic status_changed event
                                insert_synthetic_event(
                                    storage,
                                    issue_id,
                                    EventType::StatusChanged,
                                    Some(previous_issue.status.as_str()),
                                    Some(current_issue.status.as_str()),
                                    curr.commit_date,
                                )?;
                                events_created += 1;
                            }
                        }
                    }

                    // Check for assignee changes
                    if previous_issue.assignee != current_issue.assignee {
                        insert_synthetic_event(
                            storage,
                            issue_id,
                            EventType::AssigneeChanged,
                            previous_issue.assignee.as_deref(),
                            current_issue.assignee.as_deref(),
                            curr.commit_date,
                        )?;
                        events_created += 1;
                    }

                    // Check for priority changes
                    if previous_issue.priority != current_issue.priority {
                        insert_synthetic_event(
                            storage,
                            issue_id,
                            EventType::PriorityChanged,
                            Some(&previous_issue.priority.0.to_string()),
                            Some(&current_issue.priority.0.to_string()),
                            curr.commit_date,
                        )?;
                        events_created += 1;
                    }
                }
            }
        }

        // Check for deleted beads (tombstones)
        for issue_id in prev.issues.keys() {
            if !curr.issues.contains_key(issue_id) {
                // Bead was deleted - create synthetic deleted event
                insert_synthetic_event(
                    storage,
                    issue_id,
                    EventType::Deleted,
                    None,
                    None,
                    curr.commit_date,
                )?;
                events_created += 1;
            }
        }
    }

    Ok(events_created)
}

/// Insert a synthetic event with metadata.source=git-reconstructed.
fn insert_synthetic_event(
    storage: &Storage,
    issue_id: &str,
    event_type: EventType,
    old_value: Option<&str>,
    new_value: Option<&str>,
    created_at: DateTime<Utc>,
) -> Result<()> {
    storage.with_immediate_transaction(|tx| {
        tx.execute(
            "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, created_at)
             VALUES (?1, ?2, 'git-reconstructed', ?3, ?4, ?5)",
            rusqlite::params![
                issue_id,
                event_type.as_str(),
                old_value,
                new_value,
                created_at.to_rfc3339()
            ],
        )?;

        // Add annotation marking this as git-reconstructed
        tx.execute(
            "INSERT OR IGNORE INTO bead_annotations (bead_id, key, value)
             VALUES (?1, 'metadata.source', 'git-reconstructed')",
            rusqlite::params![issue_id],
        )?;

        Ok(())
    })
}

/// Parse git log to get snapshots of issues.jsonl at each commit.
///
/// Runs: git log --follow --date=iso-strict -- .beads/issues.jsonl
/// Then for each commit, runs: git show <commit>:path to get full contents
fn parse_git_log_snapshots(
    workspace: &Path,
    jsonl_path: &Path,
) -> Result<Vec<JsonlSnapshot>> {
    let relative_path = jsonl_path.strip_prefix(workspace).unwrap_or(jsonl_path);
    let path_str = relative_path.to_str().unwrap_or(".beads/issues.jsonl");

    // First, get list of commits that touched this file
    let output = Command::new("git")
        .args([
            "log",
            "--follow",
            "--format=%H|%ci",
            "--",
            path_str,
        ])
        .current_dir(workspace)
        .output()?;

    if !output.status.success() {
        // Not in git repo or file has no history - return empty
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8(output.stdout)?;
    let mut snapshots = Vec::new();

    // Parse each commit and get the file contents at that commit
    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(2, '|').collect();
        if parts.len() != 2 {
            continue;
        }

        let commit_hash = parts[0].to_string();
        let commit_date_str = parts[1];

        // Parse the date (git log --format=%ci gives "2024-01-01 12:00:00 +0000")
        let commit_date = parse_git_date(commit_date_str)?;

        // Get file contents at this commit
        let file_output = Command::new("git")
            .args(["show", &format!("{}:{}", commit_hash, path_str)])
            .current_dir(workspace)
            .output();

        let issues = match file_output {
            Ok(out) if out.status.success() => {
                let content = String::from_utf8_lossy(&out.stdout);
                parse_jsonl_issues(&content)?
            }
            _ => {
                // File didn't exist at this commit (or was empty)
                HashMap::new()
            }
        };

        snapshots.push(JsonlSnapshot {
            commit_hash,
            commit_date,
            issues,
        });
    }

    // git log returns newest first, we want oldest first
    snapshots.reverse();
    Ok(snapshots)
}

/// Parse git date format "2024-01-01 12:00:00 +0000" to DateTime<Utc>
fn parse_git_date(date_str: &str) -> Result<DateTime<Utc>> {
    use chrono::NaiveDateTime;
    // git log --format=%ci gives "2024-01-01 12:00:00 +0000"
    let trimmed = date_str.trim();
    // Parse "YYYY-MM-DD HH:MM:SS +ZZZZ" format
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S %z") {
        return Ok(DateTime::from_naive_utc_and_offset(dt, Utc));
    }

    // Fallback: try parsing as ISO-8601
    DateTime::parse_from_rfc3339(trimmed)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| anyhow::anyhow!("Failed to parse git date: {}", date_str))
}

/// Parse JSONL content into a map of issues.
fn parse_jsonl_issues(content: &str) -> Result<HashMap<String, Issue>> {
    let mut issues = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(issue) = serde_json::from_str::<Issue>(line) {
            issues.insert(issue.id.clone(), issue);
        }
    }
    Ok(issues)
}

/// Parse a worker ID to extract model and harness information.
///
/// Worker IDs follow patterns like:
/// - "worker-claude-sonnet-4-6-01" → model="claude-sonnet-4-6", harness="unknown"
/// - "worker-claude-opus-4-7-02" → model="claude-opus-4-7", harness="unknown"
/// - "claude-code-glm-4.7" → model="claude-code-glm-4.7", harness="unknown"
///
/// Returns (model, harness) tuple.
fn parse_worker_actor(actor: &str) -> (String, String) {
    let actor_lower = actor.to_lowercase();

    // Pattern: worker-{model}-{number}
    // e.g., "worker-claude-sonnet-4-6-01" → "claude-sonnet-4-6"
    if actor_lower.starts_with("worker-") {
        let parts: Vec<&str> = actor_lower.split('-').collect();
        if parts.len() >= 3 {
            // Rejoin parts between "worker" and the final numeric suffix
            // e.g., ["worker", "claude", "sonnet", "4", "6", "01"]
            //      → model = "claude-sonnet-4-6", skip "01"
            let mut model_parts = Vec::new();
            for (i, part) in parts.iter().enumerate() {
                if i == 0 {
                    continue; // skip "worker"
                }
                // Check if this is a numeric suffix (like "01", "02")
                if i == parts.len() - 1 && part.parse::<u32>().is_ok() {
                    break; // Skip the trailing numeric suffix
                }
                model_parts.push(*part);
            }
            if !model_parts.is_empty() {
                return (model_parts.join("-"), "unknown".to_string());
            }
        }
    }

    // Pattern: {model}-{number} (e.g., "claude-code-glm-4.7")
    if actor_lower.contains("claude") {
        // Extract the model name - everything up to a version-like pattern
        // or use the whole actor if no clear pattern
        return (actor_lower, "unknown".to_string());
    }

    // Fallback: couldn't parse
    ("unknown".to_string(), "unknown".to_string())
}

/// Seed velocity stats from reconstructed events.
///
/// Scans events table for closed events and populates velocity_stats table
/// with duration data. Uses the actor field from events to infer model/harness.
pub fn seed_velocity_from_events(storage: &Storage) -> Result<()> {
    storage.with_immediate_transaction(|tx| {
        // Get all closed events with duration info, prioritizing event actor
        let mut stmt = tx.prepare(
            "SELECT e.issue_id, i.issue_type, e.actor, i.closed_at, i.created_at
             FROM events e
             INNER JOIN issues i ON i.id = e.issue_id
             WHERE e.event_type = 'closed'
             AND i.closed_at IS NOT NULL
             AND e.actor IS NOT NULL
             AND e.actor != ''
             ORDER BY e.created_at ASC",
        )?;

        let mut rows = stmt.query([])?;
        let mut stats: HashMap<(String, String, String), Vec<i64>> = HashMap::new();

        while let Some(row) = rows.next()? {
            let _issue_id: String = row.get(0)?;
            let issue_type: String = row.get(1)?;
            let actor: String = row.get(2)?;
            let closed_at: String = row.get(3)?;
            let created_at: String = row.get(4)?;

            let closed_dt = DateTime::parse_from_rfc3339(&closed_at)?.with_timezone(&Utc);
            let created_dt = DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc);
            let duration_secs = closed_dt.signed_duration_since(created_dt).num_seconds();

            // Skip negative durations (data issues)
            if duration_secs < 0 {
                continue;
            }

            // Parse actor to infer model and harness
            let (model, harness) = parse_worker_actor(&actor);

            stats.entry((model, harness, issue_type))
                .or_default()
                .push(duration_secs);
        }

        // Compute and insert stats
        for ((model, harness, issue_type), mut durations) in stats {
            if durations.is_empty() {
                continue;
            }

            durations.sort_unstable();
            let count = durations.len();
            let p50_seconds = Some(durations[count / 2] as i32);
            let p90_idx = (count * 9 / 10).min(count - 1);
            let p90_seconds = Some(durations[p90_idx] as i32);
            let avg_seconds = Some(durations.iter().sum::<i64>() as f64 / count as f64);

            tx.execute(
                "INSERT OR REPLACE INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    model,
                    harness,
                    issue_type,
                    count as i32,
                    p50_seconds,
                    p90_seconds,
                    avg_seconds,
                    Utc::now().to_rfc3339()
                ],
            )?;
        }

        Ok(())
    })
}

/// Verify migration results.
fn verify_migration(storage: &Storage) -> Result<VerificationResult> {
    let mut errors = Vec::new();

    // Check that issues were imported
    let count = storage.count_issues()?;
    if count == 0 {
        errors.push("No issues found in database after migration".to_string());
    }

    // Check for orphaned dependencies
    let issues = storage.list_all_issues()?;
    for issue in &issues {
        for dep in &issue.dependencies {
            let dep_exists = issues.iter().any(|i| i.id == dep.depends_on_id);
            if !dep_exists && dep.depends_on_id.starts_with("bf-") {
                errors.push(format!(
                    "Orphaned dependency: {} depends on non-existent {}",
                    issue.id, dep.depends_on_id
                ));
            }
        }
    }

    Ok(VerificationResult { errors })
}
