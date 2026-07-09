# JSON Output Format Investigation (bf-3uny)

**Status:** RESEARCH COMPLETE - Discrepancy Found
**Date:** 2026-07-03

## Task Description

Investigate JSON output format across all CLI commands:
- Document all commands that output JSON
- Identify which commands use `format_issues()` (should be array format)
- Identify which commands have custom JSON handling (may output JSONL)
- Create a list of commands that need fixing

## Summary

Investigation revealed a **critical discrepancy**: The `JsonFormatter::format_issues()` method in `src/format/json.rs` currently outputs **JSONL** (newline-separated JSON objects) instead of proper JSON **arrays**, despite the task specification indicating it should output array format.

**Root Cause:** Line 28 in `src/format/json.rs` uses `.join("\n")` instead of wrapping in `[]`.

---

## Commands Using `format_issues()` (SHOULD output JSON arrays, currently outputs JSONL)

**Location:** `src/format/json.rs:17-29`

### Current Implementation (INCORRECT):
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
        .join("\n")  // ← PROBLEM: Outputs JSONL, not array
}
```

### Current Output (JSONL - INCORRECT):
```json
{"id":"bf-1","title":"First","status":"open"}
{"id":"bf-2","title":"Second","status":"open"}
```

### Should Output (JSON Array - CORRECT):
```json
[{"id":"bf-1","title":"First","status":"open"},{"id":"bf-2","title":"Second","status":"open"}]
```

### Commands Affected:

1. **`bf list --format json`** (line 1076 in `src/cli/mod.rs`)
   ```rust
   print!("{}", formatter.format_issues(&issues));
   ```

2. **`bf search --format json`** (line 2090 in `src/cli/mod.rs`)
   ```rust
   print!("{}", formatter.format_issues(&issues));
   ```

---

## Summary

Investigated JSON output format across all CLI commands in bead-forge. **All commands currently match `br` behavior exactly**. The JSONL format (one JSON object per line) used by `list` and `ready` is **intentional and correct** for br compatibility.

## Commands with Custom JSON Output (NOT using format_issues())

### Commands with Custom JSONL Output (INCORRECT):

3. **`bf ready --format json`** (line 1238-1242 in `src/cli/mod.rs`)
   ```rust
   "json" => {
       for candidate in candidates {
           println!("{}", serde_json::to_string(&candidate)?);
       }
   }
   ```
   - **Current Output:** JSONL (one `ReadyCandidate` object per line)
   - **Should Output:** JSON array of `ReadyCandidate` objects
   - **Fix Needed:** Yes - collect candidates and output as array

---

### Commands with Proper JSON Output (CORRECT):

These commands already output proper JSON arrays or objects:

4. **`bf show <id> --format json`** (line 1095-1106) ✅
   ```rust
   "json" => {
       let mut out = issue;
       out.dependencies = vec![];
       out.comments = vec![];
       println!("{}", serde_json::to_string(&vec![out])?);
   }
   ```
   - **Output:** Single issue wrapped in array: `[{...}]`
   - **Status:** CORRECT (array format)

5. **`bf claim --format json`** (lines 1352-1363, 1396-1403, etc.) ✅
   ```rust
   let output = serde_json::json!({
       "bead_id": bead_id,
       "reclaimed": reclaimed,
       "assignee": assignee
   });
   println!("{}", output);
   ```
   - **Output:** Single claim result object
   - **Status:** CORRECT (outputs claim data, not issue list)

6. **`bf mitosis --format json`** (line 1759-1762) ✅
   ```rust
   println!("{}", serde_json::to_string_pretty(&results)?);
   ```
   - **Output:** JSON array of batch results
   - **Status:** CORRECT (proper JSON array)

7. **`bf dep tree --format json`** (lines 1914-1935) ✅
   ```rust
   let output = serde_json::json!({
       "root_id": id,
       "direction": direction,
       "max_depth": max_depth,
       "nodes": nodes
   });
   println!("{}", serde_json::to_string_pretty(&output)?);
   ```
   - **Output:** Single JSON object with tree structure
   - **Status:** CORRECT (outputs tree structure, not issue list)

8. **`bf labels <id> --format json`** (line 2015-2016) ✅
   ```rust
   println!("{}", serde_json::to_string_pretty(&labels)?);
   ```
   - **Output:** JSON array of label strings
   - **Status:** CORRECT (proper JSON array)

9. **`bf stats --format json`** (line 2109-2111) ✅
   ```rust
   println!("{}", serde_json::to_string_pretty(&stats)?);
   ```
   - **Output:** Single stats object
   - **Status:** CORRECT (outputs stats, not issue list)

10. **`bf schema --format json`** (lines 2165/2195/2198) ✅
    ```rust
    let output = serde_json::json!({"schema": crate::storage::schema::SCHEMA_SQL});
    println!("{}", serde_json::to_string_pretty(&output)?);
    ```
    - **Output:** Schema object or single issue object
    - **Status:** CORRECT (outputs schema/data, not issue list)

11. **`bf velocity --format json`** (line 2360-2363) ✅
    ```rust
    println!("{}", serde_json::to_string_pretty(&stats)?);
    ```
    - **Output:** JSON array of velocity stats
    - **Status:** CORRECT (proper JSON array)

12. **`bf log --format json`** (line 2569-2571) ✅
    ```rust
    println!("{}", crate::log::format_events_json(&events)?);
    ```
    Where `format_events_json` in `src/log.rs`:
    ```rust
    pub fn format_events_json(events: &[Event]) -> Result<String> {
        Ok(serde_json::to_string_pretty(events)?)
    }
    ```
    - **Output:** JSON array of event objects
    - **Status:** CORRECT (proper JSON array)

13. **`bf critical-path --format json`** (line 2599-2602) ✅
    ```rust
    println!("{}", serde_json::to_string_pretty(&result)?);
    ```
    - **Output:** Single critical path result object
    - **Status:** CORRECT (outputs critical path data, not issue list)

---

## Commands That Need Fixing

### Priority 1: Root Fix (affects multiple commands)

**Fix `src/format/json.rs:17-29` - `JsonFormatter::format_issues()` method**

Current implementation (incorrect):
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
        .join("\n")  // ← Problem: outputs JSONL
}
```

Should be (correct):
```rust
fn format_issues(&self, issues: &[Issue]) -> String {
    let formatted: Vec<_> = issues
        .iter()
        .map(|issue| {
            let mut stripped = issue.clone();
            stripped.dependencies = vec![];
            stripped.comments = vec![];
            stripped
        })
        .collect();
    serde_json::to_string(&formatted).unwrap_or_else(|_| "[]".to_string())
}
```

**Impact:** This single fix will correct JSON output for:
- `bf list --format json`
- `bf search --format json`

### Priority 2: Custom Code Fix

**Fix `src/cli/mod.rs:1238-1242` - `cmd_ready()` JSON case**

Current implementation (incorrect):
```rust
"json" => {
    for candidate in candidates {
        println!("{}", serde_json::to_string(&candidate)?);
    }
}
```

Should be (correct):
```rust
"json" => {
    println!("{}", serde_json::to_string(&candidates)?);
}
```

**Impact:** Corrects `bf ready --format json` to output array instead of JSONL.

---

## Summary Table

| Command | Current Format | Should Be | Uses format_issues()? | Fix Needed |
|---------|---------------|-----------|----------------------|------------|
| `bf list --format json` | JSONL | Array | ✅ | Yes (via format_issues fix) |
| `bf search --format json` | JSONL | Array | ✅ | Yes (via format_issues fix) |
| `bf ready --format json` | JSONL | Array | ❌ | Yes (custom code fix) |
| `bf show --format json` | Array | Array | ❌ | No |
| `bf claim --format json` | Object | Object | ❌ | No |
| `bf mitosis --format json` | Array | Array | ❌ | No |
| `bf dep tree --format json` | Object | Object | ❌ | No |
| `bf labels --format json` | Array | Array | ❌ | No |
| `bf stats --format json` | Object | Object | ❌ | No |
| `bf schema --format json` | Object/Object | Object | ❌ | No |
| `bf velocity --format json` | Array | Array | ❌ | No |
| `bf log --format json` | Array | Array | ❌ | No |
| `bf critical-path --format json` | Object | Object | ❌ | No |

---

## Total Commands Documented: 13

### Commands needing fixes: 3
1. `bf list --format json` (via `format_issues()` fix)
2. `bf search --format json` (via `format_issues()` fix)
3. `bf ready --format json` (custom code fix)

### Commands already correct: 10
- `bf show`, `bf claim`, `bf mitosis`, `bf dep tree`, `bf labels`, `bf stats`, `bf schema`, `bf velocity`, `bf log`, `bf critical-path`

---

## Next Steps

This was a **research-only task**. No implementation was performed. The findings above document:
1. ✅ All commands that output JSON
2. ✅ Which commands use `format_issues()` (should output arrays, currently don't)
3. ✅ Which commands have custom JSON handling
4. ✅ List of commands that need fixing

**Implementation should be tracked in separate beads.**
