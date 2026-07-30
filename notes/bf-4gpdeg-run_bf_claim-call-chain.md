# run_bf_claim Call Chain and Metadata Flow Analysis

**Bead:** bf-4gpdeg  
**Date:** 2026-07-24  
**Purpose:** Document call sites, call chain, and metadata availability for threading model/harness/harness-version

---

## 1. Direct Call Sites for `run_bf_claim`

### Primary Implementation
- **File:** `/home/coding/NEEDLE/src/bead_store/mod.rs:839`
- **Definition:** `async fn run_bf_claim(&self, actor: &str) -> Result<String>`
- **Context:** Method on `BrCliBeadStore` struct

### Callers
1. **`BrCliBeadStore::claim_auto`** (line 1287)
   - Path: `/home/coding/NEEDLE/src/bead_store/mod.rs:1287`
   - Pattern: `match self.run_bf_claim(actor).await`

### Test/Example Code
- **File:** `/home/coding/NEEDLE/examples/test_bf_write_concurrency.rs:180`
- **Note:** This is a standalone test helper function, NOT the main implementation

---

## 2. Complete Call Chain

### Entry Point: CLI Worker Launch
```
CLI (run_worker in cli/mod.rs:830)
  ↓
Phase 1: Bead Store Discovery (line 871)
  ↓
BfCliBeadStore::discover(
  workspace,
  model: None,                    // ← Metadata source (CLI)
  harness: Some("needle"),        // ← Metadata source (CLI)
  harness_version: Some(CARGO_PKG_VERSION)  // ← Metadata source (CLI)
) (bead_store/mod.rs:473)
  ↓
Worker::new_with_telemetry(config, worker_name, store) (cli/mod.rs:954)
  ↓
Worker::build (worker/mod.rs:407)
  ↓
Worker::run() (worker/mod.rs:524)
  ↓
Worker::claim_beat() (worker/mod.rs:1066)
  ↓
Claimer::claim_auto(actor) (claim/mod.rs:412)
  ↓
BrCliBeadStore::claim_auto(actor) (bead_store/mod.rs:1282)
  ↓
BrCliBeadStore::run_bf_claim(actor) (bead_store/mod.rs:839)  ← TARGET
```

### Entry Point: Supervisor
```
Supervisor::new() (supervisor/mod.rs:83)
  ↓
BrCliBeadStore::discover(
  workspace,
  model: None,                    // ← Metadata source (Supervisor)
  harness: Some("needle"),        // ← Metadata source (Supervisor)
  harness_version: Some(CARGO_PKG_VERSION)  // ← Metadata source (Supervisor)
) (bead_store/mod.rs:86)
  ↓
Supervisor.run() loop (supervisor/mod.rs:117)
  ↓
[Spawns worker subprocess with same metadata]
```

### Entry Point: Worker Remote Workspace Switch
```
Worker::switch_store_to(workspace) (worker/mod.rs:1174)
  ↓
Resolve adapter metadata (lines 1181-1188):
  adapter = dispatcher.adapter(&config.agent.default)
  model = adapter.model           // ← Metadata source (Adapter)
  harness = adapter.harness       // ← Metadata source (Adapter)
  harness_version = adapter.harness_version  // ← Metadata source (Adapter)
  ↓
BrCliBeadStore::discover(workspace, model, harness, harness_version)
  ↓
Claims use new store → run_bf_claim with adapter metadata
```

---

## 3. Metadata Availability

### Metadata Sources

#### A. Built-in Adapters (dispatch/mod.rs)
**Location:** `/home/coding/NEEDLE/src/dispatch/mod.rs`

Each built-in adapter has metadata defined:
```rust
fn builtin_claude_sonnet() -> AgentAdapter {
    AgentAdapter {
        model: Some("claude-sonnet-4-6".to_string()),
        harness: Some("needle".to_string()),
        harness_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        // ...
    }
}
```

**Built-in adapters with metadata:**
- `claude-sonnet`: model="claude-sonnet-4-6", harness="needle"
- `claude-opus`: model="claude-opus-4-6", harness="needle"
- `opencode`: model=None, harness="needle"
- `codex`: model="gpt-5.6-terra", harness="needle"

#### B. BrCliBeadStore Struct Fields
**Location:** `/home/coding/NEEDLE/src/bead_store/mod.rs:429-444`

```rust
pub struct BrCliBeadStore {
    pub br_path: PathBuf,
    pub workspace: PathBuf,
    pub model: Option<String>,          // Passed to bf claim --model
    pub harness: Option<String>,         // Passed to bf claim --harness
    pub harness_version: Option<String>, // Passed to bf claim --harness-version
}
```

#### C. CLI Initialization
**Location:** `/home/coding/NEEDLE/src/cli/mod.rs:873-877`

```rust
let bf_store = crate::bead_store::BfCliBeadStore::discover(
    config.workspace.default.clone(),
    None,                       // model: do not filter by model
    Some("needle".to_string()), // harness
    Some(env!("CARGO_PKG_VERSION").to_string()), // harness_version
)
```

#### D. Supervisor Initialization
**Location:** `/home/coding/NEEDLE/src/supervisor/mod.rs:86-91`

```rust
BrCliBeadStore::discover(
    config.workspace.clone(),
    None,                       // model
    Some("needle".to_string()), // harness
    Some(env!("CARGO_PKG_VERSION").to_string()), // harness_version
)
```

#### E. Remote Workspace Switch
**Location:** `/home/coding/NEEDLE/src/worker/mod.rs:1181-1196`

```rust
// Resolve adapter metadata for velocity-aware scoring
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
    )
);
```

### Current Metadata Usage in run_bf_claim

**Location:** `/home/coding/NEEDLE/src/bead_store/mod.rs:850-868`

```rust
async fn run_bf_claim(&self, actor: &str) -> Result<String> {
    // ...
    let mut args: Vec<&str> = Vec::with_capacity(10);
    args.push("claim");
    
    // Velocity-aware scoring metadata passed BEFORE --assignee/--json
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
    // ...
}
```

---

## 4. Intermediate Functions Requiring Signature Updates

### Functions Between Metadata Source and `run_bf_claim`

**Note:** All intermediate functions ALREADY pass through the metadata correctly via `BrCliBeadStore` struct fields. No signature updates are required for metadata threading.

However, if dynamic metadata updates were needed (e.g., changing model mid-run), these would be the touch points:

1. **`BfCliBeadStore::new`** (bead_store/mod.rs:448)
   - Already accepts `model, harness, harness_version` parameters
   
2. **`BfCliBeadStore::discover`** (bead_store/mod.rs:473)
   - Already accepts `model, harness, harness_version` parameters

3. **`Worker::switch_store_to`** (worker/mod.rs:1174)
   - Already resolves adapter metadata and creates new store

4. **`BrCliBeadStore::claim_auto`** (bead_store/mod.rs:1282)
   - Already has access to `self.model, self.harness, self.harness_version`

5. **`Claimer::claim_auto`** (claim/mod.rs:412)
   - Calls `self.store.claim_auto(actor)` - metadata in store

### Current Implementation Status

✅ **Metadata threading is COMPLETE**
- Model, harness, and harness_version are stored in BrCliBeadStore
- CLI and Supervisor initialize with correct metadata
- Worker remote-switch resolves metadata from adapter
- run_bf_claim correctly uses the metadata in CLI args

---

## 5. Summary

### Call Chain Length
- **Depth:** ~8-10 function calls from CLI entry to `run_bf_claim`
- **Bottleneck:** All metadata flows through `BrCliBeadStore` struct fields

### Metadata Flow
1. **Static initialization** (CLI/Supervisor): metadata passed to `BfCliBeadStore::discover`
2. **Dynamic resolution** (Worker remote-switch): metadata resolved from adapter config
3. **Usage in claim:** `run_bf_claim` reads from `self.model/harness/harness_version`

### Key Insight
The metadata is ALREADY properly threaded through the call chain via the `BrCliBeadStore` struct. The architecture is sound - metadata is set at store creation time and used in `run_bf_claim` without needing to pass through intermediate function signatures.

### Potential Enhancement Areas
If dynamic metadata updates were needed (e.g., per-request model selection):
1. Add `model/harness/harness_version` parameters to `claim_auto` methods
2. Thread through `Claimer::claim_auto` → `BeadStore::claim_auto` → `run_bf_claim`
3. Fall back to store defaults when not specified

---

## 6. File Reference Map

| Function | File | Line | Notes |
|----------|------|------|-------|
| `run_bf_claim` | bead_store/mod.rs | 839 | Target function |
| `BrCliBeadStore::claim_auto` | bead_store/mod.rs | 1282 | Calls run_bf_claim |
| `BrCliBeadStore::discover` | bead_store/mod.rs | 473 | Store factory |
| `Claimer::claim_auto` | claim/mod.rs | 412 | Claims layer |
| `Worker::claim_beat` | worker/mod.rs | 1066 | Worker claim entry |
| `Worker::switch_store_to` | worker/mod.rs | 1174 | Remote workspace switch |
| `run_worker` | cli/mod.rs | 830 | CLI worker entry |
| `Supervisor::new` | supervisor/mod.rs | 83 | Supervisor entry |
| `AgentAdapter` | dispatch/mod.rs | 172 | Adapter with metadata |
