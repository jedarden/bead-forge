# Triage Report: bf-o20h (bf update missing field-edit flags)

## Bead Premise
> Confirm which fields lack flags (description, acceptance_criteria, notes, design). Locate Update clap cmd + cmd_update in src/cli/mod.rs + storage update path. Decide flag set vs br parity. Output: fix plan + acceptance criteria.

## Finding: **BEAD PREMISE IS FALSE**

The fields mentioned **already have flags** in `bf update`. This bead was based on an incorrect assumption.

## Evidence

### 1. br (beads_rust) Behavior

**br `create` supports:**
- `--title` ✓
- `--type` ✓
- `--priority` ✓
- `--description` ✓
- `--assignee` ✓
- `--label` ✓

**br `update` supports:**
- `--title` ✓
- `--status` ✓
- `--priority` ✓
- `--assignee` ✓
- `--description` ✗ (MISSING)
- `--acceptance_criteria` ✗ (MISSING)
- `--notes` ✗ (MISSING)
- `--design` ✗ (MISSING)

**br database schema (from `br schema`):**
```sql
description TEXT NOT NULL DEFAULT '',
design TEXT NOT NULL DEFAULT '',
acceptance_criteria TEXT NOT NULL DEFAULT '',
notes TEXT NOT NULL DEFAULT '',
```

The schema supports these fields, but the CLI doesn't expose them for updates.

### 2. bf (bead-forge) Behavior

**bf `create` supports (src/cli/mod.rs lines 35-59):**
- `--title` ✓
- `--type` ✓
- `--priority` ✓
- `--description` ✓
- `--assignee` ✓
- `--label` ✓

**bf `update` supports (src/cli/mod.rs lines 114-154):**
- `--title` ✓ (line 119-121)
- `--status` ✓ (line 123-125)
- `--priority` ✓ (line 127-129)
- `--assignee` ✓ (line 131-133)
- `--description` ✓ (line 135-137) **ALREADY EXISTS**
- `--acceptance_criteria` ✓ (line 139-141) **ALREADY EXISTS**
- `--notes` ✓ (line 143-145) **ALREADY EXISTS**
- `--design` ✓ (line 147-149) **ALREADY EXISTS**
- `--due_at` ✓ (line 151-153) **BONUS (not in br)**

**bf storage layer (src/storage/sqlite.rs lines 418-479):**
All these fields are persisted to the database:
- `description` (lines 422-424)
- `design` (lines 426-428)
- `acceptance_criteria` (lines 430-432)
- `notes` (lines 434-436)
- `due_at` (lines 468-470)

## Comparison Matrix

| Field | br create | br update | bf create | bf update | bf storage |
|-------|-----------|-----------|-----------|-----------|------------|
| title | ✓ | ✓ | ✓ | ✓ | ✓ |
| status | - | ✓ | - | ✓ | ✓ |
| priority | ✓ | ✓ | ✓ | ✓ | ✓ |
| assignee | ✓ | ✓ | ✓ | ✓ | ✓ |
| description | ✓ | **✗** | ✓ | **✓** | ✓ |
| acceptance_criteria | ✗ | **✗** | ✗ | **✓** | ✓ |
| notes | ✗ | **✗** | ✗ | **✓** | ✓ |
| design | ✗ | **✗** | ✗ | **✓** | ✓ |
| due_at | ✗ | **✗** | ✗ | **✓** | ✓ |

## Conclusion

**No work is needed.** The `bf update` command already supports all the fields mentioned in the bead description, and it's a strict superset of `br update` capabilities.

The bead appears to have been created without verifying the current state of the code.

## Recommendation

**CLOSE this bead as INVALID PREMISE.** All requested flags already exist and work correctly.

---

## Additional Analysis (Beyond Original Bead Scope)

While the bead's premise is false, a full review of `bf update` against the `IssueChanges` struct reveals a few fields that could be added for **complete parity with the storage layer**:

### Actually Missing from `bf update`

The `IssueChanges` struct supports these fields that lack CLI flags:

| Field | Storage Support | CLI Flag | Priority | Notes |
|-------|----------------|----------|----------|-------|
| `issue_type` | ✓ | ✗ | HIGH | Core metadata - create has `--type`, update should too |
| `owner` | ✓ | ✗ | HIGH | NEEDLE workflow uses owner != assignee |
| `estimated_minutes` | ✓ | ✗ | MEDIUM | Used in velocity tracking |
| `defer_until` | ✓ | ✗ | LOW | Niche scheduling field |
| `external_ref` | ✓ | ✗ | LOW | Rarely used |

### Already Handled via Subcommands
- `labels` → `bf label add/remove`
- `annotations` → `bf annotate set`

### Optional Future Enhancement
If desired, create a NEW bead for:
```
Title: "bf update: add missing flags for issue_type, owner, estimated_minutes"
```

With acceptance criteria:
- [ ] `bf update bf-xxx --type feature` changes issue_type
- [ ] `bf update bf-xxx --owner foo@example.com` sets owner
- [ ] `bf update bf-xxx --estimated-minutes 60` sets estimate
- [ ] All flags work in combination
- [ ] Invalid values rejected with clear errors

**Decision:** Defer to future if there's user demand. The current `bf update` is already a strict superset of `br update`.
