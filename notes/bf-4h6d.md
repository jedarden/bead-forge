# JSON Output Implementation Audit - bead-forge (bf)

## Overview
This document audits the JSON output implementation across all `bf` commands that support `--format json` or `--json` flags. This is a planning and documentation bead - no code changes.

**Key Finding:** There are TWO distinct JSON output formats in use, both intentional:
1. **JSONL (newline-delimited JSON)** - Used by `list` and `search` via `JsonFormatter` for `br` compatibility
2. **JSON arrays/objects** - Used by most other commands for structured data output

## Command-by-Command Analysis

### Commands using `get_formatter().format_issues()` (JSONL output via JsonFormatter)

#### 1. **list** (`cmd_list` in src/cli/mod.rs:995-1078)
- **Format flags:** `--format` (text/json/toon), `--json` alias
- **Implementation:**
  ```rust
  let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
  let formatter = get_formatter(output_format);
  print!("{}", formatter.format_issues(&issues));
  ```
- **Output method:** `JsonFormatter.format_issues()` (src/format/json.rs:17-29)
- **Output format:** JSONL (newline-delimited JSON objects)
  ```
  {"id":"bf-1","title":"Fix bug","status":"open","priority":2,...}
  {"id":"bf-2","title":"Add feature","status":"open","priority":1,...}
  ```
- **Dependencies/Comments:** Stripped before output (for `br` compatibility)
- **Empty output:** (no output)

#### 2. **search** (`cmd_search` in src/cli/mod.rs:2051-2091)
- **Format flags:** `--format` (text/json/toon)
- **Implementation:**
  ```rust
  let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
  let formatter = get_formatter(output_format);
  print!("{}", formatter.format_issues(&issues));
  ```
- **Output method:** Same as `list` - uses `JsonFormatter.format_issues()`
- **Output format:** JSONL (newline-delimited JSON objects)
- **Dependencies/Comments:** Stripped before output

---

### Commands using custom JSON arrays

#### 3. **ready** (`cmd_ready` in src/cli/mod.rs:1229-1270)
- **Format flags:** `--format` (text/json/toon), `--json` alias
- **Implementation:**
  ```rust
  match format {
      "json" => println!("{}", serde_json::to_string(&candidates)?),
      ...
  }
  ```
- **Output format:** JSON array of `ReadyCandidate` objects
  ```json
  [{"id":"bf-1","title":"Fix bug","priority":2,"downstream_impact":3,...}]
  ```
- **Empty output:** `[]`

#### 4. **show** (`cmd_show` in src/cli/mod.rs:1081-1141)
- **Format flags:** `--format` (text/json/toon), `--json` alias
- **Implementation:**
  ```rust
  match format {
      "json" => {
          let mut out = issue;
          out.dependencies = vec![];
          out.comments = vec![];
          println!("{}", serde_json::to_string(&vec![out])?);
      }
  }
  ```
- **Output format:** Single bead wrapped in array `[{...}]`
  ```json
  [{"id":"bf-1","title":"...","status":"open",...}]
  ```
- **Note:** Wraps in array for NEEDLE's `parse_single_bead()` compatibility
- **Dependencies/Comments:** Stripped before output

#### 5. **labels** (`cmd_labels` in src/cli/mod.rs:2009-2021)
- **Format flags:** `--format` (text/json)
- **Implementation:**
  ```rust
  if format == "json" {
      println!("{}", serde_json::to_string_pretty(&labels)?);
  }
  ```
- **Output format:** Pretty-printed JSON array of strings
  ```json
  ["bug", "phase-1", "urgent"]
  ```

#### 6. **velocity** (`cmd_velocity` in src/cli/mod.rs:2345-2413)
- **Format flags:** `--format` (text/json/toon)
- **Implementation:**
  ```rust
  match format {
      "json" => println!("{}", serde_json::to_string_pretty(&stats)?),
      ...
  }
  ```
- **Output format:** Pretty-printed JSON array of velocity stats
  ```json
  [{"model":"claude-opus-4","harness":"claude-code","issue_type":"task","sample_count":42,...}]
  ```

#### 7. **log** (`cmd_log` in src/cli/mod.rs:2457-2587)
- **Format flags:** `--format` (text/json/toon), `--json` alias
- **Implementation:**
  ```rust
  match format {
      "json" => println!("{}", crate::log::format_events_json(&events)?),
      ...
  }
  ```
- **Helper function** (src/log.rs):
  ```rust
  pub fn format_events_json(events: &[Event]) -> Result<String> {
      Ok(serde_json::to_string_pretty(events)?)
  }
  ```
- **Output format:** Pretty-printed JSON array of event objects
  ```json
  [{"id":1,"issue_id":"bf-1","event_type":"StatusChanged","actor":"worker-1",...}]
  ```

#### 8. **mitosis** (`cmd_mitosis` in src/cli/mod.rs:1737-1777)
- **Format flags:** `--format` (text/json/toon)
- **Implementation:**
  ```rust
  match format {
      "json" => println!("{}", serde_json::to_string_pretty(&results)?),
      ...
  }
  ```
- **Output format:** Pretty-printed JSON array of batch results
  ```json
  [{"op":"create","status":"ok","id":"bf-2"},{"op":"dep_add_blocker","status":"ok"},...]
  ```

---

### Commands using custom JSON objects (structured data)

#### 9. **claim** (`cmd_claim` in src/cli/mod.rs:1273-1525)
- **Format flags:** `--format` (text/json/toon), `--json` alias
- **Implementation:** Multiple claim modes, each with custom JSON output
  ```rust
  // Dry run mode
  let output = serde_json::json!({
      "bead_id": candidate.id,
      "title": candidate.title,
      "priority": candidate.priority,
      "downstream_impact": candidate.downstream_impact,
      "assignee": assignee,
      "workspace": path.display().to_string(),
      "dry_run": true
  });
  println!("{}", output);

  // Successful claim
  let output = serde_json::json!({
      "bead_id": bead_id,
      "reclaimed": reclaimed,
      "assignee": assignee,
      "workspace": workspace_path.map(|p| p.display().to_string())
  });
  println!("{}", output);
  ```
- **Output format:** Single JSON object
  ```json
  {"bead_id":"bf-1","reclaimed":false,"assignee":"worker-1","workspace":"/path"}
  ```
- **Empty output:** `{}` (empty object)

#### 10. **stats** (`cmd_stats` in src/cli/mod.rs:2094-2152)
- **Format flags:** `--format` (text/json/toon)
- **Implementation:**
  ```rust
  match format {
      "json" => println!("{}", serde_json::to_string_pretty(&stats)?),
      ...
  }
  ```
- **Output format:** Pretty-printed JSON object
  ```json
  {"total":42,"open":10,"in_progress":5,"closed":27}
  ```

#### 11. **dep tree** (`cmd_dep` -> `DepCommands::Tree` in src/cli/mod.rs:1849-1964)
- **Format flags:** `--format` (text/json), `--json` alias
- **Implementation:**
  ```rust
  if format == "json" {
      let output = serde_json::json!({
          "root_id": id,
          "direction": direction,
          "max_depth": max_depth,
          "nodes": nodes
      });
      println!("{}", serde_json::to_string_pretty(&output)?);
  }
  ```
- **Output format:** Pretty-printed JSON object
  ```json
  {"root_id":"bf-1","direction":"down","max_depth":10,"nodes":[{...},{...}]}
  ```

#### 12. **schema** (`cmd_schema` in src/cli/mod.rs:2155-2202)
- **Format flags:** `--format` (text/json), defaults to `json`
- **Two modes:**
  - `schema all`: Outputs SQL schema DDL
  - `schema <id>`: Outputs full bead with annotations
- **Implementation:**
  ```rust
  match format {
      "json" => {
          let output = serde_json::json!({"schema": crate::storage::schema::SCHEMA_SQL});
          println!("{}", serde_json::to_string_pretty(&output)?);
      }
      ...
  }
  // For bead-id target
  issue.annotations = storage.get_annotations(bead_id)?;
  match format {
      "json" => println!("{}", serde_json::to_string_pretty(&issue)?),
      ...
  }
  ```
- **Output format:** Pretty-printed JSON object

#### 13. **critical-path** (`cmd_critical_path` in src/cli/mod.rs:2590-2630)
- **Format flags:** `--format` (text/json/toon)
- **Implementation:**
  ```rust
  match format {
      "json" => println!("{}", serde_json::to_string_pretty(&result)?),
      ...
  }
  ```
- **Output format:** Pretty-printed JSON object
  ```json
  {"beads":[{"bead_id":"bf-1","float":0},...],"longest_chain":["bf-1","bf-2"],...}
  ```

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

## Summary Table

| Command | Format Flags | Method | Output Format |
|---------|-------------|--------|---------------|
| `list` | `--format json`, `--json` | `get_formatter().format_issues()` | JSONL (newline-delimited) |
| `ready` | `--format json`, `--json` | Custom `serde_json::to_string(&candidates)` | JSON array |
| `search` | `--format json` | `get_formatter().format_issues()` | JSONL (newline-delimited) |
| `show` | `--format json`, `--json` | Custom `serde_json::to_string(&vec![out])` | JSON array (single item wrapped) |
| `claim` | `--format json`, `--json` | Custom `serde_json::json!({...})` | JSON object |
| `stats` | `--format json` | Custom `serde_json::to_string_pretty(&stats)` | JSON object (pretty) |
| `velocity` | `--format json` | Custom `serde_json::to_string_pretty(&stats)` | JSON array (pretty) |
| `log` | `--format json`, `--json` | Custom `format_events_json(&events)` | JSON array (pretty) |
| `dep tree` | `--format json`, `--json` | Custom `serde_json::json!({...})` | JSON object (pretty) |
| `labels` | `--format json` | Custom `serde_json::to_string_pretty(&labels)` | JSON array (pretty) |
| `schema` | `--format json` | Custom `serde_json` | JSON object (pretty) |
| `mitosis` | `--format json` | Custom `serde_json::to_string_pretty(&results)` | JSON array (pretty) |
| `critical-path` | `--format json` | Custom `serde_json::to_string_pretty(&result)` | JSON object (pretty) |

## Commands WITHOUT JSON Support

| Command | Reason |
|---------|--------|
| `init` | Outputs initialization confirmation |
| `create` | Outputs only the created ID |
| `update` | Outputs update confirmation |
| `close` | Outputs close confirmation |
| `reopen` | Outputs reopen confirmation |
| `delete` | Outputs delete confirmation |
| `sync` | Outputs sync statistics |
| `doctor` | Outputs health check results |
| `commit-check` | Exit code only (pre-commit hook) |
| `count` | Outputs only the count |
| `batch` | Outputs batch results (could add JSON support) |
| `dep add/remove` | Outputs operation confirmation |
| `dep list` | Outputs dependency list (could add JSON support) |
| `label add/remove/list` | Outputs operation confirmation |
| `comments add/list` | Outputs operation confirmation |
| `config` | Outputs config values |
| `annotate` | Outputs operation confirmation |
| `rotate` | Outputs rotation results (could add JSON support) |
| `migrate` | Outputs migration results (could add JSON support) |

## Key Implementation Details

### JsonFormatter.format_issues() (src/format/json.rs:17-29)
```rust
fn format_issues(&self, issues: &[Issue]) -> String {
    issues
        .iter()
        .map(|issue| {
            let mut stripped = issue.clone();
            stripped.dependencies = vec![];  // Strip for br compatibility
            stripped.comments = vec![];      // Strip for br compatibility
            serde_json::to_string(&stripped)
        })
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default()
        .join("\n")  // ← JSONL format (newline-delimited)
}
```

**Notes:**
- Outputs JSONL (one JSON object per line)
- Strips dependencies and comments for `br` compatibility
- Used by `list` and `search` commands

### Dependency/Comment Stripping

The following commands strip dependencies and comments before JSON output:
- `list` (via JsonFormatter)
- `search` (via JsonFormatter)  
- `show` (explicitly strips before wrapping in array)

**Reason:** `br` compatibility - `br` expects dependencies/comments to be in specific format or omitted.

## Open Questions for Decision

1. **JSONL vs JSON Array format for `list` and `search`:**
   - Current: JSONL (newline-delimited JSON objects)
   - Alternative: JSON array `[{...}, {...}]`
   - **Decision needed:** What format does `br list --format json` actually output? Verify against `br` behavior.

2. **Consistency across list-like commands:**
   - `list` and `search` use JSONL
   - `ready`, `velocity`, `log`, `mitosis` use JSON arrays
   - **Decision needed:** Should all list outputs use the same format, or is JSONL intentional for `br` compatibility?

3. **Empty result handling:**
   - `ready` outputs `[]` for empty
   - `claim` outputs `{}` for empty
   - `list`/`search` output nothing for empty
   - **Decision needed:** Standardize empty output format?

## Conclusion

This audit documents the current state of JSON output across all `bf` commands. The main finding is that there are two distinct JSON output formats in use:

1. **JSONL format** (newline-delimited) - Used by `list` and `search` via `JsonFormatter`
2. **JSON arrays/objects** - Used by most other commands via direct `serde_json` calls

Whether this inconsistency is intentional (for `br` compatibility) or should be fixed is a decision that requires verification against actual `br` behavior. This documentation provides the foundation for making that decision.
