# bf-2ljuq: Test Epic With Description

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead (`type: epic`) exercising the epic-with-description path through `bf`.
The feature is already fully implemented — this bead confirms it works end-to-end against
the installed `bf 0.3.0` binary.

## Verification

Ran an ad-hoc end-to-end test in an isolated temp workspace (`/tmp/bf-2ljuq-test.*`):

```bash
bf init
bf create --type epic --title "Test epic with description" \
    --description "This is a test epic with a detailed description"   # → bf-187
```

| Check | Command | Result |
|-------|---------|--------|
| `--description` flag accepted by `bf create` | `bf create --type epic ... --description "..."` | ✅ created |
| Description persists to storage (JSON) | `bf show <id> --format json` → `description` | ✅ exact text preserved |
| Type stored as `epic` | `bf show <id> --format json` → `issue_type` | ✅ `epic` |
| Text display shows description | `bf show <id>` | ✅ `Description: This is a test epic with a detailed description` |
| Epic created without `--description` | `bf create --type epic --title "..."` | ✅ description defaults to empty (`""`) |
| Survives flush checkpoint (db → JSONL) | `bf sync --flush-only` then `bf show` | ✅ description intact |
| Multiline/markdown description | `--description "$(printf '# Overview\n\n...')"` | ✅ newlines preserved in JSON |

JSON output shape note for future checks: `bf show <id> --format json` returns a **list**
(`[ {...} ]`), not a bare object — parse `d[0]` when consuming it programmatically.

## Existing test coverage

The repo already has thorough library-level coverage of epic + description in
`tests/test_epic_with_description.rs` (13 test fns) — serialization roundtrips,
storage/retrieval, markdown/multiline descriptions, child wiring, priority
combinations, length limits, and description updates. No new test was needed;
this bead adds the missing **CLI end-to-end** confirmation on top of it.
