# bf-doiq — Formatter for `list` and `ready` commands (verification)

**Task:** Convert `list` and `ready` to use `get_formatter().format_issues()` instead of
custom JSON output loops.

**Outcome:** No source change required — both commands already route their JSON output
through `get_formatter().format_issues()` in committed code. This note records the
verification against every acceptance criterion.

## Acceptance criteria — all satisfied by committed code

The CLI lives in a single module (`src/cli/mod.rs`); there are no separate `list.rs` /
`ready.rs` files, so the criteria are evaluated against `cmd_list` and `cmd_ready` there.

1. **`list` uses `get_formatter().format_issues()` for JSON output** — ✅
   `cmd_list` ends with a format-agnostic render that covers text/json/toon:
   `src/cli/mod.rs:1451-1453`
   ```rust
   let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
   let formatter = get_formatter(output_format);
   print!("{}", formatter.format_issues(&issues));
   ```

2. **`ready` uses `get_formatter().format_issues()` for JSON output** — ✅
   `cmd_ready`'s `"json"` branch resolves each `ScoredBead` candidate to its full `Issue`
   via `storage.get_issue` and renders through the shared formatter:
   `src/cli/mod.rs:1650-1659` (routing landed in `1c78bc9 fix(bf-64zt)`). The text/toon
   branches surface scoring fields (`downstream_impact`, `critical_float`) that are not on
   `Issue`, so they intentionally keep their own `println!` — out of scope for JSON.

3. **No custom `println!` loops for individual JSON objects** — ✅
   Neither `cmd_list` nor `cmd_ready` serializes per-object JSON in a hand-rolled loop. The
   only JSON-specific branch in `cmd_ready` is the empty-result `println!("[]")`, which is a
   single marker (not a loop, not per-object) preserved as the empty-array contract.

4. **Output format matches br exactly (same field names, array structure)** — ✅
   `JsonFormatter.format_issues` (`src/format/json.rs`) emits one full `Issue` per line
   (JSONL) — the documented `list`/`search`/`ready` contract. `assignee` is always present
   (`null` when unset) and `labels` is always an array (`[]` when empty); that normalization
   landed in `e8ed49d fix(bf-1wj): always emit assignee/labels in ready/list/search
   --format json`. Parity is guarded by:
   - `tests/test_jsonl.rs::test_e2e_br_vs_bf_list_output_parity` — direct `bf list` vs `br`
     output parity (and 4 other bf↔br snapshot/simple/round-trip parity tests).
   - `tests/ready_json_fields.rs` — `ready --format json` always emits `assignee` + `labels`.
   - `src/format/json.rs` unit tests (`assignee_null_when_unset`,
     `labels_empty_array_when_none`, `format_issues_guarantees_fields_per_line`, …).

   Live spot-check (`bf list --format json` / `bf ready --format json`) confirmed every line
   is a valid JSON object with the full br field set: `id, title, status, priority,
   issue_type, description, assignee, labels, created_at, updated_at, acceptance_criteria,
   design, notes, source_repo, compaction_level`.

5. **`cargo build clean` — ✅** `cargo build 2>&1 | grep '^error'` → no errors (exit 0).

## Test runs (this verification)

```
$ cargo build 2>&1 | grep '^error'        # exit 0, no errors
$ cargo test --lib format::               # 4 passed (json formatter unit tests)
$ cargo test --test test_jsonl            # 20 passed (incl. list vs br parity)
$ cargo test --test ready_json_fields     # 2 passed
```

## Why no code change

Both `cmd_list` and `cmd_ready` already render JSON exclusively through
`get_formatter().format_issues()` in committed code, so there is nothing left to convert.
This bead failed its two prior attempts because the work was already complete — earlier
runs found no code to change and left no commit. This note supplies the missing commit.
