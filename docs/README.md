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

Replaces NEEDLE's crash-unsafe create+dep chains. All operations execute in one `BEGIN IMMEDIATE` transaction. A crash mid-batch leaves zero partial state — SQLite rolls back automatically.

#### Batch Operation JSON Schema

Batch operations are specified as a JSON array of operation objects. Each operation must have an `op` field that specifies the operation type, followed by type-specific fields.

**Operation types:**

| Operation | `op` value | Description |
|-----------|------------|-------------|
| Create bead | `create` | Create a new bead with specified properties |
| Update bead | `update` | Modify fields of an existing bead |
| Close bead | `close` | Close a bead with optional reason |
| Add dependency | `dep_add_blocker` | Create a blocking dependency (A blocks B) |
| Remove dependency | `dep_remove` | Remove a dependency between beads |
| Add labels | `label_add` | Add labels to a bead |
| Remove labels | `label_remove` | Remove labels from a bead |
| Add comment | `comment` | Add a comment to a bead |

**Placeholder references**: `@0`, `@1`, etc. resolve to the IDs of beads created at that position in the batch. You don't need to know child IDs in advance — `bf` substitutes them automatically.

#### Operation Schema Details

##### 1. Create (`create`)

Creates a new bead. Returns the generated bead ID in the batch result.

**Required fields:**
- `op`: `"create"`
- `title`: `string` — Bead title

**Optional fields:**
- `type`: `string` — Issue type (default: `"task"`)
- `priority`: `number` — Priority level 0-4 (default: `2`)
- `description`: `string | null` — Bead description
- `assignee`: `string | null` — Assignee username
- `labels`: `string[]` — Labels to apply (default: `[]`)

**Example:**
```json
{"op": "create", "title": "Implement auth", "type": "task", "priority": 1, "labels": ["urgent"]}
```

##### 2. Update (`update`)

Updates fields of an existing bead. Only specified fields are modified.

**Required fields:**
- `op`: `"update"`
- `id`: `string` — Bead ID (can use placeholder like `@0`)

**Optional fields:**
- `title`: `string | null` — New title
- `description`: `string | null` — New description
- `design`: `string | null` — Design document
- `acceptance_criteria`: `string | null` — Acceptance criteria
- `notes`: `string | null` — Notes
- `status`: `string` — New status (`open|in_progress|blocked|closed|...`)
- `priority`: `number` — New priority 0-4
- `assignee`: `string | null` — New assignee (empty string clears)
- `owner`: `string | null` — New owner
- `issue_type`: `string` — New issue type

**Example:**
```json
{"op": "update", "id": "bf-123", "status": "in_progress", "priority": 0}
```

##### 3. Close (`close`)

Closes a bead. Idempotent — succeeds if already closed.

**Required fields:**
- `op`: `"close"`
- `id`: `string` — Bead ID (can use placeholder like `@0`)

**Optional fields:**
- `reason`: `string` — Close reason (default: `"Completed"`)

**Example:**
```json
{"op": "close", "id": "bf-123", "reason": "Fixed in PR #45"}
```

##### 4. Add Dependency (`dep_add_blocker`)

Creates a blocking dependency: the blocker must close before the blocked bead can close. Validates against cycles and duplicates.

**Required fields:**
- `op`: `"dep_add_blocker"`
- `id`: `string` — The bead being blocked (can use placeholder)
- `blocker`: `string` — The bead that blocks (can use placeholder)

**Legacy aliases (deprecated but supported):**
- `parent` → `blocker` (the bead that blocks)
- `child` → `id` (the bead being blocked)

**Direction:** Creates dependency: `id depends_on blocker` (blocker blocks id)

**Example:**
```json
{"op": "dep_add_blocker", "id": "bf-child", "blocker": "bf-parent"}
```

##### 5. Remove Dependency (`dep_remove`)

Removes a dependency between two beads.

**Required fields:**
- `op`: `"dep_remove"`
- `id`: `string` — The bead that has the dependency
- `depends_on`: `string` — The bead to remove dependency from

**Example:**
```json
{"op": "dep_remove", "id": "bf-child", "depends_on": "bf-parent"}
```

##### 6. Add Labels (`label_add`)

Adds labels to a bead. Duplicate labels are ignored (idempotent).

**Required fields:**
- `op`: `"label_add"`
- `id`: `string` — Bead ID (can use placeholder)
- `labels`: `string[]` — Labels to add

**Example:**
```json
{"op": "label_add", "id": "bf-123", "labels": ["urgent", "backend"]}
```

##### 7. Remove Labels (`label_remove`)

Removes labels from a bead.

**Required fields:**
- `op`: `"label_remove"`
- `id`: `string` — Bead ID (can use placeholder)
- `labels`: `string[]` — Labels to remove

**Example:**
```json
{"op": "label_remove", "id": "bf-123", "labels": ["deprecated"]}
```

##### 8. Add Comment (`comment`)

Adds a comment to a bead.

**Required fields:**
- `op`: `"comment"`
- `id`: `string` — Bead ID (can use placeholder)
- `text`: `string` — Comment text

**Optional fields:**
- `author`: `string` — Comment author (default: `"batch"`)

**Example:**
```json
{"op": "comment", "id": "bf-123", "text": "Found edge case in login flow", "author": "worker-7"}
```

#### Batch Execution Examples

**NEEDLE mitosis: split 1 bead into N children atomically**

Method 1: Dedicated mitosis command (recommended)
```bash
bf mitosis bf-a3f8 \
  --children '[
    {"title": "Implement login handler", "type": "task", "priority": 2},
    {"title": "Add session tests", "type": "task", "priority": 2}
  ]' \
  --reason "Split into children"
```

Method 2: Direct batch with placeholder references
```bash
bf batch --json '[
  {"op": "create", "title": "Implement login handler", "type": "task"},
  {"op": "create", "title": "Add session tests", "type": "task"},
  {"op": "dep_add_blocker", "id": "bf-a3f8", "blocker": "@0"},
  {"op": "dep_add_blocker", "id": "bf-a3f8", "blocker": "@1"},
  {"op": "close", "id": "bf-a3f8", "reason": "Split into children"}
]'
```

**Bulk status update:**
```bash
bf batch --json '[
  {"op": "update", "id": "bf-123", "status": "in_progress"},
  {"op": "update", "id": "bf-456", "status": "in_progress"},
  {"op": "comment", "id": "bf-123", "text": "Started work"}
]'
```

**Label management:**
```bash
bf batch --json '[
  {"op": "label_add", "id": "bf-123", "labels": ["urgent", "backend"]},
  {"op": "label_remove", "id": "bf-456", "labels": ["deprecated"]}
]'
```

#### Batch Response Format

Returns a JSON array of results, one per operation:

```json
[
  {"op": 0, "status": "ok", "id": "bf-new1", "message": "Created bead bf-new1"},
  {"op": 1, "status": "ok", "id": "bf-new2", "message": "Created bead bf-new2"},
  {"op": 2, "status": "ok", "message": "ok: bf-parent blocked by bf-new1"},
  {"op": 3, "status": "ok", "message": "Closed bead bf-parent"}
]
```

On error, the transaction rolls back and no partial state is committed:

```json
[
  {"op": 0, "status": "ok", "id": "bf-new1", "message": "Created bead bf-new1"},
  {"op": 1, "status": "error", "error": "Bead not found: bf-missing", "message": null}
]
```

#### Field Validation

Unknown fields in any operation are rejected with a clear error message listing allowed fields. This catches typos early:

```bash
bf batch --json '[{"op": "create", "titlle": " typo"}]'
# Error: Unknown field 'titlle' in operation 'create'. Allowed fields: op, title, type, priority, description, assignee, labels
```

### Extensible Annotations

Arbitrary key-value metadata on any bead, transparent to `br`:

```bash
bf annotate set bf-a3f8 needle_attempt 3
bf annotate set bf-a3f8 needle_session abc123
bf annotate set bf-a3f8 review_status needs_review
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

Most commands accept `--format json|text|toon` for output and a global `-w/--workspace <path>` to target a workspace other than the current directory. The global `--no-auto-flush` flag disables the post-mutation incremental JSONL export for that one invocation (mutations stay db-only and marked dirty until `bf sync --flush-only`); see [Auto-Flush](#auto-flush-phase-71--the-checkpoint-tracks-the-live-store) below.

```
# ── Lifecycle ────────────────────────────────────────────────────────────────
bf create        --title "..." [--type <t>] [--priority <N>] [--description "..."]
                 [--assignee <a>] [--label <l> ...]
bf list          [--status <s>] [--type <t>] [--assignee <a>] [--priority <N>]
                 [--annotation k=v] [--limit <N>] [--all] [--format json|text|toon] [--json]
bf show          <id> [--format json|text|toon] [--json]
bf update        <id> [--title "..."] [--status <s>] [--priority <N>] [--assignee <a>]
                 [--description "..."] [--description-file <path>] [--acceptance-criteria "..."]
                 [--notes "..."] [--design "..."] [--due-at <RFC3339>]
bf close         <id> [--reason "..."]
bf reopen        <id>
bf delete        <id>
bf ready         [--limit <N>] [--format json|text|toon] [--json]   # unblocked beads
bf count         [--status <s>]

# ── Claiming & concurrency ───────────────────────────────────────────────────
bf claim         --assignee <id> [--model <m>] [--harness <h>] [--harness-version <v>]
                 [--any] [--fallback <mode>] [--workspace-paths <p> ...] [--dry-run]
                 [--format json|text|toon] [--json]
bf batch         [--file ops.json] [--json '[...]'] [--stdin]       # atomic multi-op
bf mitosis       <id> --children '[...]' [--reason "..."] [--format ...]   # split into children

# ── Dependencies & structure ─────────────────────────────────────────────────
bf dep add       <blocker> --blocks <blocked> [-t <type>]   # blocker must close first
bf dep remove    <issue> <depends-on>
bf dep list      <id>
bf dep tree      <id> [-d down|up|both] [--max-depth <N>] [--format ...] [--json]
bf critical-path <epic-id> [--max-depth <N>] [--format ...]

# ── Labels, comments, annotations ────────────────────────────────────────────
bf label add     --label <l> [<l> ...] <id>
bf label remove  --label <l> [<l> ...] <id>
bf label list    [<id>]                  # all unique labels if id omitted
bf labels        <id> [--format text|json]                  # efficient single-bead SELECT
bf comments add  <id> <text ...>
bf comments list <id>
bf annotate set    <id> <key> <value>
bf annotate get    <id> <key>
bf annotate remove <id> <key>
bf annotate list   <id>
bf annotate clear  <id>

# ── Query & history ──────────────────────────────────────────────────────────
bf search        [<query>] [-s <status> ...] [-t <type> ...] [--assignee <a>]
                 [-l <label> ...] [--priority-min <N>] [--priority-max <N>] [--limit <N>]
                 [--format json|text|toon]
bf recent        [--status <s>] [--type <t>] [--assignee <a>] [--priority <N>]
                 [--since <RFC3339>] [--before <RFC3339>] [-t <period>] [-n <N>]
                 [--format json|text|toon] [--json]           # period: e.g. 1h, 24h, 7d, 4w
bf log           [<id>] [--limit <N>] [--since <RFC3339>] [--actor <a>]
                 [--status-changes] [--diff] [--git] [--format json|text|toon] [--json]
bf stats         [--by-type] [--by-priority] [--by-assignee] [--by-label] [--format ...]
bf velocity      [--model <m>] [--harness <h>] [--format json|text|toon]

# ── Maintenance & config ─────────────────────────────────────────────────────
bf sync          [--flush-only] [--import-only]
bf merge-jsonl   --ours <A> --theirs <B> [--base <O>] [--output <path>]  # 3-way JSONL merge / git driver
bf doctor        [--repair [--flush-first] [--force]] [--reclaim-stale [--ttl <minutes>]] [--fix-schema] [--reconcile]  # no flags = health check
bf rotate        [--days <N>] [--dry-run]
bf migrate       [--workspace <p>] [--from-jsonl] [--seed-velocity] [--dry-run] [--skip-verify]
bf init          [--prefix <p>]
bf schema        [<target|id>] [--format json|text]      # "all" = DDL; a bead id = full JSON
bf config        list | get <key> | set <key> <value> | path
bf commit-check  # git pre-commit hook for secret scanning
```

All `br` commands work identically. `bf` is a strict superset.

---

## Output Formats

Most read commands accept `--format text|json|toon` (or the `--json` alias for
`--format json`). The JSON shape depends on **which command** you run, and it is
not always a JSON array.

### Listing commands emit JSONL — not a JSON array

`bf list`, `bf ready`, `bf search`, and `bf recent` with `--format json` emit
**JSONL**: one self-contained, compact JSON object per line, joined by newlines,
with **no array wrapper** and no commas between objects.

```bash
$ bf list --format json
{"id":"test-3qv","title":"alpha task","status":"open","priority":2,"issue_type":"task","assignee":null,"labels":[],"created_at":"2026-07-22T15:54:16Z",...}
{"id":"test-2c4","title":"beta task","status":"open","priority":2,"issue_type":"task","assignee":null,"labels":[],"created_at":"2026-07-22T15:54:16Z",...}
```

The concatenated stdout is **not** valid JSON (`{"a":1}\n{"b":2}` is two values,
not one), so you cannot `json.loads()` the whole buffer. Parse it **line by
line**:

```python
import json, subprocess
out = subprocess.check_output(["bf", "list", "--format", "json"], text=True)
beads = [json.loads(line) for line in out.splitlines() if line.strip()]
```

```bash
# jq: slurp the JSONL lines into one array, then operate on it
bf list --format json | jq -s 'map(.id)'
```

Two keys are **always** present on every emitted object, even on a bare bead:

- `assignee` — `null` when unassigned (never omitted)
- `labels` — an array, `[]` when empty (never omitted)

The on-disk `issues.jsonl` checkpoint (written by `bf sync`) omits these when
unset to stay compact; the CLI `--format json` path normalizes them back so a
downstream struct with non-optional `assignee`/`labels` fields deserializes
cleanly.

**Empty results differ by command:**

| Command | Empty `--format json` output |
|---------|------------------------------|
| `bf list` | nothing (empty stdout) |
| `bf search` / `bf recent` | nothing (empty stdout) |
| `bf ready` | `[]` |

`ready` is the one listing command that special-cases emptiness to `[]`; the
others print nothing at all.

### `bf show` emits a one-element array

Unlike the listing commands, `bf show <id> --format json` wraps the single bead
in a **JSON array of one element**:

```bash
$ bf show test-3qv --format json
[{"id":"test-3qv","title":"alpha task","status":"open","priority":2,...}]
```

This is deliberate: NEEDLE's `parse_single_bead` expects `Vec<Bead>` and takes
the first element, so `show` ships an array to stay parse-compatible with that
code path.

### `bf claim` emits a single object

`bf claim --json` prints one JSON object describing the claim (or `{}` when
nothing was claimed) — never an array, never an `Issue`:

```bash
$ bf claim --assignee worker-7 --json
{"bead_id":"test-3qv","assignee":"worker-7","reclaimed":0}
```

`bf velocity --format json` emits a JSON array of stat objects (`[]` when empty).

### `br` produces byte-identical output

`br` delegates to the `bf` binary — either directly (a symlink, as installed by
`ln -sf ~/.local/bin/bf ~/.local/bin/br`) or via a logging shim that writes a
deprecation-telemetry line and then `exec bf "$@"`. Neither path touches stdout,
so every `br` command emits **byte-identical** output to its `bf` equivalent —
same JSONL, same array wrapping for `show`, same empty-case behavior. A parser
written for `br list --format json` works unchanged for `bf list --format json`,
and vice versa.

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

### Data Authority Model

**bead-forge inverts the authority model from upstream beads/br**:

| Tool | Source of truth | Checkpoint role |
|------|----------------|-----------------|
| beads / br | JSONL (`issues.jsonl`) | SQLite is read-cache |
| bead-forge | SQLite (`beads.db`) | JSONL is git-tracked checkpoint |

This inversion is necessary for atomic multi-worker operations. All mutations (create/update/claim) go to SQLite. With `sync.auto_flush` (default **on**), every successful mutation also incrementally exports the changed beads to `issues.jsonl`, so the checkpoint tracks the live store instead of trailing it — a separate flush is no longer needed in normal operation.

### Auto-Flush (Phase 7.1) — the checkpoint tracks the live store

With `sync.auto_flush` (default **on**), every successful mutation (`create`/`update`/`claim`/`close`/`batch`) incrementally exports just the changed beads to `issues.jsonl` (a surgical line replacement, not a full rewrite). The JSONL artifact therefore stays current with SQLite after every command — no separate flush step is required in normal operation. Read-only and diagnostic commands never write the JSONL.

```yaml
# .beads/config.yaml
sync:
  auto_flush: true     # default; set false to make mutations db-only until bf sync --flush-only
```

```bash
bf create --title "..."               # auto-flushed by default → already in issues.jsonl
bf create --title "..." --no-auto-flush   # per-invocation override: db-only, marked dirty
```

**Flush failure never fails the mutation.** Auto-flush is best-effort: if the incremental export fails, the mutation still commits, the dirty mark is retained, and the failure is surfaced loudly — a `warning:` line on stderr and a non-null `warning` field in the `--json` envelope naming the recovery command. The dirty bead stays recoverable.

**Named recovery.** `bf sync --flush-only` is the explicit checkpoint and the recovery path: it clears the entire `dirty_issues` set and rewrites a **full** checkpoint (every bead, not just the dirty one). Run it after an auto-flush warning, or to produce a clean git-committable snapshot:

```bash
bf sync --flush-only
```

### Repair safety — protection is in the doctor, not a manual ritual

`bf doctor --repair` rebuilds SQLite from `issues.jsonl`. Because auto-flush keeps the checkpoint current, the old "ALWAYS `bf sync --flush-only` before repair" ritual (added after the 2026-06-10 incident where seven agents across seven workspaces each lost their first batch of fresh beads to a post-create `doctor --repair`; four db-only beads in ARMOR — bf-4rm7/5zxa/tojg/tr44 — were permanently lost) is **obsolete**. The data-loss guard now lives in the doctor command itself: it **refuses** to repair when unflushed (db-only) beads exist, and tells you exactly what to do.

```bash
bf doctor                              # health check — reports "Unflushed beads: N" if any
bf doctor --repair                     # REFUSES if unflushed beads exist (no data loss)
bf doctor --repair --flush-first       # checkpoint unflushed beads first, then rebuild
bf doctor --repair --force             # proceed anyway — unflushed beads WILL be lost
```

Killing a worker at any point between mutation and flush loses nothing that `git diff .beads/` can't show: with auto-flush on the bead is already in the artifact when the command returns; with auto-flush off the bead survives in SQLite (marked dirty) and is recovered by `bf sync --flush-only`. Pinned by `tests/recovery_and_exit_criteria.rs`.

### Multi-Box & Fleet Hardening (Phase 7.9)

When the same repo is checked out on several boxes, each keeps its own live `beads.db` and shares state through the git-committed `issues.jsonl`. Three layers defend that shared artifact:

**Three-way JSONL merge** — a plain git text merge of `issues.jsonl` is unsafe: each line is a whole bead, so a textual conflict marker corrupts JSON and "take theirs" silently drops beads. `bf merge-jsonl` merges **per-bead** against a common ancestor instead. Resolution is deterministic: a one-sided edit is taken as-is; a two-sided edit resolves last-writer-wins by `updated_at` (ties broken by content hash, so the result is independent of which box runs the merge); a delete that races a concurrent edit keeps the edit (never silently discards work). Wire it as a git merge driver:

```bash
git config merge.beads.name   "bead-forge 3-way JSONL merge"
git config merge.beads.driver "bf merge-jsonl --base %O --ours %A --theirs %B --output %A"
echo '.beads/issues.jsonl merge=beads' >> .gitattributes
```

**Merge anchor** (`.beads/beads.base.jsonl`) — every full flush/import refreshes this snapshot of the last state this box agreed on with the artifact. It is the fallback three-way base for `bf merge-jsonl` when git does not supply `%O` (out-of-band merges across checkouts). Local-only; git-ignored.

**Pre-export history backups** (`.beads/.bf_history/`) — before every full flush overwrites `issues.jsonl`, the previous version is copied into `.bf_history/` and pruned to the newest `history.max_backups` snapshots (default 20). One more recovery layer under a bad export or merge. Enabled by default; disable with `history.enabled: false` in `config.yaml`. Local-only; git-ignored.

**Fleet concurrency tests** (`tests/fleet_concurrency.rs`) — spawn N concurrent `bf` *processes* doing create/claim/close and assert the upstream bug classes stay dead: no parallel-write silent loss (`count` equals successful creates), no loss across flush + fresh-DB reimport, and no bead claimed twice under a 20-worker herd on 15 beads.

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

### Output Format Compatibility (no parser changes)

Migrating to `bf` requires **no changes to JSON consumers**. `bf` is a drop-in
replacement: every command emits byte-identical output to `br`, because `br`
delegates to the `bf` binary (via symlink, or a logging shim that `exec bf
"$@"`) and never alters stdout.

The format to keep in mind (full details in [§ Output Formats](#output-formats)):

- `list` / `ready` / `search` / `recent --format json` → **JSONL** (one object
  per line, not a JSON array). Parse line-by-line — do not `json.loads()` the
  whole buffer.
- `show <id> --format json` → a **one-element array** (for NEEDLE's
  `parse_single_bead`).
- Empty `list`/`search`/`recent` print nothing; empty `ready` prints `[]`.

Any script that already parses `br list --format json` line-by-line keeps
working unchanged against `bf`.

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
7. Verifies backward compatibility (`bf doctor` health check passes)
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

### Migration Checklist

**Step 1: Per-machine installation**

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

**Step 2: Per-workspace migration loop**

```bash
# Migrate all workspaces
for workspace in \
  /home/coding/FORGE \
  /home/coding/NEEDLE \
  /home/coding/AgentScribe \
  /home/coding/ARMOR \
  /home/coding/SIGIL \
  /home/coding/CLASP \
  /home/coding/bead-forge; do
  echo "Migrating $workspace..."
  bf migrate --workspace "$workspace"
done
```

**Step 3: Verify each migration**

```bash
# Verify each workspace passes both doctor checks
for workspace in \
  /home/coding/FORGE \
  /home/coding/NEEDLE \
  /home/coding/AgentScribe \
  /home/coding/ARMOR \
  /home/coding/SIGIL \
  /home/coding/CLASP \
  /home/coding/bead-forge; do
  echo "Checking $workspace..."
  cd "$workspace" || continue
  echo "  bf doctor:"
  bf doctor || echo "  ❌ bf doctor failed"
  echo "  br doctor:"
  br doctor || echo "  ❌ br doctor failed"
done
```

Both `bf doctor` and `br doctor` should exit 0 with no errors for each workspace.

**Step 4: Update NEEDLE adapter configs**

After migration, update NEEDLE adapter configs to pass worker metadata for velocity tracking:

```yaml
# In .config/needle/adapters/claude-sonnet.yaml, update invoke_template:
bf claim --model claude-sonnet-4-6 --harness needle --harness-version 0.5.2 ...
```

This enables velocity-aware routing (see §Velocity-Aware Scoring above).

### Verification After Migration

For each workspace, verify both tools can read the database:

```bash
# Verify bf doctor passes (no flags = health check)
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

### Local Build

```bash
# Clone and build
git clone https://github.com/jedarden/bead-forge.git
cd bead-forge
cargo build --release

# Install
cp target/release/bf ~/.local/bin/bf
chmod +x ~/.local/bin/bf

# Drop-in replace br
ln -sf ~/.local/bin/bf ~/.local/bin/br

# Verify
bf --help
bf list
br list  # should work identically
```

### CI/CD Deployment

Built via Argo Workflows on `iad-ci`. WorkflowTemplate: `bead-forge-build` in `jedarden/declarative-config`.

**Deployment steps (automated by WorkflowTemplate):**

1. **Build**: `cargo build --release` produces `target/release/bf` (7.4M optimized binary)
2. **Package**: Upload binary to GitHub Releases as `bf-linux-x86_64`, and upload a
   `SHA256SUMS` manifest alongside it (e.g. `sha256sum bf-linux-x86_64 > SHA256SUMS`).
   The auto-update client (`bf-update.sh`) **requires** this manifest and refuses to
   install without verifying the binary's checksum against it.
3. **Install**: Download and install to `~/.local/bin/bf`
4. **Symlink**: Create `~/.local/bin/br → bf` symlink for drop-in replacement
5. **Verify**: Test `bf list` and `br list` in a NEEDLE workspace

**Manual installation from release:**

```bash
# Download latest release
curl -L https://github.com/jedarden/bead-forge/releases/latest/download/bf-linux-x86_64 \
  -o ~/.local/bin/bf && chmod +x ~/.local/bin/bf

# Drop-in replace br
ln -sf ~/.local/bin/bf ~/.local/bin/br

# Verify
bf list
br list  # should work identically
```

**Verification in NEEDLE workspace:**

```bash
# In any NEEDLE workspace (e.g., /home/coding/NEEDLE)
cd /home/coding/NEEDLE
br list  # should use bf binary via symlink
bf list  # direct call
```

Both commands should produce identical output, confirming the symlink works correctly.
