# bead-forge (`bf`)

![bead-forge hero](hero.png)

**bead-forge** is a Rust CLI that replaces `br` (beads_rust) as the bead management tool for AI-supervised coding workflows. It is a drop-in replacement for `br` — every command, flag, and output format is identical — with one critical improvement: **concurrent claiming is correct**.

---

## The Problem It Solves

`br` has a race condition at its core. Claiming a bead requires two separate operations:

```
br ready --json          # read candidates (snapshot)
  ↓  (race window)
br update <id> --status in_progress --assignee <worker>
  ↓  (race window)
br show <id>             # verify the claim stuck
```

With 20 NEEDLE workers running simultaneously, multiple workers read the same candidate list, all select the same top bead, and all issue `br update`. SQLite last-writer-wins — 19 workers hold phantom claims on beads they don't actually own.

`bf` eliminates this with a single `BEGIN IMMEDIATE` SQLite transaction:

```
bf claim --assignee worker-7 --model claude-sonnet-4-6 --harness needle --json
```

The entire read-score-update sequence runs inside one write lock. No two workers can observe the same candidate state.

---

## Architecture

```
SQLite (live store)                    issues.jsonl (git artifact)
    │                                        │
    │  all reads and writes                  │  exported by bf sync --flush
    │                                        │  imported by bf sync --import
    ▼                                        ▼
  beads.db                            .beads/issues.jsonl
  (gitignored)                        (committed to git)
```

**SQLite is the live database.** JSONL is a derived snapshot committed to git for backup, cross-machine sharing, and recovery. `beads.db` is rebuilt at any time with `bf doctor --repair`.

**No daemon. No server. No flock.** Each `bf` invocation is self-contained: open DB, acquire write lock, execute, release, exit. SQLite WAL mode enables concurrent reads during writes.

---

## Key Features

### Atomic Concurrent Claiming

```
Worker 1: BEGIN IMMEDIATE → SELECT + score → UPDATE winner → COMMIT
Worker 2: BEGIN IMMEDIATE → (blocked until Worker 1 commits) → SELECT → ...
...
Worker 20: BEGIN IMMEDIATE → SELECT → NONE (queue empty) → COMMIT
```

The entire claim pipeline — stale reclamation, candidate scoring, winner selection, status update, event recording — runs in a single SQLite write transaction. Guaranteed correct under any concurrency level.

### Critical Path Scoring

`bf` computes the **float** of each bead: how many hops it can slip before delaying its epic. Beads on the critical path (`float == 0`) receive a 1000-point bonus in the claim scorer, ensuring the fleet works on the most impactful beads automatically — without human priority curation.

```
$ bf critical-path bf-epic-123

  float=0  [bf-a3f8] Implement auth token refresh        in_progress
  float=0  [bf-b2c1] Fix concurrent session state bug    open
  float=2  [bf-d5e3] Update API documentation            open
```

### Velocity-Aware Scoring

Workers declare their composition when claiming:

```bash
bf claim --assignee worker-7 \
         --model claude-sonnet-4-6 \
         --harness needle \
         --harness-version 0.5.2 \
         --json
```

`bf` tracks close times per `(model, harness, issue_type)` in `velocity_stats`. The claim scorer weights candidates by `impact / expected_duration` — the fleet maximizes throughput per unit time, not just raw priority. After 10+ completions per cohort, routing becomes measurably smarter.

```
$ bf velocity

Model                  Harness   Type     Samples  p50    p90
claude-opus-4-7        needle    task     142      8m     22m
claude-sonnet-4-6      needle    task     87       18m    45m
claude-haiku-4-5       needle    task     23       35m    2h10m
```

### Atomic Batch Operations

Replaces NEEDLE's crash-unsafe create+dep chains:

```bash
# NEEDLE mitosis: split 1 bead into N children atomically
# Method 1: Dedicated mitosis command (recommended)
bf mitosis bf-a3f8 \
  --children '[
    {"title": "Implement login handler", "type": "task", "priority": 2},
    {"title": "Add session tests", "type": "task", "priority": 2}
  ]' \
  --reason "Split into children"

# Method 2: Direct batch with placeholder references
# Use @0, @1, ... to reference beads created earlier in the batch
bf batch --json '[
  {"op": "create", "title": "Implement login handler", "type": "task"},
  {"op": "create", "title": "Add session tests", "type": "task"},
  {"op": "dep_add_blocker", "parent": "@0", "child": "bf-a3f8"},
  {"op": "dep_add_blocker", "parent": "@1", "child": "bf-a3f8"},
  {"op": "close", "id": "bf-a3f8", "reason": "Split into children"}
]'
```

**Placeholder references**: `@0`, `@1`, etc. resolve to the IDs of beads created earlier in the batch. You don't need to know the child IDs in advance — `bf` substitutes them automatically.

All operations execute in one `BEGIN IMMEDIATE` transaction. A crash mid-batch leaves zero partial state — SQLite rolls back automatically.

### Extensible Annotations

Arbitrary key-value metadata on any bead, transparent to `br`:

```bash
bf annotate bf-a3f8 needle_attempt=3 needle_session=abc123 review_status=needs_review
bf list --annotation needle_attempt=3
```

### Operation History

```bash
bf log bf-a3f8

2026-04-29 10:00  CREATED     by: human                  "Implement auth flow"
2026-04-29 14:30  CLAIMED     by: worker-3 (sonnet-4-6)  open → in_progress
2026-04-29 15:45  COMMENT     by: worker-3               "Found edge case"
2026-04-29 16:00  CLOSED      by: worker-3               "Completed"
```

---

### Secret Scanning (Pre-Commit Hook)

`bf` scans for secrets before writing to the database (see `.beads/config.yaml` → `secret_protection`). The `bf commit-check` command extends this to git — it runs as a pre-commit hook to block commits that would add secrets to `.beads/` files.

```bash
# Manual scan
bf commit-check
# Exits 0 (clean) or 1 (secrets found) with detailed error output
```

**Installation as git pre-commit hook:**

```bash
# In your workspace repo
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/sh
bf commit-check
EOF

chmod +x .git/hooks/pre-commit
```

**What gets scanned:**
- `.beads/config.yaml` — workspace configuration
- `.beads/metadata.json` — database path settings
- `.beads/issues.jsonl` — bead data (git artifact)

**Patterns detected:**
- AWS keys (`AKIA...`, secret access keys)
- GitHub tokens (`ghp_`, `gho_`, `github_pat_`)
- Private keys (`BEGIN RSA PRIVATE KEY`, `BEGIN PRIVATE KEY`)
- JWT tokens
- API keys in URLs
- Database connection strings
- And more (same scanner as write-time protection)

**Allowlist exceptions:**

If a pattern is legitimately non-secret, add to `.beads/config.yaml`:

```yaml
secret_protection:
  allowlist:
    - "^AKIAEXAMPLE$"  # exact string
    - "password: test"  # substring
```

---

## Commands

```
bf create        --title "..." --type <type> --priority <N> [--label ...]
bf list          [--status <s>] [--type <t>] [--assignee <a>] [--format json|text|toon]
bf show          <id>
bf update        <id> [--status <s>] [--assignee <a>] ...]
bf close         <id> [--reason "..."]
bf claim         --assignee <id> [--model <m>] [--harness <h>] [--harness-version <v>]
                 [--workspace <path>] [--any]
bf critical-path <epic-id>
bf batch         [--file ops.json] [--json '[...]']
bf annotate      <id> key=value [--remove key]
bf log           [<id>] [--since <date>] [--actor <a>] [--diff]
bf rotate        [--age-days N]
bf velocity      [--model <m>] [--harness <h>]
bf dep           add-blocker <blocker> <blockee>
bf dep           tree <id>
bf label         <id> add <label> [<label> ...]
bf sync          [--flush-only] [--import-only]
bf doctor        [--repair] [--check] [--reclaim-stale [--ttl <duration>]]
bf init          [--prefix <p>]
bf stats
bf search        <query>
bf commit-check  # git pre-commit hook for secret scanning
```

All `br` commands work identically. `bf` is a strict superset.

---

## NEEDLE Integration

Replace the five non-atomic `br` chains in `bead_store/mod.rs`:

| Old (racy) | New (atomic) |
|-----------|--------------|
| `br ready` → `br update` → `br show` (3 calls, 2 race windows) | `bf claim` (1 call, 1 transaction) |
| `br create` + `br dep add` (orphan if crash between) | `bf batch` (crash-safe, all-or-nothing) |
| `br show` just to get `.labels` field | `bf labels <id>` (direct `SELECT` on labels table) |
| N × `br label add` loops (N processes) | `bf label <id> add l1 l2 l3` (one transaction) |

Backward compatibility — install `br` as a symlink:

```bash
ln -sf ~/.local/bin/bf ~/.local/bin/br
```

---

## Compatibility

`bf` reads and writes the same SQLite schema and JSONL format as `br`. A workspace used by `bf` can be read by `br` and vice versa. Verified by three test suites (see plan §Compatibility Verification):

1. **JSONL round-trip**: `br list --format json` == `bf list --format json` on the same workspace
2. **SQLite compat**: `bf` writes, `br` reads, `br doctor` passes
3. **Claim race**: 20 concurrent `bf claim` processes, 10 beads — exactly 10 claims succeed, no bead claimed twice

---

## Reliability

- **rusqlite** (standard SQLite C library) — not FrankenSQLite, which had known corruption under concurrent writes
- **WAL mode** — concurrent reads never block writes; writers queue, not corrupt
- **`BEGIN IMMEDIATE`** — acquires write lock before any reads; eliminates TOCTOU
- **`SQLITE_BUSY` retry** — exponential backoff up to 5 retries; converts contention spikes to short waits
- **JSONL always authoritative** — `bf doctor --repair` rebuilds `beads.db` from `issues.jsonl` at any time

---

## Implementation

See [`docs/plan/plan.md`](plan/plan.md) for the complete implementation plan including:

- Exact SQLite schema (all 13 tables + indexes)
- `BEGIN IMMEDIATE` retry wrapper implementation
- Critical path CTE algorithm
- Velocity stats schema and claim scoring formula
- Compatibility verification test suites
- NEEDLE integration specifics (file, line numbers, before/after)

---

## Migration from br to bf

### Per-Machine Installation

```bash
# Install bf binary
curl -L https://github.com/jedarden/bead-forge/releases/latest/download/bf-linux-x86_64 \
  -o ~/.local/bin/bf && chmod +x ~/.local/bin/bf

# Drop-in replace br (all existing scripts work unchanged)
ln -sf ~/.local/bin/bf ~/.local/bin/br

# Verify installation
bf --version
br --version  # should show same version
```

### Per-Workspace Migration

**Standard migration** (Path B: explicit, with backup and verification):

```bash
# Migrate a single workspace
bf migrate --workspace /path/to/workspace

# Or migrate all workspaces in a loop
for workspace in \
  /home/coding/FORGE \
  /home/coding/NEEDLE \
  /home/coding/AgentScribe \
  /home/coding/ARMOR \
  /home/coding/SIGIL \
  /home/coding/CLASP \
  /home/coding/bead-forge; do
  bf migrate --workspace "$workspace"
done
```

**What `bf migrate` does:**
1. Acquires migration lock (prevents concurrent claims during migration)
2. Backs up `beads.db` → `beads.db.br-backup-<timestamp>`
3. Applies schema migrations (creates bf-only tables via `CREATE TABLE IF NOT EXISTS`)
4. Primes critical_path_cache for all epics
5. Seeds `config.yaml` with bf-specific defaults if missing
6. Verifies forward compatibility (br can still open the database)
7. Verifies backward compatibility (`bf doctor --check` passes)
8. Releases migration lock

**Dry-run mode** (see what would happen without making changes):

```bash
bf migrate --workspace /path/to/workspace --dry-run
```

**Recovery mode** (for corrupted/missing databases):

If `beads.db` is corrupted or missing, `bf migrate --from-jsonl` rebuilds from `issues.jsonl` and reconstructs events from git history:

```bash
bf migrate --workspace /path/to/workspace --from-jsonl [--seed-velocity]
```

### Verification After Migration

After migrating each workspace, verify both tools can read the database:

```bash
# Verify bf doctor passes
cd /path/to/workspace
bf doctor

# Verify br doctor passes (forward compatibility)
br doctor
```

Both commands should exit 0 with no errors.

**Known limitation:** The migration may show a forward compatibility warning: "Forward compatibility check failed: issues table column count mismatch". This is expected because bf adds a `content_hash` column to the issues table for sync optimization. The database remains fully functional — both `bf doctor` and `br doctor` will pass. The warning indicates that br sees an extra column, but all br operations continue to work correctly.

### NEEDLE Integration Update

After migration, update NEEDLE adapter configs to pass worker metadata for velocity tracking:

```yaml
# In .config/needle/adapters/claude-sonnet.yaml, update invoke_template:
bf claim --model claude-sonnet-4-6 --harness needle --harness-version 0.5.2 ...
```

This enables velocity-aware routing (see §Velocity-Aware Scoring above).

---

## Build & Deploy

Built via Argo Workflows on `iad-ci`. WorkflowTemplate: `bead-forge-build` in `jedarden/declarative-config`.

```bash
# Install
curl -L https://github.com/jedarden/bead-forge/releases/latest/download/bf-linux-x86_64 \
  -o ~/.local/bin/bf && chmod +x ~/.local/bin/bf

# Drop-in replace br
ln -sf ~/.local/bin/bf ~/.local/bin/br

# Verify
bf --version
bf init --prefix bf
bf create "first bead" --type task
```
