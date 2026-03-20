# Lightweight Beads Alternatives

Tools built specifically as simpler alternatives to beads for AI agent workflows.

## ticket (wedow/ticket)

- **URL**: https://github.com/wedow/ticket
- **Language**: Bash (single file)
- **Storage**: Flat files

A self-described "beads replacement" that strips away complexity. Single bash script using coreutils.

**Key features**:
- Graph-based task dependencies (core beads concept retained)
- `tk ready`, `tk blocked`, `tk dep tree` commands
- Flat files instead of SQLite — "you don't need to index everything with SQLite when you have awk"
- Markdown-based tickets stored alongside code
- Directory-based organization

**Design philosophy**: "Just a small plumbing utility that gets out of your way so you can get to work." Deliberately drops everything from beads except the dependency graph.

**Concurrency model**: None. Flat file operations aren't atomic. Multiple agents would corrupt state.

**Relevance to bead-forge**: Validates the instinct that beads is over-engineered for some use cases. But goes too far in the other direction — no concurrency, no priority queue, no structured data. The dependency graph concept is universal though.

## trekker (obsfx/trekker)

- **URL**: https://github.com/obsfx/trekker
- **Language**: Unknown
- **Integration**: Claude Code plugin

Built by someone who used beads with Claude Code but found "the project direction no longer matched what I needed."

**Key features**:
- Claude Code plugin integration
- Lightweight dashboard for monitoring agent activities
- Task progress tracking over time
- Intentionally minimal — "only the parts I actually use"
- No extra abstraction or workflow layers

**Design philosophy**: Providing an agent with a task tracker "helps a lot with focus and continuity." Strip everything else.

**Concurrency model**: Unknown, likely single-agent.

**Relevance to bead-forge**: Shows demand for beads-like functionality with less complexity. The Claude Code plugin integration model is worth noting — bead-forge could expose an MCP interface.

## dstask (naggie/dstask)

- **URL**: https://github.com/naggie/dstask
- **Language**: Go
- **Storage**: Git-synced markdown files
- **Stars**: 1k+

Personal task manager using git for sync. Each task is a versioned markdown file. Similar to Taskwarrior but with git instead of a sync protocol.

**Key features**:
- Single binary, Go
- Git-powered sync/undo/resolve (like password-store)
- Markdown note per task with checklists
- Priority sorting (most important tasks at top)
- Context system — auto-applies filters/tags to queries
- Zsh/bash completion including tags and projects

**Concurrency model**: Explicitly single-user. "Use another system for projects that involve multiple people."

**Relevance to bead-forge**: Good UX ideas (context system, completion) but not designed for multi-agent. The git-sync model works for personal use but not for 11 concurrent workers.

## Taskwarrior

- **URL**: https://taskwarrior.org
- **Language**: C++
- **Storage**: Custom format, previously Taskserver sync

The veteran CLI task manager. Taskwarrior 3.0 dropped Taskserver support, moved to cloud storage backends.

**Key features**:
- Mature CLI with powerful filtering
- Priority, due dates, recurrence, dependencies
- UDAs (user-defined attributes)
- Extensive documentation

**Concurrency model**: Taskserver (2.x) provided multi-user sync but is deprecated. Taskwarrior 3.0 uses cloud storage. "Task Server II" is in development for 2025+ with bi-directional sync as a goal, but multi-user task assignment is not yet supported.

**Relevance to bead-forge**: Good CLI ergonomics to study. The filter/query language is powerful. But it's a personal tool that doesn't handle agent fleets.

## Summary

| Tool | Language | Storage | Concurrency | Agent-aware | Dependencies |
|------|----------|---------|-------------|-------------|-------------|
| ticket | Bash | Flat files | None | No | Yes (graph) |
| trekker | Unknown | Unknown | Single-agent | Yes (Claude Code) | Unknown |
| dstask | Go | Git/markdown | Single-user | No | No |
| Taskwarrior | C++ | Custom/cloud | Deprecated sync | No | Yes |

None of these solve the multi-agent concurrent work queue problem. They validate that the core beads concept (structured tasks with dependencies for AI agents) has demand, but all assume single-user or single-agent operation.

## References

- [ticket HN Discussion](https://news.ycombinator.com/item?id=46487580)
- [trekker HN Discussion](https://news.ycombinator.com/item?id=46709872)
- [dstask GitHub](https://github.com/naggie/dstask)
- [Taskwarrior](https://taskwarrior.org/docs/)
