# bf-13jnm: Test Epic With Description

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead (`type: epic`) exercising the epic-with-description path through `bf`.
The feature is already fully implemented — this bead confirms it works end-to-end against
the installed `bf 0.3.0` binary.

## Verification

Ran an ad-hoc end-to-end test in an isolated temp workspace (`/tmp/bf-13jnm-test.*`):

```bash
bf init
bf create --type epic --title "Test epic with description" \
    --description "This is a test epic with a detailed description"   # → bf-283
```

| Check | Command | Result |
|-------|---------|--------|
| `--description` flag accepted by `bf create` | `bf create --type epic ... --description "..."` | ✅ created |
| Description persists to storage (JSON) | `bf show <id> --format json` → `description` | ✅ exact text preserved |
| Type stored as `epic` | `bf show <id> --format json` → `issue_type` | ✅ `epic` |
| Text display shows description | `bf show <id>` | ✅ `Description: This is a test epic with a detailed description` |
| Epic created without `--description` | `bf create --type epic --title "..."` | ✅ description defaults to `None`/empty |
| Survives flush checkpoint (db → JSONL) | `bf sync --flush-only` then `bf show` | ✅ description intact |
| Multiline/markdown description | `--description "# Overview\n\n..."` | ✅ newlines preserved in JSON |

## Existing test coverage

The repo already has thorough library-level coverage of epic + description in
`tests/test_epic_with_description.rs` — serialization roundtrips, storage/retrieval,
markdown/multiline/unicode/special-char descriptions, child wiring, priority
combinations, length limits, and description updates. No new test was needed;
this bead adds the missing **CLI end-to-end** confirmation on top of it.
