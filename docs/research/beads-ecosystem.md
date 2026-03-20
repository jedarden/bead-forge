# Beads Ecosystem

The original beads project and its derivatives.

## beads (steveyegge/beads)

- **URL**: https://github.com/steveyegge/beads
- **Language**: Python
- **Storage**: SQLite + JSONL
- **Stars**: 18.7k

The original. A repo-local, git-backed issue tracker designed as "memory for coding agents." Each issue is an object, each update appends to JSONL. Dependency-aware DAG for task ordering. `bd ready` returns the next unblocked task.

**Key features**:
- JSONL append-only log as source of truth
- SQLite database as query cache
- DAG-based dependency tracking
- Priority system (P0-P4)
- `bd ready` — returns next actionable task
- Git-friendly — JSONL merges cleanly

**Concurrency model**: None. Single-user, single-agent assumed. Concurrent writes to SQLite cause corruption (FrankenSQLite issue #171). No claiming protocol — agents just pick from `bd ready` and hope nobody else did too.

**Relevance to bead-forge**: This is what we're replacing. The data model (JSONL + SQLite + DAG dependencies) is sound. The concurrency story is the gap.

## beads_rust (Dicklesworthstone/beads_rust)

- **URL**: https://github.com/Dicklesworthstone/beads_rust
- **Language**: Rust
- **Storage**: FrankenSQLite/rusqlite + JSONL

Fast Rust port of beads. Our current `br` CLI is a fork of this with a rusqlite shim replacing FrankenSQLite.

**Key additions over beads**:
- 10-100x faster than Python bd
- Sync protocol (flush/import between JSONL and SQLite)
- Doctor/repair for corruption recovery
- TOON format (token-optimized output for LLMs)
- Orphan handling modes

**Concurrency model**: Same as beads — SQLite single-writer. The rusqlite shim fixed corruption but not contention. 30s busy_timeout prevents failures but doesn't prevent thundering herd.

**Relevance to bead-forge**: This is the CLI we need to be compatible with. Same JSONL format, same command interface, same .beads/ directory structure.

## beads_viewer (Dicklesworthstone/beads_viewer)

- **URL**: https://github.com/Dicklesworthstone/beads_viewer
- **Language**: Rust
- **Features**: Graph-aware TUI with PageRank, critical path, kanban, DAG visualization, robot-mode JSON API

Visualization layer on top of beads. Not relevant to the work queue problem but shows the richness of the data model.

## beads forks and variants

- **cristoslc/llm-beads** — Fork with LLM-specific additions
- **w3dev33/beads-task-issue-tracker** — Desktop app wrapper
- **Marmalade118/beads-task-manager** — Another fork

None of these address concurrency.

## References

- [Steve Yegge: Introducing Beads](https://steve-yegge.medium.com/introducing-beads-a-coding-agent-memory-system-637d7d92514a)
- [Steve Yegge: The Beads Revolution](https://steve-yegge.medium.com/the-beads-revolution-how-i-built-the-todo-system-that-ai-agents-actually-want-to-use-228a5f9be2a9)
- [Better Stack: Beads Guide](https://betterstack.com/community/guides/ai/beads-issue-tracker-ai-agents/)
- [Ian Bull: Beads Review](https://ianbull.com/posts/beads/)
