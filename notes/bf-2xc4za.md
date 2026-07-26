# Bead bf-2xc4za: CLI Test Epic P2 Verification

## Summary

Verification bead in the harness test epic. Confirms that a bead created with
`--type epic` and **no explicit `--priority`** via the CLI receives the default
priority **2 (P2 / Normal)** — the same default as every other type. No code
changes were required; this note is the commit artifact.

## Source of the default

`src/cli/mod.rs` — the `create` command declares the priority default via clap,
independent of type:

```rust
// src/cli/mod.rs:61-63
/// Priority (0=Critical, 4=Backlog)
#[arg(long, default_value = "2")]
priority: i32,
```

The `--type` flag (default `"task"`) and `--priority` flag (default `"2"`) are
parsed separately; the priority default is **not type-dependent** — every type
(epic, task, genesis, …) inherits P2 unless `--priority` is passed. The CLI
help text (`src/cli/mod.rs:52-53`) documents this:

> *"Type defaults to 'task' and priority to 2 (Normal); 0 is Critical, 4 is
> Backlog."*

The same default (2) applies to the `batch` create op
(`src/cli/mod.rs:435` — `"priority": <int>, // optional, default 2`).

## Empirical verification

Created a throwaway epic with no priority, inspected it, then deleted it so
`.beads/issues.jsonl` was not polluted by a test bead:

```
$ br create --title "TEMP epic default-priority probe (bf-2xc4za)" --type epic
created: bf-5sfxel

$ br show bf-5sfxel --json
[{"id":"bf-5sfxel","title":"...","status":"open","priority":2,
  "issue_type":"epic", ...}]

$ br delete bf-5sfxel
Deleted bead bf-5sfxel

$ grep -c bf-5sfxel .beads/issues.jsonl
0
```

- Probe epic `bf-5sfxel` received `priority=2` (P2) with no `--priority` flag. ✓
- `issue_type` was correctly `epic`. ✓
- Probe was deleted; `grep -c bf-5sfxel .beads/issues.jsonl` → `0` (clean). ✓

## Bead State

| Field | Value |
|-------|-------|
| ID | `bf-2xc4za` |
| Title | CLI Test Epic P2 Verification |
| Type | `epic` |
| Status | `in_progress` |
| Priority | `P2` (2) |
| Description | *(empty)* |
| Assignee | `claude-code-glm-4.7-h1-bforge` |

The bead itself is consistent with the finding: created as an `epic` with no
explicit priority, it carries `priority=2` (P2) — confirmed via
`br show bf-2xc4za --json` (`"priority":2,"issue_type":"epic"`).

## Conclusion

CLI-created epic default priority = **2 (P2 / Normal)**, sourced from the clap
`default_value = "2"` on `create`'s `--priority` flag (type-independent).
**Works as expected.**
