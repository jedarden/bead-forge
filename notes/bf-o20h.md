# Triage: bf update missing field-edit flags

## Finding

**bf already HAS all field-edit flags.** This bead was filed under a misapprehension.

## Evidence

### CLI Flags (src/cli/mod.rs:114-154)

The `Update` command has ALL requested fields implemented:

```rust
Update {
    id: String,
    title: Option<String>,        // br parity ✓
    status: Option<String>,        // br parity ✓
    priority: Option<i32>,         // br parity ✓
    assignee: Option<String>,     // br parity ✓
    description: Option<String>,  // bf superset ✓
    acceptance_criteria: Option<String>, // bf superset ✓
    notes: Option<String>,        // bf superset ✓
    design: Option<String>,       // bf superset ✓
    due_at: Option<String>,       // bf superset ✓
}
```

### br Parity Check

```bash
$ br update --help
Options:
  --title <TITLE>          New title
  --status <STATUS>        New status
  --priority <PRIORITY>    New priority
  --assignee <ASSIGNEE>    New assignee
```

**br does NOT have field-edit flags.** br only supports the 4 core fields (title, status, priority, assignee). This is a known br limitation.

### Storage Path (src/storage/sqlite.rs:381-479)

The `update_issue()` method handles ALL fields:

- Lines 385-412: Secret scanning for all string fields (title, description, design, acceptance_criteria, notes, assignee, owner, external_ref)
- Lines 422-437: Individual field UPDATE clauses for description, design, acceptance_criteria, notes
- Lines 468-478: Date fields (due_at, defer_until)

### IssueChanges Struct (src/model.rs:843-860)

All fields are supported:
```rust
pub struct IssueChanges {
    pub title: Option<String>,
    pub description: Option<String>,
    pub design: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub notes: Option<String>,
    pub status: Option<Status>,
    pub priority: Option<i32>,
    pub issue_type: Option<IssueType>,
    pub assignee: Option<String>,
    pub owner: Option<String>,
    pub estimated_minutes: Option<i32>,
    pub due_at: Option<DateTime<Utc>>,
    pub defer_until: Option<DateTime<Utc>>,
    pub external_ref: Option<String>,
    pub labels: Option<Vec<String>>,
    pub annotations: Option<BTreeMap<String, String>>,
}
```

### cmd_update (src/cli/mod.rs:1144-1188)

All flags are wired to IssueChanges:
```rust
let changes = IssueChanges {
    title,
    status: status.map(|s| Status::from_str(&s).ok()).flatten(),
    priority,
    assignee,
    description,           // ✓
    acceptance_criteria,  // ✓
    notes,                // ✓
    design,               // ✓
    due_at: due_at_parsed, // ✓
    ..Default::default()
};
```

## Decision: Flag Set vs br Parity

**Result: No decision needed - current implementation is correct.**

bf is explicitly designed as a "strict superset" of br:
- **br parity:** All br commands work identically (✓ achieved)
- **Superset functionality:** Additional flags for extended fields (✓ achieved)

The 5 field-edit flags (description, acceptance_criteria, notes, design, due_at) are bf-specific features that br does not have. This is intentional and correct.

## Conclusion

**No fix needed.** The bead can be closed with a note that all requested flags are already implemented.

### Why This Bead Was Filed

Likely causes:
1. The filer assumed bf was missing flags because br lacks them
2. Confusion about whether bf should exactly match br's flag set vs. being a superset
3. The `deferred` label suggests this was a placeholder that was never acted on

### Recommendation

Close bead as "already implemented - no work needed."
