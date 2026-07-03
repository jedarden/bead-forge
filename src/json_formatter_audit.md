# JSON Output Implementation Audit

**Bead:** bf-4h6d  
**Date:** 2026-07-03  
**Purpose:** Document current JSON output format for each command with `--format json` flag

## Summary

This audit tracks which commands use standardized `get_formatter().format_issues()` versus custom `println!` with `serde_json`, and identifies consistency issues.

**Total commands audited:** 13  
**Commands using `get_formatter().format_issues()`:** 2  
**Commands using custom JSON serialization:** 11  
**Commands needing fixes:** 2 (ready for inconsistency, show for NEEDLE compatibility documentation)

---

## Command-by-Command Analysis

### 1. `list` (cmd_list)
**Location:** `src/cli/mod.rs:995-1079`

**Current Method:** `get_formatter().format_issues()`
```rust
let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
let formatter = get_formatter(output_format);
print!("{}", formatter.format_issues(&issues));
```

**JSON Output Format:** JSONL (one JSON object per line, newline-separated)
- Empty: `(no output)`
- Single: `{"id":"bf-123",...}`
- Multiple: `{"id":"bf-123",...}\n{"id":"bf-456",...}`

**Notes:** Uses `JsonFormatter::format_issues()` which returns newline-separated JSON objects (NOT a JSON array)

**Needs Fix:** NO

---

### 2. `ready` (cmd_ready)
**Location:** `src/cli/mod.rs:1229-1271`

**Current Method:** Custom `println!` with `serde_json::to_string()`
```rust
println!("{}", serde_json::to_string(&candidates)?);
```

**JSON Output Format:** JSON array
- Empty: `[]`
- Single: `[{...}]`
- Multiple: `[{...},{...}]`

**Output Structure:**
```json
[{
  "id": "bf-123",
  "title": "...",
  "priority": 2,
  "downstream_impact": 5,
  "critical_float": 0
}]
```

**Needs Fix:** **YES** - inconsistent with `list`/`search` (returns JSON array instead of JSONL for bead-like objects)

---

### 3. `claim` (cmd_claim)
**Location:** `src/cli/mod.rs:1273-1526`

**Current Method:** Custom `println!` with `serde_json::json!`
```rust
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
```

**JSON Output Format:** Single JSON object (never array)
- Success: `{"bead_id":"bf-123",...}`
- Empty: `{}`

**Output Structure:**
```json
{
  "bead_id": "bf-123",
  "reclaimed": false,
  "assignee": "worker-name",
  "workspace": "/path/to/workspace"
}
```

**Needs Fix:** NO - claim returns a single bead result, object format is appropriate

---

### 4. `search` (cmd_search)
**Location:** `src/cli/mod.rs:2051-2092`

**Current Method:** `get_formatter().format_issues()`
```rust
let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
let formatter = get_formatter(output_format);
print!("{}", formatter.format_issues(&issues));
```

**JSON Output Format:** JSONL (newline-separated objects, same as `list`)

**Needs Fix:** NO - consistent with `list` command

---

### 5. `stats` (cmd_stats)
**Location:** `src/cli/mod.rs:2094-2153`

**Current Method:** Custom `println!` with `serde_json::to_string_pretty()`
```rust
println!("{}", serde_json::to_string_pretty(&stats)?);
```

**JSON Output Format:** Single JSON object with nested stats
```json
{
  "total": 42,
  "open": 10,
  "in_progress": 5,
  "closed": 27
}
```

**Needs Fix:** NO - stats is a different data structure, object format is appropriate

---

### 6. `velocity` (cmd_velocity)
**Location:** `src/cli/mod.rs:2345-2414`

**Current Method:** Custom `println!` with `serde_json::to_string_pretty()`
```rust
println!("{}", serde_json::to_string_pretty(&stats)?);
```

**JSON Output Format:** JSON array of velocity stat objects
```json
[
  {
    "model": "claude-sonnet-5",
    "harness": "claude-code",
    "issue_type": "task",
    "sample_count": 15,
    "p50_seconds": 120,
    "p90_seconds": 300,
    "avg_seconds": 180.5
  }
]
```

**Needs Fix:** NO - velocity is a different data structure

---

### 7. `log` (cmd_log)
**Location:** `src/cli/mod.rs:2457-2588`  
**Helper:** `src/log.rs:142-144`

**Current Method:** Custom `println!` with `crate::log::format_events_json()`
```rust
println!("{}", crate::log::format_events_json(&events)?);
// In log.rs:
pub fn format_events_json(events: &[Event]) -> Result<String> {
    Ok(serde_json::to_string_pretty(events)?)
}
```

**JSON Output Format:** JSON array
```json
[
  {
    "id": 1,
    "issue_id": "bf-123",
    "event_type": "StatusChanged",
    "actor": "claude",
    "old_value": "open",
    "new_value": "in_progress",
    "comment": null,
    "created_at": "2026-07-03T12:00:00Z"
  }
]
```

**Needs Fix:** NO - events are a different data structure

---

### 8. `dep tree` (cmd_dep -> DepCommands::Tree)
**Location:** `src/cli/mod.rs:1890-1962`

**Current Method:** Custom `println!` with `serde_json::json!`
```rust
let output = serde_json::json!({
    "root_id": id,
    "direction": direction,
    "max_depth": max_depth,
    "nodes": nodes
});
println!("{}", serde_json::to_string_pretty(&output)?);
```

**JSON Output Format:** Single JSON object
```json
{
  "root_id": "bf-123",
  "direction": "down",
  "max_depth": 10,
  "nodes": [
    {
      "id": "bf-456",
      "depth": 1,
      "dep_type": "blocks",
      "status": "open",
      "priority": 2,
      "title": "...",
      "path": "bf-123 > bf-456"
    }
  ]
}
```

**Needs Fix:** NO - dep tree is a different data structure

---

### 9. `show` (cmd_show)
**Location:** `src/cli/mod.rs:1081-1142`

**Current Method:** Custom `println!` with `serde_json::to_string()`
```rust
let mut out = issue;
out.dependencies = vec![];
out.comments = vec![];
// Wrap in array so NEEDLE's parse_single_bead (Vec<Bead> → first) works
println!("{}", serde_json::to_string(&vec![out])?);
```

**JSON Output Format:** **ALWAYS a JSON array with single element**
```json
[{
  "id": "bf-123",
  "title": "...",
  "status": "open",
  "priority": 2,
  ...
}]
```

**Needs Fix:** **MAYBE** - wraps single bead in array for NEEDLE compatibility. This is intentional but should be documented.

---

### 10. `labels` (cmd_labels)
**Location:** `src/cli/mod.rs:2009-2022`

**Current Method:** Custom `println!` with `serde_json::to_string_pretty()`
```rust
println!("{}", serde_json::to_string_pretty(&labels)?);
```

**JSON Output Format:** JSON array of strings
```json
["bug", "enhancement", "phase-1"]
```

**Needs Fix:** NO - labels are a different data structure

---

### 11. `schema` (cmd_schema)
**Location:** `src/cli/mod.rs:2155-2203`

**Current Method:** Custom `println!` with `serde_json::to_string_pretty()`
```rust
// For "all" target:
let output = serde_json::json!({"schema": crate::storage::schema::SCHEMA_SQL});
println!("{}", serde_json::to_string_pretty(&output)?);

// For specific bead:
println!("{}", serde_json::to_string_pretty(&issue)?);
```

**JSON Output Format:** Varies by target
- `target="all"`: `{"schema": "CREATE TABLE..."}`
- `target="<bead-id>"`: Full issue object with annotations

**Needs Fix:** NO - schema output is special-purpose

---

### 12. `mitosis` (cmd_mitosis)
**Location:** `src/cli/mod.rs:1737-1778`

**Current Method:** Custom `println!` with `serde_json::to_string_pretty()`
```rust
println!("{}", serde_json::to_string_pretty(&results)?);
```

**JSON Output Format:** JSON array of batch operation results
```json
[
  {
    "op": "create",
    "id": "bf-456",
    "status": "ok",
    "error": null
  },
  {
    "op": "dep_add_blocker",
    "id": null,
    "status": "ok",
    "error": null
  }
]
```

**Needs Fix:** NO - mitosis output is batch results, not issue data

---

### 13. `critical-path` (cmd_critical_path)
**Location:** `src/cli/mod.rs:2590-2631`

**Current Method:** Custom `println!` with `serde_json::to_string_pretty()`
```rust
println!("{}", serde_json::to_string_pretty(&result)?);
```

**JSON Output Format:** Single JSON object
```json
{
  "root_id": "bf-123",
  "min_remaining": 3,
  "longest_chain": ["bf-456", "bf-789", "bf-123"],
  "beads": [
    {
      "bead_id": "bf-456",
      "float": 0
    }
  ]
}
```

**Needs Fix:** NO - critical path is a different data structure

---

## Consolidated Table

| Command | Current Method | Output Type | Array Format | Needs Fix? |
|---------|---------------|-------------|--------------|------------|
| `list` | `get_formatter().format_issues()` | Issue list | JSONL (newline-separated) | NO |
| `ready` | Custom `serde_json::to_string()` | ReadyCandidate array | `[{...}]` | **YES** - inconsistent |
| `claim` | Custom `serde_json::json!()` | Single ClaimResult | Object `{...}` | NO |
| `search` | `get_formatter().format_issues()` | Issue list | JSONL (newline-separated) | NO |
| `stats` | Custom `serde_json::to_string_pretty()` | Stats object | Object `{...}` | NO |
| `velocity` | Custom `serde_json::to_string_pretty()` | VelocityStat array | `[{...}]` | NO |
| `log` | Custom `format_events_json()` | Event array | `[{...}]` | NO |
| `dep tree` | Custom `serde_json::json!()` | DepTree object | Object with nodes array | NO |
| `show` | Custom `serde_json::to_string()` | Issue array (single) | `[{...}]` | **MAYBE** - intentional for NEEDLE |
| `labels` | Custom `serde_json::to_string_pretty()` | String array | `["a","b"]` | NO |
| `schema` | Custom `serde_json::to_string_pretty()` | Schema/Issue object | Varies | NO |
| `mitosis` | Custom `serde_json::to_string_pretty()` | BatchResult array | `[{...}]` | NO |
| `critical-path` | Custom `serde_json::to_string_pretty()` | CriticalPath object | Object with beads array | NO |

---

## Expected JSON Array Format for Consistency

### Issue List Commands (list, search, ready)

**Current (list/search - JSONL):**
```json
{"id":"bf-123","title":"First","status":"open"}
{"id":"bf-456","title":"Second","status":"closed"}
```

**Current (ready - JSON array):**
```json
[{"id":"bf-123","title":"First","priority":2}]
```

**Expected (for array consistency):**
```json
[
  {"id":"bf-123","title":"First","status":"open"},
  {"id":"bf-456","title":"Second","status":"closed"}
]
```

**Rationale:** 
- JSONL is non-standard and harder to parse with standard JSON tools
- Most CLIs output proper JSON arrays for list results
- `ready` already uses JSON arrays, creating inconsistency

### Single Issue Commands (show)

**Current:**
```json
[{"id":"bf-123","title":"..."}]
```

**Expected (if not for NEEDLE compatibility):**
```json
{"id":"bf-123","title":"..."}
```

**Rationale:** Wrapping single items in arrays is unusual. The current behavior is intentional for NEEDLE compatibility (`parse_single_bead` expects `Vec<Bead>`) but should be documented with a comment.

---

## JsonFormatter Behavior

**Location:** `src/format/json.rs`

**Key behavior:**
- `format_issues()` returns newline-separated JSON (JSONL), NOT a JSON array
- Strips `dependencies` and `comments` from issues before serializing (br compatibility)
- Empty result returns `""` (no output)

**Code:**
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
        .join("\n")
}
```

---

## Key Findings

### Commands using `get_formatter().format_issues()`
1. `list` (line 1076)
2. `search` (line 2089)

### Commands using custom JSON serialization
1. `ready` - `serde_json::to_string()` (candidate array)
2. `claim` - `serde_json::json!()` (claim result object)
3. `stats` - `serde_json::to_string_pretty()` (stats object)
4. `velocity` - `serde_json::to_string_pretty()` (velocity array)
5. `log` - `format_events_json()` → `serde_json::to_string_pretty()` (event array)
6. `dep tree` - `serde_json::json!()` → `serde_json::to_string_pretty()` (dep tree object)
7. `show` - `serde_json::to_string()` (single-issue array)
8. `labels` - `serde_json::to_string_pretty()` (label string array)
9. `schema` - `serde_json::json!()` or `serde_json::to_string_pretty()`
10. `mitosis` - `serde_json::to_string_pretty()` (batch result array)
11. `critical-path` - `serde_json::to_string_pretty()` (critical path object)

### Inconsistencies Identified

1. **`list`/`search` use JSONL, `ready` uses JSON array**
   - All three return bead-like objects
   - Inconsistent format makes parsing harder for consumers

2. **`show` wraps single issue in array for NEEDLE**
   - Not documented in code
   - Surprising behavior for users expecting a single object

3. **Pretty-printing inconsistency**
   - Some commands use `to_string_pretty()` (formatted)
   - Others use compact `to_string()` (single line)
   - `list`/`search` use JSONL (not an array at all)

---

## Recommendations

1. **Standardize issue list output format:**
   - Option A: Change `list`/`search` to JSON arrays (breaking change, more standard)
   - Option B: Change `ready` to JSONL (consistent with current approach)
   - Option C: Add a `--array` flag to choose between JSONL and array formats

2. **Document `show` NEEDLE compatibility:**
   ```rust
   // Wrap in array so NEEDLE's parse_single_bead (Vec<Bead> → first) works
   println!("{}", serde_json::to_string(&vec![out])?);
   ```

3. **Consider adding `JsonArrayFormatter`:**
   - New formatter type that returns proper JSON arrays
   - Allows users to choose between JSONL and array formats via `--format json-array`

4. **Document expected behavior in CLI help:**
   - Add notes to `--help` output about JSON formats
   - Document in user-facing README
