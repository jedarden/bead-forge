# Field Naming and Remaining Format Discrepancies

**Task:** Document field naming and any remaining differences between expected and actual batch operations format.

**Dependencies:**
- Structural differences documented in `notes/bf-50bh66.md`
- Expected format documented in `notes/bf-2lfh5z.md`
- Actual format documented in `notes/bf-1dzcxb.md`

---

## Executive Summary

While both formats use **snake_case naming convention**, the **field sets are completely disjoint**—no overlap exists between expected bead fields and actual operation result fields. This is not a naming convention mismatch but a **fundamental divergence in what the output represents**.

---

## 1. Field Naming Convention Analysis

### Expected Format Fields (Bead Objects)
| Field | Naming Convention | Domain |
|-------|------------------|--------|
| `priority` | snake_case | Bead data |
| `labels` | snake_case | Bead data |
| `title` | snake_case | Bead data |
| `type` | snake_case | Bead data |
| `description` | snake_case | Bead data |
| `status` | snake_case | Bead data |
| `id` | kebab-case (e.g., `bf-abc123`) | Bead identifier |

**Conclusion:** Expected format consistently uses **snake_case** for field names and **kebab-case** for ID values.

### Actual Format Fields (BatchResult Objects)
| Field | Naming Convention | Domain |
|-------|------------------|--------|
| `op` | snake_case (abbreviation of "operation") | Operation metadata |
| `status` | snake_case | Operation result |
| `id` | kebab-case (e.g., `bf-abc123`) or null | Created bead ID |
| `error` | snake_case | Error message |
| `message` | snake_case | Success message |
| `version` | snake_case | Envelope metadata |
| `kind` | snake_case | Envelope metadata |
| `data` | snake_case | Envelope data container |
| `warning` | snake_case | Envelope warning |

**Conclusion:** Actual format consistently uses **snake_case** for field names and **kebab-case** for ID values—same convention as expected.

---

## 2. Field Set Comparison

### Expected Fields (Present in test expectation)
| Field | Type | Present in Actual? | Notes |
|-------|------|-------------------|-------|
| `priority` | number | ❌ No | Bead field |
| `labels` | array of strings | ❌ No | Bead field |
| `title` | string | ❌ No | Bead field |
| `type` | string | ❌ No | Bead field |
| `description` | string | ❌ No | Bead field |
| `status` | string | ⚠️  Ambiguous | Same name, different meaning (bead status vs operation status) |
| `id` | string | ⚠️  Ambiguous | Same name, different context (bead ID vs created ID) |

### Actual Fields (Present in implementation)
| Field | Type | Present in Expected? | Notes |
|-------|------|---------------------|-------|
| `op` | number | ❌ No | Operation index |
| `status` | string | ⚠️  Ambiguous | Same name as bead status, but different domain |
| `id` | string or null | ⚠️  Ambiguous | Same name as bead ID, but only for create results |
| `error` | string or null | ❌ No | Error message |
| `message` | string or null | ❌ No | Success message |
| `version` | number | ❌ No | Envelope field |
| `kind` | string | ❌ No | Envelope field |
| `warning` | string or null | ❌ No | Envelope field |

---

## 3. Field Name Collisions (Ambiguity)

### `status` Field - Same Name, Different Domains

**In expected format (bead):**
```json
{
  "status": "in_progress"  // Bead lifecycle state
}
```
**Domain:** Bead workflow state (`todo`, `in_progress`, `completed`, `blocked`)

**In actual format (BatchResult):**
```json
{
  "status": "ok"  // Operation execution result
}
```
**Domain:** Operation result (`ok`, `error`)

**Impact:** If both formats were merged, `status` would be ambiguous without namespacing.

### `id` Field - Same Name, Different Contexts

**In expected format (bead):**
```json
{
  "id": "bf-abc123"  // Bead identifier
}
```
**Context:** Always present, identifies the bead

**In actual format (BatchResult):**
```json
{
  "id": "bf-abc123"  // Created bead ID (present only for successful create ops)
}
```
**Context:** Only present for successful `create` operations; `null` otherwise

**Impact:** Same name, different cardinality and presence rules.

---

## 4. Type Differences

### Bead Object Types (Expected)
| Field | Type | Example |
|-------|------|---------|
| `priority` | number (integer) | `0` |
| `labels` | array of strings | `["critical", "batch"]` |
| `title` | string | `"Fix authentication bug"` |
| `type` | string | `"bug"` |
| `status` | string | `"in_progress"` |
| `id` | string (kebab-case) | `"bf-abc123"` |

### BatchResult Object Types (Actual)
| Field | Type | Example |
|-------|------|---------|
| `op` | number (integer, zero-based) | `0` |
| `status` | string (enum: `"ok"`, `"error"`) | `"ok"` |
| `id` | string or null | `"bf-abc123"` or `null` |
| `error` | string or null | `"Bead not found"` or `null` |
| `message` | string or null | `"Created bead bf-abc123"` or `null` |

**Type compatibility:** Both use primitive JSON types (numbers, strings, arrays, null), so no JSON-level type incompatibility exists—only semantic domain mismatch.

---

## 5. Missing vs Extra Fields

### Fields Missing from Actual (Expected → Actual)
```
Expected format has these fields that are NOT in actual output:
  - priority
  - labels
  - title
  - type
  - description
  - (other bead fields)
```

**Impact:** Test cannot verify bead properties directly from batch output.

### Fields Extra in Actual (Actual → Expected)
```
Actual format has these fields that are NOT in expected output:
  - op
  - error
  - message
  - version
  - kind
  - warning
```

**Impact:** Implementation provides metadata that the test does not expect or validate.

---

## 6. Envelope Field Discrepancies

### Expected Envelope (from test)
```json
{
  "data": [...]
}
```

**Fields present:**
- `data` — array of bead objects

**Fields absent:**
- `version`
- `kind`
- `warning`

### Actual Envelope (from implementation)
```json
{
  "version": 1,
  "kind": "batch",
  "data": [...],
  "warning": null
}
```

**Fields present:**
- `version` — envelope version number
- `kind` — command identifier
- `data` — array of BatchResult objects
- `warning` — optional auto-flush warning

**Envelope field overlap:**
- ✅ `data` field exists in both (but contains different array element types)

**Envelope field differences:**
- ❌ Actual has `version`, `kind`, `warning` — not in expected
- ⚠️  Actual has same `data` field name but different element type

---

## 7. Cardinality Differences

### Expected Format Cardinality
- **One bead per array element** in `data` array
- Bead ID is **always present** in each bead object
- `status` field has **multiple enum values** (workflow states)

### Actual Format Cardinality
- **One operation result per array element** in `data` array
- Bead ID is **conditionally present** (only for successful `create` operations)
- `status` field has **two enum values** (`ok`, `error`)

---

## 8. Nullability Differences

### Expected Format Nullability
- `labels` may be empty array but is typically present
- Most fields are **non-nullable** in bead objects
- No conditional nullability based on operation type

### Actual Format Nullability
- `id` is **nullable** (present for create, null otherwise)
- `error` is **nullable** (present on error, null on success)
- `message` is **nullable** (present on success, null on error)
- `warning` in envelope is **nullable** (present only on auto-flush failure)

**Pattern:** Actual format uses **conditional nullability** to represent different operation outcomes; expected format uses **consistent presence** of all fields.

---

## 9. Semantic Domain Mismatch Summary

| Aspect | Expected Domain | Actual Domain | Compatible? |
|--------|----------------|---------------|------------|
| **Array element** | Bead (data record) | BatchResult (operation result) | ❌ |
| **priority** | Bead property | N/A | ❌ |
| **labels** | Bead property | N/A | ❌ |
| **op** | N/A | Operation index | ❌ |
| **error** | N/A | Error message | ❌ |
| **message** | N/A | Success message | ❌ |
| **status** (bead) | Workflow state | N/A | ❌ |
| **status** (result) | N/A | Operation result | ❌ |
| **id** (bead) | Bead identifier | N/A | ⚠️  Partial |
| **id** (result) | N/A | Created bead ID | ⚠️  Partial |
| **version** | N/A | Envelope version | ❌ |
| **kind** | N/A | Command type | ❌ |
| **warning** | N/A | Auto-flush warning | ❌ |

**Conclusion:** Zero semantic overlap except for ambiguous field name collisions (`status`, `id`).

---

## 10. Complete Field Inventory

### Expected Format Fields (from bead objects)
```json
{
  "id": "bf-abc123",
  "title": "...",
  "type": "...",
  "status": "...",
  "priority": 0,
  "labels": [...],
  "description": "...",
  // ... other bead fields
}
```

### Actual Format Fields (from BatchResult objects)
```json
{
  "op": 0,
  "status": "ok",
  "id": "bf-abc123",
  "error": null,
  "message": "..."
}
```

### Actual Format Envelope Fields
```json
{
  "version": 1,
  "kind": "batch",
  "data": [...],
  "warning": null
}
```

---

## 11. Summary of Discrepancies

### Field Naming
✅ **No naming convention conflict** — both formats use snake_case consistently

### Field Sets
❌ **Zero field overlap** — expected bead fields are disjoint from actual result fields

### Field Name Collisions
⚠️  **Ambiguous collisions** — `status` and `id` exist in both domains with different meanings

### Type Compatibility
✅ **JSON types compatible** — both use primitive JSON types, no type-level conflicts

### Semantic Domains
❌ **Fundamentally incompatible** — bead data domain vs operation result domain

### Envelope Structure
⚠️  **Superset mismatch** — actual has extra envelope fields (`version`, `kind`, `warning`)

### Cardinality
❌ **Different semantics** — one bead per element vs one operation result per element

### Nullability
❌ **Different patterns** — consistent presence vs conditional presence based on operation type

---

## 12. Root Cause

The discrepancies are **not superficial formatting issues** but stem from **different mental models**:

- **Test mental model:** "I run a batch command and want to see the beads I created"
  → Output should contain bead data

- **Implementation mental model:** "I run a batch command and want to know if each operation succeeded"
  → Output should contain operation results

This is a **design choice divergence**, not a bug or formatting oversight.

---

## 13. Conclusion

Field naming conventions are consistent (snake_case throughout), but the **field sets are completely disjoint**:

- **Expected format:** Bead data fields (`priority`, `labels`, `title`, `type`, etc.)
- **Actual format:** Operation result fields (`op`, `status`, `error`, `message`)

Two field name collisions (`status`, `id`) create ambiguity—same names, different domains.

The envelope structure is a **superset mismatch**: actual has `version`, `kind`, `warning` fields not present in expected.

**Recommendation:** Choose a resolution path (Option A: change test, Option B: change implementation, Option C: hybrid approach) as documented in `notes/bf-50bh66.md` Section 9.

---

**Next Steps:**
- Review resolution options in `notes/bf-50bh66.md`
- Decide whether to align implementation with test expectations or update test to validate operation results
- This documentation completes the format comparison task for bead bf-3xofss
