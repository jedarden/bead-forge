# bf-2r0a8: Epic With P2 And Description

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead (`type: epic`) exercising the epic-with-explicit-P2-and-description path
through `bf`. The feature is already fully implemented — this bead confirms it works end-to-end
against the installed `bf 0.3.0` binary, on top of existing library-level test coverage.

(Companion to `bf-3n4al` / `bf-hw10k`, which verified the description path. This bead adds the
explicit `--priority 2` axis on top of it.)

## Verification

Ran an ad-hoc end-to-end test in an isolated temp workspace (`/tmp/bf-2r0a8-test.*`, cleaned up
afterwards):

```bash
bf init --prefix bf
EPIC=$(bf create --type epic --priority 2 --title "Epic with P2 and description" \
    --description "This is a test epic for validating epic creation with P2 priority")   # → bf-3qa
```

| # | Check | Command | Result |
|---|-------|---------|--------|
| 1 | `--priority 2` + `--description` accepted by `bf create` | `bf create --type epic --priority 2 ... --description "..."` | ✅ created bf-3qa |
| 2 | Description persists to storage (JSON) | `bf show bf-3qa --format json` → `description` | ✅ exact text preserved |
| 3 | Priority stored as `2` (P2) | `bf show bf-3qa --format json` → `priority` | ✅ `2` |
| 4 | Type stored as `epic` | `bf show bf-3qa --format json` → `issue_type` | ✅ `epic` |
| 5 | Text display shows priority + description | `bf show bf-3qa` | ✅ `Priority: P2` + `Description: This is a test epic for validating epic creation with P2 priority` |
| 6 | `--priority` genuinely drives stored value (not just default coincidence) | `--priority 1` → `1`, `--priority 3` → `3` | ✅ flag respected (default is also 2, but value tracks the flag) |
| 7 | Survives flush checkpoint (db → JSONL) | `bf sync --flush-only` then `grep issues.jsonl` | ✅ description intact in checkpoint |

### Output-shape notes (consistent with bf-3n4al / bf-hw10k)

- `bf show <id> --format json` returns a **list** (`[ {...} ]`), not a bare object — parse `d[0]`.
- The description flows into the `description` field only; `design`, `acceptance_criteria`, and
  `notes` remain empty strings for an epic created solely with `--description`.
- `--priority` accepts the numeric value (`0`=Critical … `4`=Backlog); the text view renders it as
  the symbolic form (`P2`).

## Existing test coverage

The repo already has library-level coverage of epic + description
(`tests/test_epic_with_description.rs`). No new test or code change was needed; this bead adds
the **CLI end-to-end** confirmation on top of it, with the explicit-P2 axis.
