# bf-2kw3: Test Long Description

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead exercising the long-description storage path through `bf`. The feature
is already fully implemented — this bead confirms it works end-to-end against the installed
`bf 0.3.0` binary, on top of existing library-level test coverage.

## What "long description" means here

The description field must store and retrieve multi-paragraph text — including special
characters, quotes, unicode, emojis, and large payloads — without truncation or corruption,
across the full create → show → flush(JSONL) → re-import lifecycle.

## Storage has no length CHECK (unlike title)

The description column carries **no** length constraint (`src/storage/schema.rs:16`):

```sql
title       TEXT NOT NULL CHECK(length(title) <= 500),   -- capped at 500
description TEXT NOT NULL DEFAULT '',                     -- no CHECK → effectively unlimited
design      TEXT NOT NULL DEFAULT '',
```

Only `title` is capped (`<= 500`). `description` (and `design`/`acceptance_criteria`/`notes`)
are plain `TEXT`, so they're bounded only by SQLite's compile-time `SQLITE_MAX_LENGTH`
(default 1 GiB) — far beyond any realistic bead body. At the model level `description` is
`Option<String>` (`src/model.rs:442`), and Rust's `String` imposes no size cap.

## Verification

Ran ad-hoc end-to-end tests in an isolated temp workspace (`/tmp/bf-2kw3-test/`) using the
installed binary and `-w <workspace>`. Each row compares the retrieved description byte-for-byte
against the input (`==`).

| # | Check | Input | Result |
|---|-------|-------|--------|
| 1 | Multi-paragraph + special chars roundtrips through `create`/`show` | 600 chars: 3 paragraphs, `"`/`<>`/`{}`/`&`, bullets, `émojis 🚀`, `ünïcode`, en-dash, tabs | ✅ exact (600==600) |
| 2 | Survives flush checkpoint (db → JSONL) | same 600-char body | ✅ exact in `issues.jsonl` |
| 3 | Survives full re-import (nuke db → `sync --import`) | same 600-char body | ✅ exact after rebuild |
| 4 | Large payload (~50 KB) roundtrips | 49,928 chars across 120 paragraphs | ✅ exact (49928==49928) |
| 5 | Large payload survives flush | same ~50 KB body | ✅ exact in `issues.jsonl` |

`bf show <id> --format json` returns a **list** (`[ {...} ]`); description extracted from
`d[0]["description"]`.

## Existing library-level test coverage

The repo already has thorough coverage, all passing:

```
cargo test --test test_epic_with_description     # 13 passed (incl. 10k-char roundtrip,
                                                  #        multiline, unicode, special chars)
cargo test --test test_create long               #  1 passed (test_create_long_description)
cargo test --test test_create_command            # 14 passed
```

Notably `tests/test_epic_with_description.rs:395` builds a 10,000-character description and
asserts it survives both storage and serde roundtrip. No new test was needed; this bead adds
the **CLI end-to-end + JSONL-durability** confirmation on top.

## Finding: the only practical ceiling is the OS argv limit, not bf storage

Descriptions are argv-only: both `bf create` and `bf update` accept description solely via
`--description <DESCRIPTION>` — there is **no** `--description-file` and **no** stdin path
(`bf create --help` / `bf update --help` show only the `--description` flag).

So the effective ceiling for a description passed on the command line is the OS/shell
`ARG_MAX` (~128 KB on this Linux box), **not** bead-forge's storage. Demonstrated:

```bash
# 183,288-char description via --description "<huge>"
bash: .../bf: Argument list too long   # exit 126 — shell/OS refusal, never reaches bf
```

Storage itself happily held a ~50 KB description exactly (Check 4–5). The argv wall is a
generic CLI constraint affecting every `--<flag>` value, not a description-specific bug.

For genuinely huge bodies the current workaround is to author them through the JSONL
import path (`bf sync --import` from an `issues.jsonl` whose `description` field is
arbitrarily long — not subject to argv), or to keep descriptions concise and put long-form
detail in `design`/`notes`/linked docs. A future enhancement could add
`--description-file <path>` (or read `-` from stdin) to lift the argv ceiling for the
interactive `create`/`update` commands — flagged here, out of scope for this verification
bead (no spec requires it).
