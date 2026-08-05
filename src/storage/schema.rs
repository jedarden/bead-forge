//! Database schema definitions for bead-forge.
//!
//! This module provides DDL for the 14 core tables in the bead-forge schema.
//! Each table is defined as a const function returning &str for easy composition.

use rusqlite::Connection;

/// Issues table - core bead data
/// All 35 columns with br-compatible types, defaults, and CHECK constraints.
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

/// Labels table - reference data for label definitions
pub const fn labels_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS labels (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    color TEXT,
    description TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
)"#
}

/// Issue labels table - junction table for many-to-many relationship
pub const fn issue_labels_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS issue_labels (
    issue_id TEXT NOT NULL,
    label_id TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (issue_id, label_id),
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE,
    FOREIGN KEY (label_id) REFERENCES labels(id) ON DELETE CASCADE
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

/// Dirty issues table - tracks beads that need flushing to JSONL
pub const fn dirty_issues_table() -> &'static str {
    r#"CREATE TABLE IF NOT EXISTS dirty_issues (
    bead_id TEXT NOT NULL PRIMARY KEY,
    FOREIGN KEY (bead_id) REFERENCES issues(id) ON DELETE CASCADE
)"#
}

/// Complete SQL schema for all 14 tables
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

-- Labels reference table
CREATE TABLE IF NOT EXISTS labels (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    color TEXT,
    description TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Issue labels junction table
CREATE TABLE IF NOT EXISTS issue_labels (
    issue_id TEXT NOT NULL,
    label_id TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (issue_id, label_id),
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE,
    FOREIGN KEY (label_id) REFERENCES labels(id) ON DELETE CASCADE
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
    FOREIGN KEY (bead_id) REFERENCES issues(id) ON DELETE CASCADE
);
"#;

/// Execute multiple SQL statements separated by semicolons.
fn execute_batch(conn: &Connection, sql: &str) -> anyhow::Result<()> {
    conn.execute_batch(sql)?;
    Ok(())
}

pub fn apply_schema(conn: &Connection) -> anyhow::Result<()> {
    execute_batch(conn, SCHEMA_SQL)?;
    Ok(())
}

pub fn ensure_wal_mode(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 30000;
         PRAGMA cache_size = -8000;
         PRAGMA synchronous = NORMAL;",
    )?;
    Ok(())
}
