# Bead bf-1itsrb: P2 Verification Epic 1784838645

## Summary

Verification epic in the harness-test series. Confirms that an epic-type bead
carries the default priority **2 (P2 / Normal)** — the same default clap assigns
to every type. The bead `bf-1itsrb` *itself* is the evidence: created as an
epic with no explicit `--priority`, it holds `priority=2`. No code changes were
required; this note is the commit artifact.

## Source of the default

`src/cli/mod.rs` — the `create` command declares the priority default via clap,
independent of type:

```rust
// src/cli/mod.rs:62-67
/// Bead type
#[arg(long, default_value = "task")]
type_: String,

/// Priority (0=Critical, 4=Backlog)
#[arg(long, default_value = "2")]
priority: i32,
```

The `--type` (default `"task"`) and `--priority` (default `"2"`) flags are parsed
separately; the priority default is **not type-dependent** — every type (epic,
task, genesis, …) inherits P2 unless `--priority` is passed. The CLI help text
(`src/cli/mod.rs:53-55`) documents this:

> *"Type defaults to 'task' and priority to 2 (Normal); 0 is Critical, 4 is
> Backlog."*

## Verification

Rather than creating/deleting a throwaway bead in the shared live store (which
races with other needle agents and risks polluting `.beads/issues.jsonl`), this
epic bead itself serves as the probe — it was created with no `--priority`:

```
$ br show bf-1itsrb --json
[{"id":"bf-1itsrb","title":"P2 Verification Epic 1784838645",
  "description":"","design":"","acceptance_criteria":"","notes":"",
  "status":"in_progress","priority":2,"issue_type":"epic",
  "assignee":"claude-code-glm-4.7-h1-bforge", ...}]
```

- `issue_type` = `epic`. ✓
- `priority` = `2` (P2 / Normal), with no `--priority` flag at creation. ✓
- Empty `description`/`acceptance_criteria` — consistent with a bare
  `br create --title "…" --type epic` (no priority override). ✓

This is consistent with the sibling finding (bf-2xc4za, "CLI Test Epic P2
Verification"): CLI-created epics default to P2.

## Bead State

| Field | Value |
|-------|-------|
| ID | `bf-1itsrb` |
| Title | P2 Verification Epic 1784838645 |
| Type | `epic` |
| Status | `in_progress` |
| Priority | `P2` (2) |
| Description | *(empty)* |
| Assignee | `claude-code-glm-4.7-h1-bforge` |
| Dependencies | none (`br dep list bf-1itsrb` → "No dependencies found") |

The title's `1784838645` token is a uniqueness seed only — it appears nowhere
else in the repo, `issues.jsonl`, or docs (grep returned just the bead's own
title line).

## Conclusion

Epic default priority = **2 (P2 / Normal)**, sourced from the clap
`default_value = "2"` on `create`'s `--priority` flag (type-independent).
Bead `bf-1itsrb` is itself an epic at priority 2 — **works as expected.**
