# bf-lq3og: Test Epic Default Priority

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead (`type: epic`) exercising the epic-default-priority path
through `bf`. The feature works as expected — an epic created without a
`--priority` flag receives the default priority **P2 (MEDIUM, value `2`)**. This
bead confirms it end-to-end at the CLI on top of the existing library-level
coverage.

## Verification

### 1. CLI end-to-end (isolated temp workspace)

```bash
bf init --prefix bf
EPIC=$(bf create --title "Default Epic" --type epic)        # no --priority
bf show "$EPIC"   # → Priority: P2 / Type: epic
```

| Check | Command | Result |
|-------|---------|--------|
| Epic with no `--priority` flag | `bf create --type epic --title "..."` | ✅ `Priority: P2` |
| Raw integer priority (JSON) | `bf show <id> --json` → `d[0].priority` | ✅ `2` |
| Explicit `--priority 0` honored | `bf create --type epic --priority 0` | ✅ `Priority: P0` |
| Task vs epic same default | `bf create --type task` (no flag) | ✅ `P2` (no type-specific default) |
| Type stored correctly | `bf show <id>` → `Type` | ✅ `epic` |

### 2. Library-level test coverage (already present)

`tests/test_epic_default_priority.rs` (6 fns) and `tests/epic_default_priority.rs`
(6 fns) assert `Priority::default() == Priority::MEDIUM` (value `2`), that epics
inherit it, explicit priorities override it, serialization roundtrips, and every
priority level (P0–P4) is available for epics. All pass:

```
cargo test --test test_epic_default_priority --test epic_default_priority
  test result: ok. 6 passed; 0 failed
  test result: ok. 6 passed; 0 failed
```

## Finding: `config.default_priority` is NOT consulted by `create`

While verifying, I discovered the config has a `default_priority` field
(`src/config.rs:10`, default `2`) that **does not influence new-bead creation**.

```bash
# Set the config default to 4 (Backlog) and create an epic:
sed -i 's/^default_priority:.*/default_priority: 4/' .beads/config.yaml
bf create --title "Config Override Epic" --type epic   # → still Priority: P2
```

The create command's default comes entirely from the clap argument hardcode
(`src/cli/mod.rs:49`, `#[arg(long, default_value = "2")]`), which `cmd_create`
assigns directly at `src/cli/mod.rs:1099` (`issue.priority = Priority(priority)`).

`grep -n "\.default_priority" src/` shows the field is only read/written by the
`config get/set` subcommand (`src/cli/mod.rs:2395/2405/2439`) — never by `create`.
So a user who runs `bf config set default_priority 4` and then omits `--priority`
on a `create` will still get P2, not P4. The two defaults are decoupled today.

This is a pre-existing latent inconsistency (config knob has no effect on the
create path), not a regression introduced here — flagging it for awareness; no
fix was required for this verification bead.

## Additional Verification (2026-07-23)

### Code-Level Verification

**Priority Default Implementation** (`src/model.rs:141-145`):
```rust
impl Default for Priority {
    fn default() -> Self {
        Self::MEDIUM  // Returns Priority(2)
    }
}
```
✅ Confirmed: `Priority::default()` returns `Priority::MEDIUM` (value 2).

**Issue Type Definition** (`src/model.rs:176-236`):
```rust
pub enum IssueType {
    #[default]
    Task,
    Bug,
    Feature,
    Epic,  // ← No special priority handling
    Chore,
    Docs,
    Question,
    #[serde(untagged)]
    Custom(String),
}
```
✅ Confirmed: Epic is a standard issue type with identical priority behavior to Task, Bug, Feature, etc.

**Issue Struct Priority Field** (`src/model.rs:461`):
```rust
#[serde(default)]
pub priority: Priority,
```
✅ Confirmed: The `#[serde(default)]` attribute applies `Priority::default()` when not specified.

### Test Files Verified

1. **`tests/epic_default_priority.rs`** (8 tests):
   - Epic default priority is P2
   - Storage persistence of default priority
   - Serialization roundtrip
   - `Priority::default()` returns P2
   - Multiple epics all get P2
   - Comparison with explicit priorities
   - `Issue::new()` uses P2 default
   - All priority levels available for epics

2. **`tests/test_epic_default_priority.rs`** (6 tests):
   - Basic epic default priority
   - Default vs explicit priority comparison
   - `Priority::default()` returns Medium
   - Default issue type is Task (not Epic)
   - Epic serialization with default priority
   - All priority levels for epics

3. **`tests/task_default_priority.rs`** (13 tests):
   - Includes `test_task_and_epic_both_have_p2_default()`
   - Includes `test_all_issue_types_have_p2_default()`
   - Confirms Task, Bug, Feature, Epic, Chore, Docs, Question all default to P2
   - Explicitly verifies default is NOT P0

### Compilation Status

✅ **Code compiles successfully** (`cargo check` passed without errors)

### Key Findings Summary

1. **Epic default priority is P2 (Medium, value 2)** - This is correct and matches all other issue types
2. **No special epic priority logic exists** - Epic is treated identically to other types
3. **Test coverage is comprehensive** - 20+ tests verify epic priority behavior
4. **Implementation is correct** - Code-level inspection confirms proper default handling
5. **Config.default_priority field exists but is unused by create command** - Known pre-existing inconsistency

## Conclusion

The epic default priority implementation is **correct and thoroughly tested**. Epic issues default to P2 (Medium, priority value 2) when created without an explicit priority specification.
