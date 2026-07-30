# run_bf_claim Call Chain and Metadata Flow Analysis

**Task:** Identify `run_bf_claim` call sites and document metadata flow  
**Date:** 2026-07-24  
**Scope:** NEEDLE codebase (`/home/coding/NEEDLE`)

---

## Overview

`run_bf_claim` is the core function that calls the `bf claim` CLI command for atomic bead selection and claiming. This analysis traces the complete call chain from entry points to `run_bf_claim` and documents where model/harness/harness-version metadata is available.

---

## Call Chain Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    METADATA SOURCES                                       │
├─────────────────────────────────────────────────────────────────────────┤
│ 1. CLI Entry Point (src/cli/mod.rs:871-888)                               │
│    - model: None                                                          │
│    - harness: Some("needle")                                             │
│    - harness_version: Some(env!("CARGO_PKG_VERSION"))                     │
│                                                                          │
│ 2. Worker Remote Store (src/worker/mod.rs:1181-1188)                      │
│    - model: from adapter (or None)                                        │
│    - harness: from adapter or fallback to "needle"                        │
│    - harness_version: from adapter or fallback to CARGO_PKG_VERSION       │
│                                                                          │
│ 3. Supervisor (src/supervisor/mod.rs:86-91)                                │
│    - model: None                                                          │
│    - harness: Some("needle")                                             │
│    - harness_version: Some(env!("CARGO_PKG_VERSION"))                     │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    BrCliBeadStore CREATION                                │
├─────────────────────────────────────────────────────────────────────────┤
│ • BrCliBeadStore::discover(workspace, model, harness, harness_version)   │
│ • Stores metadata in struct fields:                                      │
│   - self.model: Option<String>                                          │
│   - self.harness: Option<String>                                        │
│   - self.harness_version: Option<String>                                │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    BeadStore::claim_auto CALL                             │
├─────────────────────────────────────────────────────────────────────────┤
│ 1. src/worker/mod.rs:1070                                                │
│    self.claimer.claim_auto(&self.qualified_id(), strand)                │
│                                                                          │
│ 2. src/claim/mod.rs:412                                                  │
│    Claimer::claim_auto(actor, strand)                                    │
│    → self.store.claim_auto(actor).await                                 │
│                                                                          │
│ 3. src/bead_store/mod.rs:1282                                           │
│    BrCliBeadStore::claim_auto(actor)                                     │
│    → self.run_bf_claim(actor).await                                     │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    run_bf_claim IMPLEMENTATION                             │
├─────────────────────────────────────────────────────────────────────────┤
│ Location: src/bead_store/mod.rs:839-909                                  │
│                                                                          │
│ Uses stored metadata fields to build bf claim CLI arguments:              │
│   let mut args: Vec<&str> = Vec::with_capacity(10);                      │
│   args.push("claim");                                                     │
│   if let Some(model) = &self.model {                                     │
│       args.push("--model");                                              │
│       args.push(model.as_str());                                         │
│   }                                                                       │
│   if let Some(harness) = &self.harness {                                │
│       args.push("--harness");                                            │
│       args.push(harness.as_str());                                       │
│   }                                                                       │
│   if let Some(harness_version) = &self.harness_version {                 │
│       args.push("--harness-version");                                    │
│       args.push(harness_version.as_str());                               │
│   }                                                                       │
│   args.push("--assignee");                                               │
│   args.push(actor);                                                      │
│   args.push("--json");                                                   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    bf claim CLI INVOCATION                                 │
├─────────────────────────────────────────────────────────────────────────┤
│ bf claim --model <model> --harness <harness> \                            │
│         --harness-version <version> --assignee <actor> --json            │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Detailed Call Sites

### 1. Primary Call Site (Home Workspace Claims)

**File:** `src/bead_store/mod.rs:1282-1287`
```rust
async fn claim_auto(&self, actor: &str) -> Result<ClaimResult> {
    // ...
    match self.run_bf_claim(actor).await {
        Ok(stdout) => {
            // Parse response and return ClaimResult::Claimed
        }
        Err(e) => {
            // Fallback to br-style claim
        }
    }
}
```

**Path:**
1. Worker main loop (`src/worker/mod.rs:1070`)
2. → `Claimer::claim_auto()` (`src/claim/mod.rs:412`)
3. → `BeadStore::claim_auto()` (`src/bead_store/mod.rs:1282`)
4. → `run_bf_claim()` (`src/bead_store/mod.rs:839`)

### 2. Remote Workspace Claims

**File:** `src/worker/mod.rs:1174-1216`
```rust
fn switch_store_to(&mut self, workspace: &std::path::Path) -> Result<()> {
    // Resolve metadata from adapter
    let adapter = self.dispatcher.adapter(&self.config.agent.default);
    let model = adapter.and_then(|a| a.model.clone());
    let harness = adapter.and_then(|a| a.harness.clone())
        .or_else(|| Some("needle".to_string()));
    let harness_version = adapter.and_then(|a| a.harness_version.clone())
        .or_else(|| Some(env!("CARGO_PKG_VERSION").to_string()));

    let remote_store = Arc::new(
        crate::bead_store::BrCliBeadStore::discover(
            workspace.to_path_buf(),
            model,
            harness,
            harness_version,
        )?
    );
    // ... rebuild Claimer with new store
}
```

This ensures remote-workspace claims carry the same velocity-scoring metadata as home-workspace claims.

### 3. Test/Verification Call Sites

**File:** `examples/test_bf_write_concurrency.rs`
```rust
fn run_bf_claim(
    bf_path: &Path,
    workspace: &Path,
    bead_id: &str,
    assignee: &str,
    iteration: usize,
) {
    // Test implementation for concurrency verification
}
```

---

## Metadata Sources and Flow

### Current Metadata Availability

| Location | Source | model | harness | harness_version |
|----------|--------|-------|---------|-----------------|
| **CLI init** (`cli/mod.rs:873`) | Static fallback | `None` | `"needle"` | `env!("CARGO_PKG_VERSION")` |
| **Worker remote** (`worker/mod.rs:1181-1188`) | Adapter resolution | `adapter.model` | `adapter.harness` \| `"needle"` | `adapter.harness_version` \| `CARGO_PKG_VERSION` |
| **Supervisor** (`supervisor/mod.rs:86-91`) | Static fallback | `None` | `"needle"` | `env!("CARGO_PKG_VERSION")` |

### Metadata Flow Path

```
Config/Adapter ──► BrCliBeadStore::discover() ──► BrCliBeadStore fields ──► run_bf_claim() ──► bf claim CLI
                    (metadata passed as              (stored in              (reads from              (CLI args
                     constructor args)                struct fields)           struct fields)            --model/--harness/--harness-version)
```

---

## Intermediate Functions Between Metadata and run_bf_claim

### Functions that Already Have Metadata Access (No changes needed):

1. **`BrCliBeadStore::discover()`** (`src/bead_store/mod.rs:473-497`)
   - **Signature:** Already accepts `model`, `harness`, `harness_version`
   - **Status:** ✅ Complete - metadata flows through constructor

2. **`BrCliBeadStore::new()`** (`src/bead_store/mod.rs:448-465`)
   - **Signature:** Already accepts `model`, `harness`, `harness_version`
   - **Status:** ✅ Complete - stores in struct fields

3. **`BrCliBeadStore::run_bf_claim()`** (`src/bead_store/mod.rs:839-909`)
   - **Status:** ✅ Complete - reads from `self.model`, `self.harness`, `self.harness_version`

4. **`Worker::switch_store_to()`** (`src/worker/mod.rs:1174-1216`)
   - **Status:** ✅ Complete - resolves metadata from adapter and passes to `BrCliBeadStore::discover()`

### Functions That Create BrCliBeadStore (Already thread metadata):

1. **CLI main** (`cli/mod.rs:873`) - ✅ Threads metadata
2. **Supervisor** (`supervisor/mod.rs:86`) - ✅ Threads metadata  
3. **Worker remote** (`worker/mod.rs:1191`) - ✅ Threads metadata

---

## Key Finding: Metadata Flow is Already Complete

**The metadata threading from entry points to `run_bf_claim` is already fully implemented:**

1. ✅ Entry points (CLI, Supervisor, Worker) call `BrCliBeadStore::discover()` with metadata
2. ✅ `BrCliBeadStore` stores metadata in struct fields  
3. ✅ `run_bf_claim()` reads metadata from struct fields and builds CLI args
4. ✅ Remote workspace claims resolve adapter metadata and thread it through

**The per-task issue `bf-11i6pf` title "Add --model/--harness/--harness-version flags to bf claim call" appears to refer to work that is already completed.** The code at `src/bead_store/mod.rs:854-865` already adds these flags:

```rust
// Build the claim args. Velocity-aware scoring metadata is passed
// BEFORE --assignee/--json; missing values are simply omitted.
let mut args: Vec<&str> = Vec::with_capacity(10);
args.push("claim");
if let Some(model) = &self.model {
    args.push("--model");
    args.push(model.as_str());
}
if let Some(harness) = &self.harness {
    args.push("--harness");
    args.push(harness.as_str());
}
if let Some(harness_version) = &self.harness_version {
    args.push("--harness-version");
    args.push(harness_version.as_str());
}
args.push("--assignee");
args.push(actor);
args.push("--json");
```

---

## Related Bead Context

Based on bead `bf-2cnq0g` findings mentioned in the task, the metadata threading verification has been completed. This analysis confirms:

1. **All call sites identified** - `run_bf_claim` is only called from `BrCliBeadStore::claim_auto()`
2. **Call chain documented** - Complete path from worker → claimer → store → `run_bf_claim`
3. **Metadata availability confirmed** - Three metadata sources identified (CLI, adapter, fallbacks)
4. **No signature updates needed** - All intermediate functions already support metadata threading

---

## Additional References

- `bf-2cnq0g`: Metadata threading verification (mentioned in task context)
- `bf-11i6pf`: Per-task issue for adding CLI flags (already implemented)
- `src/bead_store/mod.rs:429-443`: `BrCliBeadStore` struct definition with metadata fields
- `src/bead_store/mod.rs:854-865`: CLI argument building with metadata flags

---

## Conclusion

The `run_bf_claim` call chain and metadata flow analysis reveals that:

1. **Single call site:** `run_bf_claim` is only called from `BrCliBeadStore::claim_auto()`
2. **Clear call chain:** Worker → Claimer → BeadStore trait → BrCliBeadStore → run_bf_claim
3. **Complete metadata threading:** All entry points thread metadata through to `bf claim` CLI args
4. **No blocking issues:** The implementation referenced in related beads appears to be complete

The metadata flow from adapter resolution through `BrCliBeadStore` creation to `bf claim` CLI invocation is fully implemented and functional.