# bead-forge Implementation Plan

> **User-facing documentation:** [`../README.md`](../README.md) — overview, command reference, NEEDLE integration guide, and hero image.

## Overview

**bead-forge** (`bf`) is a Rust binary that replaces `br` (beads_rust). SQLite is the live database — all reads and writes go through it. `issues.jsonl` is a git-synced artifact exported on demand: one entry per bead, committed to the repo for backup, sharing, and recovery. Concurrent claiming is handled by SQLite's `BEGIN IMMEDIATE` transaction — no server, no daemon, no flock required.

## Core Principles

1. **SQLite is the live store** — all mutations are SQL writes; reads are SQL queries. Fast, concurrent-read, serialized-write via WAL mode.
2. **JSONL is the git artifact** — exported on `bf sync`, committed to git, one entry per bead. Used for backup, cross-machine sharing, and recovery. Never written during normal operation.
3. **Drop-in CLI** — all `br` commands work under `bf` with the same flags and output formats
4. **`BEGIN IMMEDIATE` for atomic claim** — the read-score-update sequence runs inside a single SQLite write transaction, eliminating the race condition `br` has without requiring flock or a daemon

---

## Phase 1: Core Library — Data Model & JSONL Import/Export

### 1.1 Data Model (`src/model.rs`)

Port the exact `Issue`, `Status`, `Priority`, `IssueType`, `Dependency`, `DependencyType`, `Comment`, and `Event` structs from beads_rust with identical Serde attributes. This is the wire format — every field, every `skip_serializing_if`, every rename must match.

Key structs and their JSONL serialization:

```
Status:       open | in_progress | blocked | deferred | draft | closed | tombstone | pinned | Custom(String)
Priority:     transparent i32 (0=Critical, 4=Backlog)
IssueType:    task | bug | feature | epic | chore | docs | question | Custom(String)
Dependency:   { issue_id, depends_on_id, type (renamed from dep_type), created_at, created_by?, metadata?, thread_id? }
Comment:      { id: i64, issue_id, author, text (renamed from body), created_at }
Event:        { id: i64, issue_id, event_type, actor, old_value?, new_value?, comment?, created_at }
```

Issue fields (alphabetical by JSON key):
```
id, title, description?, design?, acceptance_criteria?, notes?,
status, priority, issue_type, assignee?, owner?, estimated_minutes?,
created_at, created_by?, updated_at, closed_at?, close_reason?,
closed_by_session?, due_at?, defer_until?, external_ref?,
source_system?, source_repo, deleted_at?, deleted_by?, delete_reason?,
original_type?, compaction_level (default 0), compacted_at?,
compacted_at_commit?, original_size?, sender?, ephemeral (default false),
pinned (default false), is_template (default false),
labels: Vec<String>, dependencies: Vec<Dependency>, comments: Vec<Comment>
```

### 1.2 JSONL Import (`src/jsonl.rs`)

JSONL is read only during `bf sync --import` (pulling state from git) and `bf doctor --repair` (rebuilding SQLite from the artifact). Never read during normal CLI operation.

```rust
fn import_jsonl(jsonl_path: &Path, storage: &Storage) -> Result<ImportResult> {
    let file = File::open(jsonl_path)?;
    let reader = BufReader::new(file);
    storage.with_write_transaction(|tx| {
        for line in reader.lines() {
            let issue: Issue = serde_json::from_str(&line?)?;
            tx.upsert_issue(&issue)?;
        }
        Ok(())
    })
}
```

Functions:
- `import_jsonl(path, storage)` — bulk upsert from JSONL into SQLite
- `stream_issues(path) -> impl Iterator<Item = Result<Issue>>` — lazy line iterator (used by import and doctor)

### 1.3 JSONL Export (`src/jsonl.rs`)

JSONL is written only during `bf sync --flush` (preparing the git artifact). Exports current bead state from SQLite, one entry per bead, sorted by ID for stable diffs. Atomic temp+rename.

```rust
fn export_jsonl(jsonl_path: &Path, storage: &Storage) -> Result<ExportResult> {
    let issues = storage.list_all_issues()?;  // sorted by ID
    let temp_path = jsonl_path.with_extension("jsonl.tmp");
    let mut writer = BufWriter::new(File::create_new(&temp_path)?);
    for issue in &issues {
        serde_json::to_writer(&mut writer, issue)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    drop(writer);
    fs::rename(&temp_path, jsonl_path)?;
    Ok(ExportResult { count: issues.len() })
}
```

Functions:
- `export_jsonl(path, storage)` — dump all beads from SQLite to JSONL (atomic)
- `export_jsonl_dirty(path, storage)` — export only dirty beads (incremental, faster for large workspaces)

### 1.4 Config (`src/config.rs`)

Parse `.beads/config.yaml` with the same fields br uses:

```yaml
issue_prefixes: [bd, fg]
default_priority: 2
default_type: task
scoring:
  priority_weight: 0.4
  blockers_weight: 0.3
  age_weight: 0.2
  labels_weight: 0.1
  max_age_hours: 20
  max_blockers: 3
# bead-forge additions (additive, br ignores these)
claim_ttl_minutes: 30
```

Parse `.beads/metadata.json`:
```json
{ "database": "beads.db", "jsonl_export": "issues.jsonl" }
```

### 1.5 ID Generation (`src/id.rs`)

Port br's ID scheme: `<prefix>-<hash>` where hash is base36 lowercase. Use the birthday-problem adaptive length (3-8 chars). Read `issue_prefixes` from config (first entry is the default prefix).

```rust
fn generate_id(prefix: &str, existing_count: usize) -> String {
    let len = optimal_hash_length(existing_count);
    let random_bytes = rand::random::<[u8; 16]>();
    let hash = sha256(&random_bytes);
    base36_encode(&hash[..len])
    format!("{}-{}", prefix, hash)
}
```

---

## Phase 2: SQLite Primary Store

### 2.1 Schema Application (`src/storage/schema.rs`)

Apply the exact same `CREATE TABLE IF NOT EXISTS` DDL from br's `SCHEMA_SQL`. Tables:

| Table | Purpose |
|-------|---------|
| `issues` | All 35 columns, same types, defaults, CHECK constraints |
| `dependencies` | (issue_id, depends_on_id) PK, type, metadata, thread_id |
| `labels` | (issue_id, label) PK |
| `comments` | AUTOINCREMENT id, issue_id, author, text, created_at |
| `events` | AUTOINCREMENT id, issue_id, event_type, actor, old/new/comment |
| `config` | key-value |
| `metadata` | key-value |
| `dirty_issues` | Track which issues need JSONL export |
| `export_hashes` | Incremental export tracking |
| `blocked_issues_cache` | Materialized view of blocked issues |
| `child_counters` | Hierarchical IDs (bd-abc.1, bd-abc.2) |
| `recovery_sessions` | Anomaly/recovery audit trail |
| `anomaly_audit` | Recovery event tracking |
| `bead_annotations` | bf-only: arbitrary key-value metadata per bead; separate table so br's `rebuild_issues_table()` never touches it |

All indexes from br must be created identically, including the critical `idx_issues_ready` composite partial index.

### 2.2 Storage Engine (`src/storage/sqlite.rs`)

Use `rusqlite` (not FrankenSQLite) bundled with system SQLite. Apply WAL mode, foreign keys, busy_timeout=30s, cache_size=-8000, synchronous=NORMAL.

All mutations go through here. JSONL is never touched during normal operation.

Functions:
- `open(path: &Path) -> Result<Storage>` — open/create DB, apply schema
- `get_issue(id: &str) -> Result<Option<Issue>>` — point lookup by ID
- `list_issues(filter: IssueFilter) -> Result<Vec<Issue>>` — filtered query with WHERE clauses
- `list_all_issues() -> Result<Vec<Issue>>` — full table scan sorted by ID (for JSONL export)
- `create_issue(issue: &Issue) -> Result<()>` — INSERT
- `update_issue(id: &str, changes: IssueChanges) -> Result<()>` — UPDATE + mark dirty
- `close_issue(id: &str, reason: &str) -> Result<()>` — status transition + mark dirty
- `with_write_transaction(f) -> Result<T>` — execute closure in a write transaction
- `with_immediate_transaction(f) -> Result<T>` — `BEGIN IMMEDIATE` with retry (see §2.5) — acquires write lock before any reads (used by claim)
- `rebuild_blocked_cache() -> Result<()>` — recalculate blocked issues materialized view

### 2.3 Sync Protocol (`src/sync.rs`)

Port br's sync semantics:

**Flush (SQLite → JSONL, for git)**:
1. Call `export_jsonl_dirty` — query dirty bead IDs, export full current state of each from SQLite
2. Atomic temp+rename over `issues.jsonl`
3. Clear `dirty_issues`, update `export_hashes`

For `bf sync` (full flush): call `export_jsonl` — dump all beads sorted by ID, unconditionally.

**Import (JSONL → SQLite, from git pull)**:
1. Stream `issues.jsonl` line by line
2. Compare each with SQLite state using `content_hash`
3. INSERT new, UPDATE changed, SKIP unchanged
4. Handle collisions (same ID, different content) with deterministic resolution

### 2.5 Concurrent Write Safety (`src/storage/sqlite.rs`)

SQLite WAL mode serializes concurrent writers through a single write lock — they queue, not corrupt. `SQLITE_BUSY` is returned when a writer cannot acquire the lock within `busy_timeout`. With claim latency of ~1-2ms and a 20-worker fleet, the entire queue clears in ~20-40ms, so `SQLITE_BUSY` should never occur in normal operation. However, a retry wrapper is the cheap safety net that makes this a guarantee rather than an assumption:

```rust
const MAX_RETRIES: u32 = 5;
const RETRY_BASE_MS: u64 = 50;

fn with_immediate_transaction<T>(
    conn: &Connection,
    f: impl Fn(&Transaction) -> Result<T>,
) -> Result<T> {
    let mut attempt = 0;
    loop {
        match conn.execute("BEGIN IMMEDIATE", []) {
            Ok(_) => {
                let tx = conn.unchecked_transaction()?;
                return match f(&tx) {
                    Ok(val) => { tx.commit()?; Ok(val) }
                    Err(e) => { let _ = tx.rollback(); Err(e) }
                };
            }
            Err(e) if is_busy_error(&e) && attempt < MAX_RETRIES => {
                attempt += 1;
                std::thread::sleep(Duration::from_millis(RETRY_BASE_MS * attempt as u64));
                continue;
            }
            Err(e) => return Err(e.into()),
        }
    }
}

fn is_busy_error(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error { code: rusqlite::ErrorCode::DatabaseBusy, .. },
            _
        )
    )
}
```

**Why this is sufficient and not a workaround**: `SQLITE_BUSY` with WAL mode is not data corruption — it is a clean "lock unavailable, try again" signal. The database is fully consistent when this occurs. The retry converts a transient contention spike (e.g., 20 workers all starting within the same millisecond) into a short wait rather than a user-visible error.

**All write operations** (`claim`, `create`, `update`, `close`, `batch`) use `with_immediate_transaction`. Read-only operations (`list`, `show`, `ready`) use a plain `BEGIN DEFERRED` transaction and are never affected by write contention.

### 2.7 Doctor/Repair (`src/doctor.rs`)

- `doctor --repair`: Delete `beads.db`, reimport from JSONL
- `doctor --check`: Validate JSONL line integrity, check SQLite <-> JSONL consistency
- Recovery from corruption: JSONL export is always authoritative — `beads.db` can be rebuilt from it at any time

---

## Phase 3: CLI Commands

### 3.1 Command Structure (`src/cli/`)

Use `clap` with derive macros. Binary name: `bf`. Subcommands mirror br exactly.

```
bf create    --title "..." --type <type> --priority <N> [--description "..."] [--label ...]
bf list      [--status <s>] [--type <t>] [--assignee <a>] [--limit N] [--format json|text|toon]
bf show      <id>
bf update    <id> [--status <s>] [--assignee <a>] [--priority <N>] [--title "..."] ...
bf close     <id> [--reason "..."]
bf reopen    <id>
bf delete    <id>
bf ready         [--limit N]                     # unblocked, open, sorted by priority
bf claim         --assignee <id>                 # atomic claim (see Phase 4)
                 [--model <model>]               # e.g. claude-sonnet-4-6
                 [--harness <name>]              # e.g. needle
                 [--harness-version <ver>]       # e.g. 0.5.2
                 [--workspace <path>] [--any]
bf critical-path <epic-id>                       # show critical path + float for each bead
bf dep           add-blocker <parent> <child>
bf dep           remove-blocker <parent> <child>
bf dep           tree <id>
bf label         <id> add <label>
bf label         <id> remove <label>
bf comment       <id> [--author <a>] [--text "..."]
bf sync          --flush-only
bf sync          --import-only
bf doctor        [--repair]
bf migrate       [<workspace>]                  # migrate br workspace to bf (see Phase 4C)
                 [--from-jsonl]                 # rebuild SQLite from JSONL + git history
                 [--seed-velocity]              # populate velocity_stats from events history
                 [--dry-run]
bf init          [--prefix <p>]
bf stats
bf count         [--status <s>]
bf search        <query>
bf schema
bf config        get <key>
bf config        set <key> <value>
```

### 3.2 Output Formats

Three formats, matching br exactly:
- **text**: Human-readable table (default for terminal)
- **json**: One JSON object per line (for piping)
- **toon**: Token-optimized output for LLM context windows

### 3.3 Execution Model

Each command:
1. Locate `.beads/` directory (walk up from CWD)
2. Load config + metadata
3. Open SQLite (`beads.db`), apply schema if new
4. Execute: mutations are SQL writes; reads are SQL queries
5. `bf sync` is the only command that touches `issues.jsonl` — flush exports it, import reads it

---

## Phase 4: Concurrent Claiming

### 4.1 Architecture: SQLite `BEGIN IMMEDIATE`

The thundering herd problem — N workers racing to claim the same bead — is solved by running the entire read-score-update sequence inside a single `BEGIN IMMEDIATE` transaction. SQLite's write lock is acquired before any reads, so no two workers can read the same candidate state. No flock, no server, no daemon.

```
Worker 1: BEGIN IMMEDIATE → SELECT candidates → score → UPDATE winner → COMMIT
Worker 2: BEGIN IMMEDIATE → (blocked at write lock until Worker 1 COMMITs) → SELECT → score → UPDATE → COMMIT
Worker 3: BEGIN IMMEDIATE → (blocked) → ...
...
Worker 20: BEGIN IMMEDIATE → (blocked) → SELECT → NONE (queue empty) → COMMIT (no-op)
```

Each claim is a single CLI invocation. The critical section (lock held) is: SELECT scoring fields → score in memory → UPDATE winner + stale resets → COMMIT. Lock held for ~0.5-2ms.

**Throughput**: SQLite handles claims purely in memory after the initial page cache warm-up:

| Workspace | Active beads | Claim latency | Throughput |
|-----------|-------------|---------------|-----------|
| Typical (FORGE) | 248 | ~0.5ms | ~2000/sec |
| Medium (NEEDLE) | 518 | ~1ms | ~1000/sec |
| Large | 2000 | ~2ms | ~500/sec |

With 20 workers claiming simultaneously, the last worker waits ~20-40ms (19 queued transactions × ~1-2ms each). The fleet needs ~1-2 claims/sec at steady state — SQLite is overkill by 3 orders of magnitude.

### 4.2 Atomic Claim (`src/claim.rs`)

The entire claim sequence runs inside a single `BEGIN IMMEDIATE` transaction. SQLite acquires the write lock before any reads, so no two workers can observe the same candidate state.

```rust
fn claim(storage: &Storage, worker: &str, claim_ttl: Duration) -> Result<Option<Issue>> {
    storage.with_immediate_transaction(|tx| {
        let now = Utc::now();

        // 1. Reclaim stale in_progress beads
        tx.execute(
            "UPDATE issues SET status='open', assignee=NULL, updated_at=?1
             WHERE status='in_progress' AND updated_at < ?2",
            params![now, now - claim_ttl],
        )?;

        // 2. Load scoring fields for open, unblocked candidates
        //    (blocked_issues_cache is a materialized view maintained on dep changes)
        let candidates: Vec<ScoreEntry> = tx.query_map(
            "SELECT id, priority, updated_at,
                    (SELECT COUNT(*) FROM dependencies d
                     JOIN issues b ON b.id = d.depends_on_id
                     WHERE d.issue_id IN (
                         SELECT issue_id FROM dependencies
                         WHERE depends_on_id = issues.id AND type='blocks'
                     ) AND b.status = 'open') AS downstream_impact
             FROM issues
             WHERE status = 'open'
               AND id NOT IN (SELECT issue_id FROM blocked_issues_cache)
             ORDER BY downstream_impact DESC, priority ASC, created_at ASC",
            [],
            |row| ScoreEntry::from_row(row),
        )?;

        if candidates.is_empty() {
            return Ok(None);
        }

        // 3. Claim the top candidate
        let winner = &candidates[0];
        tx.execute(
            "UPDATE issues SET status='in_progress', assignee=?1, updated_at=?2
             WHERE id=?3",
            params![worker, now, winner.id],
        )?;
        tx.execute(
            "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, created_at)
             VALUES (?1, 'claimed', ?2, 'open', 'in_progress', ?3)",
            params![winner.id, worker, now],
        )?;
        tx.mark_dirty(&winner.id)?;

        Ok(Some(tx.get_issue(&winner.id)?))
    })
}
```

**Key properties**:
- **Correctness**: `BEGIN IMMEDIATE` holds the SQLite write lock for the entire read-score-update sequence. No two workers can observe the same candidate state.
- **Crash safety**: If the process crashes mid-transaction, SQLite rolls back automatically. No partial state is committed.
- **Stale reclamation is a side effect**: The first statement in every claim resets expired `in_progress` beads before scoring. No separate process.
- **Impact-weighted scoring**: The query orders by `downstream_impact` (beads that would unblock the most work) then priority, then FIFO.
- **No flock, no file I/O**: The entire claim is in-memory SQL execution after page cache warm-up. `issues.jsonl` is not touched.

### 4.3 Multi-Workspace Claiming (`src/claim.rs`)

For `bf claim --any` (claim from any workspace), open each workspace's SQLite, run the scoring query on each, pick the global winner, claim it:

```rust
fn claim_any(workspace_paths: &[PathBuf], worker: &str, claim_ttl: Duration) -> Result<Option<Issue>> {
    // Score across all workspaces — each Storage::open is cheap (WAL, page cache)
    let mut best: Option<(Score, PathBuf)> = None;
    for path in workspace_paths {
        let storage = Storage::open(&path.join(".beads/beads.db"))?;
        if let Some(score) = storage.top_candidate_score(claim_ttl)? {
            if best.as_ref().map(|(b, _)| score > *b).unwrap_or(true) {
                best = Some((score, path.clone()));
            }
        }
    }

    match best {
        None => Ok(None),
        Some((_, path)) => {
            let storage = Storage::open(&path.join(".beads/beads.db"))?;
            claim(&storage, worker, claim_ttl)
        }
    }
}
```

No cross-workspace locking needed — each `claim()` call uses `BEGIN IMMEDIATE` on its own SQLite file. Two workers racing on the same workspace are serialized by SQLite; two workers racing on different workspaces proceed in parallel.

### 4.4 Stale Claim Detection

No background process needed. Stale detection runs as part of every `bf claim` invocation:

1. Inside `BEGIN IMMEDIATE`: `UPDATE issues SET status='open' WHERE status='in_progress' AND updated_at < now - claim_ttl`
2. This runs before candidate scoring — stale beads become eligible again in the same transaction
3. Single SQL statement, no extra I/O

**Claim TTL**: Default 30 minutes, configurable in `.beads/config.yaml`:

```yaml
claim_ttl_minutes: 30
```

**TTL tuning**: With 20 workers and tasks taking 5-15 minutes, 30 minutes is conservative. A worker that's actively working but slow will have its bead remain `in_progress` — stale reclamation only triggers if the worker hasn't updated the bead in 30 minutes. Since the worker closes the bead when done (writing a new entry), active workers never get reclaimed.

**Manual stale scan**: For workspaces with no active claiming (no fleet running), `bf doctor --reclaim-stale` performs the same scan without claiming a bead. Can be run via cron for long-running workspaces with occasional stuck claims.

### 4.5 CLI Usage

```bash
# Claim from current workspace
bf claim --assignee worker-7 --json

# Claim with worker metadata (feeds velocity-aware scoring — see §4B.6)
bf claim --assignee worker-7 \
         --model claude-sonnet-4-6 \
         --harness needle \
         --harness-version 0.5.2 \
         --json

# Claim from a specific workspace
bf claim --workspace containers/zai-proxy/ --assignee worker-3 --json

# Claim highest priority across all workspaces
bf claim --any --assignee worker-12 --json

# Prefer a workspace, fall back to any
bf claim --workspace containers/zai-proxy/ --fallback any --assignee worker-5 --json
```

All commands work identically in all modes — no server required, no daemon running. Each `bf claim` is a self-contained CLI invocation: lock, read, claim, write, unlock, exit.

Worker metadata (`--model`, `--harness`, `--harness-version`) is stored in the claim event row and the `worker_sessions` table. It drives velocity-aware scoring (§4B.6) and appears in `bf log` output so you can see exactly which model/harness combination claimed each bead.

### 4.6 Why No Server

The plan originally included a coordination server with `Arc<Mutex<ServerState>>`, in-memory priority queues, heartbeats, and a Unix socket protocol. This was removed because:

1. **SQLite is sufficient**: `BEGIN IMMEDIATE` provides serialized write access. Claim latency is ~0.5-2ms — 4-16× faster than the previous flock+streaming-rewrite approach.
2. **Simpler operations**: No flock infrastructure, no streaming rewrite, no temp files during normal operation. All mutation logic is SQL.
3. **Crash recovery is automatic**: SQLite rolls back incomplete transactions on next open. No partial state, no temp file cleanup.
4. **Real-world scale validates it**: A 2,000-bead workspace takes ~2ms per claim. A 20-worker fleet needs ~1-2 claims/sec. SQLite delivers ~500 claims/sec at that size.
5. **Server is a future optimization**: A persistent in-memory server would reduce per-claim latency from ~2ms to ~0.1ms by eliminating SQLite file I/O entirely. Premature for current scale — can be added transparently later.

### 4.7 Comparison to br's Race Condition

| Scenario | br (beads_rust) | bf (bead-forge) |
|----------|-----------------|-----------------|
| 20 workers claim simultaneously | 20 read same JSONL, all see same "next" bead, all write `in_progress` — last writer wins, 19 workers have phantom claims | 20 workers serialized by SQLite write lock — each BEGIN IMMEDIATE sees post-commit state, each gets a distinct bead |
| Worker crashes mid-claim | N/A (no lock) | SQLite rolls back the incomplete transaction automatically on next open |
| Stale claim (worker died) | Bead stays `in_progress` forever | Next claim invocation reclaims beads older than TTL |
| Cross-workspace claiming | Not supported | `--any` locks all JSONL files in sorted order |

---

## Phase 4B: Extended Features

### 4B.1 JSONL Rotation (`src/rotate.rs`)

Keep `issues.jsonl` lean for fast claiming. With one entry per bead, the file grows only when new beads are created — but over a long project lifetime the accumulation of closed beads still grows the file. Rotation evicts closed beads into numbered archive files, keeping the active scan set small.

```
.beads/
├── issues.jsonl          # active: open, in_progress, blocked, deferred + recently closed
├── issues.jsonl.1        # archive: closed beads rotated out most recently
├── issues.jsonl.2        # archive: older closed beads
└── issues.jsonl.3        # archive: oldest
```

**Rotation rules**:
- `bf rotate` moves closed beads older than `rotate_age_days` (default 30) from `issues.jsonl` into `issues.jsonl.1` via streaming rewrite (active file) + streaming append (archive file)
- When `issues.jsonl.1` exceeds `rotate_max_size` (default 5MB), it becomes `issues.jsonl.2`, `.2` → `.3`, etc.
- Maximum archive files: `rotate_max_archives` (default 5). Oldest archive is deleted.

**Claim only reads the active file**. This keeps claim latency proportional to the number of active (non-closed) beads rather than total workspace history.

**Reading archived beads**: `bf show <id>` searches the active file first, then archives. `bf list --all` includes archives. `bf log <id>` searches all files for the full history.

**git integration**: `.beads/issues.jsonl` is committed to git. Archive files (`.jsonl.1`, `.2`, ...) are also committed — they're part of the project history. The `.beads/` directory is the bead store and should be version-controlled alongside the code.

**Secret protection**: Before any write, bf scans bead fields for secret patterns (AWS keys, private keys, API tokens, passwords) using a built-in regex set similar to git-secrets. If a match is found:
- `bf create` / `bf update` prints a warning and refuses the write
- `bf commit-check` (git pre-commit hook) scans staged changes to `.beads/` for secrets
- Patterns are configurable in `.beads/config.yaml`:

```yaml
secret_protection:
  enabled: true
  # Additional patterns (matched against all string fields)
  deny_patterns:
    - "-----BEGIN (RSA |EC |DSA )?PRIVATE KEY-----"
    - "AKIA[0-9A-Z]{16}"          # AWS access key
    - "sk-[a-zA-Z0-9]{32,}"       # OpenAI/Anthropic API key
  # Fields to exclude from scanning (e.g., known test values)
  allowlist_fields: []
```

Config:

```yaml
rotate_age_days: 30
rotate_max_size_mb: 5
rotate_max_archives: 5
```

### 4B.2 Batch Operations (`src/batch.rs`)

Execute multiple operations atomically under a single SQLite `BEGIN IMMEDIATE` transaction. Replaces sequences of N separate CLI invocations with one.

```bash
# From a file
bf batch --file operations.json

# From stdin (one operation per line)
echo 'create --title "child 1" --type task
create --title "child 2" --type task
dep add-blocker fg-a3f8.1 fg-a3f8
dep add-blocker fg-a3f8.2 fg-a3f8
close fg-a3f8 --reason "Split into children"' | bf batch

# Programmatic JSON input
# NEEDLE mitosis: use @0, @1 to reference created beads by position
bf batch --json '[
  {"op": "create", "title": "child 1", "type": "task"},
  {"op": "create", "title": "child 2", "type": "task"},
  {"op": "dep_add_blocker", "parent": "@0", "child": "fg-a3f8"},
  {"op": "dep_add_blocker", "parent": "@1", "child": "fg-a3f8"},
  {"op": "close", "id": "fg-a3f8", "reason": "Split into children"}
]'
```

**Placeholder references**: `@0`, `@1`, etc. resolve to the IDs of beads created at that position. Critical for mitosis — you don't know child IDs until they're created.

**Atomicity**: All operations run inside a single SQLite write transaction (`BEGIN IMMEDIATE`). If any operation fails (invalid ID, bad status transition), the transaction rolls back — no partial state is committed. Kill -9 during batch leaves the workspace in its original state.

**Use cases**:
- NEEDLE mitosis: split 1 bead into N children (2N+1 operations → 1 batch)
- Bulk status changes: close all beads with a given label
- Migration: import beads from another tracker
- Workspace setup: create epics + child beads + dependencies in one shot

**Output**: Returns JSON array of results, one per operation:

```json
[
  {"op": 0, "status": "ok", "id": "fg-b4d1"},
  {"op": 1, "status": "ok", "id": "fg-b4d2"},
  {"op": 2, "status": "ok"},
  {"op": 3, "status": "ok"}
]
```

### 4B.3 Annotations — Extensible Metadata (`src/model.rs`, `src/cli/annotate.rs`)

Arbitrary key-value pairs on any bead, stored in a **separate `bead_annotations` table** — never as a column on `issues`.

**Why not a column on `issues`**: br's `issues_column_order_matches()` check in `schema.rs:630` fires on every `br` open and returns `false` the moment the actual column count differs from `EXPECTED_ISSUE_COLUMN_ORDER.len()`. This triggers `rebuild_issues_table()` which rewrites `issues` from br's canonical column list — destroying any bf-added columns. Adding an `annotations` column to `issues` would be silently wiped on every `br` invocation.

**Schema** (br never touches this table):

```sql
CREATE TABLE IF NOT EXISTS bead_annotations (
    bead_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    key     TEXT NOT NULL,
    value   TEXT NOT NULL,
    PRIMARY KEY (bead_id, key)
);
CREATE INDEX IF NOT EXISTS idx_bead_annotations_key_value
    ON bead_annotations (key, value);  -- for bf list --annotation key=value
```

**Issue struct** (annotations loaded via JOIN, serialized into the JSONL field):

```rust
#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
pub annotations: BTreeMap<String, String>,
```

JSONL export serializes annotations into the `Issue` JSON object. On import, annotations are upserted into `bead_annotations`. br ignores the `annotations` JSON field when reading JSONL (`skip_unknown_fields`), so JSONL round-trips are clean.

**CLI**:

```bash
# Set annotations
bf annotate fg-a3f8 needle_attempt=3 needle_session=abc123
bf annotate fg-a3f8 review_status=needs_review

# Remove annotations
bf annotate fg-a3f8 --remove needle_error

# Query by annotation
bf list --annotation needle_attempt=3
bf list --annotation review_status=needs_review

# Show annotations (included in bf show output)
bf show fg-a3f8 --json | jq '.annotations'
```

**br backward compatibility**: br never reads, writes, or rebuilds `bead_annotations`. When br opens a bf-migrated database: its `issues_column_order_matches()` check passes (no extra columns on `issues`), no rebuild fires, annotation data is untouched. When br creates a bead, `bead_annotations` has no row for it — that is correct, the bead just has no annotations yet.

**Use cases**:
- NEEDLE: `needle_attempt`, `needle_session`, `needle_error`, `needle_worker_version`
- Custom tools: `ci_build_id`, `deployment_sha`, `jira_key`
- Fleet tracking: `claimed_at`, `heartbeat_at`, `estimated_minutes_remaining`

### 4B.4 Operation History — `bf log` (`src/log.rs`)

Show the history of mutations on a specific bead or workspace, sourced from the SQLite events table and git history.

```bash
bf log fg-a3f8                          # full history of this bead
bf log --since "2026-04-25"             # all workspace changes since date
bf log --actor worker-3                 # changes by a specific worker
bf log --limit 20                       # last 20 mutations
bf log --status-changes                 # only status transitions
```

Output:

```
2026-04-25 10:00  CREATED     by: human        "Implement auth flow"
2026-04-25 10:05  DEP_ADD     by: human        blocked by fg-b2c1
2026-04-25 14:30  CLAIMED     by: worker-3     status: open → in_progress
2026-04-25 15:45  COMMENT     by: worker-3     "Found edge case in token refresh"
2026-04-25 16:00  CLOSED      by: worker-3     "Completed"
```

**Implementation**: History is sourced from two places:

1. **SQLite events table** (primary): every mutation records an event row (`event_type`, `actor`, `old_value`, `new_value`). `bf log` queries this table directly — O(1) lookup by issue ID, no file scanning.

2. **Git history** (fallback/deep history): `issues.jsonl` is committed to git on each sync. `git log -p .beads/issues.jsonl` shows every past state of the file. `bf log --git` uses this for history older than the events table retention window.

Since JSONL has one entry per bead (not a mutation log), JSONL diffing is not used for history. The events table is the authoritative mutation record.

**`bf log --diff`**: Show the field-level diff from the events table:

```bash
bf log fg-a3f8 --diff
# 2026-04-25 14:30  CLAIMED
#   - status: "open" → "in_progress"
#   + assignee: null → "worker-3"
#   + annotations.needle_attempt: "1" → "2"
```

### 4B.5 Critical Path Computation (`src/critical_path.rs`)

Computes the longest dependency chain through the workspace graph and annotates each bead with its **float** — how many hops it can slip before delaying the epic. Beads with `float == 0` are on the critical path; every day they're blocked delays completion by a day.

**Command**:

```bash
$ bf critical-path fg-epic-auth

Critical path for fg-epic-auth (14 open beads, 3 on critical path):

  float=0  [fg-a3f8] Implement auth token refresh        in_progress (worker-3)
  float=0  [fg-b2c1] Fix concurrent session state bug    open
  float=0  [fg-c4d9] Integration tests: auth flow        open
  float=2  [fg-d5e3] Update API documentation            open
  float=2  [fg-e6f4] Refactor error response format      open
  float=5  [fg-f7g5] Add rate limiting headers           open
  ...

Longest chain: fg-a3f8 → fg-b2c1 → fg-c4d9 → [epic closes]
Minimum remaining time: 3 bead-completions on critical path
```

**Algorithm**: Two-pass walk on the dependency DAG using SQLite recursive CTEs.

Pass 1 (forward — compute earliest start `ES`): start from beads with no open predecessors (`ES=0`). For each bead, `ES = max(ES of all open predecessors) + 1`.

Pass 2 (backward — compute latest start `LS`): start from the epic. For each bead, `LS = min(LS of all successors) - 1`. `float = LS - ES`. Zero-float beads form the critical path.

```sql
WITH RECURSIVE
  -- Forward pass: earliest start for each open bead
  forward(id, es) AS (
    SELECT i.id, 0
    FROM issues i
    WHERE i.status IN ('open', 'in_progress')
      AND NOT EXISTS (
        SELECT 1 FROM dependencies d
        JOIN issues pred ON pred.id = d.depends_on_id
        WHERE d.issue_id = i.id AND d.type = 'blocks'
          AND pred.status IN ('open', 'in_progress')
      )
    UNION ALL
    SELECT d.issue_id, MAX(f.es) + 1
    FROM dependencies d
    JOIN forward f ON f.id = d.depends_on_id
    WHERE d.type = 'blocks'
    GROUP BY d.issue_id
  )
SELECT id, es,
       (SELECT MAX(es) FROM forward) - es AS float
FROM forward
ORDER BY float ASC, es ASC;
```

**Schema addition**: `critical_path_cache` table — recomputed whenever a dependency is added/removed or a bead changes status. Invalidated by a trigger on `dependencies` and `issues.status`.

```sql
CREATE TABLE critical_path_cache (
    bead_id     TEXT PRIMARY KEY REFERENCES issues(id),
    epic_id     TEXT REFERENCES issues(id),
    es          INTEGER NOT NULL,   -- earliest start (hops from root)
    ls          INTEGER NOT NULL,   -- latest start
    float       INTEGER NOT NULL,   -- ls - es; 0 = critical path
    updated_at  DATETIME NOT NULL
);
```

**Integration with claim scoring**: The claim query joins `critical_path_cache` and adds a large bonus for zero-float beads:

```sql
1000.0 / (COALESCE(cp.float, 999) + 1) AS critical_path_bonus
```

A zero-float bead gets `1000 / 1 = 1000` bonus. A float-5 bead gets `1000 / 6 ≈ 167`. A bead not on any epic path gets `1000 / 1000 = 1`. This ensures critical-path beads always sort above non-critical beads regardless of their listed priority.

**`bf dep add-blocker` side effect**: Every time a dependency is created or removed, the cache for the affected epic is invalidated and recomputed inside the same `BEGIN IMMEDIATE` transaction. Recompute cost is O(V + E) — for a 500-bead workspace with 300 deps, under 1ms.

### 4B.6 Velocity-Aware Impact Scoring (`src/velocity.rs`)

The claim scorer currently orders by `downstream_impact / priority`. That's correct for maximizing throughput when all workers and task types take similar time. In practice, a `claude-opus-4-7` worker closes `bug` beads in 8 minutes while the same harness on `claude-haiku-4-5` takes 35 minutes for the same type — a 4× difference. The optimal claim for each worker is the task where `impact / expected_duration` is maximized for *that specific worker configuration*, not the global impact rank.

**Worker metadata on claim**: Workers pass their composition to `bf claim`:

```bash
bf claim --assignee worker-01 \
         --model claude-sonnet-4-6 \
         --harness needle \
         --harness-version 0.5.2 \
         --json
```

**Schema**:

```sql
-- One row per worker session (created on bf claim, updated on bf close)
CREATE TABLE worker_sessions (
    session_id      TEXT PRIMARY KEY,  -- UUID generated by worker
    worker_id       TEXT NOT NULL,
    model           TEXT,              -- 'claude-sonnet-4-6', 'claude-opus-4-7', etc.
    harness         TEXT,              -- 'needle', 'custom'
    harness_version TEXT,              -- '0.5.2'
    bead_id         TEXT REFERENCES issues(id),
    claimed_at      DATETIME,
    closed_at       DATETIME,
    duration_seconds INTEGER           -- populated on bf close
);

-- Aggregated velocity statistics per (model, harness, issue_type)
CREATE TABLE velocity_stats (
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
```

**Update on close**: When `bf close <id>` is called, if the closing worker passed `--session-id <uuid>`, the session row is updated with `closed_at` and `duration_seconds`. A trigger recomputes `velocity_stats` for that `(model, harness, issue_type)` tuple using a window over the last 50 sessions.

**Claim scoring integration**: The claim query joins `velocity_stats` for the requesting worker and computes `score = impact / expected_seconds`:

```sql
SELECT
    i.id,
    i.priority,
    i.issue_type,
    COALESCE(cp.float, 999)            AS critical_float,
    COALESCE(vs.p50_seconds, 1800)     AS expected_seconds,
    -- Combined score: more impact per unit time = higher priority
    (
      downstream_impact * 3.0
      + (4 - i.priority) * 2.0
      + 1000.0 / (COALESCE(cp.float, 999) + 1)
    ) / COALESCE(vs.p50_seconds, 1800) AS score
FROM issues i
LEFT JOIN critical_path_cache cp  ON cp.bead_id = i.id
LEFT JOIN velocity_stats vs       ON vs.issue_type = i.issue_type
                                 AND vs.model    = ?model
                                 AND vs.harness  = ?harness
WHERE i.status = 'open'
  AND i.id NOT IN (SELECT issue_id FROM blocked_issues_cache)
ORDER BY score DESC
LIMIT 1
```

Workers with no velocity history use the 1800s (30-minute) default — conservative but correct. After ~10 completions per type, the estimate is meaningful.

**`bf velocity` command**: Show the velocity table for all known worker configurations:

```
$ bf velocity

Model                  Harness   Type     Samples  p50    p90
claude-opus-4-7        needle    task     142      8m     22m
claude-opus-4-7        needle    bug      38       14m    51m
claude-sonnet-4-6      needle    task     87       18m    45m
claude-sonnet-4-6      needle    bug      24       31m    1h12m
claude-haiku-4-5       needle    task     23       35m    2h10m
```

This table surfaces which model+harness combinations are most efficient for which task types — direct input for fleet composition decisions (which models to provision for which workspace).

**Fallback**: If `--model` is not passed to `bf claim`, the scorer uses the average across all models for that issue type. Backward compatible — undecorated `bf claim` calls work exactly as before, just without per-model scoring.

---

## Phase 4C: br→bf Workspace Migration (`src/migrate.rs`)

Existing repos using `br` migrate to `bf` without data loss. Three migration paths cover all scenarios.

### Migration Path A — Drop-in Replace (zero downtime, zero data conversion)

The `br`→`bf` symlink is the migration for most repos:

```bash
ln -sf ~/.local/bin/bf ~/.local/bin/br
```

`apply_migrations()` in `Storage::open` handles schema differences automatically on first open:

```rust
fn apply_migrations(conn: &Connection) -> Result<()> {
    // All bf-only tables use CREATE TABLE IF NOT EXISTS — no-op on existing DBs.
    // Annotations are stored in bead_annotations, NOT as a column on issues.
    // Reason: br's issues_column_order_matches() in schema.rs:630 fires on every
    // br open and triggers rebuild_issues_table() if the column count differs —
    // which silently destroys any extra column. bead_annotations is a separate
    // table br never reads, writes, or rebuilds.
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS bead_annotations (
            bead_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
            key     TEXT NOT NULL,
            value   TEXT NOT NULL,
            PRIMARY KEY (bead_id, key)
        );
        CREATE INDEX IF NOT EXISTS idx_bead_annotations_key_value
            ON bead_annotations (key, value);
        CREATE TABLE IF NOT EXISTS worker_sessions ( ... );
        CREATE TABLE IF NOT EXISTS velocity_stats ( ... );
        CREATE TABLE IF NOT EXISTS critical_path_cache ( ... );
    ")?;
    Ok(())
}
```

All bf-only tables are created with `CREATE TABLE IF NOT EXISTS` — the call is a no-op when the database was already created by `bf`. No column migrations on `issues` are needed.

**br backward compatibility after migration**: `br` opens a bf-migrated database and finds the `issues` table column count matches exactly — `issues_column_order_matches()` passes, `rebuild_issues_table()` never fires. br ignores `bead_annotations`, `worker_sessions`, `velocity_stats`, and `critical_path_cache` entirely. All `br` commands (list, show, update, doctor) continue working.

### Migration Path B — `bf migrate` (explicit, with backup and verification)

For workspaces where you want an audit point and rollback safety:

```bash
bf migrate /path/to/workspace [--dry-run]
```

Steps:
1. **Pause fleet** (optional): write a `migration_lock` row to the workspace DB so `bf claim` refuses new claims during migration
2. **Backup**: `cp beads.db beads.db.br-backup-<timestamp>`
3. **Apply migrations**: call `apply_migrations()` explicitly
4. **Prime caches**: populate `critical_path_cache` for all epics in one pass
5. **Seed config**: add `bf`-specific keys with defaults to `config.yaml` if absent:
   ```yaml
   # bead-forge additions (br ignores these)
   claim_ttl_minutes: 30
   rotate_age_days: 30
   rotate_max_size_mb: 5
   ```
6. **Verify forward compat**: run `br doctor --db beads.db` — must exit 0
7. **Verify backward compat**: run `bf doctor --check` — must exit 0
8. **Release fleet**: remove `migration_lock`

Output:
```
Migrating /home/coding/FORGE/.beads
  ✓ Backed up beads.db → beads.db.br-backup-2026-04-30T09:15:00Z
  ✓ Applied schema migrations (created bead_annotations, worker_sessions, velocity_stats, critical_path_cache tables)
  ✓ Primed critical_path_cache for 3 epics (247 beads)
  ✓ Updated config.yaml with bf defaults
  ✓ br doctor: OK
  ✓ bf doctor: OK
Migration complete. br symlink: ln -sf ~/.local/bin/bf ~/.local/bin/br
```

### Migration Path C — `bf migrate --from-jsonl` (events table lost)

When `beads.db` is missing or corrupted beyond repair and only `issues.jsonl` is available (the `br doctor --repair` scenario). The standard reimport loses the `events` table — all claim/close history vanishes.

`--from-jsonl` recovers history from git:

```bash
bf migrate /path/to/workspace --from-jsonl [--seed-velocity]
```

1. **Reimport**: stream `issues.jsonl` into fresh SQLite (standard import)
2. **Reconstruct events from git log**:
   ```bash
   git log --follow -p .beads/issues.jsonl
   ```
   For each commit that touched `issues.jsonl`:
   - Diff the JSONL before/after
   - New bead appearing → synthetic `created` event with commit timestamp + author
   - Bead status changing `open→in_progress` → synthetic `claimed` event
   - Bead status changing `in_progress→closed` → synthetic `closed` event + compute `duration_seconds`
   - Synthetic events get `metadata: {"source": "git-reconstructed", "commit": "<sha>"}`
3. **Seed velocity stats** (`--seed-velocity`): from reconstructed `closed` events with known actor, compute `(model=unknown, harness=unknown, issue_type)` averages. Actor name patterns (`worker-claude-sonnet-4-6-01`) can infer model if the naming convention was consistent.

Recovery completeness depends on how often `bf sync` was run (each sync = one JSONL commit in git). With daily syncs: ~95% of events reconstructed. With hourly syncs: ~99%.

### Migration Checklist for All Repos

```bash
# Per-machine: install bf, symlink br
curl -L https://github.com/jedarden/bead-forge/releases/latest/download/bf-linux-x86_64 \
  -o ~/.local/bin/bf && chmod +x ~/.local/bin/bf
ln -sf ~/.local/bin/bf ~/.local/bin/br

# Per-workspace: explicit migration with backup
for workspace in \
  /home/coding/FORGE \
  /home/coding/NEEDLE \
  /home/coding/AgentScribe \
  /home/coding/ARMOR \
  /home/coding/SIGIL \
  /home/coding/CLASP \
  /home/coding/bead-forge; do
  bf migrate "$workspace"
done

# Update NEEDLE config to pass worker metadata
# In .config/needle/adapters/claude-sonnet.yaml, add to invoke_template:
#   bf claim --model claude-sonnet-4-6 --harness needle --harness-version 0.5.2 ...
```

### Schema Compatibility Matrix

| Scenario | br behavior | Result |
|----------|-------------|--------|
| br opens bf-created DB | `issues` column count matches br's expected count exactly (no extra columns); br ignores `bead_annotations`, `worker_sessions`, `velocity_stats`, `critical_path_cache` | ✓ Full compat |
| bf opens br-created DB | `apply_migrations()` creates `bead_annotations` + bf-only tables via `CREATE TABLE IF NOT EXISTS` | ✓ Transparent |
| bf opens very old br DB (missing `issues` columns) | br's own `rebuild_issues_table()` handles this on next `br` open; bf schema also applies its own `CREATE TABLE IF NOT EXISTS` for bf-only tables | ✓ Handled |
| br and bf open same DB concurrently | `CREATE TABLE IF NOT EXISTS` is idempotent; WAL serializes writes | ✓ Safe |
| bf opens bf-created DB | `apply_migrations()` is no-op — all tables already exist | ✓ Idempotent |

---

## Phase 5: NEEDLE Integration

NEEDLE's `bead_store/mod.rs` chains several `br` CLI calls that have race windows between them. Each race is eliminated by the corresponding `bf` command.

### 5.1 Race 1 — `claim()`: two-process verify (lines 518-558)

**br (racy)**:
```rust
// Call 1: br update <id> --status in_progress --assignee <actor>
// ← race window: another worker sees this bead as open and claims it ←
// Call 2: br show <id>  (to verify .assignee == actor)
```
Two processes, two SQLite opens. If two workers race, both issue `br update` and `br` does not enforce exclusivity — it just overwrites. The `br show` verify is too late: the wrong worker may have overwritten the `assignee` field between the two calls.

**bf (atomic)**:
```bash
bead=$(bf claim --workspace "$WORKSPACE" --assignee "$WORKER" --json)
```
`bf claim` runs the full read-score-update sequence inside a single `BEGIN IMMEDIATE` transaction. No second call, no verify step needed.

### 5.2 Race 2 — `ready()` → `claim()`: TOCTOU (lines 487-507)

**br (racy)**:
```rust
// Call 1: br ready --json  → returns candidate list (a snapshot)
// ← race window: another worker claims the top candidate ←
// Call 2: claim(selected_id)  → may be claiming an already-claimed bead
```
The winning bead is selected from a stale snapshot. By the time `claim()` issues `br update`, the bead may have been claimed by another worker.

**bf (atomic)**:
```bash
bead=$(bf claim --workspace "$WORKSPACE" --assignee "$WORKER" --json)
```
Scoring and claiming happen inside the same `BEGIN IMMEDIATE` transaction. The candidate list is read and the winner is updated atomically — no snapshot staleness.

### 5.3 Race 3 — Mitosis: `create_bead` + `add_dependency` (lines 607-639)

**br (non-atomic)**:
```rust
// Call 1: br create "child title"  → returns new bead ID
// ← process crash here → orphaned bead with no dependency link ←
// Call 2: br dep add <parent_id> <child_id>
```
If NEEDLE dies between `br create` and `br dep add`, the child bead exists but is not linked to the parent. The parent is never closed; the orphaned child is never picked up.

**bf (atomic)**:
```bash
bf batch --json '[
  {"op": "create", "title": "child 1", "type": "task"},
  {"op": "create", "title": "child 2", "type": "task"},
  {"op": "dep_add_blocker", "parent": "fg-a3f8.1", "child": "fg-a3f8"},
  {"op": "dep_add_blocker", "parent": "fg-a3f8.2", "child": "fg-a3f8"},
  {"op": "close", "id": "fg-a3f8", "reason": "Split into children"}
]'
```
All five operations run inside one `BEGIN IMMEDIATE` transaction. If NEEDLE dies mid-batch, the transaction rolls back automatically — no partial state.

### 5.4 Inefficiency — `labels()` round-trip (lines 584-588)

**br (wasteful)**:
```rust
async fn labels(&self, id: &BeadId) -> Result<Vec<String>> {
    let bead = self.show(id).await?;  // full br show just to get .labels
    Ok(bead.labels)
}
```
One full `br show` (with JSON deserialization of all 35 fields) to extract one field.

**bf (direct)**:
```bash
bf labels <id> --json   # SELECT label FROM labels WHERE issue_id = ?
```
Direct `SELECT` on the `labels` table. Single column, single round-trip.

### 5.5 Inefficiency — Multiple `br label add` calls

**br (unbatched)**:
```rust
for label in &labels {
    br_label_add(id, label).await?;   // separate process per label
}
```
N labels = N process invocations = N SQLite open/close cycles.

**bf (batched)**:
```bash
bf label <id> add label1 label2 label3   # one INSERT per label, one transaction
```

### 5.6 Pluck Strand Update

One-line change in NEEDLE's `pluck.sh` or `bead_store/mod.rs`:

```bash
# Old (racy — two br calls with a race window between them):
# br ready --json | pick_top | xargs br update --status in_progress --assignee "$WORKER"

# New (atomic — single bf call, BEGIN IMMEDIATE claim):
bead=$(bf claim --workspace "$WORKSPACE" --assignee "$WORKER" --json)
```

### 5.7 Close/Block Remains Same

```bash
bf close "$bead_id" --reason "Completed: ..."
bf update "$bead_id" --status blocked
```

These are single-operation writes with no race condition — no change needed.

### 5.8 Symlink for Backward Compatibility

```bash
ln -sf ~/.local/bin/bf ~/.local/bin/br
```

All existing scripts using `br` commands work without modification. NEEDLE's non-atomic chains are replaced incrementally — `bf claim` and `bf batch` are the high-priority fixes; the rest can migrate when convenient.

---

## Phase 6: Build & Deploy

### 6.1 Project Structure

```
bead-forge/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library root
│   ├── model.rs             # Issue, Status, Priority, Annotations
│   ├── jsonl.rs             # JSONL import (sync --import) + export (sync --flush)
│   ├── config.rs            # Config + Metadata parsing
│   ├── id.rs                # ID generation
│   ├── claim.rs             # Atomic BEGIN IMMEDIATE claiming + impact scoring
│   ├── rotate.rs            # JSONL log rotation
│   ├── batch.rs             # Batch operations under single BEGIN IMMEDIATE transaction
│   ├── log.rs               # Operation history from JSONL event log
│   ├── secrets.rs           # Secret pattern scanning
│   ├── cli/
│   │   ├── mod.rs           # clap app definition
│   │   ├── create.rs
│   │   ├── list.rs
│   │   ├── show.rs
│   │   ├── update.rs
│   │   ├── close.rs
│   │   ├── reopen.rs
│   │   ├── delete.rs
│   │   ├── ready.rs
│   │   ├── claim.rs         # `bf claim` subcommand
│   │   ├── annotate.rs      # `bf annotate` subcommand
│   │   ├── batch.rs         # `bf batch` subcommand
│   │   ├── log.rs           # `bf log` subcommand
│   │   ├── rotate.rs        # `bf rotate` subcommand
│   │   ├── dep.rs
│   │   ├── label.rs
│   │   ├── comment.rs
│   │   ├── sync.rs
│   │   ├── doctor.rs
│   │   ├── init.rs
│   │   ├── search.rs
│   │   └── stats.rs
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── schema.rs        # SQLite DDL (identical to br)
│   │   └── sqlite.rs        # rusqlite storage backend
│   └── format/
│       ├── mod.rs
│       ├── text.rs
│       ├── json.rs
│       └── toon.rs
├── tests/
│   ├── common/
│   │   └── mod.rs           # Shared test harness — TempWorkspace, fixture loading
│   ├── jsonl_compat.rs      # Round-trip tests with br JSONL files (read-only fixtures)
│   ├── schema_compat.rs     # Schema match + forward-compat: bf writes, br reads
│   ├── claim_race.rs        # Concurrent BEGIN IMMEDIATE claim tests (isolated tempdir)
│   ├── batch_atomic.rs      # Batch rollback on failure
│   ├── br_isolation.rs      # Verify bf never touches live br workspaces
│   └── secret_scanning.rs   # Secret pattern detection
└── build.rs
```

### 6.2 Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
chrono = { version = "0.4", features = ["serde"] }
rusqlite = { version = "0.31", features = ["bundled"] }
sha2 = "0.10"
rand = "0.8"
regex = "1"                 # Secret pattern scanning
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1"
thiserror = "1"

[dev-dependencies]
tempfile = "3"              # Ephemeral test workspaces — never touch live br workspaces
```

### 6.3 CI via Argo Workflows

Add a `bead-forge-build` WorkflowTemplate to `jedarden/declarative-config`. Builds the Rust binary in a container, pushes to `ronaldraygun/bead-forge`.

### 6.4 Deployment

Deploy as a standalone binary to the Hetzner server at `~/.local/bin/bf`. Symlink `br` -> `bf` for backward compatibility. No daemon, no systemd unit — each `bf` invocation is self-contained.

---

## Test Strategy & Isolation

bf must be testable without touching, modifying, or interfering with any live `br` workspace. Three isolation concerns drive the test strategy:

### Rule 1: Never Share a Live Database

bf tests **never** auto-discover or open a `.beads/beads.db` that `br` is actively using. SQLite WAL mode serializes writers, so concurrent opens are safe from a corruption standpoint — but test mutations (creates, claims, closes) would contaminate real workspaces with spurious beads.

All tests target exactly one of:
- **`:memory:`** — rusqlite in-memory database (unit tests, zero disk I/O)
- **`tempfile::tempdir()`** — ephemeral directory deleted on drop (integration tests)
- **Read-only fixture copy** — `issues.jsonl` copied out of a real workspace into a tempdir, never written back (compat tests)

### Rule 2: Never Point `br` at a bf Test Database

The schema compat test must verify `br` can read a `bf`-created database — but it cannot use br's own workspace for this. The test creates a fresh tempdir, runs `bf init` + `bf create` in it, then invokes `br --db /path/to/temp/beads.db` against that file. The real `br` workspace is untouched.

### Rule 3: JSONL Fixture = Read-Only Copy

The JSONL round-trip test uses a snapshot of a real workspace's `issues.jsonl` as a fixture, not a live file. The fixture is checked into `tests/fixtures/` (or copied at test time with `--allow-stale`). `bf sync --import` loads it into a tempdir database; `bf list --format json` is compared against the expected output derived from `br list --format json` run against the same snapshot.

```
tests/fixtures/
├── forge-snapshot.jsonl    # snapshot of FORGE workspace (committed, static)
└── needle-snapshot.jsonl   # snapshot of NEEDLE workspace
```

These fixtures are updated manually when the schema changes: `cp /home/coding/FORGE/.beads/issues.jsonl tests/fixtures/forge-snapshot.jsonl`.

### Test Harness (`tests/common/mod.rs`)

```rust
pub struct TempWorkspace {
    dir: tempfile::TempDir,   // deleted on drop — no cleanup needed
    pub db_path: PathBuf,
    pub storage: Storage,
}

impl TempWorkspace {
    pub fn new() -> Result<Self> {
        let dir = tempfile::tempdir()?;
        let db_path = dir.path().join(".beads/beads.db");
        fs::create_dir(dir.path().join(".beads"))?;
        let storage = Storage::open(&db_path)?;
        Ok(Self { dir, db_path, storage })
    }

    pub fn from_fixture(jsonl: &Path) -> Result<Self> {
        let ws = Self::new()?;
        import_jsonl(jsonl, &ws.storage)?;
        Ok(ws)
    }

    // Run br against this workspace's db (for compat tests)
    pub fn br(&self, args: &[&str]) -> std::process::Output {
        Command::new("br")
            .arg("--db").arg(&self.db_path)
            .args(args)
            .output()
            .expect("br not in PATH")
    }
}
```

Unit tests in `#[cfg(test)]` modules use `Storage::open(":memory:")` directly — no `TempWorkspace` needed.

### Schema Forward-Compatibility

bf adds tables that br doesn't know about. No columns are added to the shared `issues` table.

| Addition | br behavior |
|----------|-------------|
| `bead_annotations` table | br ignores unknown tables entirely; `issues` column count is unchanged so `issues_column_order_matches()` passes and `rebuild_issues_table()` never fires |
| `worker_sessions` table | Same — ignored |
| `velocity_stats` table | Same — ignored |
| `critical_path_cache` table | Same — ignored |

**Why no column on `issues`**: br's `issues_column_order_matches()` in `schema.rs:630` checks `actual_columns.len() != EXPECTED_ISSUE_COLUMN_ORDER.len()`. Any extra column causes this to return `false`, which triggers `rebuild_issues_table()` — a full DROP + recreate using only br's canonical column list. An `annotations` column on `issues` would be silently destroyed on every `br` open. Storing annotations in `bead_annotations` sidesteps this entirely.

Migration on `Storage::open` (fully idempotent — all `CREATE TABLE IF NOT EXISTS`):

```rust
fn apply_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS bead_annotations (
            bead_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
            key     TEXT NOT NULL,
            value   TEXT NOT NULL,
            PRIMARY KEY (bead_id, key)
        );
        CREATE INDEX IF NOT EXISTS idx_bead_annotations_key_value
            ON bead_annotations (key, value);
        -- other bf-only tables follow
    ")?;
    Ok(())
}
```

No `ALTER TABLE` on `issues`, ever. The migration is a pure table-creation pass.

---

## Compatibility Verification

All tests run against **isolated temp workspaces** — no live br workspace is touched.

### JSONL Round-Trip Test

```bash
# 1. Copy real workspace JSONL as a read-only fixture (never modify the original)
FIXTURE=$(mktemp -d)
cp /home/coding/FORGE/.beads/issues.jsonl "$FIXTURE/issues.jsonl"

# 2. bf imports the fixture into a fresh tempdir
BFDIR=$(mktemp -d)
bf init --prefix fg --db "$BFDIR/beads.db"
bf sync --import-only --db "$BFDIR/beads.db" --jsonl "$FIXTURE/issues.jsonl"

# 3. bf exports its view of the data
bf list --format json --db "$BFDIR/beads.db" | jq -S '.' > /tmp/bf-output.jsonl

# 4. br reads the original JSONL directly (read-only, not our tempdir)
br list --format json --db "$FIXTURE/issues.jsonl" --no-db | jq -S '.' > /tmp/br-output.jsonl

# 5. Compare (order-independent by ID)
diff /tmp/bf-output.jsonl /tmp/br-output.jsonl
# Must be empty — bf and br produce identical output from the same JSONL

# Cleanup
rm -rf "$FIXTURE" "$BFDIR"
```

### Schema Forward-Compatibility Test

```bash
# 1. bf creates a workspace in a fresh tempdir (bf schema, including bf-only tables)
BFDIR=$(mktemp -d)
bf init --prefix fg --db "$BFDIR/beads.db"
bf create --title "compat test" --type task --priority 1 --db "$BFDIR/beads.db"
bf create --title "another bead" --type bug --priority 0 --db "$BFDIR/beads.db"
bf annotate fg-xxx needle_attempt=1 --db "$BFDIR/beads.db"
# Annotation is stored in bead_annotations table, NOT in issues column

# 2. br opens the same file (not its own workspace — pointed explicitly at the temp db)
br list --db "$BFDIR/beads.db"         # must list 2 beads without error
br doctor --db "$BFDIR/beads.db"       # must pass integrity check
# Key assertion: br's issues_column_order_matches() sees exact column count → passes
# rebuild_issues_table() is NOT triggered → bead_annotations table untouched
br update fg-xxx --status closed --db "$BFDIR/beads.db"  # must succeed

# 3. bf re-reads the br-mutated database (bead_annotations must survive br's UPDATE)
bf show fg-xxx --db "$BFDIR/beads.db" --json | jq '.annotations'
# Must still contain {"needle_attempt": "1"} — annotation row in bead_annotations
# is untouched by br's UPDATE to the issues table

# Cleanup
rm -rf "$BFDIR"
```

### Reverse Compatibility Test

```bash
# 1. br creates a workspace (br schema — no bead_annotations table, no velocity_stats table)
BRDIR=$(mktemp -d)
br init --prefix fg --db "$BRDIR/beads.db"
br create "br-created bead" --type task --priority 2 --db "$BRDIR/beads.db"

# 2. bf opens the br-created database
# Storage::open runs apply_migrations() which creates bead_annotations + other bf-only
# tables via CREATE TABLE IF NOT EXISTS. No ALTER TABLE on issues is needed.
bf list --db "$BRDIR/beads.db"         # must list the br-created bead without error
bf annotate fg-yyy test_key=hello --db "$BRDIR/beads.db"  # inserts into bead_annotations
bf show fg-yyy --db "$BRDIR/beads.db" --json | jq '.annotations'
# Must contain {"test_key": "hello"} — fetched via JOIN on bead_annotations

# 3. br can still read the migrated database (new tables are ignored by br)
# Critical: issues table column count is still exactly what br expects
br list --db "$BRDIR/beads.db"         # must still work
br doctor --db "$BRDIR/beads.db"       # must still pass — no issues column changes

# Cleanup
rm -rf "$BRDIR"
```

### Claim Race Test

```bash
# Setup: create a test workspace with 10 beads of varying priority
cd /tmp && rm -rf bf-test && mkdir bf-test && cd bf-test && bf init --prefix bf
for i in $(seq 1 10); do
  bf create --title "test-$i" --priority $((i % 5))
done

# === Test 1: Basic thundering herd (20 workers, 10 beads) ===
# 20 workers claim simultaneously. Exactly 10 get a bead, 10 get NONE.
for i in $(seq 1 20); do
  bf claim --any --assignee "worker-$i" --json > /tmp/claim-$i.out 2>&1 &
done
wait

# Verify: exactly 10 claims succeeded, 10 returned NONE
grep -cl '"id"' /tmp/claim-*.out | wc -l  # must be 10
grep -cl 'NONE' /tmp/claim-*.out | wc -l  # must be 10

# Verify: no bead claimed by more than one worker
cat /tmp/claim-*.out | jq -r '.id' | sort | uniq -c | sort -rn
# Every ID must appear exactly once

# === Test 2: Priority ordering ===
# The 10 workers that succeeded should have beads in priority order
# (priority 0 beads claimed before priority 4)
# Verify by correlating claim output order with priority values

# === Test 3: Sequential claims (depletion) ===
# Create 20 more beads, claim one at a time, verify queue drains
for i in $(seq 11 30); do
  bf create --title "test-$i" --priority $((i % 5))
done
for i in $(seq 1 20); do
  result=$(bf claim --any --assignee "seq-worker-$i" --json)
  [ -n "$result" ] && echo "$result" | jq -r '.id'
done
bf list --status open --format json
# Must return empty — all beads claimed

# === Test 4: Stale claim reclamation ===
# Claim a bead, manually age its updated_at past claim_ttl
# Then claim again — the stale bead should be reclaimed and re-claimable
bf claim --any --assignee "dead-worker" --json
# (test harness: set claim_ttl to 5s for this test, or mock updated_at)
bf doctor --reclaim-stale --ttl 0s
bf list --status open --format json
# The dead-worker's bead must be back in open status

# === Test 5: Stale reclamation as side effect ===
# Dead worker's bead is reclaimed when another worker claims
bf claim --any --assignee "scavenger" --json
# scavenger gets a bead; dead-worker's bead is also reclaimed to open

# === Test 6: Multi-workspace claiming ===
# Create two workspaces, claim --any gets highest priority across both
mkdir /tmp/ws-a && cd /tmp/ws-a && bf init --prefix wa
mkdir /tmp/ws-b && cd /tmp/ws-b && bf init --prefix wb
bf create --title "ws-a high" --priority 0   # in ws-b
bf create --title "ws-a low"  --priority 4   # in ws-b
cd /tmp/ws-a && bf create --title "ws-a mid" --priority 2
bf claim --any --assignee "multi-worker" --json --workspace /tmp/ws-a --workspace /tmp/ws-b
# Should get ws-b's "ws-a high" (priority 0)
```

---

## Implementation Order

| Phase | Description | Estimated Effort |
|-------|-------------|-----------------|
| 1 | Core library (model, JSONL read/write, config, ID gen) | Foundation |
| 2 | SQLite cache layer (schema, storage, sync, doctor) | Medium |
| 3 | CLI commands (all br-compatible commands) | Medium |
| 4 | Concurrent claiming (BEGIN IMMEDIATE, impact scoring, stale reclamation, multi-workspace) | Medium |
| 4B | Extended features (log rotation, batch ops, annotations, operation history, secret scanning, critical path, velocity scoring) | Medium |
| 5 | NEEDLE integration (pluck.sh update, backward compat) | Small |
| 6 | Build & deploy (CI, binary) | Small |

Phases 1-3 produce a fully functional standalone `bf` that replaces `br`. Phase 4 adds atomic claiming for fleet operations. Phase 4B adds extended features that make bf more powerful than br. Phases 5-6 integrate and deploy.

---

## Key Design Decisions

1. **SQLite-primary, JSONL as git artifact**: SQLite is the live database — all reads and writes go through it. JSONL is exported on `bf sync` for git commit, providing backup, cross-machine sharing, and recovery. This matches how bd and br work, but with rusqlite (no corruption) and `BEGIN IMMEDIATE` for atomic claim (no race condition).

2. **rusqlite, not FrankenSQLite**: We already proved in the beads_rust fork that FrankenSQLite has corruption issues. Start clean with rusqlite.

3. **`BEGIN IMMEDIATE` for atomic claim**: The read-score-update sequence runs in a single SQLite write transaction. No flock, no streaming rewrite, no temp files. SQLite's own locking serializes concurrent claims correctly.

4. **No automatic git commits**: Match br's explicit, non-invasive philosophy. Users run `bf sync` when they want to commit JSONL changes.

5. **Cattle, not pets**: Workers are disposable fleet units with no identity, no memory, and no conversation. Claims are atomic SQL dequeues from a priority queue, not negotiations.

6. **Stale reclamation as a side effect of claiming**: The first statement in every `BEGIN IMMEDIATE` claim transaction resets expired `in_progress` beads. No separate heartbeat system, no background process.

7. **Server as future optimization, not v1**: A persistent in-memory server would reduce per-claim latency from ~2ms to ~0.1ms by eliminating SQLite file I/O. Premature for current scale — can be added transparently later.

8. **Rotation keeps JSONL artifact lean**: Closed beads older than `rotate_age_days` are exported to archive files (`issues.jsonl.1`, etc.) rather than the active `issues.jsonl`. Keeps the committed artifact small and git diffs readable.

9. **Annotations as extension mechanism**: Arbitrary `BTreeMap<String, String>` on beads lets external tools (NEEDLE, custom scripts) store structured metadata without schema changes. Stored in a separate `bead_annotations` table — never as a column on `issues` — because br's `issues_column_order_matches()` triggers `rebuild_issues_table()` on any column count mismatch, silently destroying any extra column on every br open. br never reads or writes `bead_annotations`, so annotation data survives any br operation unchanged.

10. **Events table for history**: Every mutation records an event row. `bf log` queries the events table — O(1) by issue ID. Long-term history via git log on `issues.jsonl`.

11. **Secret scanning before write**: Built-in regex-based secret detection prevents credentials from entering SQLite or the JSONL export. Configurable patterns, git pre-commit hook integration.

12. **`.beads/` is committed to git**: `issues.jsonl` + config are version-controlled. `beads.db` is gitignored (binary, rebuilt from JSONL on `bf sync --import`). Secret scanning protects the exported JSONL.

13. **Critical path drives claim priority, not just listed priority**: Listed priority is a human signal, but it decays as the project evolves and humans stop curating it. Critical path float is computed from the live dependency graph and is always current. A priority-3 bead on the critical path outranks a priority-0 bead with float=5 — the fleet naturally works on what matters most without human recuration.

14. **Worker composition is first-class metadata**: `--model`, `--harness`, `--harness-version` on `bf claim` record what kind of agent did the work. This transforms the events table from a bare audit log into a performance dataset: which model+harness combinations close which task types fastest. The fleet becomes self-optimizing — velocity data informs which workers to dispatch to which workspace, without any external ML pipeline.

---

## Alternative Approaches Considered

### In-Memory Server (deferred)

The plan originally included a coordination server: persistent process, Unix socket protocol, `Arc<Mutex<ServerState>>`, in-memory BTreeMap priority queues, heartbeat-based stale detection. This was deferred to a future optimization because SQLite `BEGIN IMMEDIATE` delivers sufficient throughput at current scale.

The server would reduce per-claim latency from ~2ms (SQLite transaction) to ~0.1ms (memory pop). This matters only if fleets exceed 50 workers or claim frequency exceeds ~500/sec — neither is on the horizon. The server can be added transparently: `bf claim` detects the server socket, delegates if present, falls back to SQLite if not.

### MCP Agent Mail (Dicklesworthstone/mcp_agent_mail_rust)

Jeffrey Emanuel's companion system for inter-agent coordination. A Rust MCP server with 36 tools, 33 resources, Git-backed archive, SQLite indexing, HTTP/TUI/Robot surfaces.

**Architecture**: Agents register identities, claim file reservations with TTL, exchange threaded messages, negotiate work assignments via acknowledgements. Coordination happens through conversation — "I'm taking src/auth/**, can you work on tests/?"

**Why it doesn't fit bead-forge**: Agent Mail is built for the **pets** model — named, persistent agents that maintain identity across sessions, negotiate via messaging, and coordinate through conversation. NEEDLE uses the **cattle** model — disposable workers that are interchangeable, stateless, and need atomic dequeue without any conversation.

| Aspect | Agent Mail (Pets) | bead-forge (Cattle) |
|--------|-------------------|---------------------|
| Agent identity | Registered, named, persistent | Anonymous, stateless, disposable |
| Work assignment | Negotiation, messaging, agreement | Atomic SQL dequeue (BEGIN IMMEDIATE) |
| Coordination mechanism | Threaded messages, acknowledgements | Kernel-provided mutual exclusion |
| File/workspace ownership | Advisory file reservations | None — beads are the unit of work |
| Communication pattern | Multi-turn conversation | Single CLI invocation |
| Failure mode | Agent re-joins conversation | Agent dies, claim TTL expires, bead reopens |
| Agent count | Small (3-5 named agents) | Large fleet (11-20+ disposable workers) |

**What to borrow from Agent Mail**:
- **TTL-based claim expiration**: Agent Mail uses TTL on file reservations. bead-forge applies the same concept — claims older than `claim_ttl` are reclaimed to `open` on the next claim invocation.

**What to explicitly reject**:
- **Agent identities**: Workers don't register or maintain identity. The `--assignee` field is for logging, not identity.
- **Messaging/threading**: Workers don't send messages to each other. The only communication is claim/close through the JSONL file.
- **File reservations**: Workers operate on entire beads, not file-level reservations.
- **MCP layer**: bead-forge is a CLI. No stdio-based MCP protocol.
- **Advisory locks**: Claims are enforced by `BEGIN IMMEDIATE`, not advisory. Once claimed (inside the transaction), no other worker sees that bead as open.

### BeadHub (juanre/beadhub)

A coordination server for AI agent teams built on top of Steve Yegge's Dolt-based `bd`. The `bdh` CLI wraps `bd` and adds work claiming, file reservation, presence awareness, and inter-agent messaging. Includes a web dashboard.

**Why it doesn't fit**: Tied to Dolt-based `bd` architecture (not JSONL). bead-forge needs JSONL+SQLite compatibility with br, not Dolt compatibility with bd. However, the coordination server concept validates that this problem is real and worth solving.

### Beads Viewer (Dicklesworthstone/beads_viewer)

Graph-aware TUI with PageRank-based prioritization, critical path analysis, and kanban board view.

**Why it doesn't fit**: Read-only visualization tool. Cannot perform writes (claiming, status changes). The PageRank and critical path concepts are interesting for future prioritization but don't solve the concurrency problem.
