# JSON Output Implementation Audit

**Date:** 2026-07-03  
**Purpose:** Document current JSON output implementations across all commands  
**Scope:** Commands with `--format json` flag

## Summary

There are **significant inconsistencies** in JSON output formatting across commands:

1. **Array formatting inconsistency**: `list` and `search` output newline-separated JSON objects (NOT a proper JSON array), while `ready` and `velocity` output proper JSON arrays
2. **Object vs Array inconsistency**: `claim` outputs a single JSON object, `stats` outputs a single JSON object with nested data
3. **Pretty printing inconsistency**: `stats` and `velocity` use `to_string_pretty()` while `list` and `search` use `to_string()`

---

## Commands Using `get_formatter().format_issues()`

These commands use the centralized `JsonFormatter` from `src/format/json.rs`:

### `list` (cmd_list, line 995-1079)

**Location:** `src/cli/mod.rs:995-1079`  
**Implementation:**
```rust
let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
let formatter = get_formatter(output_format);
print!("{}", formatter.format_issues(&issues));
```

**JSON Format:** Newline-separated JSON objects (NOT a proper JSON array)
```json
{"id":"bf-1","title":"...","status":"open",...}
{"id":"bf-2","title":"...","status":"closed",...}
```

**Source:** `src/format/json.rs:17-29`
```rust
fn format_issues(&self, issues: &[Issue]) -> String {
    issues
        .iter()
        .map(|issue| {
            let mut stripped = issue.clone();
            stripped.dependencies = vec![];
            stripped.comments = vec![];
            serde_json::to_string(&stripped)
        })
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default()
        .join("\n")  // ← Note: newline-separated, NOT a JSON array
}
```

**Issue:** This produces **newline-delimited JSON (NDJSON)**, not a valid JSON array. Consumers expecting `[{...}, {...}]` will fail to parse this.

---

### `search` (cmd_search, line 2051-2092)

**Location:** `src/cli/mod.rs:2051-2092`  
**Implementation:** Identical to `list` - uses `get_formatter().format_issues()`

**JSON Format:** Newline-separated JSON objects (same issue as `list`)

**Issue:** Same NDJSON problem as `list`.

---

## Commands Using Custom JSON Output

### `ready` (cmd_ready, line 1229-1271)

**Location:** `src/cli/mod.rs:1229-1271`  
**Implementation:**
```rust
match format {
    "json" => {
        println!("{}", serde_json::to_string(&candidates)?);
    }
    // ...
}
```

**JSON Format:** Proper JSON array
```json
[{"id":"bf-1","title":"...","priority":2,...}]
```

**Consistency:** ✓ Outputs a proper JSON array (unlike list/search)

---

### `claim` (cmd_claim, line 1273-1526)

**Location:** `src/cli/mod.rs:1273-1526`  
**Implementation:** Multiple custom `serde_json::json!()` macros

**JSON Format:** Single JSON object (not an array)
```json
{"bead_id":"bf-1","reclaimed":false,"assignee":"worker-1"}
```

**Special cases:**
- Dry run mode includes `workspace` and `dry_run` fields
- Empty result outputs `"{}"` (empty object literal)

**Consistency:** ✓ Outputs a single object (appropriate for single-item result)

**Issue:** Empty result outputs `"{}"` string literal instead of `null` or missing output

---

### `stats` (cmd_stats, line 2094-2153)

**Location:** `src/cli/mod.rs:2094-2153`  
**Implementation:**
```rust
match format {
    "json" => {
        println!("{}", serde_json::to_string_pretty(&stats)?);
    }
    // ...
}
```

**JSON Format:** Single JSON object with nested data
```json
{
  "total": 42,
  "open": 10,
  "in_progress": 5,
  "closed": 27
}
```

**Source:** `src/storage/sqlite.rs` - `Stats` struct:
```rust
pub struct Stats {
    pub total: usize,
    pub open: usize,
    pub in_progress: usize,
    pub closed: usize,
}
```

**Consistency:** ✓ Outputs a single object with aggregated stats

**Note:** With `--by-type`, `--by-priority`, etc., only text output is shown - these flags don't affect JSON output

---

### `velocity` (cmd_velocity, line 2345-2414)

**Location:** `src/cli/mod.rs:2345-2414`  
**Implementation:**
```rust
match format {
    "json" => {
        println!("{}", serde_json::to_string_pretty(&stats)?);
    }
    // ...
}
```

**JSON Format:** JSON array of velocity stats
```json
[
  {
    "model": "claude-4.7",
    "harness": "cli",
    "issue_type": "task",
    "sample_count": 42,
    "p50_seconds": 300,
    "p90_seconds": 600,
    "avg_seconds": 350.5,
    "last_updated": "2026-07-03T12:00:00Z"
  }
]
```

**Source:** `src/velocity.rs:48-58` - `VelocityStats` struct

**Consistency:** ✓ Outputs a proper JSON array with multiple stats entries

---

## Other Commands with JSON Output

### `show` (cmd_show, line 1081-1142)

**Location:** `src/cli/mod.rs:1081-1142`  
**Implementation:** Custom serde_json output

**JSON Format:** Single-issue array (NEEDLE compatibility)
```json
[{"id":"bf-1","title":"...","status":"open",...}]
```

**Special:** Wraps single issue in array for NEEDLE's `parse_single_bead()` function

---

### `mitosis` (cmd_mitosis, line 1737-1778)

**Location:** `src/cli/mod.rs:1737-1778`  
**Implementation:** `serde_json::to_string_pretty(&results)`

**JSON Format:** Array of batch operation results

---

### `dep tree` (cmd_dep, line 1890-1962)

**Location:** `src/cli/mod.rs:1890-1962`  
**Implementation:** `serde_json::json!()` with nested structure

**JSON Format:** Object with root_id, direction, max_depth, and nodes array

---

### `log` (cmd_log, line 2457-2588)

**Location:** `src/cli/mod.rs:2457-2588`  
**Implementation:** Custom `format_events_json()` function

**JSON Format:** Pretty-printed array of events
```rust
pub fn format_events_json(events: &[Event]) -> Result<String> {
    Ok(serde_json::to_string_pretty(events)?)
}
```

**Source:** `src/log.rs:142-144`

**Consistency:** ✓ Proper JSON array, pretty-printed

---

## Critical Issues

### 1. NDJSON vs JSON Array (HIGH PRIORITY)

**Commands affected:** `list`, `search`

**Problem:** These commands output newline-delimited JSON (NDJSON):
```bash
bf list --format json
{"id":"bf-1","title":"..."}
{"id":"bf-2","title":"..."}
```

**Expected:** Most JSON consumers expect a proper JSON array:
```json
[{"id":"bf-1","title":"..."},{"id":"bf-2","title":"..."}]
```

**Impact:** Scripts using `jq` or standard JSON parsers will fail:
```bash
bf list --format json | jq '.[]'  # Fails - expects array input
```

**Fix required:** Either:
- Option A: Change `JsonFormatter::format_issues()` to output a proper JSON array
- Option B: Document this as NDJSON and update command help text

### 2. Empty Result Inconsistency

**Commands affected:** `claim`, `ready`

**Problem:** Empty outputs are inconsistent:
- `claim`: Prints `"{}"` (empty object literal as string)
- `ready`: Prints `"[]"` (empty array)

**Expected:** Either `null`, empty array `[]`, or no output

### 3. Pretty Printing Inconsistency

**Commands affected:** Various

**Problem:** Some commands use `to_string_pretty()`, others use `to_string()`:
- Pretty: `stats`, `velocity`, `log`, `dep tree`
- Compact: `list`, `search` (via JsonFormatter)

**Impact:** Inconsistent output formatting makes parsing harder

---

## Recommendations

### Immediate (High Priority)

1. **Fix NDJSON issue in `list` and `search`:**
   - Modify `JsonFormatter::format_issues()` to output a proper JSON array
   - OR create a separate formatter for NDJSON if that's intentional

2. **Standardize empty result handling:**
   - Empty array results → `[]`
   - Empty single-item results → `null` or no output
   - Never output `"{}"` as a string literal

### Medium Priority

3. **Add JSON schema documentation:**
   - Document expected JSON structure for each command
   - Add examples to command `--help` output

4. **Consider a unified JSON output strategy:**
   - All multi-item results → JSON array
   - All single-item results → JSON object
   - All empty results → `null` or empty array `[]`

### Low Priority

5. **Add `--json-pretty` flag:**
   - Allow users to choose compact vs pretty output
   - Default to compact for machine parsing
   - Pretty for human readability

---

## Command Reference Table

| Command | JSON Format | Uses Formatter | Array/Object | Pretty | Empty Result |
|---------|-------------|----------------|--------------|-------|--------------|
| `list` | NDJSON | Yes | Object (newline-separated) | No | (no output) |
| `search` | NDJSON | Yes | Object (newline-separated) | No | (no output) |
| `ready` | Array | No | Array | No | `[]` |
| `claim` | Object | No | Object | No | `"{}"` |
| `stats` | Object | No | Object | Yes | (no output) |
| `velocity` | Array | No | Array | Yes | (no output) |
| `show` | Array (single item) | No | Array | No | N/A |
| `log` | Array | No | Array | Yes | `[]` |
| `mitosis` | Array | No | Array | Yes | N/A |
| `dep tree` | Object (with array) | No | Object | Yes | N/A |

---

## Files Referenced

- `src/cli/mod.rs` - Main command implementations
- `src/format/mod.rs` - Formatter trait and factory
- `src/format/json.rs` - JsonFormatter implementation (NDJSON issue)
- `src/velocity.rs` - Velocity stats structure
- `src/log.rs` - Event log formatting
- `src/storage/sqlite.rs` - Stats structure
