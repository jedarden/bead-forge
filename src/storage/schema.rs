//! Database schema definitions for bead-forge.
//!
//! This module provides DDL for the 14 core tables in the bead-forge schema.
//! Each table is defined as a const function returning &str for easy composition.

use rusqlite::Connection;

/// Issues table - core bead data
/// All 36 columns with br-compatible types, defaults, and CHECK constraints.
/// Includes manual_status for derived blocked status (Phase 7.8).
pub const fn issues_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS issues (
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
    manual_status TEXT DEFAULT NULL,
    CHECK (
        (status = 'closed' AND closed_at IS NOT NULL) OR
        (status = 'tombstone') OR
        (status NOT IN ('closed', 'tombstone') AND closed_at IS NULL)
    )
)"#
}

/// Issues table indexes
pub const fn issues_indexes() -> &'static str {
    r#"CREATE INDEX IF NOT EXISTS idx_issues_status ON issues(status);
CREATE INDEX IF NOT EXISTS idx_issues_priority ON issues(priority);
CREATE INDEX IF NOT EXISTS idx_issues_issue_type ON issues(issue_type);
CREATE INDEX IF NOT EXISTS idx_issues_assignee ON issues(assignee) WHERE assignee IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_issues_created_at ON issues(created_at);
CREATE INDEX IF NOT EXISTS idx_issues_updated_at ON issues(updated_at);
CREATE INDEX IF NOT EXISTS idx_issues_content_hash ON issues(content_hash);
CREATE INDEX IF NOT EXISTS idx_issues_external_ref ON issues(external_ref) WHERE external_ref IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_issues_external_ref_unique ON issues(external_ref) WHERE external_ref IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_issues_ephemeral ON issues(ephemeral) WHERE ephemeral = 1;
CREATE INDEX IF NOT EXISTS idx_issues_pinned ON issues(pinned) WHERE pinned = 1;
CREATE INDEX IF NOT EXISTS idx_issues_tombstone ON issues(status) WHERE status = 'tombstone';
CREATE INDEX IF NOT EXISTS idx_issues_due_at ON issues(due_at) WHERE due_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_issues_defer_until ON issues(defer_until) WHERE defer_until IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_issues_ready
    ON issues(status, priority, created_at)
    WHERE status = 'open'
    AND ephemeral = 0
    AND pinned = 0
    AND is_template = 0;
CREATE INDEX IF NOT EXISTS idx_issues_list_active_order
    ON issues(priority, created_at DESC)
    WHERE status NOT IN ('closed', 'tombstone')
    AND (is_template = 0 OR is_template IS NULL);"#
}

/// Dependencies table - tracks relationships between beads
pub const fn dependencies_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS dependencies (
    issue_id TEXT NOT NULL,
    depends_on_id TEXT NOT NULL,
    type TEXT NOT NULL DEFAULT 'blocks',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by TEXT NOT NULL DEFAULT '',
    metadata TEXT DEFAULT '{}',
    thread_id TEXT DEFAULT '',
    PRIMARY KEY (issue_id, depends_on_id),
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
)"#
}

/// Dependencies table indexes
pub const fn dependencies_indexes() -> &'static str {
    r#"CREATE INDEX IF NOT EXISTS idx_dependencies_issue ON dependencies(issue_id);
CREATE INDEX IF NOT EXISTS idx_dependencies_depends_on ON dependencies(depends_on_id);
CREATE INDEX IF NOT EXISTS idx_dependencies_type ON dependencies(type);
CREATE INDEX IF NOT EXISTS idx_dependencies_depends_on_type ON dependencies(depends_on_id, type);
CREATE INDEX IF NOT EXISTS idx_dependencies_thread ON dependencies(thread_id) WHERE thread_id != '';
CREATE INDEX IF NOT EXISTS idx_dependencies_blocking
    ON dependencies(depends_on_id, issue_id)
    WHERE (type = 'blocks' OR type = 'parent-child' OR type = 'conditional-blocks' OR type = 'waits-for');"#
}

/// Comments table - discussion on beads
pub const fn comments_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS comments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    issue_id TEXT NOT NULL,
    author TEXT NOT NULL,
    text TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
)"#
}

/// Comments table indexes
pub const fn comments_indexes() -> &'static str {
    r#"CREATE INDEX IF NOT EXISTS idx_comments_issue ON comments(issue_id);
CREATE INDEX IF NOT EXISTS idx_comments_created_at ON comments(created_at);"#
}

/// Events table - audit trail for all bead mutations
pub const fn events_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    issue_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    actor TEXT NOT NULL DEFAULT '',
    old_value TEXT,
    new_value TEXT,
    comment TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
)"#
}

/// Events table indexes
pub const fn events_indexes() -> &'static str {
    r#"CREATE INDEX IF NOT EXISTS idx_events_issue ON events(issue_id);
CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_created_at ON events(created_at);
CREATE INDEX IF NOT EXISTS idx_events_actor ON events(actor) WHERE actor != '';"#
}

/// Labels table - direct issue-to-label mapping
/// Stores labels as strings attached directly to issues
pub const fn labels_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS labels (
    issue_id TEXT NOT NULL,
    label TEXT NOT NULL,
    PRIMARY KEY (issue_id, label),
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
)"#
}

/// Bead labels table - alternative label storage
/// Provides a separate table for bead-specific label storage
pub const fn bead_labels_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS bead_labels (
    bead_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    label    TEXT NOT NULL,
    PRIMARY KEY (bead_id, label)
)"#
}

/// Issue labels table - junction table for many-to-many relationship
pub const fn issue_labels_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS issue_labels (
    issue_id TEXT NOT NULL,
    label_id TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (issue_id, label_id),
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
)"#
}

/// Issue labels table indexes
pub const fn issue_labels_indexes() -> &'static str {
    r#"CREATE INDEX IF NOT EXISTS idx_issue_labels_issue_id ON issue_labels(issue_id);
CREATE INDEX IF NOT EXISTS idx_issue_labels_label_id ON issue_labels(label_id);"#
}

/// Priorities table - reference data for priority levels
pub const fn priorities_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS priorities (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    value INTEGER NOT NULL,
    color TEXT,
    description TEXT
)"#
}

/// Statuses table - reference data for status values
pub const fn statuses_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS statuses (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    category TEXT NOT NULL,
    description TEXT,
    is_terminal INTEGER NOT NULL DEFAULT 0
)"#
}

/// Issue types table - reference data for issue type values
pub const fn issue_types_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS issue_types (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    icon TEXT,
    color TEXT
)"#
}

/// Issue relations table - additional relationship types beyond dependencies
pub const fn issue_relations_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS issue_relations (
    issue_id TEXT NOT NULL,
    related_id TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by TEXT DEFAULT '',
    PRIMARY KEY (issue_id, related_id, relation_type),
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE,
    FOREIGN KEY (related_id) REFERENCES issues(id) ON DELETE CASCADE
)"#
}

/// Issue relations table indexes
pub const fn issue_relations_indexes() -> &'static str {
    r#"CREATE INDEX IF NOT EXISTS idx_issue_relations_issue_id ON issue_relations(issue_id);
CREATE INDEX IF NOT EXISTS idx_issue_relations_related_id ON issue_relations(related_id);
CREATE INDEX IF NOT EXISTS idx_issue_relations_type ON issue_relations(relation_type);"#
}

/// Assignees table - reference data for assignees
pub const fn assignees_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS assignees (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    email TEXT,
    avatar_url TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
)"#
}

/// Issue assignees table - junction table for tracking assignments
pub const fn issue_assignees_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS issue_assignees (
    issue_id TEXT NOT NULL,
    assignee_id TEXT NOT NULL,
    assigned_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    assigned_by TEXT DEFAULT '',
    PRIMARY KEY (issue_id, assignee_id),
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE,
    FOREIGN KEY (assignee_id) REFERENCES assignees(id) ON DELETE CASCADE
)"#
}

/// Issue assignees table indexes
pub const fn issue_assignees_indexes() -> &'static str {
    r#"CREATE INDEX IF NOT EXISTS idx_issue_assignees_issue_id ON issue_assignees(issue_id);
CREATE INDEX IF NOT EXISTS idx_issue_assignees_assignee_id ON issue_assignees(assignee_id);
CREATE INDEX IF NOT EXISTS idx_issue_assignees_assigned_at ON issue_assignees(assigned_at);"#
}

/// Bead annotations table - bf-only table for arbitrary key-value metadata
/// IMPORTANT: This is a SEPARATE table (not a column on issues) because br's
/// issues_column_order_matches() check triggers rebuild_issues_table() when
/// the column count differs, which would silently destroy any extra column.
pub const fn bead_annotations_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS bead_annotations (
    bead_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    key     TEXT NOT NULL,
    value   TEXT NOT NULL,
    PRIMARY KEY (bead_id, key)
)"#
}

/// Bead annotations table indexes
pub const fn bead_annotations_indexes() -> &'static str {
    r#"CREATE INDEX IF NOT EXISTS idx_bead_annotations_key_value
    ON bead_annotations (key, value);
CREATE INDEX IF NOT EXISTS idx_bead_annotations_bead_id
    ON bead_annotations (bead_id);"#
}

/// Labels table indexes
pub const fn labels_indexes() -> &'static str {
    r#"CREATE INDEX IF NOT EXISTS idx_labels_label ON labels(label);
CREATE INDEX IF NOT EXISTS idx_labels_issue ON labels(issue_id);"#
}

/// Bead labels table indexes
pub const fn bead_labels_indexes() -> &'static str {
    r#"CREATE INDEX IF NOT EXISTS idx_bead_labels_label ON bead_labels(label);
CREATE INDEX IF NOT EXISTS idx_bead_labels_issue ON bead_labels(bead_id);"#
}

/// Priorities table indexes (minimal - PK and UNIQUE constraints sufficient)
pub const fn priorities_indexes() -> &'static str {
    ""
}

/// Statuses table indexes (minimal - PK and UNIQUE constraints sufficient)
pub const fn statuses_indexes() -> &'static str {
    ""
}

/// Issue types table indexes (minimal - PK and UNIQUE constraints sufficient)
pub const fn issue_types_indexes() -> &'static str {
    ""
}

/// Assignees table indexes (minimal - PK and UNIQUE constraints sufficient)
pub const fn assignees_indexes() -> &'static str {
    ""
}

/// Dirty issues table indexes (minimal - PK sufficient)
pub const fn dirty_issues_indexes() -> &'static str {
    ""
}

/// Dirty issues table - tracks beads that need flushing to JSONL
pub const fn dirty_issues_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS dirty_issues (
    bead_id TEXT NOT NULL PRIMARY KEY,
    marked_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (bead_id) REFERENCES issues(id) ON DELETE CASCADE
)"#
}

/// Blocked issues cache table - materialized view for derived blocked status (Phase 7.8)
pub const fn blocked_issues_cache_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS blocked_issues_cache (
    issue_id TEXT NOT NULL PRIMARY KEY,
    blocked_by INTEGER NOT NULL DEFAULT 0,
    blocked_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
)"#
}

/// Blocked issues cache indexes
pub const fn blocked_issues_cache_indexes() -> &'static str {
    r#"CREATE INDEX IF NOT EXISTS idx_blocked_issues_cache_blocked_at
    ON blocked_issues_cache (blocked_at);"#
}

/// Critical path cache table (already defined elsewhere, adding reference for completeness)
pub const fn critical_path_cache_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS critical_path_cache (
    bead_id TEXT NOT NULL PRIMARY KEY,
    epic_id TEXT,
    es INTEGER NOT NULL DEFAULT 0,
    ls INTEGER NOT NULL DEFAULT 0,
    float INTEGER NOT NULL DEFAULT 0,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (bead_id) REFERENCES issues(id) ON DELETE CASCADE,
    FOREIGN KEY (epic_id) REFERENCES issues(id) ON DELETE CASCADE
)"#
}

/// Critical path cache indexes
pub const fn critical_path_cache_indexes() -> &'static str {
    r#"CREATE INDEX IF NOT EXISTS idx_critical_path_cache_epic_id
    ON critical_path_cache (epic_id);
CREATE INDEX IF NOT EXISTS idx_critical_path_cache_float
    ON critical_path_cache (float);"#
}

/// Complete SQL schema for all 16 tables
pub const SCHEMA_SQL: &str = r#"
-- Issues table
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
    manual_status TEXT DEFAULT NULL,
    CHECK (
        (status = 'closed' AND closed_at IS NOT NULL) OR
        (status = 'tombstone') OR
        (status NOT IN ('closed', 'tombstone') AND closed_at IS NULL)
    )
);

-- Issues indexes
CREATE INDEX IF NOT EXISTS idx_issues_status ON issues(status);
CREATE INDEX IF NOT EXISTS idx_issues_priority ON issues(priority);
CREATE INDEX IF NOT EXISTS idx_issues_issue_type ON issues(issue_type);
CREATE INDEX IF NOT EXISTS idx_issues_assignee ON issues(assignee) WHERE assignee IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_issues_created_at ON issues(created_at);
CREATE INDEX IF NOT EXISTS idx_issues_updated_at ON issues(updated_at);
CREATE INDEX IF NOT EXISTS idx_issues_content_hash ON issues(content_hash);
CREATE INDEX IF NOT EXISTS idx_issues_external_ref ON issues(external_ref) WHERE external_ref IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_issues_external_ref_unique ON issues(external_ref) WHERE external_ref IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_issues_ephemeral ON issues(ephemeral) WHERE ephemeral = 1;
CREATE INDEX IF NOT EXISTS idx_issues_pinned ON issues(pinned) WHERE pinned = 1;
CREATE INDEX IF NOT EXISTS idx_issues_tombstone ON issues(status) WHERE status = 'tombstone';
CREATE INDEX IF NOT EXISTS idx_issues_due_at ON issues(due_at) WHERE due_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_issues_defer_until ON issues(defer_until) WHERE defer_until IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_issues_ready
    ON issues(status, priority, created_at)
    WHERE status = 'open'
    AND ephemeral = 0
    AND pinned = 0
    AND is_template = 0;
CREATE INDEX IF NOT EXISTS idx_issues_list_active_order
    ON issues(priority, created_at DESC)
    WHERE status NOT IN ('closed', 'tombstone')
    AND (is_template = 0 OR is_template IS NULL);

-- Dependencies table
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
);

-- Dependencies indexes
CREATE INDEX IF NOT EXISTS idx_dependencies_issue ON dependencies(issue_id);
CREATE INDEX IF NOT EXISTS idx_dependencies_depends_on ON dependencies(depends_on_id);
CREATE INDEX IF NOT EXISTS idx_dependencies_type ON dependencies(type);
CREATE INDEX IF NOT EXISTS idx_dependencies_depends_on_type ON dependencies(depends_on_id, type);
CREATE INDEX IF NOT EXISTS idx_dependencies_thread ON dependencies(thread_id) WHERE thread_id != '';
CREATE INDEX IF NOT EXISTS idx_dependencies_blocking
    ON dependencies(depends_on_id, issue_id)
    WHERE (type = 'blocks' OR type = 'parent-child' OR type = 'conditional-blocks' OR type = 'waits-for');

-- Comments table
CREATE TABLE IF NOT EXISTS comments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    issue_id TEXT NOT NULL,
    author TEXT NOT NULL,
    text TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
);

-- Comments indexes
CREATE INDEX IF NOT EXISTS idx_comments_issue ON comments(issue_id);
CREATE INDEX IF NOT EXISTS idx_comments_created_at ON comments(created_at);

-- Events table
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

-- Events indexes
CREATE INDEX IF NOT EXISTS idx_events_issue ON events(issue_id);
CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_created_at ON events(created_at);
CREATE INDEX IF NOT EXISTS idx_events_actor ON events(actor) WHERE actor != '';

-- Labels table - direct issue-to-label mapping
CREATE TABLE IF NOT EXISTS labels (
    issue_id TEXT NOT NULL,
    label TEXT NOT NULL,
    PRIMARY KEY (issue_id, label),
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
);

-- Labels indexes
CREATE INDEX IF NOT EXISTS idx_labels_label ON labels(label);
CREATE INDEX IF NOT EXISTS idx_labels_issue ON labels(issue_id);

-- Bead labels table - alternative label storage
CREATE TABLE IF NOT EXISTS bead_labels (
    bead_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    label    TEXT NOT NULL,
    PRIMARY KEY (bead_id, label)
);

-- Bead labels indexes
CREATE INDEX IF NOT EXISTS idx_bead_labels_label ON bead_labels(label);
CREATE INDEX IF NOT EXISTS idx_bead_labels_issue ON bead_labels(bead_id);

-- Issue labels junction table
CREATE TABLE IF NOT EXISTS issue_labels (
    issue_id TEXT NOT NULL,
    label_id TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (issue_id, label_id),
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
);

-- Issue labels indexes
CREATE INDEX IF NOT EXISTS idx_issue_labels_issue_id ON issue_labels(issue_id);
CREATE INDEX IF NOT EXISTS idx_issue_labels_label_id ON issue_labels(label_id);

-- Priorities reference table
CREATE TABLE IF NOT EXISTS priorities (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    value INTEGER NOT NULL,
    color TEXT,
    description TEXT
);

-- Statuses reference table
CREATE TABLE IF NOT EXISTS statuses (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    category TEXT NOT NULL,
    description TEXT,
    is_terminal INTEGER NOT NULL DEFAULT 0
);

-- Issue types reference table
CREATE TABLE IF NOT EXISTS issue_types (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    icon TEXT,
    color TEXT
);

-- Issue relations table
CREATE TABLE IF NOT EXISTS issue_relations (
    issue_id TEXT NOT NULL,
    related_id TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by TEXT DEFAULT '',
    PRIMARY KEY (issue_id, related_id, relation_type),
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE,
    FOREIGN KEY (related_id) REFERENCES issues(id) ON DELETE CASCADE
);

-- Issue relations indexes
CREATE INDEX IF NOT EXISTS idx_issue_relations_issue_id ON issue_relations(issue_id);
CREATE INDEX IF NOT EXISTS idx_issue_relations_related_id ON issue_relations(related_id);
CREATE INDEX IF NOT EXISTS idx_issue_relations_type ON issue_relations(relation_type);

-- Assignees reference table
CREATE TABLE IF NOT EXISTS assignees (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    email TEXT,
    avatar_url TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Issue assignees junction table
CREATE TABLE IF NOT EXISTS issue_assignees (
    issue_id TEXT NOT NULL,
    assignee_id TEXT NOT NULL,
    assigned_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    assigned_by TEXT DEFAULT '',
    PRIMARY KEY (issue_id, assignee_id),
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE,
    FOREIGN KEY (assignee_id) REFERENCES assignees(id) ON DELETE CASCADE
);

-- Issue assignees indexes
CREATE INDEX IF NOT EXISTS idx_issue_assignees_issue_id ON issue_assignees(issue_id);
CREATE INDEX IF NOT EXISTS idx_issue_assignees_assignee_id ON issue_assignees(assignee_id);
CREATE INDEX IF NOT EXISTS idx_issue_assignees_assigned_at ON issue_assignees(assigned_at);

-- Bead annotations table (bf-only, never touched by br)
CREATE TABLE IF NOT EXISTS bead_annotations (
    bead_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    key     TEXT NOT NULL,
    value   TEXT NOT NULL,
    PRIMARY KEY (bead_id, key)
);

-- Bead annotations indexes
CREATE INDEX IF NOT EXISTS idx_bead_annotations_key_value
    ON bead_annotations (key, value);
CREATE INDEX IF NOT EXISTS idx_bead_annotations_bead_id
    ON bead_annotations (bead_id);

-- Dirty issues table (tracks beads that need flushing to JSONL)
CREATE TABLE IF NOT EXISTS dirty_issues (
    bead_id TEXT NOT NULL PRIMARY KEY,
    marked_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (bead_id) REFERENCES issues(id) ON DELETE CASCADE
);

-- Critical path cache table (stores computed CPM float values for beads)
CREATE TABLE IF NOT EXISTS critical_path_cache (
    bead_id TEXT NOT NULL PRIMARY KEY,
    epic_id TEXT,
    es INTEGER NOT NULL DEFAULT 0,
    ls INTEGER NOT NULL DEFAULT 0,
    float INTEGER NOT NULL DEFAULT 0,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (bead_id) REFERENCES issues(id) ON DELETE CASCADE,
    FOREIGN KEY (epic_id) REFERENCES issues(id) ON DELETE CASCADE
);

-- Critical path cache indexes
CREATE INDEX IF NOT EXISTS idx_critical_path_cache_epic_id
    ON critical_path_cache (epic_id);
CREATE INDEX IF NOT EXISTS idx_critical_path_cache_float
    ON critical_path_cache (float);

-- Blocked issues cache table (materialized view of blocked beads for Phase 7.8)
CREATE TABLE IF NOT EXISTS blocked_issues_cache (
    issue_id TEXT NOT NULL PRIMARY KEY,
    blocked_by INTEGER NOT NULL DEFAULT 0,
    blocked_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
);

-- Blocked issues cache indexes
CREATE INDEX IF NOT EXISTS idx_blocked_issues_cache_blocked_at
    ON blocked_issues_cache (blocked_at);
"#;

/// Execute multiple SQL statements separated by semicolons.
fn execute_batch(conn: &Connection, sql: &str) -> anyhow::Result<()> {
    conn.execute_batch(sql)?;
    Ok(())
}

pub fn apply_schema(conn: &Connection) -> anyhow::Result<()> {
    execute_batch(conn, SCHEMA_SQL)?;
    migrate_legacy_columns(conn)?;
    Ok(())
}

/// Rename columns left over from schema revisions that predate the current
/// `CREATE TABLE IF NOT EXISTS` definitions.
///
/// `apply_schema()` is a no-op for tables that already exist, so a column
/// rename in the DDL above never reaches a database created before the
/// rename landed. `dirty_issues` was originally created with an `issue_id`
/// primary key column; every write path (claim.rs, batch.rs) now names it
/// `bead_id` explicitly, so an unmigrated database fails every claim/release
/// with "table dirty_issues has no column named bead_id" — not corruption,
/// just a schema that was never brought forward. This runs on every
/// `Storage::open()` (via `apply_schema`) and is a no-op once migrated.
fn migrate_legacy_columns(conn: &Connection) -> anyhow::Result<()> {
    let has_legacy_issue_id: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('dirty_issues') WHERE name = 'issue_id'")?
        .exists([])?;
    if has_legacy_issue_id {
        conn.execute_batch("ALTER TABLE dirty_issues RENAME COLUMN issue_id TO bead_id;")?;
    }
    Ok(())
}

pub fn ensure_wal_mode(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA cache_size = -8000;
         PRAGMA synchronous = NORMAL;",
    )?;
    Ok(())
}

// Function name aliases for exact bead specification compatibility
/// Alias for issues_table() - matches bead bf-ybn5qu acceptance criteria
pub const fn table_issues() -> &'static str {
    issues_table()
}

/// Alias for dependencies_table() - matches bead bf-ybn5qu acceptance criteria
pub const fn table_dependencies() -> &'static str {
    dependencies_table()
}

/// Alias for comments_table() - matches bead bf-ybn5qu acceptance criteria
pub const fn table_comments() -> &'static str {
    comments_table()
}

/// Alias for events_table() - matches bead bf-ybn5qu acceptance criteria
pub const fn table_events() -> &'static str {
    events_table()
}

/// Alias for labels_table() - matches bead bf-2rt721 acceptance criteria
pub const fn table_labels() -> &'static str {
    labels_table()
}

/// Alias for issue_labels_table() - matches bead bf-2rt721 acceptance criteria
pub const fn table_issue_labels() -> &'static str {
    issue_labels_table()
}

/// Alias for assignees_table() - matches bead bf-2rt721 acceptance criteria
pub const fn table_assignees() -> &'static str {
    assignees_table()
}

/// Alias for issue_assignees_table() - matches bead bf-2rt721 acceptance criteria
pub const fn table_issue_assignees() -> &'static str {
    issue_assignees_table()
}

/// Alias for issue_relations_table() - matches bead bf-2rt721 acceptance criteria
pub const fn table_issue_relations() -> &'static str {
    issue_relations_table()
}

/// Alias for priorities_table() - matches bead bf-2roxos acceptance criteria
pub const fn table_priorities() -> &'static str {
    priorities_table()
}

/// Alias for statuses_table() - matches bead bf-2roxos acceptance criteria
pub const fn table_statuses() -> &'static str {
    statuses_table()
}

/// Alias for issue_types_table() - matches bead bf-2roxos acceptance criteria
pub const fn table_issue_types() -> &'static str {
    issue_types_table()
}

/// Alias for bead_annotations_table() - matches bead bf-2roxos acceptance criteria
pub const fn table_bead_annotations() -> &'static str {
    bead_annotations_table()
}

/// All table DDL strings - returns all 16 table definitions in dependency order
pub fn all_tables() -> Vec<&'static str> {
    vec![
        issues_table(),
        dependencies_table(),
        comments_table(),
        events_table(),
        labels_table(),
        issue_labels_table(),
        priorities_table(),
        statuses_table(),
        issue_types_table(),
        issue_relations_table(),
        assignees_table(),
        issue_assignees_table(),
        bead_annotations_table(),
        dirty_issues_table(),
        blocked_issues_cache_table(),
        critical_path_cache_table(),
    ]
}

/// All index DDL strings - returns all index definitions
pub fn all_indexes() -> Vec<&'static str> {
    vec![
        issues_indexes(),
        dependencies_indexes(),
        comments_indexes(),
        events_indexes(),
        labels_indexes(),
        issue_labels_indexes(),
        priorities_indexes(),
        statuses_indexes(),
        issue_types_indexes(),
        issue_relations_indexes(),
        assignees_indexes(),
        issue_assignees_indexes(),
        bead_annotations_indexes(),
        dirty_issues_indexes(),
        blocked_issues_cache_indexes(),
        critical_path_cache_indexes(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_issues_table_ddl() {
        let ddl = issues_table();
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS issues"));
        assert!(ddl.contains("id TEXT PRIMARY KEY"));
        assert!(ddl.contains("title TEXT NOT NULL CHECK(length(title) <= 500)"));
        assert!(ddl.contains("status TEXT NOT NULL DEFAULT 'open'"));
        assert!(ddl.contains("priority INTEGER NOT NULL DEFAULT 2 CHECK(priority >= 0 AND priority <= 4)"));
        assert!(ddl.contains("manual_status TEXT DEFAULT NULL"));
    }

    #[test]
    fn test_dependencies_table_ddl() {
        let ddl = dependencies_table();
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS dependencies"));
        assert!(ddl.contains("issue_id TEXT NOT NULL"));
        assert!(ddl.contains("depends_on_id TEXT NOT NULL"));
        assert!(ddl.contains("type TEXT NOT NULL DEFAULT 'blocks'"));
        assert!(ddl.contains("PRIMARY KEY (issue_id, depends_on_id)"));
        assert!(ddl.contains("FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE"));
    }

    #[test]
    fn test_comments_table_ddl() {
        let ddl = comments_table();
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS comments"));
        assert!(ddl.contains("id INTEGER PRIMARY KEY AUTOINCREMENT"));
        assert!(ddl.contains("issue_id TEXT NOT NULL"));
        assert!(ddl.contains("author TEXT NOT NULL"));
        assert!(ddl.contains("text TEXT NOT NULL"));
    }

    #[test]
    fn test_events_table_ddl() {
        let ddl = events_table();
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS events"));
        assert!(ddl.contains("id INTEGER PRIMARY KEY AUTOINCREMENT"));
        assert!(ddl.contains("issue_id TEXT NOT NULL"));
        assert!(ddl.contains("event_type TEXT NOT NULL"));
        assert!(ddl.contains("actor TEXT NOT NULL DEFAULT ''"));
    }

    #[test]
    fn test_labels_table_ddl() {
        let ddl = labels_table();
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS labels"));
        assert!(ddl.contains("issue_id TEXT NOT NULL"));
        assert!(ddl.contains("label TEXT NOT NULL"));
        assert!(ddl.contains("PRIMARY KEY (issue_id, label)"));
    }

    #[test]
    fn test_bead_annotations_table_ddl() {
        let ddl = bead_annotations_table();
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS bead_annotations"));
        assert!(ddl.contains("bead_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE"));
        assert!(ddl.contains("key     TEXT NOT NULL"));
        assert!(ddl.contains("value   TEXT NOT NULL"));
        assert!(ddl.contains("PRIMARY KEY (bead_id, key)"));
    }

    #[test]
    fn test_blocked_issues_cache_table_ddl() {
        let ddl = blocked_issues_cache_table();
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS blocked_issues_cache"));
        assert!(ddl.contains("issue_id TEXT NOT NULL PRIMARY KEY"));
        assert!(ddl.contains("blocked_by INTEGER NOT NULL DEFAULT 0"));
        assert!(ddl.contains("blocked_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP"));
    }

    #[test]
    fn test_critical_path_cache_table_ddl() {
        let ddl = critical_path_cache_table();
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS critical_path_cache"));
        assert!(ddl.contains("bead_id TEXT NOT NULL PRIMARY KEY"));
        assert!(ddl.contains("epic_id TEXT"));
        assert!(ddl.contains("es INTEGER NOT NULL DEFAULT 0"));
        assert!(ddl.contains("ls INTEGER NOT NULL DEFAULT 0"));
        assert!(ddl.contains("float INTEGER NOT NULL DEFAULT 0"));
    }

    #[test]
    fn test_issues_indexes_ddl() {
        let ddl = issues_indexes();
        assert!(ddl.contains("CREATE INDEX IF NOT EXISTS idx_issues_status ON issues(status)"));
        assert!(ddl.contains("CREATE INDEX IF NOT EXISTS idx_issues_priority ON issues(priority)"));
        assert!(ddl.contains("CREATE INDEX IF NOT EXISTS idx_issues_ready"));
        assert!(ddl.contains("WHERE status = 'open'"));
        assert!(ddl.contains("CREATE UNIQUE INDEX IF NOT EXISTS idx_issues_external_ref_unique"));
    }

    #[test]
    fn test_dependencies_indexes_ddl() {
        let ddl = dependencies_indexes();
        assert!(ddl.contains("CREATE INDEX IF NOT EXISTS idx_dependencies_issue ON dependencies(issue_id)"));
        assert!(ddl.contains("CREATE INDEX IF NOT EXISTS idx_dependencies_depends_on ON dependencies(depends_on_id)"));
        assert!(ddl.contains("CREATE INDEX IF NOT EXISTS idx_dependencies_blocking"));
        assert!(ddl.contains("WHERE (type = 'blocks' OR type = 'parent-child'"));
    }

    #[test]
    fn test_all_tables_count() {
        let tables = all_tables();
        assert_eq!(tables.len(), 16, "Should have 16 table definitions");

        // Verify critical tables are present
        assert!(tables.iter().any(|t| t.contains("CREATE TABLE IF NOT EXISTS issues")));
        assert!(tables.iter().any(|t| t.contains("CREATE TABLE IF NOT EXISTS dependencies")));
        assert!(tables.iter().any(|t| t.contains("CREATE TABLE IF NOT EXISTS comments")));
        assert!(tables.iter().any(|t| t.contains("CREATE TABLE IF NOT EXISTS events")));
        assert!(tables.iter().any(|t| t.contains("CREATE TABLE IF NOT EXISTS bead_annotations")));
        assert!(tables.iter().any(|t| t.contains("CREATE TABLE IF NOT EXISTS blocked_issues_cache")));
        assert!(tables.iter().any(|t| t.contains("CREATE TABLE IF NOT EXISTS critical_path_cache")));
    }

    #[test]
    fn test_all_indexes_count() {
        let indexes = all_indexes();
        assert_eq!(indexes.len(), 16, "Should have 16 index definition groups");

        // Verify critical index groups are present
        assert!(indexes.iter().any(|i| i.contains("idx_issues_status")));
        assert!(indexes.iter().any(|i| i.contains("idx_dependencies_issue")));
        assert!(indexes.iter().any(|i| i.contains("idx_comments_issue")));
        assert!(indexes.iter().any(|i| i.contains("idx_events_issue")));
    }

    #[test]
    fn test_table_aliases_match_base_functions() {
        // Test that all alias functions return the same DDL as their base functions
        assert_eq!(table_issues(), issues_table());
        assert_eq!(table_dependencies(), dependencies_table());
        assert_eq!(table_comments(), comments_table());
        assert_eq!(table_events(), events_table());
        assert_eq!(table_labels(), labels_table());
        assert_eq!(table_issue_labels(), issue_labels_table());
        assert_eq!(table_assignees(), assignees_table());
        assert_eq!(table_issue_assignees(), issue_assignees_table());
        assert_eq!(table_issue_relations(), issue_relations_table());
        assert_eq!(table_priorities(), priorities_table());
        assert_eq!(table_statuses(), statuses_table());
        assert_eq!(table_issue_types(), issue_types_table());
        assert_eq!(table_bead_annotations(), bead_annotations_table());
    }

    #[test]
    fn test_apply_schema_creates_all_tables() {
        // Create a temporary database
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Apply the schema
        apply_schema(&conn).unwrap();

        // Verify all tables were created
        let tables = all_tables();
        for table_ddl in tables {
            let table_name = extract_table_name(table_ddl);
            let mut stmt = conn.prepare(&format!(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
                table_name
            )).unwrap();

            let table_exists: bool = stmt.exists([]).unwrap();
            assert!(table_exists, "Table '{}' should exist after schema application", table_name);
        }
    }

    #[test]
    fn test_apply_schema_is_idempotent() {
        // Create a temporary database
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Apply schema twice
        apply_schema(&conn).unwrap();
        apply_schema(&conn).unwrap();

        // Verify tables still exist (no errors on second application)
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='table'").unwrap();
        let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
        assert!(count >= 16, "Should have at least 16 tables after double schema application");
    }

    #[test]
    fn test_ensure_wal_mode() {
        // Create a temporary database
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Ensure WAL mode
        ensure_wal_mode(&conn).unwrap();

        // Verify WAL mode is set
        let wal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(wal_mode.to_lowercase(), "wal");

        // Verify foreign keys are enabled
        let fk_enabled: String = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk_enabled, "1");
    }

    #[test]
    fn test_schema_sql_constant_completeness() {
        // Verify SCHEMA_SQL contains all tables and indexes
        let schema = SCHEMA_SQL;

        // Check for all 16 tables
        assert!(schema.contains("CREATE TABLE IF NOT EXISTS issues"));
        assert!(schema.contains("CREATE TABLE IF NOT EXISTS dependencies"));
        assert!(schema.contains("CREATE TABLE IF NOT EXISTS comments"));
        assert!(schema.contains("CREATE TABLE IF NOT EXISTS events"));
        assert!(schema.contains("CREATE TABLE IF NOT EXISTS labels"));
        assert!(schema.contains("CREATE TABLE IF NOT EXISTS issue_labels"));
        assert!(schema.contains("CREATE TABLE IF NOT EXISTS priorities"));
        assert!(schema.contains("CREATE TABLE IF NOT EXISTS statuses"));
        assert!(schema.contains("CREATE TABLE IF NOT EXISTS issue_types"));
        assert!(schema.contains("CREATE TABLE IF NOT EXISTS issue_relations"));
        assert!(schema.contains("CREATE TABLE IF NOT EXISTS assignees"));
        assert!(schema.contains("CREATE TABLE IF NOT EXISTS issue_assignees"));
        assert!(schema.contains("CREATE TABLE IF NOT EXISTS bead_annotations"));
        assert!(schema.contains("CREATE TABLE IF NOT EXISTS dirty_issues"));
        assert!(schema.contains("CREATE TABLE IF NOT EXISTS blocked_issues_cache"));
        assert!(schema.contains("CREATE TABLE IF NOT EXISTS critical_path_cache"));

        // Check for key indexes
        assert!(schema.contains("CREATE INDEX IF NOT EXISTS idx_issues_status"));
        assert!(schema.contains("CREATE INDEX IF NOT EXISTS idx_dependencies_issue"));
        assert!(schema.contains("CREATE INDEX IF NOT EXISTS idx_comments_issue"));
        assert!(schema.contains("CREATE INDEX IF NOT EXISTS idx_events_issue"));
    }

    #[test]
    fn test_issues_table_check_constraints() {
        let ddl = issues_table();

        // Verify title length constraint
        assert!(ddl.contains("CHECK(length(title) <= 500)"));

        // Verify priority range constraint
        assert!(ddl.contains("CHECK(priority >= 0 AND priority <= 4)"));

        // Verify status/closed_at consistency constraint
        assert!(ddl.contains("CHECK ("));
        assert!(ddl.contains("(status = 'closed' AND closed_at IS NOT NULL)"));
        assert!(ddl.contains("(status = 'tombstone')"));
        assert!(ddl.contains("(status NOT IN ('closed', 'tombstone') AND closed_at IS NULL)"));
    }

    #[test]
    fn test_foreign_key_constraints() {
        // Test that tables with foreign keys have proper constraints
        let dep_ddl = dependencies_table();
        assert!(dep_ddl.contains("FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE"));

        let comments_ddl = comments_table();
        assert!(comments_ddl.contains("FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE"));

        let events_ddl = events_table();
        assert!(events_ddl.contains("FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE"));
    }

    #[test]
    fn test_partial_indexes() {
        let issues_indexes = issues_indexes();

        // Verify partial indexes (WHERE clauses)
        assert!(issues_indexes.contains("WHERE assignee IS NOT NULL"));
        assert!(issues_indexes.contains("WHERE external_ref IS NOT NULL"));
        assert!(issues_indexes.contains("WHERE ephemeral = 1"));
        assert!(issues_indexes.contains("WHERE pinned = 1"));
        assert!(issues_indexes.contains("WHERE status = 'tombstone'"));
        assert!(issues_indexes.contains("WHERE due_at IS NOT NULL"));
        assert!(issues_indexes.contains("WHERE defer_until IS NOT NULL"));
    }

    #[test]
    fn test_composite_indexes() {
        let issues_indexes = issues_indexes();

        // Verify composite indexes
        assert!(issues_indexes.contains("idx_issues_ready"));
        assert!(issues_indexes.contains("ON issues(status, priority, created_at)"));

        assert!(issues_indexes.contains("idx_issues_list_active_order"));
        assert!(issues_indexes.contains("ON issues(priority, created_at DESC)"));
    }

    #[test]
    fn test_unique_indexes() {
        let issues_indexes = issues_indexes();

        // Verify unique index on external_ref
        assert!(issues_indexes.contains("CREATE UNIQUE INDEX IF NOT EXISTS idx_issues_external_ref_unique"));
    }

    #[test]
    fn test_migrate_legacy_columns() {
        // Create a temporary database
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create dirty_issues table with legacy column name
        conn.execute_batch(
            "CREATE TABLE dirty_issues (
                issue_id TEXT NOT NULL PRIMARY KEY,
                marked_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );"
        ).unwrap();

        // Verify legacy column exists
        let has_legacy: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info('dirty_issues') WHERE name = 'issue_id'")
            .unwrap()
            .exists([])
            .unwrap();
        assert!(has_legacy, "Legacy column should exist before migration");

        // Run migration
        migrate_legacy_columns(&conn).unwrap();

        // Verify column was renamed
        let has_legacy_after: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info('dirty_issues') WHERE name = 'issue_id'")
            .unwrap()
            .exists([])
            .unwrap();
        assert!(!has_legacy_after, "Legacy column should not exist after migration");

        let has_new_column: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info('dirty_issues') WHERE name = 'bead_id'")
            .unwrap()
            .exists([])
            .unwrap();
        assert!(has_new_column, "New column should exist after migration");
    }

    #[test]
    fn test_migrate_legacy_columns_idempotent() {
        // Create a temporary database
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create dirty_issues table with new column name
        conn.execute_batch(
            "CREATE TABLE dirty_issues (
                bead_id TEXT NOT NULL PRIMARY KEY,
                marked_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );"
        ).unwrap();

        // Run migration twice
        migrate_legacy_columns(&conn).unwrap();
        migrate_legacy_columns(&conn).unwrap();

        // Verify table still exists and has correct column
        let has_new_column: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info('dirty_issues') WHERE name = 'bead_id'")
            .unwrap()
            .exists([])
            .unwrap();
        assert!(has_new_column, "Column should still exist after double migration");
    }

    /// Helper function to extract table name from DDL
    fn extract_table_name(ddl: &str) -> &str {
        // Extract table name from "CREATE TABLE IF NOT EXISTS <table_name>"
        let ddl_start = ddl.find("CREATE TABLE IF NOT EXISTS ").unwrap();
        let name_start = ddl_start + "CREATE TABLE IF NOT EXISTS ".len();
        let name_end = ddl[name_start..].find('(').unwrap();
        ddl[name_start..name_start + name_end].trim()
    }
}
