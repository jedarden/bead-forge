# NEEDLE Model/Harness Metadata Sources

## Task
Locate where the worker's model/harness/harness-version metadata is already known in NEEDLE.

## Findings

### 1. Adapter Configuration Files
**File:** `/home/coding/NEEDLE/src/dispatch/mod.rs` (lines 172-221, 256-305)

**Structure:**
```rust
pub struct AgentAdapter {
    pub name: String,           // e.g., "claude-sonnet"
    pub agent_cli: String,
    pub provider: Option<String>, // e.g., "anthropic"
    pub model: Option<String>,    // e.g., "claude-sonnet-4-6"
    // ... other fields
    // NOTE: No harness/harness_version fields!
}
```

**Built-in adapter example:**
```rust
fn builtin_claude_sonnet() -> AgentAdapter {
    AgentAdapter {
        name: "claude-sonnet".to_string(),
        agent_cli: "claude".to_string(),
        provider: Some("anthropic".to_string()),
        model: Some("claude-sonnet-4-6".to_string()),
        // ...
    }
}
```

### 2. Worker Initialization
**File:** `/home/coding/NEEDLE/src/worker/mod.rs` (lines 407-532)

**Key variables:**
- `self.dispatcher: Dispatcher` - Holds `HashMap<String, AgentAdapter>`
- `self.config.agent.default: String` - Default adapter name

**Adapter lookup:**
```rust
self.dispatcher.adapter(&adapter_name) -> Option<&AgentAdapter>
```

### 3. Metadata Extraction Point
**File:** `/home/coding/NEEDLE/src/worker/mod.rs` (lines 1174-1211)

**Function:** `switch_store_to()` - called when switching to a remote workspace

```rust
// Extract model from adapter (dynamic)
let model = self
    .dispatcher
    .adapter(&self.config.agent.default)
    .and_then(|a| a.model.clone());

// HARDCODED values:
let harness = Some("needle".to_string());
let harness_version = Some(env!("CARGO_PKG_VERSION").to_string());

let remote_store = Arc::new(
    crate::bead_store::BrCliBeadStore::discover(
        workspace.to_path_buf(),
        model,              // ← from adapter
        harness,            // ← hardcoded "needle"
        Some(harness_version), // ← hardcoded CARGO_PKG_VERSION
    )
    .context("failed to create bead store for remote workspace")?,
);
```

### 4. BeadStore and run_bf_claim
**File:** `/home/coding/NEEDLE/src/bead_store/mod.rs`

**BrCliBeadStore struct** (lines 429-445):
```rust
pub struct BrCliBeadStore {
    pub br_path: PathBuf,
    pub workspace: PathBuf,
    /// Model name for velocity-aware claim scoring
    pub model: Option<String>,
    /// Harness name for velocity-aware claim scoring
    pub harness: Option<String>,
    /// Harness version for velocity-aware claim scoring
    pub harness_version: Option<String>,
}
```

**run_bf_claim function** (lines 839-909):
```rust
async fn run_bf_claim(&self, actor: &str) -> Result<String> {
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
    // ... rest of implementation
}
```

## Complete Data Flow

1. **Config loading**: `ConfigLoader::load_resolved()` loads `agent.adapters_dir` and `agent.default`
2. **Dispatcher initialization**: `Dispatcher::new()` loads adapters from YAML + built-ins
3. **Adapter lookup**: `dispatcher.adapter(&config.agent.default)` → `AgentAdapter` with `model` field
4. **Workspace switching**: `switch_store_to()` extracts `adapter.model`, hardcodes `harness` and `harness_version`
5. **BeadStore construction**: `BrCliBeadStore::discover()` receives all three metadata values
6. **bf claim execution**: `run_bf_claim()` builds CLI args with `--model`, `--harness`, `--harness-version`

## Key Insight

**Current state: Partially implemented**

| Metadata | Source | Status |
|----------|--------|--------|
| `model` | `AgentAdapter.model` (from YAML/built-in) | ✅ Extracted from adapter |
| `harness` | Hardcoded `"needle"` | ❌ Not in adapter struct |
| `harness_version` | Hardcoded `env!("CARGO_PKG_VERSION")` | ❌ Not in adapter struct |

**Missing link:** The `AgentAdapter` struct lacks `harness` and `harness_version` fields. To support fully dynamic harness metadata (e.g., for non-needle harnesses), these fields should be:
1. Added to `AgentAdapter` struct
2. Populated during adapter loading (from YAML or built-ins)
3. Extracted in `switch_store_to()` instead of hardcoding
