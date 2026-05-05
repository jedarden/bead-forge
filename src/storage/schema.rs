//! Database schema definitions matching beads_rust (br) for interoperability.

use rusqlite::Connection;

/// The complete SQL schema for the beads database.
/// Schema matches beads_rust (br) for interoperability.
pub const SCHEMA_SQL: &str = r"
    -- Issues table
    -- Note: TEXT fields use DEFAULT '' for bd (Go) compatibility.
    -- bd's sql.Scan doesn't handle NULL well when scanning into string fields.
    -- Closed-at invariant is enforced by the CHECK clause below.
    CREATE TABLE IF NOT EXISTS issues (
        id TEXT PRIMARY KEY,
        content_hash TEXT,
        title TEXT NOT NULL CHECK(length(title) <= 500),
        description TEXT NOT NULL DEFAULT '',
        design TEXT NOT NULL DEFAULT '',
        acceptance_criteria TEXT NOT NULL DEFAULT '',
        notes TEXT NOT NULL DEFAULT '',
        status TEXT NOT NULL DEFAULT 'open',
        priority INTEGER NOT NULL DEFAULT 2 CHECK(priority >= 0 AND priority <= 4),
        issue_type TEXT NOT NULL DEFAULT 'task',
        assignee TEXT,
        owner TEXT DEFAULT '',
        estimated_minutes INTEGER,
        created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
        created_by TEXT DEFAULT '',
        updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
        closed_at DATETIME,
        close_reason TEXT DEFAULT '',
        closed_by_session TEXT DEFAULT '',
        due_at DATETIME,
        defer_until DATETIME,
        external_ref TEXT,
        source_system TEXT DEFAULT '',
        source_repo TEXT NOT NULL DEFAULT '.',
        deleted_at DATETIME,
        deleted_by TEXT DEFAULT '',
        delete_reason TEXT DEFAULT '',
        original_type TEXT DEFAULT '',
        compaction_level INTEGER DEFAULT 0,
        compacted_at DATETIME,
        compacted_at_commit TEXT,
        original_size INTEGER,
        sender TEXT DEFAULT '',
        ephemeral INTEGER NOT NULL DEFAULT 0,
        pinned INTEGER NOT NULL DEFAULT 0,
        is_template INTEGER NOT NULL DEFAULT 0,
        CHECK (
            (status = 'closed' AND closed_at IS NOT NULL) OR
            (status = 'tombstone') OR
            (status NOT IN ('closed', 'tombstone') AND closed_at IS NULL)
        )
    );

    -- Primary access patterns
    CREATE INDEX IF NOT EXISTS idx_issues_status ON issues(status);
    CREATE INDEX IF NOT EXISTS idx_issues_priority ON issues(priority);
    CREATE INDEX IF NOT EXISTS idx_issues_issue_type ON issues(issue_type);
    CREATE INDEX IF NOT EXISTS idx_issues_assignee ON issues(assignee) WHERE assignee IS NOT NULL;
    CREATE INDEX IF NOT EXISTS idx_issues_created_at ON issues(created_at);
    CREATE INDEX IF NOT EXISTS idx_issues_updated_at ON issues(updated_at);

    -- Export/sync patterns
    CREATE INDEX IF NOT EXISTS idx_issues_content_hash ON issues(content_hash);
    CREATE INDEX IF NOT EXISTS idx_issues_external_ref ON issues(external_ref) WHERE external_ref IS NOT NULL;
    CREATE UNIQUE INDEX IF NOT EXISTS idx_issues_external_ref_unique ON issues(external_ref) WHERE external_ref IS NOT NULL;

    -- Special states
    CREATE INDEX IF NOT EXISTS idx_issues_ephemeral ON issues(ephemeral) WHERE ephemeral = 1;
    CREATE INDEX IF NOT EXISTS idx_issues_pinned ON issues(pinned) WHERE pinned = 1;
    CREATE INDEX IF NOT EXISTS idx_issues_tombstone ON issues(status) WHERE status = 'tombstone';

    -- Time-based
    CREATE INDEX IF NOT EXISTS idx_issues_due_at ON issues(due_at) WHERE due_at IS NOT NULL;
    CREATE INDEX IF NOT EXISTS idx_issues_defer_until ON issues(defer_until) WHERE defer_until IS NOT NULL;

    -- Ready work composite index (most important for performance)
    CREATE INDEX IF NOT EXISTS idx_issues_ready
        ON issues(status, priority, created_at)
        WHERE status = 'open'
        AND ephemeral = 0
        AND pinned = 0
        AND is_template = 0;

    -- Common active list path: non-terminal issues sorted by priority/created_at
    CREATE INDEX IF NOT EXISTS idx_issues_list_active_order
        ON issues(priority, created_at DESC)
        WHERE status NOT IN ('closed', 'tombstone')
        AND (is_template = 0 OR is_template IS NULL);

    -- Dependencies
    CREATE TABLE IF NOT EXISTS dependencies (
        issue_id TEXT NOT NULL,
        depends_on_id TEXT NOT NULL,
        type TEXT NOT NULL DEFAULT 'blocks',
        created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
        created_by TEXT NOT NULL DEFAULT '',
        metadata TEXT DEFAULT '{}',
        thread_id TEXT DEFAULT '',
        PRIMARY KEY (issue_id, depends_on_id),
        FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
        -- Note: depends_on_id FK intentionally removed to allow external issue references
    );
    CREATE INDEX IF NOT EXISTS idx_dependencies_issue ON dependencies(issue_id);
    CREATE INDEX IF NOT EXISTS idx_dependencies_depends_on ON dependencies(depends_on_id);
    CREATE INDEX IF NOT EXISTS idx_dependencies_type ON dependencies(type);
    CREATE INDEX IF NOT EXISTS idx_dependencies_depends_on_type ON dependencies(depends_on_id, type);
    CREATE INDEX IF NOT EXISTS idx_dependencies_thread ON dependencies(thread_id) WHERE thread_id != '';
    -- Composite for blocking lookups
    CREATE INDEX IF NOT EXISTS idx_dependencies_blocking
        ON dependencies(depends_on_id, issue_id)
        WHERE (type = 'blocks' OR type = 'parent-child' OR type = 'conditional-blocks' OR type = 'waits-for');

    -- Labels
    CREATE TABLE IF NOT EXISTS labels (
        issue_id TEXT NOT NULL,
        label TEXT NOT NULL,
        PRIMARY KEY (issue_id, label),
        FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_labels_label ON labels(label);
    CREATE INDEX IF NOT EXISTS idx_labels_issue ON labels(issue_id);

    -- Comments
    CREATE TABLE IF NOT EXISTS comments (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        issue_id TEXT NOT NULL,
        author TEXT NOT NULL,
        text TEXT NOT NULL,
        created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_comments_issue ON comments(issue_id);
    CREATE INDEX IF NOT EXISTS idx_comments_created_at ON comments(created_at);

    -- Events (Audit)
    CREATE TABLE IF NOT EXISTS events (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        issue_id TEXT NOT NULL,
        event_type TEXT NOT NULL,
        actor TEXT NOT NULL DEFAULT '',
        old_value TEXT,
        new_value TEXT,
        comment TEXT,
        created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_events_issue ON events(issue_id);
    CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
    CREATE INDEX IF NOT EXISTS idx_events_created_at ON events(created_at);
    CREATE INDEX IF NOT EXISTS idx_events_actor ON events(actor) WHERE actor != '';

    -- Config (Runtime)
    -- NOTE: Avoid PRIMARY KEY/UNIQUE constraints here because the current
    -- storage engine does not reliably maintain unique autoindexes.
    -- Application code enforces key replacement via DELETE + INSERT.
    CREATE TABLE IF NOT EXISTS config (
        key TEXT NOT NULL,
        value TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_config_key ON config(key);

    -- Metadata
    -- Same rationale as config: keep it as key-value with explicit index.
    CREATE TABLE IF NOT EXISTS metadata (
        key TEXT NOT NULL,
        value TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_metadata_key ON metadata(key);

    -- Dirty Issues (for export)
    CREATE TABLE IF NOT EXISTS dirty_issues (
        issue_id TEXT PRIMARY KEY,
        marked_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_dirty_issues_marked_at ON dirty_issues(marked_at);

    -- Export Hashes (for incremental export)
    CREATE TABLE IF NOT EXISTS export_hashes (
        issue_id TEXT PRIMARY KEY,
        content_hash TEXT NOT NULL,
        exported_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
    );

    -- Blocked Issues Cache (Materialized view)
    -- Rebuilt on dependency or status changes.
    -- `blocked_by` stores a JSON array of blocking issue IDs.
    CREATE TABLE IF NOT EXISTS blocked_issues_cache (
        issue_id TEXT PRIMARY KEY,
        blocked_by TEXT NOT NULL,
        blocked_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_blocked_cache_blocked_at ON blocked_issues_cache(blocked_at);

    -- Child Counters (for hierarchical IDs like bd-abc.1, bd-abc.2)
    CREATE TABLE IF NOT EXISTS child_counters (
        parent_id TEXT PRIMARY KEY,
        last_child INTEGER NOT NULL DEFAULT 0,
        FOREIGN KEY (parent_id) REFERENCES issues(id) ON DELETE CASCADE
    );

    -- Recovery Sessions (for anomaly/recovery audit trail)
    CREATE TABLE IF NOT EXISTS recovery_sessions (
        correlation_id TEXT PRIMARY KEY,
        started_at TEXT NOT NULL,
        completed_at TEXT,
        final_outcome TEXT,
        session_type TEXT NOT NULL,
        trigger_reason TEXT,
        workspace_path TEXT,
        summary TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_recovery_sessions_started ON recovery_sessions(started_at);
    CREATE INDEX IF NOT EXISTS idx_recovery_sessions_outcome ON recovery_sessions(final_outcome);
    CREATE INDEX IF NOT EXISTS idx_recovery_sessions_type ON recovery_sessions(session_type);

    -- Anomaly Audit Records (for anomaly/recovery event tracking)
    CREATE TABLE IF NOT EXISTS anomaly_audit (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        correlation_id TEXT NOT NULL,
        timestamp TEXT NOT NULL,
        event_type TEXT NOT NULL,
        anomaly_type TEXT,
        recovery_strategy TEXT,
        outcome TEXT,
        evidence TEXT,
        metadata TEXT,
        created_at DATETIME NOT NULL DEFAULT (CURRENT_TIMESTAMP),
        FOREIGN KEY (correlation_id) REFERENCES recovery_sessions(correlation_id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_anomaly_audit_correlation ON anomaly_audit(correlation_id);
    CREATE INDEX IF NOT EXISTS idx_anomaly_audit_timestamp ON anomaly_audit(timestamp);
    CREATE INDEX IF NOT EXISTS idx_anomaly_audit_event_type ON anomaly_audit(event_type);
    CREATE INDEX IF NOT EXISTS idx_anomaly_audit_anomaly_type ON anomaly_audit(anomaly_type);
    CREATE INDEX IF NOT EXISTS idx_anomaly_audit_outcome ON anomaly_audit(outcome);

    -- Critical Path Cache (for impact-weighted claim scoring)
    -- Computed from dependency graph; lower float = more critical (0 = on critical path)
    CREATE TABLE IF NOT EXISTS critical_path_cache (
        bead_id     TEXT PRIMARY KEY,
        epic_id     TEXT REFERENCES issues(id),
        es          INTEGER NOT NULL,   -- earliest start (hops from root)
        ls          INTEGER NOT NULL,   -- latest start
        float       INTEGER NOT NULL,   -- ls - es; 0 = critical path
        updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY (bead_id) REFERENCES issues(id) ON DELETE CASCADE
    );

    -- Bead Annotations (bf-only table, never touched by br)
    -- Stores arbitrary key-value metadata per bead.
    -- IMPORTANT: This is a SEPARATE table (not a column on issues) because br's
    -- issues_column_order_matches() check triggers rebuild_issues_table() when
    -- the column count differs, which would silently destroy any extra column.
    CREATE TABLE IF NOT EXISTS bead_annotations (
        bead_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
        key     TEXT NOT NULL,
        value   TEXT NOT NULL,
        PRIMARY KEY (bead_id, key)
    );
    CREATE INDEX IF NOT EXISTS idx_bead_annotations_key_value
        ON bead_annotations (key, value);

    -- Worker Sessions (bf-only table for multi-workspace claiming)
    -- Tracks worker metadata (model, harness, version) for each claim operation.
    -- Used by velocity-aware scoring and audit trails.
    CREATE TABLE IF NOT EXISTS worker_sessions (
        worker_id        TEXT NOT NULL,
        model            TEXT,
        harness          TEXT,
        harness_version  TEXT,
        claimed_at       DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
        bead_id          TEXT REFERENCES issues(id) ON DELETE SET NULL,
        workspace_path   TEXT NOT NULL,
        PRIMARY KEY (worker_id, claimed_at)
    );
    CREATE INDEX IF NOT EXISTS idx_worker_sessions_worker ON worker_sessions(worker_id);
    CREATE INDEX IF NOT EXISTS idx_worker_sessions_model ON worker_sessions(model);
    CREATE INDEX IF NOT EXISTS idx_worker_sessions_harness ON worker_sessions(harness);

    -- Velocity Stats (bf-only table for performance-aware claim scoring)
    -- Aggregated statistics per (model, harness, issue_type) tuple.
    -- Updated on bead close to inform velocity-aware scoring.
    CREATE TABLE IF NOT EXISTS velocity_stats (
        model           TEXT NOT NULL,
        harness         TEXT NOT NULL,
        issue_type      TEXT NOT NULL,
        sample_count    INTEGER DEFAULT 0,
        p50_seconds     INTEGER,
        p90_seconds     INTEGER,
        avg_seconds     REAL,
        last_updated    DATETIME,
        PRIMARY KEY (model, harness, issue_type)
    );
    CREATE INDEX IF NOT EXISTS idx_velocity_stats_last_updated ON velocity_stats(last_updated);

    -- Migration Lock (bf-only table for coordinating workspace migrations)
    -- Singleton table that prevents new claims during migration operations.
    -- Checked by bf claim at the start of each BEGIN IMMEDIATE transaction.
    CREATE TABLE IF NOT EXISTS migration_lock (
        id          INTEGER PRIMARY KEY CHECK (id = 1),
        locked_by   TEXT NOT NULL,
        locked_at   DATETIME NOT NULL,
        expires_at  DATETIME NOT NULL
    );
";

/// Split a SQL script into individual statements, respecting string literals,
/// quoted identifiers, and comments.
///
/// A naive `split(';')` breaks when SQL string literals contain semicolons
/// (e.g., `INSERT INTO t(v) VALUES('a;b')`). This function uses a small state
/// machine to track whether the current position is inside:
/// - A single-quoted string literal (`'...'`, with `''` as escape)
/// - A double-quoted identifier (`"..."`, with `""` as escape)
/// - A line comment (`-- ...`)
/// - A block comment (`/* ... */`)
///
/// Only semicolons at the top level (outside all of the above) are treated as
/// statement terminators.
fn split_sql_statements(sql: &str) -> Vec<&str> {
    let bytes = sql.as_bytes();
    let len = bytes.len();
    let mut stmts = Vec::new();
    let mut start = 0; // byte offset where the current statement begins
    let mut i = 0;

    // State flags — at most one is true at a time.
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while i < len {
        let b = bytes[i];

        // --- Line comment state ---
        if in_line_comment {
            if b == b'\n' {
                in_line_comment = false;
            }
            i += 1;
            continue;
        }

        // --- Block comment state ---
        if in_block_comment {
            if b == b'*' && i + 1 < len && bytes[i + 1] == b'/' {
                in_block_comment = false;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }

        // --- Single-quoted string state ---
        if in_single_quote {
            if b == b'\'' {
                // '' is an escaped quote inside a string literal
                if i + 1 < len && bytes[i + 1] == b'\'' {
                    i += 2;
                } else {
                    in_single_quote = false;
                    i += 1;
                }
            } else {
                i += 1;
            }
            continue;
        }

        // --- Double-quoted identifier state ---
        if in_double_quote {
            if b == b'"' {
                if i + 1 < len && bytes[i + 1] == b'"' {
                    i += 2;
                } else {
                    in_double_quote = false;
                    i += 1;
                }
            } else {
                i += 1;
            }
            continue;
        }

        // --- Top-level parsing ---
        if b == b'\'' {
            in_single_quote = true;
            i += 1;
        } else if b == b'"' {
            in_double_quote = true;
            i += 1;
        } else if b == b'-' && i + 1 < len && bytes[i + 1] == b'-' {
            in_line_comment = true;
            i += 2;
        } else if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            in_block_comment = true;
            i += 2;
        } else if b == b';' {
            // Statement terminator at top level.
            let stmt = &sql[start..i];
            if !stmt.trim().is_empty() {
                stmts.push(stmt.trim());
            }
            start = i + 1;
            i += 1;
        } else {
            i += 1;
        }
    }

    // Trailing statement without a final semicolon.
    if start < len {
        let stmt = &sql[start..len];
        if !stmt.trim().is_empty() {
            stmts.push(stmt.trim());
        }
    }

    stmts
}

/// Execute multiple SQL statements separated by semicolons.
fn execute_batch(conn: &Connection, sql: &str) -> anyhow::Result<()> {
    conn.execute_batch(sql)?;
    Ok(())
}

pub fn apply_schema(conn: &Connection) -> anyhow::Result<()> {
    execute_batch(conn, SCHEMA_SQL)?;
    apply_migrations(conn)?;
    Ok(())
}

/// Apply database migrations for schema changes.
/// This handles adding new columns to existing tables.
fn apply_migrations(conn: &Connection) -> anyhow::Result<()> {
    // Migration 1: Update critical_path_cache schema if it has the old format
    // Old format: bead_id, float (REAL), updated_at
    // New format: bead_id, epic_id, es (INTEGER), ls (INTEGER), float (INTEGER), updated_at
    let has_es: bool = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('critical_path_cache') WHERE name='es'",
        [],
        |row| row.get::<_, i64>(0).map(|n| n > 0),
    ).unwrap_or(false);

    if !has_es {
        // Table has old schema - recreate with new schema
        conn.execute_batch(
            "BEGIN;
             CREATE TABLE IF NOT EXISTS critical_path_cache_new (
                 bead_id     TEXT PRIMARY KEY,
                 epic_id     TEXT,
                 es          INTEGER NOT NULL DEFAULT 0,
                 ls          INTEGER NOT NULL DEFAULT 0,
                 float       INTEGER NOT NULL DEFAULT 0,
                 updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 FOREIGN KEY (bead_id) REFERENCES issues(id) ON DELETE CASCADE
             );
             INSERT INTO critical_path_cache_new (bead_id, float, updated_at)
                 SELECT bead_id, CAST(float AS INTEGER), updated_at FROM critical_path_cache;
             DROP TABLE critical_path_cache;
             ALTER TABLE critical_path_cache_new RENAME TO critical_path_cache;
             COMMIT;"
        )?;
    }

    // Create indexes for critical_path_cache (idempotent)
    let _ = conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_critical_path_cache_epic ON critical_path_cache(epic_id);
         CREATE INDEX IF NOT EXISTS idx_critical_path_cache_float ON critical_path_cache(float);"
    );

    Ok(())
}

pub fn ensure_wal_mode(conn: &Connection) -> anyhow::Result<()> {
    // Use execute_batch so PRAGMAs that return rows don't cause errors
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 30000;
         PRAGMA cache_size = -8000;
         PRAGMA synchronous = NORMAL;"
    )?;
    Ok(())
}
