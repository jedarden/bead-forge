# Task bf-2bhvv0: cmd_show function analysis

## Summary
Analyzed the `cmd_show` function implementation in `src/cli/mod.rs` to understand current JSON output structure and assignee handling.

## Findings

### Current JSON Output Structure
- Location: `src/cli/mod.rs` lines 1742-1791+
- JSON handling: lines 1756-1774
- Calls `formatter.format_issue(&out)` at line 1767
- Two output modes:
  1. **Without envelope** (default): Prints as single-element array `[{...}]` at line 1773
  2. **With envelope** (`--envelope` flag): Wraps in envelope with `kind='show'` at line 1770

### formatter.format_issue Call Location
- Line 1767: `let json_str = formatter.format_issue(&out);`
- Uses `JsonFormatter::format_issue` from `src/format/json.rs:46-48`
- This calls `issue_to_value()` which strips dependencies/comments (lines 27-36)

### Assignee Handling - PRESENT ✅
**ASSIGNEE IS INCLUDED** in JSON output:
- `JsonFormatter::ensure_display_fields()` (lines 39-43) guarantees assignee key exists
- Line 41: `map.entry("assignee").or_insert(Value::Null);`
- Assignee is set to `null` when unset, never omitted
- Verified by tests at lines 123-127 (`assignee_null_when_unset`)

### Issue Processing Before JSON Output
- Issue retrieved at lines 1747-1754 (from DB or archives)
- Dependencies stripped at line 1763: `out.dependencies = vec![];`
- Comments stripped at line 1764: `out.comments = vec![];`
- Assignee field is preserved through this process

### Additional Notes
- The 'toon' format also handles assignee explicitly at lines 1785-1787
- JSON formatter ensures consistent shape for CLI consumers (assignee always present vs sometimes omitted in storage)

## Conclusion
The `cmd_show` function already properly includes the assignee field in its JSON output. The assignee is always present in the JSON structure, set to `null` when not assigned, and populated with the assignee string when assigned. This is handled by the `ensure_display_fields()` function in the JSON formatter.
