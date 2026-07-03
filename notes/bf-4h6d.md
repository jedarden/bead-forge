# JSON Output Audit - bead-forge Commands

## Overview
This document audits JSON output implementations across all `bf` (bead-forge) commands to identify consistency issues and establish the expected format.

## Command-by-Command Analysis

### 1. **list** (`cmd_list` in src/cli/mod.rs:1011-1095)
- **Format parameter:** `--format` (text/json/toon), `--json` alias
- **Current method:** Uses `get_formatter().format_issues(&issues)`
- **JSON output:** JSONL format (one JSON object per line, newline-separated) ⚠️
  ```rust
  // src/format/json.rs:17-29
  fn format_issues(&self, issues: &[Issue]) -> String {
      issues.iter()
          .map(|issue| { /* strip deps/comments */ serde_json::to_string(&stripped) })
          .collect::<Vec<_>>()
          .join("\n")  // ← Problem: joins with "\n" instead of array wrapper
  }
  ```
- **Needs fix?** **YES** - should output proper JSON array like other commands
- **Current output format:** JSONL (newline-separated JSON objects, NOT an array)
- **Expected output format:** JSON array `[...]`

### 2. **ready** (`cmd_ready` in src/cli/mod.rs:1245-1287)
- **Format parameter:** `--format` (text/json/toon), `--json` alias
- **Current method:** Custom `serde_json::to_string(&candidates)`
- **JSON output:** JSON array of `ReadyCandidate` objects ✓
  ```rust
  // src/cli/mod.rs:1256
  println!("{}", serde_json::to_string(&candidates)?);
  ```
- **Needs fix?** No - already outputs proper JSON array (consistent with other commands)
- **Output format:** JSON array `[...]`

### 3. **search** (`cmd_search` in src/cli/mod.rs:2067-2108)
- **Format parameter:** `--format` (text/json/toon)
- **Current method:** Uses `get_formatter().format_issues(&issues)`
- **JSON output:** JSONL format via formatter (same as list) ⚠️
  ```rust
  // src/cli/mod.rs:2104-2105
  let formatter = get_formatter(OutputFormat::Json);
  print!("{}", formatter.format_issues(&issues));
  ```
- **Needs fix?** **YES** - should output proper JSON array like other commands
- **Current output format:** JSONL (newline-separated JSON objects)
- **Expected output format:** JSON array `[...]`

### 4. **claim** (`cmd_claim` in src/cli/mod.rs:1289-1542)
- **Format parameter:** `--format` (text/json/toon), `--json` alias
- **Current method:** Custom `serde_json::json!({...})` with `println!("{}", output)`
- **JSON output:** Single JSON object ✓ (appropriate for single-item result)
  ```rust
  // Example from successful claim (line 1412-1419)
  let output = serde_json::json!({
      "bead_id": bead_id,
      "reclaimed": reclaimed,
      "assignee": assignee,
      "workspace": workspace_path.map(|p| p.display().to_string())
  });
  println!("{}", output);
  ```
- **Empty case:** Prints `{}` (empty JSON object) - could be improved but acceptable
  ```rust
  // Line 1430
  if format == "json" { println!("{{}}"); }
  ```
- **Needs fix?** No - single object is appropriate for single-item results
- **Output format:** Single JSON object

### 5. **stats** (`cmd_stats` in src/cli/mod.rs:2094-2153)
- **Format parameter:** `--format` (text/json/toon)
- **Current method:** Custom `serde_json::to_string_pretty(&stats)`
- **JSON output:** Pretty-printed JSON object (not array, not JSONL)
  ```rust
  // src/cli/mod.rs:2108-2110
  match format {
      "json" => println!("{}", serde_json::to_string_pretty(&stats)?)
  }
  ```
- **Needs fix?** No - stats is metadata, not a bead list, so object format is appropriate
- **Output format:** Single JSON object (pretty-printed)

### 6. **velocity** (`cmd_velocity` in src/cli/mod.rs:2361-2430)
- **Format parameter:** `--format` (text/json/toon)
- **Current method:** Custom `serde_json::to_string_pretty(&stats)`
- **JSON output:** Pretty-printed JSON array ✓
  ```rust
  // src/cli/mod.rs:2377
  println!("{}", serde_json::to_string_pretty(&stats)?);
  ```
- **Needs fix?** No - already outputs proper JSON array
- **Output format:** JSON array (pretty-printed)

### 7. **log** (`cmd_log` in src/cli/mod.rs:2473-2604)
- **Format parameter:** `--format` (text/json/toon), `--json` alias
- **Current method:** Custom `crate::log::format_events_json(&events)`
- **JSON output:** Pretty-printed JSON array ✓
  ```rust
  // src/log.rs:142-144
  pub fn format_events_json(events: &[Event]) -> Result<String> {
      Ok(serde_json::to_string_pretty(events)?)
  }
  ```
- **Needs fix?** No - already outputs proper JSON array
- **Output format:** JSON array (pretty-printed)

### 8. **dep tree** (`cmd_dep` -> `DepCommands::Tree` in src/cli/mod.rs:1890-1962)
- **Format parameter:** `--format` (text/json), `--json` alias
- **Current method:** Custom `serde_json::json!({...})` with pretty print
- **JSON output:** Pretty-printed JSON object
  ```rust
  // Line 1914-1935
  if format == "json" {
      if direction == "both" {
          let output = serde_json::json!({
              "root_id": id,
              "direction": direction,
              "max_depth": max_depth,
              "downward": down_nodes,
              "upward": up_nodes
          });
          println!("{}", serde_json::to_string_pretty(&output)?);
      } else {
          let output = serde_json::json!({
              "root_id": id,
              "direction": direction,
              "max_depth": max_depth,
              "nodes": nodes
          });
          println!("{}", serde_json::to_string_pretty(&output)?);
      }
  }
  ```
- **Needs fix?** No - dependency tree is structured data, object format is appropriate
- **Output format:** Single JSON object (pretty-printed)

### 9. **show** (`cmd_show` in src/cli/mod.rs:1097-1158)
- **Format parameter:** `--format` (text/json/toon), `--json` alias
- **Current method:** Custom `serde_json::to_string(&vec![out])`
- **JSON output:** Array containing single issue (wrapped for NEEDLE compatibility) ✓
  ```rust
  // src/cli/mod.rs:1117-1121
  let mut out = issue;
  out.dependencies = vec![];
  out.comments = vec![];
  // Wrap in array so NEEDLE's parse_single_bead (Vec<Bead> → first) works
  println!("{}", serde_json::to_string(&vec![out])?);
  ```
- **Needs fix?** No - array format is consistent with other list commands
- **Output format:** JSON array with single element (NEEDLE compatibility)

### 10. **labels** (`cmd_labels` in src/cli/mod.rs:2025-2038)
- **Format parameter:** `--format` (text/json)
- **Current method:** Custom `serde_json::to_string_pretty(&labels)`
- **JSON output:** Pretty-printed JSON array of strings ✓
  ```rust
  // src/cli/mod.rs:2031
  println!("{}", serde_json::to_string_pretty(&labels)?);
  ```
- **Needs fix?** No - already outputs proper JSON array
- **Output format:** JSON array (pretty-printed)

### 11. **schema** (`cmd_schema` in src/cli/mod.rs:2155-2203)
- **Format parameter:** `--format` (text/json)
- **Current method:** Custom `serde_json`
- **JSON output:** Two modes:
  - `schema all`: Pretty JSON object with schema SQL
  - `schema <id>`: Pretty JSON object with full issue including annotations
- **Needs fix?** No - schema is metadata/object data, not a list
- **Output format:** Single JSON object (pretty-printed)

### 12. **mitosis** (`cmd_mitosis` in src/cli/mod.rs:1753-1794)
- **Format parameter:** `--format` (text/json/toon)
- **Current method:** Custom `serde_json::to_string_pretty(&results)`
- **JSON output:** Pretty-printed JSON array of batch results ✓
  ```rust
  // src/cli/mod.rs:1776
  println!("{}", serde_json::to_string_pretty(&results)?);
  ```
- **Needs fix?** No - already outputs proper JSON array
- **Output format:** JSON array (pretty-printed)

### 13. **critical-path** (`cmd_critical_path` in src/cli/mod.rs:2590-2631)
- **Format parameter:** `--format` (text/json/toon)
- **Current method:** Custom `serde_json::to_string_pretty(&result)`
- **JSON output:** Pretty-printed JSON object
  ```rust
  // src/cli/mod.rs:2599-2601
  match format {
      "json" => println!("{}", serde_json::to_string_pretty(&result)?)
  }
  ```
- **Needs fix?** No - critical path is structured analysis data, object format is appropriate
- **Output format:** Single JSON object (pretty-printed)

## Summary Table (CORRECTED)

| Command | Current Method | Output Format | Needs Fix? |
|---------|---------------|---------------|------------|
| list | `get_formatter().format_issues()` | **JSONL** ⚠️ | **YES** - should be array |
| ready | Custom `serde_json::to_string(&candidates)` | JSON array ✓ | No |
| search | `get_formatter().format_issues()` | **JSONL** ⚠️ | **YES** - should be array |
| claim | Custom `serde_json::json!({...})` | JSON object ✓ | No |
| stats | Custom `serde_json::to_string_pretty(&stats)` | JSON object ✓ | No |
| velocity | Custom `serde_json::to_string_pretty(&stats)` | JSON array ✓ | No |
| log | Custom `crate::log::format_events_json()` | JSON array ✓ | No |
| dep tree | Custom `serde_json::json!({...})` | JSON object ✓ | No |
| show | Custom `serde_json::to_string(&vec![out])` | JSON array ✓ | No |
| labels | Custom `serde_json::to_string_pretty(&labels)` | JSON array ✓ | No |
| schema | Custom `serde_json` | JSON object ✓ | No |
| mitosis | Custom `serde_json::to_string_pretty(&results)` | JSON array ✓ | No |
| critical-path | Custom `serde_json::to_string_pretty(&result)` | JSON object ✓ | No |

## Commands Using `get_formatter().format_issues()` (BROKEN - outputs JSONL)

- **list** (src/cli/mod.rs:1091-1092) - Needs fixing
- **search** (src/cli/mod.rs:2104-2105) - Needs fixing

These commands use the JsonFormatter which returns **JSONL format** (newline-delimited), NOT proper JSON arrays.

The fix is in `src/format/json.rs:17-29`:
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
        .join("\n")  // ← WRONG: joins with "\n" instead of array wrapper
}
```

Should be changed to:
```rust
fn format_issues(&self, issues: &[Issue]) -> String {
    let stripped: Vec<Issue> = issues
        .iter()
        .map(|issue| {
            let mut s = issue.clone();
            s.dependencies = vec![];
            s.comments = vec![];
            s
        })
        .collect();
    serde_json::to_string(&stripped).unwrap_or_else(|_| "[]".to_string())
}
```

## Commands Using Custom println! with serde_json (CORRECT - all use proper JSON)

- **ready** - `serde_json::to_string(&candidates)` (JSON array) ✓
- **claim** - `serde_json::json!({...})` (single object) ✓
- **stats** - `serde_json::to_string_pretty(&stats)` (object) ✓
- **velocity** - `serde_json::to_string_pretty(&stats)` (array) ✓
- **log** - `crate::log::format_events_json(&events)` → `serde_json::to_string_pretty(events)` (array) ✓
- **dep tree** - `serde_json::json!({...})` + `to_string_pretty` (object) ✓
- **show** - `serde_json::to_string(&vec![out])` (array) ✓
- **labels** - `serde_json::to_string_pretty(&labels)` (array) ✓
- **schema** - `serde_json` (object) ✓
- **mitosis** - `serde_json::to_string_pretty(&results)` (array) ✓
- **critical-path** - `serde_json::to_string_pretty(&result)` (object) ✓

## CRITICAL FINDING: JSONL vs JSON Array Inconsistency

**The initial audit conclusion was INCORRECT.** Upon closer examination of the codebase:

### The Actual Problem

Only **2 commands** use JSONL format (via `format_issues()`):
- `list` 
- `search`

All **other 11 commands** use proper JSON arrays or objects via direct `serde_json` calls.

This suggests **JSONL is the legacy inconsistency**, NOT the standard format.

### Evidence

1. **Most commands already use JSON arrays**: `ready`, `show`, `velocity`, `log`, `mitosis`, `labels` all output proper JSON arrays
2. **Only `list` and `search` use JSONL**: They use `get_formatter().format_issues()` which returns newline-delimited JSON
3. **The `format_issues()` implementation in `src/format/json.rs:17-29` is the anomaly**:
   ```rust
   fn format_issues(&self, issues: &[Issue]) -> String {
       issues.iter()
           .map(|issue| { /* strip deps/comments */ serde_json::to_string(&stripped) })
           .collect::<Vec<_>>()
           .join("\n")  // ← This is the problem
   }
   ```

### Corrected Expected Format

The **expected standard format for bead listings is proper JSON arrays**:

```json
[
  {"id":"bf-001","title":"...","status":"open",...},
  {"id":"bf-002","title":"...","status":"closed",...},
  {"id":"bf-003","title":"...","status":"in_progress",...}
]
```

**NOT JSONL (newline-delimited):**
```
{"id":"bf-001","title":"...","status":"open",...}
{"id":"bf-002","title":"...","status":"closed",...}
```

### Commands that NEED fixing (currently use JSONL):

- **list** ✗ (uses JSONL via `format_issues()`, should be JSON array)
- **search** ✗ (uses JSONL via `format_issues()`, should be JSON array)

### Commands that are CORRECT (already use proper JSON):

- **ready** ✓ (already outputs JSON array)
- **claim** ✓ (outputs single object - appropriate)
- **stats** ✓ (outputs single object - appropriate for metadata)
- **velocity** ✓ (already outputs JSON array)
- **log** ✓ (already outputs JSON array)
- **dep tree** ✓ (outputs single object - appropriate for structured data)
- **show** ✓ (outputs JSON array with 1 item - NEEDLE compatibility)
- **labels** ✓ (already outputs JSON array)
- **schema** ✓ (outputs single object - appropriate)
- **mitosis** ✓ (already outputs JSON array)
- **critical-path** ✓ (outputs single object - appropriate for analysis)

## Recommended Fix

**Modify `src/format/json.rs::format_issues()`** to return a proper JSON array instead of JSONL:

```rust
fn format_issues(&self, issues: &[Issue]) -> String {
    let stripped: Vec<Issue> = issues
        .iter()
        .map(|issue| {
            let mut s = issue.clone();
            s.dependencies = vec![];
            s.comments = vec![];
            s
        })
        .collect();
    serde_json::to_string(&stripped).unwrap_or_else(|_| "[]".to_string())
}
```

This single change would fix both `list` and `search` commands, making them consistent with all other commands.

## Updated Summary Table

| Command | Current Method | Output Format | Needs Fix? |
|---------|---------------|---------------|------------|
| list | `get_formatter().format_issues()` | **JSONL** ⚠️ | **YES** |
| ready | Custom `serde_json::to_string(&candidates)` | JSON array ✓ | No |
| search | `get_formatter().format_issues()` | **JSONL** ⚠️ | **YES** |
| claim | Custom `serde_json::json!({...})` | JSON object ✓ | No |
| stats | Custom `serde_json::to_string_pretty(&stats)` | JSON object ✓ | No |
| velocity | Custom `serde_json::to_string_pretty(&stats)` | JSON array ✓ | No |
| log | Custom `crate::log::format_events_json()` | JSON array ✓ | No |
| dep tree | Custom `serde_json::json!({...})` | JSON object ✓ | No |
| show | Custom `serde_json::to_string(&vec![out])` | JSON array ✓ | No |
| labels | Custom `serde_json::to_string_pretty(&labels)` | JSON array ✓ | No |
| schema | Custom `serde_json` | JSON object ✓ | No |
| mitosis | Custom `serde_json::to_string_pretty(&results)` | JSON array ✓ | No |
| critical-path | Custom `serde_json::to_string_pretty(&result)` | JSON object ✓ | No |

## Next Steps

This is a documentation-only bead. The implementation fix should be tracked in a separate bead:

1. **bf-4h6e**: Fix `src/format/json.rs::format_issues()` to output JSON array instead of JSONL

This single change will fix both `list` and `search` commands, making them consistent with all other commands.
