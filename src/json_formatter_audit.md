# JSON Output Implementation Audit

This document audits the current JSON output implementations across all commands with the `--format json` flag.

## Summary

**Commands Audited:** list, ready, search, claim, stats, velocity

**Key Finding:** There is significant inconsistency in JSON output approaches across commands. Some use the `get_formatter()` pattern, while others use custom `serde_json` serialization directly.

## Commands Using `get_formatter().format_issues()`

### 1. `list` command (cmd_list, line 995-1079)

**Location:** `src/cli/mod.rs:995-1079`

**Implementation:**
```rust
let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
let formatter = get_formatter(output_format);
print!("{}", formatter.format_issues(&issues));
```

**JSON Output Format:** JSONL (newline-delimited JSON objects)

**Why:** Delegates to `JsonFormatter::format_issues()` which outputs JSONL format

**Output Example:**
```json
{"id":"bf-1","title":"Fix bug","status":"open",...}
{"id":"bf-2","title":"Add feature","status":"closed",...}
```

### 2. `search` command (cmd_search, line 2051-2092)

**Location:** `src/cli/mod.rs:2051-2092`

**Implementation:**
```rust
let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
let formatter = get_formatter(output_format);
print!("{}", formatter.format_issues(&issues));
```

**JSON Output Format:** JSONL (newline-delimited JSON objects)

**Why:** Same pattern as `list` - uses `JsonFormatter::format_issues()`

**Output Example:** Same as `list` above

## Commands Using Custom JSON Serialization

### 3. `ready` command (cmd_ready, line 1229-1271)

**Location:** `src/cli/mod.rs:1229-1271`

**Implementation:**
```rust
match format {
    "json" => {
        // Output as JSON array: [] for empty, [candidate] for single, [c1, c2, ...] for multiple
        println!("{}", serde_json::to_string(&candidates)?);
    }
    // ...
}
```

**JSON Output Format:** JSON array

**Why:** Direct serialization using `serde_json::to_string()`

**Output Example:**
```json
[{"id":"bf-1","title":"Fix bug","priority":2,...}]
```

**Inconsistency:** Outputs as JSON array (`[{...}]`) not JSONL, unlike `list`/`search`

### 4. `claim` command (cmd_claim, line 1273-1526)

**Location:** `src/cli/mod.rs:1273-1526`

**Implementation:** Multiple custom JSON outputs depending on claim mode

**Dry run mode:**
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

**Normal claim mode:**
```rust
let output = serde_json::json!({
    "bead_id": bead_id,
    "reclaimed": reclaimed,
    "assignee": assignee,
    "workspace": workspace_path.map(|p| p.display().to_string())
});
println!("{}", output);
```

**No beads case:**
```rust
println!("{{}}");  // Empty object
```

**JSON Output Format:** JSON object

**Why:** Uses `serde_json::json!` macro for structured claim results

**Output Example:**
```json
{"bead_id":"bf-1","reclaimed":false,"assignee":"worker-1","workspace":"/path/to/workspace"}
```

### 5. `stats` command (cmd_stats, line 2094-2153)

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

**JSON Output Format:** Pretty-printed JSON object

**Why:** Direct serialization with `serde_json::to_string_pretty()`

**Output Example:**
```json
{
  "total": 100,
  "open": 45,
  "in_progress": 12,
  "closed": 43
}
```

**Note:** The `--by-type`, `--by-priority`, `--by-assignee`, and `--by-label` flags are **NOT** reflected in JSON output - they only affect text output. This is a significant inconsistency.

### 6. `velocity` command (cmd_velocity, line 2345-2414)

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

**JSON Output Format:** Pretty-printed JSON array

**Why:** Direct serialization with `serde_json::to_string_pretty()`

**Output Example:**
```json
[
  {
    "model": "claude-opus-4",
    "harness": "claude-code",
    "issue_type": "task",
    "sample_count": 42,
    "p50_seconds": 180,
    "p90_seconds": 420,
    "avg_seconds": 215.5
  }
]
```

## Inconsistencies Found

### 1. Array vs Object vs JSONL formatting

| Command | Format | Structure |
|---------|--------|-----------|
| `list` | JSONL | Newline-delimited objects |
| `search` | JSONL | Newline-delimited objects |
| `ready` | JSON Array | `[{...}, {...}]` |
| `claim` | JSON Object | `{...}` |
| `stats` | JSON Object | `{...}` |
| `velocity` | JSON Array | `[{...}, {...}]` |

### 2. Pretty-printing

- `stats` and `velocity` use `to_string_pretty()` for human-readable formatting
- `ready` and `claim` use compact `to_string()`
- `list` and `search` use JSONL format (newline-delimited, no outer array)

### 3. Empty result handling

| Command | Empty Output |
|---------|-------------|
| `list` | (empty string - no JSON objects) |
| `search` | (empty string - no JSON objects) |
| `ready` | `[]` |
| `claim` | `{}` |
| `stats` | Stats object with zero counts |
| `velocity` | `[]` |

### 4. Missing breakdown data in `stats` JSON

The `stats` command's JSON output only includes the 4 core stats fields (`total`, `open`, `in_progress`, `closed`). The breakdown fields requested via `--by-type`, `--by-priority`, `--by-assignee`, and `--by-label` are **not** included in JSON output - they only appear in text mode.

## Recommendation

Consider standardizing on one of these approaches:

1. **JSONL for lists** (`list`, `search`): Current approach is reasonable for streaming/large result sets
2. **JSON Array for structured data** (`ready`, `velocity`): Current approach makes sense for query results
3. **JSON Object for single results** (`claim`, `stats`): Current approach is appropriate for single-value responses

The main inconsistencies to address:
1. Decide whether `list`/`search` should output JSONL or JSON arrays
2. Include breakdown data in `stats` JSON output when breakdown flags are used
3. Standardize pretty-printing (either use it everywhere or nowhere)
