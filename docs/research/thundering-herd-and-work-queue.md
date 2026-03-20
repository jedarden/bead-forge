# Thundering Herd Problem in Multi-Worker Bead Systems

## Background

NEEDLE deploys fleets of LLM-powered workers (Claude Code with GLM-5, GLM-5-turbo, GLM-4.7, Sonnet models) that claim and execute beads (work items) from shared workspaces. Each workspace has a `.beads/` directory with a SQLite database and JSONL append log managed by the `br` CLI (beads_rust).

## The Problem

When multiple workers target the same workspace, they all race to claim the highest-priority bead. The current `br` CLI uses a client-side read-then-write pattern:

```
Worker A: br list → sees bd-123 (open, highest priority)
Worker B: br list → sees bd-123 (same bead)
Worker C: br list → sees bd-123 (same bead)

Worker A: br update bd-123 --status in_progress --assignee A → SUCCESS
Worker B: br update bd-123 --status in_progress --assignee B → FAILS (already claimed)
Worker C: br update bd-123 --status in_progress --assignee C → FAILS (already claimed)

Worker B: br list → sees bd-123 still (stale) → retry → FAILS again
Worker C: br list → retries...
```

With 11 workers, the first claim succeeds and 10 workers waste cycles retrying. Each retry hits SQLite's write lock, serializing the contention. Observed behavior:
- 4 workers claiming the same bead simultaneously (kt-eocq, bd-muv incidents)
- `database is busy` errors under 11-worker load
- Workers spending more time in claim retries than doing actual work

## SQLite Limitations

SQLite with WAL mode (currently enabled) allows concurrent reads but serializes writes. The `br` CLI sets a 30-second `busy_timeout`, which prevents failures but doesn't prevent contention:

- **Single writer**: Only one `br update` can execute at a time per database file
- **Client-side claiming**: Each worker independently reads the queue and races to claim
- **No skip-locked**: SQLite has no equivalent to PostgreSQL's `SELECT FOR UPDATE SKIP LOCKED`
- **Cross-workspace exploration**: Workers explore into other workspaces, multiplying contention on popular repos

The rusqlite shim (replacing FrankenSQLite) fixed corruption but not contention.

## Observed Scale

| Date | Workers | Beads completed | Claim races observed | DB busy errors |
|------|--------:|----------------:|---------------------:|---------------:|
| 2026-03-19 | 11 | 39 | 5+ (kt-eocq x3, bd-muv x4) | 1 |
| 2026-03-20 | 11 | 536 | Pervasive (most beads claimed by 2+ workers) | Multiple |

## Solutions Evaluated

### 1. SQLite WAL + Busy Timeout (current)
- **Status**: Deployed
- **Concurrency**: Single writer, readers don't block
- **Claiming**: Client-side race with retries
- **Scale ceiling**: ~10-15 workers before contention dominates
- **Verdict**: Adequate for current fleet, won't scale to 40+

### 2. JSONL-Only Mode (`br --no-db`)
- **Status**: Built into br
- **Concurrency**: Append-only writes (no lock contention for writes)
- **Claiming**: Would need atomic file-based locking
- **Scale ceiling**: Limited by filesystem atomicity guarantees
- **Verdict**: Avoids SQLite but doesn't solve claiming

### 3. PostgreSQL with SKIP LOCKED
- **Status**: Available (CNPG on ardenone-cluster)
- **Concurrency**: Row-level locking, true parallel writes
- **Claiming**: `SELECT FOR UPDATE SKIP LOCKED` — zero contention
- **Scale ceiling**: Hundreds of concurrent workers
- **Verdict**: Solves the problem completely but requires network dependency and br storage backend rewrite

### 4. Valkey/Redis Sorted Sets
- **Status**: Available (Valkey on apexalgo-iad)
- **Concurrency**: Single-threaded but microsecond operations
- **Claiming**: `ZPOPMIN` — atomic pop from priority queue
- **Scale ceiling**: Thousands of concurrent workers
- **Verdict**: Fastest claiming possible but RAM-only without Kvrocks

### 5. Apache Kvrocks (Redis protocol + RocksDB)
- **Status**: Available as Docker image
- **Concurrency**: Redis protocol with disk-backed storage
- **Claiming**: Same as Valkey (`ZPOPMIN`) but persistent on disk
- **Scale ceiling**: Thousands of workers, dataset larger than RAM
- **Verdict**: Best of both worlds — Redis speed, disk durability

### 6. Embedded RocksDB
- **Status**: Available as Rust crate
- **Concurrency**: True concurrent reads and writes (no single-writer bottleneck)
- **Claiming**: Application-level atomic operations
- **Scale ceiling**: Hundreds on single machine
- **Verdict**: Best for embedded use but doesn't help multi-cluster

## Recommendation: bead-forge

Build a drop-in replacement for `br` that embeds a coordination server. Two modes:

1. **Standalone mode** (backward compatible): Same as `br` — SQLite + JSONL, works offline
2. **Server mode**: Runs a coordination server that handles concurrent claiming atomically

Workers connect to the server (unix socket locally, TCP/Tailscale remotely). The server maintains the priority queue in memory (backed by RocksDB or Kvrocks) and handles claiming as an atomic server-side operation — no client-side races.

The server reads and writes the same `.beads/issues.jsonl` format so the system remains backward compatible with `br`.

## Data Model Mapping (Redis/Kvrocks)

```
# Priority queue per workspace
beads:{workspace_hash}:open       → sorted set (score = priority, member = bead_id)
beads:{workspace_hash}:in_progress → sorted set (score = claim_time, for timeout detection)
beads:{workspace_hash}:closed     → set

# Bead data
bead:{id}          → hash map
  title            → string
  description      → string
  status           → open|in_progress|closed|blocked|tombstone
  priority         → 0-4
  issue_type       → task|bug|feature|genesis|docs|refactor
  assignee         → worker session name
  created_at       → ISO 8601 timestamp
  updated_at       → ISO 8601 timestamp
  workspace        → path

# Dependencies
bead:{id}:blockers → set of bead IDs that block this bead
bead:{id}:blocks   → set of bead IDs this bead blocks

# Workspace index
workspace:{path}   → set of bead IDs in this workspace
```

## Atomic Claim Operation

```
# Server-side (pseudocode)
fn claim(workspace: &str, worker: &str) -> Option<BeadId> {
    // Atomic: pop lowest-score (highest priority) from open set
    let bead_id = ZPOPMIN(beads:{workspace}:open);
    if bead_id.is_none() { return None; }

    // Move to in_progress with claim timestamp
    ZADD(beads:{workspace}:in_progress, now(), bead_id);

    // Update bead metadata
    HSET(bead:{bead_id}, status, "in_progress");
    HSET(bead:{bead_id}, assignee, worker);
    HSET(bead:{bead_id}, updated_at, now());

    // Append to JSONL audit log
    append_jsonl(bead_id, status="in_progress", assignee=worker);

    return Some(bead_id);
}
```

Zero contention. 100 simultaneous claim requests each get a different bead in microseconds.

## Stale Claim Detection

```
# Periodic scan (mend equivalent)
fn detect_stale_claims(timeout: Duration) {
    let stale = ZRANGEBYSCORE(beads:{workspace}:in_progress, 0, now() - timeout);
    for bead_id in stale {
        // Return to open queue
        ZREM(beads:{workspace}:in_progress, bead_id);
        ZADD(beads:{workspace}:open, priority(bead_id), bead_id);
        HSET(bead:{bead_id}, status, "open", assignee, null);
    }
}
```

## References

- [Apache Kvrocks](https://github.com/apache/kvrocks) — Redis protocol + RocksDB storage
- [RocksDB](https://rocksdb.org/) — Embedded persistent key-value store
- [Valkey Persistence](https://valkey.io/topics/persistence/) — AOF and RDB persistence modes
- [beads_rust](https://github.com/dicklesworthstone/beads_rust) — Current bead implementation
- [FrankenSQLite corruption issue](https://github.com/dicklesworthstone/beads_rust/issues/171) — Upstream SQLite corruption bug that motivated the rusqlite shim
