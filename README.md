# bead-forge

Drop-in replacement for [`beads_rust`](https://github.com/dicklesworthstone/beads_rust) (`br`) with atomic claiming, critical-path scoring, velocity tracking, and crash-safe batch operations — designed for NEEDLE multi-worker fleets.

## The Beads Ecosystem

bead-forge is the third generation of a repo-local issue tracker lineage:

**[beads](https://github.com/steveyegge/beads) (Steve Yegge, Python)**
The original. A git-backed issue tracker designed as "memory for coding agents." Each issue is a JSON object; every update appends to a JSONL log. SQLite is a read cache, not the source of truth. A DAG of `blocks/blocked_by` dependencies lets `bd ready` return the next actionable task. Designed for single-agent use — no concurrency story at all.

**[beads_rust](https://github.com/dicklesworthstone/beads_rust) (`br`, Jeffrey Emanuel, Rust)**
Fast Rust port of beads. 10–100× faster than the Python original. Adds `sync --flush-only / --import-only`, a `doctor --repair` command, TOON output format (token-optimized for LLMs), and orphan handling. Maintains full JSONL and `.beads/` compatibility. Our codebase runs a fork of this with a rusqlite shim replacing the upstream FrankenSQLite backend ([upstream issue #171](https://github.com/dicklesworthstone/beads_rust/issues/171)). Concurrency story is identical to `beads` — SQLite single-writer, no atomic claiming.

**bead-forge (`bf`, this repo)**
Superset of `br`. Preserves 100% of the JSONL format, `.beads/` directory layout, and CLI surface so existing scripts, NEEDLE workers, and CLAUDE.md instructions that reference `br` continue to work without changes. Adds atomic claiming, critical-path scoring, velocity tracking, and crash-safe batch operations — the features required for reliable multi-worker fleets.

## Problem

`br` claiming is a client-side read-then-write race:

```
Worker A: br list → sees bead-123 (open, highest priority)
Worker B: br list → sees bead-123 (same bead)
Worker C: br list → sees bead-123 (same bead)

Worker A: br update bead-123 --status in_progress → SUCCESS
Worker B: br update bead-123 --status in_progress → FAILS (already claimed)
Worker C: br update bead-123 --status in_progress → FAILS (already claimed)
```

With 11+ workers, 10 workers waste cycles on retries. Observed in production: 4 workers simultaneously claiming the same bead, pervasive phantom claims with 20-worker fleets.

The `br` busy_timeout prevents `database is busy` failures but doesn't prevent the thundering herd. The rusqlite shim fixed SQLite corruption but not contention.

## Solution: `BEGIN IMMEDIATE` transaction

bead-forge runs the entire read-score-pick-update sequence inside a single `BEGIN IMMEDIATE` transaction:

```sql
BEGIN IMMEDIATE;
  SELECT id, priority, critical_path_float
    FROM issues
   WHERE status = 'open'
     AND NOT EXISTS (
           SELECT 1 FROM dependencies d
            WHERE d.blocks_id = issues.id
              AND EXISTS (SELECT 1 FROM issues b WHERE b.id = d.blocked_by_id AND b.status != 'closed')
         )
   ORDER BY priority ASC, critical_path_float DESC
   LIMIT 1;
UPDATE issues SET status = 'in_progress', assignee = ?, updated_at = ? WHERE id = ?;
COMMIT;
```

SQLite's write lock serializes the transaction. The second worker to arrive blocks for ~0.5–2 ms (microseconds of actual work) then picks the next available bead. No phantom claims. No server required — the database file itself is the coordination primitive.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        .beads/                              │
│  issues.jsonl   ← append-only audit log (source of truth)  │
│  beads.db       ← SQLite query cache + bf-only tables       │
│  config.yaml    ← workspace config (bf fields are additive) │
└─────────────────────────────────────────────────────────────┘
          ↑ read/write
┌─────────────────────────────────────────────────────────────┐
│                     bf (bead-forge CLI)                      │
│                                                             │
│  Claiming  ──── BEGIN IMMEDIATE tx (atomic, serialized)     │
│  Mitosis   ──── BEGIN IMMEDIATE tx (atomic split)           │
│  Batch     ──── BEGIN IMMEDIATE tx (atomic multi-op)        │
│                                                             │
│  Critical path ── DAG float via recursive CTE               │
│  Velocity      ── p50/p90 close times per model+type        │
│  Annotations   ── key/value on any bead                     │
│  Event log     ── operation history per bead                │
│  Secret scan   ── pre-commit hook (gitleaks rules)          │
│  Rotation      ── archive closed beads older than N days    │
└─────────────────────────────────────────────────────────────┘
```

### bf-only SQLite tables

These tables are additive — `br` ignores them when operating on a migrated workspace, so `br` and `bf` can be used interchangeably on the same `.beads/` directory.

| Table | Purpose |
|-------|---------|
| `bead_annotations` | Arbitrary key/value metadata per bead |
| `worker_sessions` | Active worker registry (name, model, started_at) |
| `velocity_stats` | p50/p90 close times per model + harness + issue_type |
| `critical_path_cache` | Pre-computed DAG float scores (invalidated on dep changes) |
| `operation_log` | Full event history per bead |

### Data authority: SQLite vs JSONL

**CRITICAL**: bead-forge inverts the authority model from upstream beads/br:

| Tool | Source of truth | Checkpoint role |
|------|----------------|-----------------|
| beads / br | JSONL (`issues.jsonl`) | SQLite is read-cache |
| bead-forge | SQLite (`beads.db`) | JSONL is git-tracked checkpoint |

**Why the inversion**: Multi-worker fleets need atomic read-modify-write operations (`bf claim`, `bf mitosis`, `bf batch`). These execute inside `BEGIN IMMEDIATE` transactions on SQLite. JSONL is append-only and cannot support atomic read-write cycles.

**Implication**: Beads created or modified since the last `bf sync --flush-only` exist **only in SQLite**. Running `bf doctor --repair` without flushing first destroys these unflushed beads.

### Flush-before-repair rule

`bf doctor --repair` rebuilds SQLite from JSONL. To protect against data loss:

```bash
# ALWAYS flush first (or use --flush-first)
bf sync --flush-only
bf doctor --repair

# OR use the safe flag
bf doctor --repair --flush-first

# OR force (WARNING: unflushed beads are lost)
bf doctor --repair --force
```

If unflushed beads exist and neither `--flush-first` nor `--force` is specified, `repair` refuses with an error listing the beads that would be lost.

**Why this matters**: On 2026-06-10, seven independent agents across seven workspaces each lost their entire first batch of freshly created beads by running `doctor --repair` after bulk creates. Four db-only beads were permanently lost.

### Claim scoring

`bf claim` selects the highest-priority ready bead using a composite score:

1. **Priority** — P0 always beats P1, etc.
2. **Critical path float** — among equal-priority beads, pick the one on the longest blocking chain (finishing it unblocks the most downstream work)
3. **Velocity affinity** — optional: route beads to workers whose historical p50 close time for that `issue_type` is fastest

### Compatibility with br

bead-forge is a strict superset of `br`. Every `br` command works identically:

| Command | Behavior |
|---------|----------|
| `br create / list / show / update / close / reopen` | Identical output, identical JSONL entries |
| `br ready` | Same DAG-based filtering; bf adds critical-path sort |
| `br sync --flush-only / --import-only` | Same JSONL format |
| `br doctor --repair` | Same repair logic; bf-only tables are not affected |
| `br dep / br label / br search / br stats` | Identical |

The symlink `br → bf` is all that's needed on a migrated machine. Scripts that call `br` call `bf` automatically.

**Forward-compatibility note**: `bf` adds a `content_hash` column to the `issues` table. If `br` is run directly on a migrated workspace, `br doctor` may emit a column-count mismatch warning. This is cosmetic — data is not affected and `br` reads/writes continue to work correctly.

## NEEDLE Integration

[NEEDLE](https://github.com/jedarden/NEEDLE) is a fleet orchestrator that spawns LLM-powered workers (Claude Code, GLM variants) to process beads in parallel. bead-forge was built specifically to support NEEDLE fleets at scale.

### How workers claim beads

NEEDLE's `pluck` strand calls `bf claim` instead of the old `br list` + `br update` two-step:

```bash
# Old (race condition with 11+ workers)
BEAD=$(br list --format json | jq -r '.[0].id')
br update $BEAD --status in_progress --assignee $WORKER

# New (atomic, no race)
bf claim --assignee $WORKER --format json
```

A single `bf claim` call acquires the next available bead in one `BEGIN IMMEDIATE` transaction. Twenty workers can call `bf claim` simultaneously and each gets a distinct bead in ~1ms.

### Mitosis: crash-safe bead splitting

When a worker determines a bead is too large to complete atomically, it splits it into children using `bf mitosis`:

```bash
bf mitosis bf-a3f8 \
  --children '[
    {"title": "Implement the handler", "type": "task", "priority": 2},
    {"title": "Add test coverage",     "type": "task", "priority": 2}
  ]' \
  --reason "Scope too large for single session"
```

All operations — create children, wire dependencies, close parent — execute inside one `BEGIN IMMEDIATE` transaction. If the process is killed mid-operation, the workspace is left in its original state (parent still open, no orphan children).

NEEDLE's current implementation uses five separate `br` calls with crash-unsafe windows between them. Migrating to `bf mitosis` is a one-function change in `bead_store/mod.rs` — see [`docs/needle-mitosis-migration.md`](docs/needle-mitosis-migration.md).

### Batch operations

`bf batch` lets a worker apply multiple operations atomically:

```bash
bf batch --json '[
  {"op": "create",          "title": "subtask A", "type": "task", "priority": 2},
  {"op": "dep_add_blocker", "parent": "@0",        "child": "bf-xyz"},
  {"op": "close",           "id": "bf-xyz",        "reason": "Decomposed into subtasks"}
]'
```

The `@0`, `@1` syntax refers to beads created earlier in the same batch — no two-pass coordination needed.

### Velocity tracking

`bf velocity` shows historical close times per model and issue type:

```
model              type    beads   p50      p90
claude-sonnet-4-6  task    142     4m 12s   18m 44s
claude-opus-4-7    feature  38     22m 05s  1h 12m
glm-5              bug      91     6m 33s   28m 01s
```

NEEDLE uses this to route beads: if a `bug`-type bead arrives and a GLM-5 worker is available, it may get priority over a Sonnet worker if GLM-5's p50 for bugs is lower.

## Installation

No release binary is currently published. Build from source:

```bash
git clone https://github.com/jedarden/bead-forge
cd bead-forge
cargo build --release
cp target/release/bf ~/.local/bin/bf
ln -sf ~/.local/bin/bf ~/.local/bin/br   # drop-in replace br
```

## Migrating an existing workspace

```bash
cd /path/to/repo
bf migrate .beads/
```

Migration primes the bf-only tables (`bead_annotations`, `worker_sessions`, `velocity_stats`, `critical_path_cache`, `operation_log`) from the existing JSONL data. The workspace continues to work with `br` after migration — bf-only tables are ignored by `br`.

## Commands

All `br` commands plus:

| Command | Description |
|---------|-------------|
| `bf claim` | Atomic claim — picks next ready bead in a single transaction |
| `bf mitosis <id> --children '[...]'` | Crash-safe split: create children + wire deps + close parent atomically |
| `bf batch --json '[...]'` | Run multiple operations in one transaction |
| `bf velocity` | Show p50/p90 close times per model + issue type |
| `bf annotate <id> --key k --value v` | Attach arbitrary metadata to a bead |
| `bf log <id>` | Show full event history for a bead |
| `bf critical-path` | Show longest blocking dependency chain |
| `bf rotate --older-than 30d` | Archive closed beads older than threshold |
| `bf migrate .beads/` | Migrate existing br workspace to bf |
| `bf commit-check` | Pre-commit hook: scan staged .beads/ changes for secrets |

## References

- [beads (original)](https://github.com/steveyegge/beads) — Steve Yegge
- [beads_rust](https://github.com/dicklesworthstone/beads_rust) — Jeffrey Emanuel
- [FrankenSQLite corruption issue #171](https://github.com/dicklesworthstone/beads_rust/issues/171)
- [Steve Yegge: Introducing Beads](https://steve-yegge.medium.com/introducing-beads-a-coding-agent-memory-system-637d7d92514a)
- [NEEDLE mitosis migration guide](docs/needle-mitosis-migration.md)
- [Thundering herd analysis](docs/research/thundering-herd-and-work-queue.md)
