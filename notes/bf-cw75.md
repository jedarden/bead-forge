# bf-cw75: stats command JSON output via formatter

Task: update `cmd_stats` to route JSON output through `get_formatter()`
instead of a custom `serde_json::to_string_pretty` on the stats struct.

## Finding — already implemented

The change this bead asks for is **already present in the tree**. The task
description's "line 2108 outputs stats struct directly" is stale — the function
has grown and migrated to `src/cli/mod.rs:2665`, and it no longer serializes the
stats struct directly. Current `cmd_stats` tail:

```rust
let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
let formatter = get_formatter(output_format);
print!("{}", formatter.format_stats(&output));
```

The "extend the formatter system" half of the task is likewise already done:

- `Formatter::format_stats(&self, stats: &StatsOutput) -> String` is a trait
  method (`src/format/mod.rs:118`).
- `JsonFormatter::format_stats` serializes the projection compactly
  (`src/format/json.rs:66`).
- `StatsOutput` (`src/format/mod.rs:78`) carries the four counts plus optional
  `by_type` / `by_priority` / `by_assignee` / `by_label` maps (each
  `skip_serializing_if = Option::is_none`).

`cmd_stats` builds a `StatsOutput`, folding the requested breakdowns into it as
nested `BTreeMap`s (stringified keys; `"None"` bucket for unassigned), then
hands it to `formatter.format_stats`. There is **no** remaining
`serde_json::to_string_pretty(&stats)` anywhere in the stats path
(`grep` confirms only the single `format_stats` call at `:2712`).

This is exactly the shape the JSON audit (`src/json_formatter_audit.md`) §5.1
recommended as its fix: folding breakdowns into the object is what keeps
`bf stats --format json --by-type` valid JSON, instead of the old object-
followed-by-plaintext stdout. (Note: the audit doc's master table/§5.1 still
describe the *pre-fix* `to_string_pretty` + appended-text behavior and are now
stale relative to the code; left untouched here to keep this change scoped to a
single notes path.)

## Verification (this bead)

No source changes were needed, so the work was build + behavior verification:

- `cargo build` — clean, 0 errors.
- `bf stats --format json` → valid JSON object, keys
  `['closed','in_progress','open','total']`.
- `bf stats --format json --by-type --by-priority --by-label` → valid JSON,
  breakdowns folded in: keys
  `['by_label','by_priority','by_type','closed','in_progress','open','total']`
  (validated by piping stdout through `python3 -m json.tool` / `json.load`).
- `bf stats --format text` unchanged.

## Files

- new: `notes/bf-cw75.md` (this file)
- no source changes — implementation already committed.
