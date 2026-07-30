# Bead bf-4u82vk: Epic Default Priority

## Summary

Verification bead in the harness-test epic-priority series. Independently
re-confirms that a bead created with `--type epic` and **no explicit
`--priority`** receives the default priority **2 (P2 / Normal)** — the same
default clap assigns to every type. This bead is a fresh confirmation of the
finding already established by siblings `bf-1itsrb`, `bf-2xc4za`, and
`bf-269i1l`. No code changes were required; this note is the commit artifact.

## Source of the default

`src/cli/mod.rs` — the `create` command declares `--type` and `--priority` as
two independent flags, each with its own clap `default_value`:

```rust
// src/cli/mod.rs:61-67
/// Bead type
#[arg(long, default_value = "task")]
type_: String,

/// Priority (0=Critical, 4=Backlog)
#[arg(long, default_value = "2")]
priority: i32,
```

The priority default is **not type-dependent**: the two flags are parsed
separately, so every type (`epic`, `task`, `genesis`, …) inherits P2 unless
`--priority` is passed. The CLI help text (`src/cli/mod.rs:53-55`) documents
this:

> *"Type defaults to 'task' and priority to 2 (Normal); 0 is Critical, 4 is
> Backlog."*

## No epic-specific priority branching

`cmd_create` receives `priority` forwarded verbatim out of the clap struct as a
plain `i32` (`src/cli/mod.rs:1116-1131`) — there is no branching on the flag's
provenance or on `issue_type`. A repo-wide search for any epic-specific priority
logic returned nothing:

```
$ grep -niE "epic.*priority|priority.*epic" src/
(none found)
```

So an epic cannot pick up a priority other than `2` by default — there is no
code path that special-cases the type when assigning priority.

## Empirical verification

Created a throwaway epic with no `--priority`, inspected it, then deleted it so
`.beads/issues.jsonl` was not polluted by a test bead (shared live store — races
with other needle agents):

```
$ br create --title "TEMP epic default-priority probe (bf-4u82vk)" --type epic
created: bf-36rwo1

$ br show bf-36rwo1 --json
[{"id":"bf-36rwo1","title":"TEMP epic default-priority probe (bf-4u82vk)",
  "description":"","design":"","acceptance_criteria":"","notes":"",
  "status":"open","priority":2,"issue_type":"epic",
  "created_at":"2026-07-26T00:15:35.825880594Z",
  "updated_at":"2026-07-26T00:15:35.825880594Z","source_repo":".",
  "compaction_level":0}]

$ br delete bf-36rwo1
Deleted bead bf-36rwo1

$ grep -c bf-36rwo1 .beads/issues.jsonl
0
```

- Probe epic `bf-36rwo1` received `priority=2` (P2) with no `--priority` flag. ✓
- `issue_type` was correctly `epic`. ✓
- Probe was deleted; `grep -c bf-36rwo1 .beads/issues.jsonl` → `0` (clean). ✓

## Bead State

| Field | Value |
|-------|-------|
| ID | `bf-4u82vk` |
| Title | Epic Default Priority |
| Type | `epic` |
| Status | `in_progress` |
| Priority | `P2` (2) |
| Description | *(empty)* |
| Assignee | `claude-code-glm-4.7-h1-bforge` |

The bead itself is consistent with the finding: it was created as an `epic`
with no explicit priority and carries `priority=2` (P2) — confirmed via
`br show bf-4u82vk` (`Priority: P2`, `Type: epic`).

## Relationship to sibling beads

| Bead | Scenario | Result |
|------|----------|--------|
| `bf-244qnw` | `task`, no `--priority` (default) | P2 ✓ |
| `bf-1itsrb` | `epic`, no `--priority` (default) — bead itself | P2 ✓ |
| `bf-2xc4za` | `epic`, CLI-created, no `--priority` (default) | P2 ✓ |
| `bf-269i1l` | `epic`, explicit `--priority 2` | P2 ✓ |
| **`bf-4u82vk`** | **`epic`, CLI-created, no `--priority` (default) — re-verified** | **P2 ✓** |

Together these confirm priority assignment is both type-independent and
default-vs-explicit-agnostic: every epic inherits `2` (P2) from the clap
`default_value = "2"` unless an explicit `--priority` overrides it.

## Conclusion

Epic default priority = **2 (P2 / Normal)**, sourced from the clap
`default_value = "2"` on `create`'s `--priority` flag (type-independent), with
no epic-specific branching anywhere in `src/`. **Works as expected.**
