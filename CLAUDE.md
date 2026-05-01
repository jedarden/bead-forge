# bead-forge (`bf`)

You are implementing **bead-forge** — a Rust CLI that replaces `br` (beads_rust) as the bead management tool for AI-supervised coding workflows.

## Authoritative References

- **Full implementation plan:** `docs/plan/plan.md` — read the relevant section for any bead you claim
- **User-facing docs:** `docs/README.md` — command reference and architecture overview

## What Is Already Implemented

Check `src/` before starting any bead — significant scaffolding exists:

- `src/model.rs` — `Issue`, `Status`, `Priority`, `IssueType`, `Dependency`, `Comment`, `Event` structs
- `src/storage/schema.rs` — SQLite DDL (all br-compatible tables)
- `src/storage/sqlite.rs` — rusqlite storage backend with `with_immediate_transaction()`
- `src/claim.rs` — `BEGIN IMMEDIATE` atomic claiming
- `src/batch.rs` — multi-op batch under single transaction
- `src/jsonl.rs` — JSONL import/export
- `src/config.rs` — `.beads/config.yaml` parsing
- `src/id.rs` — ID generation
- `src/cli/mod.rs` — clap CLI skeleton

## Build and Validate

```bash
# Always verify your changes compile before closing a bead
cargo build 2>&1 | grep -E "^error"

# Run tests if they exist
cargo test 2>&1 | tail -20
```

## Critical Design Constraints

### 1. Annotations live in `bead_annotations` table — NOT a column on `issues`

br's `issues_column_order_matches()` in `beads_rust/src/storage/schema.rs:630` checks the exact column count on every br open. If it differs from br's expected count, it triggers `rebuild_issues_table()` which drops and recreates `issues` using only br's canonical columns — silently destroying any extra column.

**Always** store annotation data in:
```sql
CREATE TABLE IF NOT EXISTS bead_annotations (
    bead_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    key     TEXT NOT NULL,
    value   TEXT NOT NULL,
    PRIMARY KEY (bead_id, key)
);
```

Never `ALTER TABLE issues ADD COLUMN annotations ...`.

### 2. `BEGIN IMMEDIATE` for all writes that must be atomic

The `with_immediate_transaction()` wrapper in `src/storage/sqlite.rs` handles exponential backoff on `SQLITE_BUSY`. Use it for claim, batch, and any read-score-update sequences. Plain reads use `BEGIN DEFERRED`.

### 3. br compatibility — exact flag and output parity

`bf` is a strict superset of `br`. Every `br` flag, every output format (text/json/toon), every field name must be identical. Run `br <cmd> --help` and `bf <cmd> --help` side by side when implementing a command to verify.

### 4. rusqlite — NOT FrankenSQLite

`Cargo.toml` already uses `rusqlite = { version = "0.31", features = ["bundled"] }`. Do not change this.

## Bead Labels → Plan Sections

| Label | Plan Section |
|-------|-------------|
| `phase-1` | §1 Core Library |
| `phase-2` | §2 SQLite Primary Store |
| `phase-3` | §3 CLI Commands |
| `phase-4` | §4 Concurrent Claiming |
| `phase-4b` | §4B Extended Features |
| `phase-4c` | §4C Migration |
| `phase-5` | §5 NEEDLE Integration |
| `phase-6` | §6 Build & Deploy |

## Closing a Bead

Close with a reason that summarizes what was implemented and where:

```bash
br close <id> --reason "Implemented X in src/Y.rs. cargo build clean."
```

If you cannot complete a bead (missing prerequisite, blocked by another bead, or the spec is ambiguous), update it with the blocker and leave it open:

```bash
br update <id> --status blocked
br comment <id> --text "Blocked: <reason>"
```
