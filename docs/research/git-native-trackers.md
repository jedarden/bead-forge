# Git-Native Issue Trackers

Tools that embed issue tracking directly in git, without external databases or services.

## git-bug

- **URL**: https://github.com/git-bug/git-bug
- **Language**: Go
- **Stars**: 9.7k
- **License**: GPLv3

The most mature git-native tracker. Stores issues as git objects (not files) using a DAG-based operation model.

**Key features**:
- Issues stored as git objects — not files in the working tree
- Operations represented as a DAG with Lamport timestamps for ordering
- Concurrent edits merge automatically via operation-based conflict resolution
- Bridges to GitHub, GitLab, Jira, Launchpad for import/export
- CLI, TUI, and web interfaces
- Millisecond list/search performance
- Offline-first with push/pull sync

**Concurrency model**: Best-in-class for a git-native tool. The operation DAG allows concurrent modifications across machines with deterministic merge. Lamport timestamps provide causal ordering. No server needed — synchronization happens via `git push/pull`.

**Limitations**:
- No priority queue / work claiming
- No atomic "take next task" operation
- No awareness of workers or agents
- GPLv3 license (copyleft)

**Relevance to bead-forge**: The operation DAG and distributed merge model is interesting for multi-cluster scenarios where clusters may be offline. However, it doesn't solve the thundering herd — multiple agents reading the same git state will still race. The git-object storage model (vs files) is elegant but incompatible with beads JSONL.

## git-native-issue

- **URL**: https://github.com/remenoscodes/git-native-issue
- **Language**: Unknown
- **Storage**: Git commits under `refs/issues/`

Minimal approach — issues are stored as git commits in a separate ref namespace. No database, no JSON, no working-tree files.

**Key features**:
- `git issue` command interface
- Issues stored as commits under refs/issues/
- Uses git's native data model directly

**Limitations**:
- Very minimal — no dependencies, no priority, no agent support
- No query capabilities beyond git log

**Relevance to bead-forge**: Minimal. Too simple for agent work queues.

## git-issue (dspinellis/git-issue)

- **URL**: https://github.com/dspinellis/git-issue
- **Language**: Go
- **Storage**: Git-backed

Distributed bug tracker in Go. Similar concept to git-bug but simpler.

**Relevance to bead-forge**: Same limitations as git-bug — no work queue semantics.

## sciit

- **URL**: https://sciit.gitlab.io/sciit/
- **Storage**: Source code comments

Issues created as block comments in source code. Version-tracked automatically by git.

**Relevance to bead-forge**: Novel approach but not applicable — we need structured work items, not code comments.

## Summary

Git-native trackers solve distribution and offline-first well, but none provide:
- Atomic claiming / work queue semantics
- Priority-based dequeuing
- Worker awareness / heartbeats
- Thundering herd prevention

These are fundamentally database problems, not version control problems. Git is great for the audit log (JSONL) and sync, but the hot path (claiming) needs a coordination server.

## References

- [git-bug README](https://github.com/git-bug/git-bug)
- [git-bug HN Discussion](https://news.ycombinator.com/item?id=43971620)
- [git-native-issue](https://github.com/remenoscodes/git-native-issue)
