# JSON Export Code Paths for Issue Serialization

**Bead:** bf-py65rp  
**Date:** 2026-08-05  
**Purpose:** Comprehensive audit of all Issue serialization paths to inform subsequent fixes

## Overview

This audit identifies every location where `Issue` structs are serialized to JSON in the bead-forge codebase. Each path is documented with its location, serialization method, and whether it uses standard serde attributes or custom logic.

## 1. Primary Model Definition

**Location:** `src/model.rs:428-567`

**Function:** `Issue` struct definition with serde attributes

**Serialization Method:** Standard serde with extensive custom attributes

**Key Serde Attributes:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Issue {
    pub id: String,
    #[serde(skip)]  // Never serialized
    pub content_hash: Option<String>,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // ... many more fields with similar patterns
    
    // Relations with conditional serialization
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub labels: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub dependencies: Vec<Dependency>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub comments: Vec<Comment>,
    
    // Annotations (bf-only feature)
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, String>,
}
```

**Custom Logic:**
- `content_hash` is never serialized (always skipped)
- Empty vectors are skipped (`labels`, `dependencies`, `comments`)
- `None` optionals are skipped
- `compaction_level` uses custom serializer (`serialize_compaction_level`) that outputs `0` when `None` (for bd conformance)

---

## 2. JSONL Import/Export

**Location:** `src/jsonl.rs`

### 2.1 Full Export (`export_jsonl`)

**Lines:** 74-100

**Serialization Method:** Standard serde via `serde_json::to_writer`

**Code:**
```rust
pub fn export_jsonl<F>(path: &Path, mut list_all: F) -> Result<ExportResult>
where
    F: FnMut() -> Result<Vec<Issue>>,
{
    let mut issues = list_all()?;
    issues.sort_by(|a, b| a.id.cmp(&b.id));  // Stable sorting
    // ... 
    for issue in &issues {
        serde_json::to_writer(&mut writer, issue)?;  // Standard Issue serialization
        writer.write_all(b"\n")?;
    }
}
```

**What it does:** Uses standard `Issue` serde attributes, writes one JSON object per line

### 2.2 Incremental/Merge Export (`export_jsonl_merge`)

**Lines:** 118-183

**Serialization Method:** Standard serde via `serde_json::to_string`

**Code:**
```rust
for issue in upserts {
    by_id.insert(issue.id.clone(), serde_json::to_string(issue)?);  // Standard Issue serialization
}
```

**What it does:** 
- Uses standard `Issue` serde attributes
- Surgical line replacement (preserves untouched lines byte-for-byte)
- Sorts by ID for stable diffs

### 2.3 Incremental Dirty Flush (`export_jsonl_dirty` / `incremental_flush`)

**Lines:** 202-266

**Serialization Method:** Calls `export_jsonl_merge` (indirectly standard serde)

**Code:**
```rust
pub fn incremental_flush(conn: &rusqlite::Connection, path: &Path) -> Result<FlushResult> {
    let list_dirty = || -> Result<Vec<crate::model::Issue>> {
        // Queries dirty issues from SQLite
        let mut stmt = conn.prepare(
            "SELECT i.id, ... GROUP_CONCAT(bl.label) AS labels
             FROM issues i
             INNER JOIN dirty_issues d ON i.id = d.issue_id
             LEFT JOIN bead_labels bl ON i.id = bl.bead_id
             GROUP BY i.id"
        )?;
        // ... returns Vec<Issue> with full labels
    };
    let result = export_jsonl_dirty(path, list_dirty, clear_dirty)?;
}
```

**What it does:**
- Uses standard `Issue` serde attributes (via `export_jsonl_merge`)
- Only writes dirty (modified) beads
- Includes full `labels` from `bead_labels` table
- Surgical line replacement like `export_jsonl_merge`

### 2.4 Import (`import_jsonl`)

**Lines:** 49-72

**Serialization Method:** Standard serde via `serde_json::from_str`

**Code:**
```rust
for line in reader.lines() {
    let line = line?;
    let issue: Issue = serde_json::from_str(&line)?;  // Standard Issue deserialization
    match upsert(&issue)? { ... }
}
```

**What it does:** Standard deserialization using `Issue` serde attributes

---

## 3. CLI Formatters

**Location:** `src/format/json.rs`

### 3.1 Single Issue Formatting (`issue_to_value`)

**Lines:** 27-37

**Serialization Method:** **CUSTOM** - strips fields, ensures display fields

**Code:**
```rust
fn issue_to_value(issue: &Issue) -> Value {
    let mut stripped = issue.clone();
    stripped.dependencies = vec![];  // ALWAYS REMOVED
    stripped.comments = vec![];        // ALWAYS REMOVED

    let mut value = serde_json::to_value(&stripped).unwrap_or(Value::Null);
    if let Value::Object(ref mut map) = value {
        ensure_display_fields(map);  // ALWAYS ensures assignee + labels
    }
    value
}

fn ensure_display_fields(map: &mut Map<String, Value>) {
    map.entry("assignee").or_insert(Value::Null);  // Force null when None
    map.entry("labels").or_insert_with(|| Value::Array(vec![]));  // Force [] when empty
}
```

**What it does differently:**
1. **Strips `dependencies` and `comments`** (br compatibility - different format)
2. **Forces `assignee` field** to be present (as `null` when `None`)
3. **Forces `labels` field** to be present (as `[]` when empty)

This is **different from standard Issue serialization** which:
- Skips `assignee` when `None`
- Skips `labels` when empty

### 3.2 Multiple Issues Formatting (`format_issues`)

**Lines:** 50-57

**Serialization Method:** Calls `issue_to_value` (CUSTOM) for each issue

**Code:**
```rust
fn format_issues(&self, issues: &[Issue]) -> String {
    issues
        .iter()
        .map(|issue| serde_json::to_string(&issue_to_value(issue)))  // Uses CUSTOM path
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default()
        .join("\n")
}
```

**What it does:** Produces JSONL (one object per line) using CUSTOM serialization

### 3.3 Single Issue Formatting (`format_issue`)

**Lines:** 46-48

**Serialization Method:** Calls `issue_to_value` (CUSTOM)

**Code:**
```rust
fn format_issue(&self, issue: &Issue) -> String {
    serde_json::to_string(&issue_to_value(issue)).unwrap_or_else(|_| "{}".to_string())
}
```

**What it does:** Uses CUSTOM serialization

---

## 4. CLI Commands Using JSON Serialization

**Location:** `src/cli/mod.rs`

### 4.1 `bf show --format json`

**Lines:** 1746-1786 (cmd_show)

**Serialization Method:** **HYBRID** - strips relations, uses JsonFormatter

**Code:**
```rust
fn cmd_show(...) {
    let mut out = issue;
    out.dependencies = vec![];  // MANUAL STRIPPING
    out.comments = vec![];       // MANUAL STRIPPING
    let formatter = get_formatter(OutputFormat::Json);
    let json_str = formatter.format_issue(&out);  // Then uses JsonFormatter (CUSTOM)
}
```

**What it does:**
- Manually strips `dependencies` and `comments`
- Passes to JsonFormatter (which applies CUSTOM display field logic)
- Output wrapped in array: `[{...}]` (NEEDLE contract)

### 4.2 `bf list --format json`

**Lines:** 1635-1743 (cmd_list)

**Serialization Method:** Uses JsonFormatter via `format_issues` (CUSTOM)

**Code:**
```rust
fn cmd_list(...) {
    let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
    let formatter = get_formatter(output_format);
    match output_format {
        OutputFormat::Json => {
            let jsonl = formatter.format_issues(&issues);  // Uses CUSTOM serialization
            if envelope { ... } else {
                if !jsonl.is_empty() {
                    println!("{}", jsonl);  // Raw JSONL output
                }
            }
        }
    }
}
```

**What it does:**
- Uses CUSTOM serialization via `formatter.format_issues`
- Outputs raw JSONL (no array wrapper) unless empty
- Optional envelope wrapping

### 4.3 `bf ready --format json`

**Lines:** 1949-2012 (cmd_ready)

**Serialization Method:** **READYCONVERSION** → JsonFormatter (CUSTOM)

**Code:**
```rust
fn cmd_ready(...) {
    let candidates = storage.with_immediate_transaction(|tx| get_ready_candidates(tx, limit, None, None))?;
    
    let issues: Vec<Issue> = candidates
        .iter()
        .filter_map(|c| {
            ReadyCandidate::from_scored_bead(c)
                .ok()
                .map(|candidate| candidate.to_issue())  // CONVERTS ReadyCandidate → Issue
        })
        .collect();

    let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
    let formatter = get_formatter(output_format);
    match output_format {
        OutputFormat::Json => {
            let jsonl = formatter.format_issues(&issues);  // Uses CUSTOM serialization
            // ... outputs JSONL with special case for empty (prints "[]")
        }
    }
}
```

**What it does:**
1. Converts `ReadyCandidate` → `Issue` (with default fields)
2. Uses CUSTOM serialization via `formatter.format_issues`
3. Special case: empty ready prints `[]` instead of empty output

### 4.4 `bf search --format json`

**Lines:** 3040-3093 (cmd_search)

**Serialization Method:** Uses JsonFormatter via `format_issues` (CUSTOM)

**Code:**
```rust
fn cmd_search(...) {
    let issues = storage.search_issues(...)?;
    
    let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
    let formatter = get_formatter(output_format);
    match output_format {
        OutputFormat::Json => {
            let jsonl = formatter.format_issues(&issues);  // Uses CUSTOM serialization
            if !jsonl.is_empty() {
                println!("{}", jsonl);  // Raw JSONL
            }
        }
    }
}
```

**What it does:** Uses CUSTOM serialization via `formatter.format_issues`

### 4.5 `bf recent --format json`

**Lines:** 3746-3823 (cmd_recent)

**Serialization Method:** Uses JsonFormatter via `format_issues` (CUSTOM)

**Code:**
```rust
fn cmd_recent(...) {
    let issues = storage.list_issues(&filter)?;
    
    let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
    let formatter = get_formatter(output_format);
    match output_format {
        OutputFormat::Json => {
            let json_str = formatter.format_issues(&issues);  // Uses CUSTOM serialization
            println!("{}", formatter.format_with_envelope("recent", &json_str));
        }
    }
}
```

**What it does:** Uses CUSTOM serialization via `formatter.format_issues` with envelope

### 4.6 `bf labels --format json` (all beads mode)

**Lines:** 2952-3005 (cmd_labels)

**Serialization Method:** **MANUAL JSON construction** (via serde_json::json!)

**Code:**
```rust
fn cmd_labels(...) {
    if let Some(issue_id) = id {
        // Single bead mode - uses direct array serialization
        let labels = storage.get_labels(issue_id)?;
        if format == "json" {
            println!("{}", serde_json::to_string(&labels)?);  // Direct array serialization
        }
    } else {
        // All beads mode - uses MANUAL object construction
        let issues = storage.list_issues(&filter)?;
        if format == "json" {
            if issues.is_empty() {
                println!("[]");  // Special case
            } else {
                for issue in &issues {
                    let obj = serde_json::json!({  // MANUAL object construction
                        "id": issue.id,
                        "title": issue.title,
                        "labels": issue.labels
                    });
                    println!("{}", serde_json::to_string(&obj)?);  // Manual serialization
                }
            }
        }
    }
}
```

**What it does:**
- Single bead: Direct array serialization
- All beads: **MANUAL** construction of `{id, title, labels}` objects
- Not using full `Issue` serialization

### 4.7 `bf dep tree --format json`

**Lines:** 2828-2900 (cmd_dep → DepCommands::Tree)

**Serialization Method:** **MANUAL JSON construction** (via serde_json::json!)

**Code:**
```rust
DepCommands::Tree { id, direction, max_depth, format, json } => {
    if format == "json" {
        if direction == "both" {
            let down_nodes = storage.get_dep_tree(&id, "down", max_depth)?;
            let up_nodes = storage.get_dep_tree(&id, "up", max_depth)?;
            let output = serde_json::json!({  // MANUAL construction
                "root_id": id,
                "direction": direction,
                "max_depth": max_depth,
                "downward": down_nodes,
                "upward": up_nodes
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            let nodes = storage.get_dep_tree(&id, direction, max_depth)?;
            let output = serde_json::json!({  // MANUAL construction
                "root_id": id,
                "direction": direction,
                "max_depth": max_depth,
                "nodes": nodes
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }
}
```

**What it does:** **MANUAL** JSON object construction (not using `Issue` serialization)

### 4.8 `bf create --json`

**Lines:** 1542-1633 (cmd_create)

**Serialization Method:** **MANUAL JSON construction** (via serde_json::json!)

**Code:**
```rust
fn cmd_create(...) {
    // ... create the issue ...
    
    if json {
        let formatter = get_formatter(OutputFormat::Json);
        let data = serde_json::json!({  // MANUAL object construction
            "id": id,
            "title": issue.title,
            "type": issue.issue_type.to_string(),
            "priority": issue.priority.0,
            "status": issue.status.to_string(),
            "description": issue.description,
            "assignee": issue.assignee,
            "labels": issue.labels
        });
        let json_str = serde_json::to_string(&data)?;
        println!("{}", formatter.format_with_envelope_and_warning("create", &json_str, warning.as_deref()));
    }
}
```

**What it does:** **MANUAL** construction of create response (not full `Issue` serialization)

### 4.9 `bf schema <bead_id>`

**Lines:** 3160-3208 (cmd_schema)

**Serialization Method:** Standard serde via `serde_json::to_string_pretty`

**Code:**
```rust
fn cmd_schema(target: &str, format: &str) {
    match target {
        bead_id => {
            let mut issue = match storage.get_issue(bead_id)? { ... };
            issue.annotations = storage.get_annotations(bead_id)?;  // Load annotations
            
            match format {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&issue)?);  // STANDARD serialization
                }
            }
        }
    }
}
```

**What it does:** Uses **STANDARD** `Issue` serde attributes (no custom stripping)

### 4.10 Other Commands with Stats/Custom Output

These commands serialize custom structs, not `Issue` directly:

- `bf stats --format json` → serializes `StatsOutput` struct (custom format)
- `bf velocity --format json` → serializes `VelocityStats` array (custom format)
- `bf claim --format json` → serializes `ClaimResultOutput` struct (custom format)
- `bf batch --format json` → serializes `BatchResult` array (custom format)
- `bf critical-path --format json` → serializes critical path result (custom format)

---

## Summary of Serialization Types

### Standard Issue Serialization (JSONL + schema command)
- Uses `Issue` serde attributes as-is
- Skips `content_hash` always
- Skips empty vectors (`labels`, `dependencies`, `comments`)
- Skips `None` optionals
- `compaction_level` → `0` when `None`
- **Used by:** `export_jsonl`, `export_jsonl_merge`, `incremental_flush`, `import_jsonl`, `cmd_schema`

### Custom Issue Serialization (CLI formatters)
- **Strips** `dependencies` and `comments` (always removed)
- **Forces** `assignee` field (as `null` when `None`)
- **Forces** `labels` field (as `[]` when empty)
- **Used by:** `JsonFormatter::format_issue`, `JsonFormatter::format_issues`
- **Called by:** `cmd_show`, `cmd_list`, `cmd_ready`, `cmd_search`, `cmd_recent`

### Manual JSON Construction
- **Does NOT** use `Issue` serialization at all
- Constructs custom objects with specific fields
- **Used by:** `cmd_create` (create response), `cmd_labels` (labels command), `cmd_dep` (tree command)

---

## Key Findings

1. **Dual serialization paths:**
   - Standard path for JSONL export/import (preserves all fields)
   - Custom path for CLI JSON output (strips relations, ensures display fields)

2. **Field presence inconsistencies:**
   - Standard: `assignee` and `labels` skipped when empty/None
   - Custom: `assignee` and `labels` always present (null/[])

3. **Labels handling:**
   - JSONL export: Full labels from `bead_labels` table
   - CLI display: Depends on formatter path

4. **Relation stripping:**
   - `dependencies` and `comments` stripped by CLI formatters (br compatibility)
   - Preserved in JSONL export

5. **Special cases:**
   - `bf ready --format json` prints `[]` when empty (unlike list/search)
   - `bf show --format json` wraps in array (NEEDLE contract)
   - `bf labels --format json` uses manual construction for all-beads mode

---

## Implications for Fixes

When fixing serialization issues, need to consider:

1. **Which path** is affected (standard vs custom vs manual)
2. **Field presence** requirements (skip vs force)
3. **Backward compatibility** with existing JSONL files
4. **br/beads_rust compatibility** for relation stripping
5. **CLI consistency** across different commands
6. **Round-trip integrity** for JSONL import/export
7. **Testing coverage** for all serialization paths

The existence of **multiple serialization paths** means fixes need to be applied consistently or intentionally differ where required by the use case.