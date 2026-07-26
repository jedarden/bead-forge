# Bead bf-244qnw: Test task default priority

## Summary

Verification bead in the harness test epic. Confirms that a bead created with
`--type task` and **no explicit `--priority`** receives the default priority
**2 (P2 / Normal)**. No code changes were required; this note is the commit
artifact.

## Source of the default

`src/cli/mod.rs` — the `create` command declares the priority default via clap:

```rust
// src/cli/mod.rs:65-67
/// Priority (0=Critical, 4=Backlog)
#[arg(long, default_value = "2")]
priority: i32,
```

The same default (2) applies to the `batch` create op:
`src/cli/mod.rs:435` — `"priority": <int>, // optional, default 2 (0=Critical, 4=Backlog)`.

The default is **not type-dependent** — every type (task, epic, genesis, …)
inherits priority 2 unless `--priority` is passed. The CLI help text
(`src/cli/mod.rs:53-54`) documents this: *"Type defaults to 'task' and priority
to 2 (Normal)"*.

## Empirical verification

Created a throwaway task with no priority, inspected it, then deleted it so
`.beads/issues.jsonl` was not polluted by a test bead:

```
$ br create --title "TEMP default-priority probe (bf-244qnw)" --type task
created: bf-4jvpvo

$ br show bf-4jvpvo --json   # via json: type + priority fields
type=task priority=2

$ br delete bf-4jvpvo
Deleted bead bf-4jvpvo
```

- Probe bead `bf-4jvpvo` received `priority=2` (P2) with no `--priority` flag. ✓
- Probe was deleted; `grep -c bf-4jvpvo .beads/issues.jsonl` → `0` (clean). ✓

## Bead State

| Field | Value |
|-------|-------|
| ID | `bf-244qnw` |
| Title | Test task default priority |
| Type | `task` |
| Status | `in_progress` |
| Priority | `P2` (2) |
| Description | *(empty)* |

The bead itself is consistent with the finding: created as a `task` with no
explicit priority, it carries `priority=2` (P2).

## Conclusion

Task default priority = **2 (P2 / Normal)**, sourced from the clap
`default_value = "2"` on `create`'s `--priority` flag. **Works as expected.**
