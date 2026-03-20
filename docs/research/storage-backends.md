# Storage Backend Options

## Requirements

1. **Concurrent writes** — 40+ workers claiming simultaneously without contention
2. **Persistence** — data survives process restart
3. **Priority queue** — claim highest-priority bead atomically
4. **Network accessible** — workers on remote clusters can connect over Tailscale
5. **Embeddable** — can run as a single binary (no external server dependency for standalone mode)
6. **Compatible** — reads/writes the same JSONL format as br

## Candidates

### RocksDB (embedded)

**What**: LSM-tree key-value store from Meta. Embedded library, no server.

**Pros**:
- True concurrent reads and writes (no single-writer bottleneck like SQLite)
- Rust crate: `rust-rocksdb`
- Persistent on disk, handles datasets much larger than RAM
- Column families can model different data types (beads, deps, queues)
- Used at massive scale (Meta, CockroachDB, TiKV)

**Cons**:
- No built-in network protocol — need to build the server layer
- No sorted set primitive — need to implement priority queue on top
- Compaction can cause latency spikes (tunable)

**Verdict**: Best for the embedded storage layer inside bead-forge server.

### Kvrocks (server)

**What**: Apache project. Redis protocol, RocksDB storage engine.

**Pros**:
- Redis protocol — every language has a client
- Sorted sets (`ZPOPMIN`) give atomic priority queue claiming out of the box
- Disk-backed via RocksDB — dataset not limited by RAM
- Clustering, replication built-in
- Docker image available

**Cons**:
- External process to manage (though could be embedded/forked by bead-forge)
- Adds operational complexity vs single binary

**Verdict**: Best if bead-forge runs as a server that workers connect to. Could be embedded as a subprocess.

### redb (embedded Rust)

**What**: Pure-Rust embedded key-value store. Simpler than RocksDB.

**Pros**:
- Pure Rust, no C dependencies
- ACID transactions
- Concurrent readers, single writer (but with much lower overhead than SQLite)
- Simple API

**Cons**:
- Single writer (same fundamental limitation as SQLite, but faster)
- Smaller community, less battle-tested
- No sorted set primitive

**Verdict**: Good for standalone mode, doesn't solve the multi-writer problem.

### sled (embedded Rust)

**What**: Embedded database written in Rust. Lock-free concurrent B+ tree.

**Pros**:
- Lock-free concurrent reads AND writes
- Pure Rust
- Built-in serializable transactions

**Cons**:
- Stability concerns — API still evolving
- Known data loss issues in some crash scenarios
- Maintainer activity reduced

**Verdict**: Promising architecture but reliability concerns rule it out for a work queue.

### SQLite (current)

**What**: The incumbent. Single-file embedded database.

**Pros**:
- Universal, battle-tested, zero-config
- WAL mode handles concurrent reads well
- 30s busy timeout prevents failures

**Cons**:
- Single writer — the root cause of our contention
- No skip-locked equivalent
- Client-side claiming requires retry loops

**Verdict**: Keep as fallback for standalone mode. Not suitable for server mode.

## Recommended Architecture

```
┌─────────────────────────────────────────────┐
│              bead-forge binary               │
│                                              │
│  ┌──────────────┐  ┌─────────────────────┐  │
│  │ CLI Interface │  │ Server Mode         │  │
│  │ (bf command)  │  │ (bf server)         │  │
│  │               │  │                     │  │
│  │ Standalone:   │  │ Listens on:         │  │
│  │  SQLite/redb  │  │  Unix socket        │  │
│  │  (br compat)  │  │  TCP (Tailscale)    │  │
│  └───────┬───────┘  └──────────┬──────────┘  │
│          │                     │              │
│  ┌───────▼─────────────────────▼──────────┐  │
│  │         Storage Layer                   │  │
│  │                                         │  │
│  │  ┌─────────┐  ┌──────────┐  ┌────────┐ │  │
│  │  │ RocksDB │  │ SQLite   │  │ JSONL  │ │  │
│  │  │ (server)│  │ (compat) │  │ (audit)│ │  │
│  │  └─────────┘  └──────────┘  └────────┘ │  │
│  └─────────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

- **Server mode**: RocksDB for state + JSONL for audit log
- **Standalone mode**: SQLite for state + JSONL for audit log (identical to br)
- **JSONL always written**: Source of truth, backward compatible, git-friendly
