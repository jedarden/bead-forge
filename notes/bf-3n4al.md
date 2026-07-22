# bf-3n4al: Test Epic With Description

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead (`type: epic`) exercising the epic-with-description path through `bf`.
The feature is already fully implemented — this bead confirms it works end-to-end against the
installed `bf 0.3.0` binary, on top of existing library-level test coverage.

(This is a retry/variant of `bf-hw10k`, which verified the same path. This bead re-confirms it
independently on the current binary.)

## Verification

Ran an ad-hoc end-to-end test in an isolated temp workspace (`/tmp/bf-3n4al-test.*`):

```bash
bf init --prefix bf
EPIC=$(bf create --type epic --title "Test epic with description" \
    --description "This is a test epic with a detailed description")   # → bf-564
```

| # | Check | Command | Result |
|---|-------|---------|--------|
| 1 | `--description` flag accepted by `bf create` | `bf create --type epic ... --description "..."` | ✅ created bf-564 |
| 2 | Description persists to storage (JSON) | `bf show bf-564 --format json` → `description` | ✅ exact text preserved |
| 3 | Type stored as `epic` | `bf show bf-564 --format json` → `issue_type` | ✅ `epic` |
| 4 | Priority default | `bf show bf-564 --format json` → `priority` | ✅ `2` (P2 / MEDIUM) |
| 5 | Text display shows description | `bf show bf-564` | ✅ `Description: This is a test epic with a detailed description` |
| 6 | Epic created without `--description` | `bf create --type epic --title "..."` | ✅ description defaults to empty (`''`) |
| 7 | Multiline/markdown description | `--description "$(printf '# Overview\n\n...')"` | ✅ newlines preserved (5 lines) |
| 8 | Survives flush checkpoint (db → JSONL) | `bf sync --flush-only` then `grep issues.jsonl` | ✅ description intact in checkpoint |

### Output-shape notes (consistent with bf-hw10k)

- `bf show <id> --format json` returns a **list** (`[ {...} ]`), not a bare object — parse `d[0]`.
- The description flows into the `description` field only; `design`, `acceptance_criteria`, and
  `notes` remain empty strings for an epic created solely with `--description`.

## Existing test coverage

The repo already has library-level coverage of epic + description
(`tests/test_epic_with_description.rs`). No new test or code change was needed; this bead adds
the **CLI end-to-end** confirmation on top of it.
