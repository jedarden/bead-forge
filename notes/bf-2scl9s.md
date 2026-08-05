# Custom Issue Serialization Audit

## Task
Audit custom Issue serialization logic in src/format/ formatters.

## Files Audited
- `src/format/json.rs` - JSON formatter with custom stripping
- `src/format/text.rs` - Text formatter (manual string construction)
- `src/format/toon.rs` - Toon formatter (manual string construction)
- `src/model.rs` - Issue struct with serde attributes

---

## Findings

### 1. src/format/json.rs - Custom Relation Stripping

**Location:** Lines 22-28

```rust
fn issue_to_value(issue: &Issue) -> Value {
    let mut stripped = issue.clone();
    stripped.dependencies = vec![];
    stripped.comments = vec![];

    serde_json::to_value(&stripped).unwrap_or(Value::Null)
}
```

**Purpose:** 
- Manually strips `dependencies` and `comments` relations before serde serialization
- Used by `format_issue()` and `format_issues()` for JSONL output

**Why it exists:**
1. **br compatibility** - Per code comments: "for `br` compatibility"
2. **Compact output** - Reduces JSON payload size by removing bulky nested structures
3. **JSONL format** - Each issue on one line, relations would make lines extremely long

**Impact of removal:**
- Would break br compatibility expectations
- Would significantly increase JSONL line length
- Would change JSON output format for list/ready/search commands
- Tests expect this stripped format (see test `assignee_skipped_when_unset`)

**Recommendation:** **KEEP** - This is intentional br compatibility behavior, not a bug.

---

### 2. src/format/text.rs - No Serde Usage

**Finding:** Text formatter does NOT use serde for Issue serialization.

**Implementation:**
- `format_issue()` (lines 9-30): Manually constructs key-value strings
- `format_issues()` (lines 32-41): Uses `format!` macro with `[id] title - status (priority)` pattern
- No `serde_json::to_string()` calls on Issue objects

**Why it exists:**
- Text format is human-readable, not JSON
- Manual formatting allows custom layout (labels on separate lines, timestamps formatted)
- More control over whitespace and line breaks than serde provides

**Impact:** None - this is the correct implementation for text output.

---

### 3. src/format/toon.rs - No Serde Usage

**Finding:** Toon formatter does NOT use serde for Issue serialization.

**Implementation:**
- `format_issue()` (lines 9-29): Manually constructs key-value strings  
- `format_toon_issue_line()` (lines 107-115): Uses compact `[id] title - status (priority)` format
- No `serde_json::to_string()` calls on Issue objects

**Why it exists:**
- Toon format is "ASCII art" style, not JSON
- Same reasons as text formatter

**Impact:** None - this is the correct implementation for toon output.

---

### 4. model.rs skip_serializing_if Attributes

**Finding:** Extensive use of `skip_serializing_if` attributes throughout Issue struct.

**Complete attribute inventory:**

```rust
// Line 441-454: Optional fields skip when None
#[serde(default, skip_serializing_if = "Option::is_none")]
pub description: Option<String>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub design: Option<String>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub acceptance_criteria: Option<String>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub notes: Option<String>

// Line 469-508: More optional fields
#[serde(default, skip_serializing_if = "Option::is_none")]
pub assignee: Option<String>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub owner: Option<String>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub estimated_minutes: Option<i32>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub created_by: Option<String>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub closed_at: Option<DateTime<Utc>>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub close_reason: Option<String>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub closed_by_session: Option<String>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub due_at: Option<DateTime<Utc>>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub defer_until: Option<DateTime<Utc>>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub external_ref: Option<String>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub source_system: Option<String>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub source_repo: Option<String>

// Lines 523-542: Tombstone and compaction fields
#[serde(default, skip_serializing_if = "Option::is_none")]
pub deleted_at: Option<DateTime<Utc>>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub deleted_by: Option<String>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub delete_reason: Option<String>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub original_type: Option<String>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub compacted_at: Option<DateTime<Utc>>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub compacted_at_commit: Option<String>
#[serde(default, skip_serializing_if = "Option::is_none")]
pub original_size: Option<i32>

// Lines 545-553: Messaging and boolean fields  
#[serde(default, skip_serializing_if = "Option::is_none")]
pub sender: Option<String>
#[serde(default, skip_serializing_if = "is_false")]
pub ephemeral: bool
#[serde(default, skip_serializing_if = "is_false")]
pub archived: bool
#[serde(default, skip_serializing_if = "is_false")]
pub pinned: bool

// Lines 557-562: Collections skip when empty
#[serde(skip_serializing_if = "Vec::is_empty", default)]
pub labels: Vec<String>
#[serde(skip_serializing_if = "Vec::is_empty", default)]
pub dependencies: Vec<Dependency>
#[serde(skip_serializing_if = "Vec::is_empty", default)]
pub comments: Vec<Comment>

// Line 566: Annotations skip when empty
#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
pub annotations: BTreeMap<String, String>
```

**Purpose:** These attributes ensure compact JSON output by omitting:
- `None` optional fields
- Empty collections (`Vec`, `BTreeMap`)
- `false` boolean flags

**Custom serializer:**
```rust
// Line 535: Compaction level always serialized as integer (0 when None)
#[serde(default, serialize_with = "serialize_compaction_level")]
pub compaction_level: Option<i32>
```

**Why this exists:** bd's Go SQL scanner cannot handle NULL for integer columns, so None must serialize as 0.

---

## Summary Table

| Formatter | Uses Serde | Custom Logic | Purpose |
|-----------|------------|--------------|---------|
| json.rs   | Yes        | Manual strip of deps/comments | br compatibility + compact JSONL |
| text.rs   | No         | Manual string construction | Human-readable output |
| toon.rs   | No         | Manual string construction | ASCII art style output |

---

## Recommendations

### 1. Keep Manual Stripping in json.rs
✅ **DO NOT REMOVE** the `issue_to_value()` function that strips dependencies and comments.

**Rationale:**
- Documented purpose: "for `br` compatibility"
- Reduces JSONL line length significantly
- Tests depend on this behavior
- Changing it would be a breaking format change

### 2. Current Implementation is Correct
✅ **NO CHANGES NEEDED** in text.rs or toon.rs

**Rationale:**
- Text/toon formats are not JSON, so serde is not appropriate
- Manual formatting provides better control over output layout
- This is the correct pattern for non-JSON formatters

### 3. model.rs Attributes are Appropriate
✅ **KEEP** all `skip_serializing_if` attributes

**Rationale:**
- Ensures compact JSON output
- Matches br behavior expectations
- Properly handles bd conformance (compaction_level)

---

## Test Coverage

The json.rs formatter has comprehensive test coverage:
- `assignee_skipped_when_unset` - Verifies skip_serializing_if works
- `labels_skipped_when_empty` - Verifies collection skipping
- `format_issues_guarantees_fields_per_line` - Verifies consistent JSONL structure
- `format_issues_empty_yields_empty_string` - Edge case handling
- `format_issues_single_yields_one_valid_json_line` - Single issue JSONL
- `format_issues_multiple_yields_jsonl_one_object_per_line` - Multi-issue JSONL

All tests expect the current stripped format.

---

## Conclusion

The audit found **no issues requiring fixes**. All custom serialization logic is intentional and appropriate:

1. **json.rs:** Manual stripping exists for br compatibility - keep it
2. **text.rs/toon.rs:** Correctly avoid serde for non-JSON formats
3. **model.rs:** Appropriate use of serde attributes for compact output

The manual relation stripping in `issue_to_value()` is a **feature, not a bug** - it ensures br compatibility and keeps JSONL lines manageable in length.
