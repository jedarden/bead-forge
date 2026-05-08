# NEEDLE Label Operations Analysis (bf-1fw)

## Task
Verify NEEDLE uses `bf labels` instead of `br show + label add` loops.

## Findings

### 1. NEEDLE's `labels()` is inefficient

**Location:** `/home/coding/NEEDLE/src/bead_store/mod.rs`

Both `BrStore` (line 671-676) and `BfStore` (line 1157-1160) implement `labels()` as:

```rust
async fn labels(&self, id: &BeadId) -> Result<Vec<String>> {
    let bead = self.show(id).await?;
    Ok(bead.labels)
}
```

This is wasteful because `show()` does a full SELECT on all tables (issues, dependencies, labels, comments, events) when we only need labels.

### 2. `bf labels <id> --json` exists and is efficient

**Location:** `/home/coding/bead-forge/src/cli/mod.rs:1459-1471`

```rust
fn cmd_labels(beads_dir: &PathBuf, id: &str, format: &str) -> Result<()> {
    let storage = Storage::open(&db_path)?;
    let labels = storage.get_labels(id)?;  // Direct SELECT
    // ... output as JSON array
}
```

The underlying storage call is a direct SELECT:
```rust
// src/storage/sqlite.rs:831-833
pub fn get_labels(&self, issue_id: &str) -> Result<Vec<String>> {
    self.load_labels(issue_id)  // SELECT label FROM labels WHERE issue_id = ?
}
```

**Verified output format:**
```bash
$ bf labels needle-xeh --format json
["failure-count:1"]
```

### 3. `bf label add` supports batching

The `bf label add` command accepts multiple labels:
```bash
bf label add <ID> --label label1 --label label2 --label label3
```

**Location:** `/home/coding/bead-forge/src/cli/mod.rs:521-531`

```rust
/// Label(s) to add (multiple labels supported)
#[arg(short = 'l', long, value_name = "LABEL")]
labels: Vec<String>,
```

### 4. NEEDLE does NOT batch label adds

**Location:** `/home/coding/NEEDLE/src/bead_store/mod.rs:678-684` (BrStore)
**Location:** `/home/coding/NEEDLE/src/bead_store/mod.rs:1162-1168` (BfStore)

```rust
async fn add_label(&self, id: &BeadId, label: &str) -> Result<()> {
    self.run_br(&["label", "add", id_str, label]).await?  // Single label per call
    Ok(())
}
```

Each call spawns a new process. To add 3 labels, NEEDLE spawns 3 processes.

## Recommendations

### Fix 1: Use `bf labels` instead of `show()`

Change both `BrStore` and `BfStore` `labels()` implementations to call the `labels` subcommand directly:

```rust
// For BfStore:
async fn labels(&self, id: &BeadId) -> Result<Vec<String>> {
    let id_str = id.as_ref();
    let stdout = self.run_bf(&["labels", id_str, "--format", "json"]).await?;
    let labels: Vec<String> = serde_json::from_str(&stdout)?;
    Ok(labels)
}

// For BrStore - keep using show() for now since br doesn't have `labels` subcommand
// Or add the subcommand to br if needed
```

### Fix 2: Add batch label operations to NEEDLE

Add a new trait method and batch implementation:

```rust
// In BeadStore trait:
async fn add_labels(&self, id: &BeadId, labels: &[&str]) -> Result<()> {
    // Default: call add_label for each (inefficient but works)
    for label in labels {
        self.add_label(id, label).await?;
    }
    Ok(())
}

// In BfStore - override with efficient implementation:
async fn add_labels(&self, id: &BeadId, labels: &[&str]) -> Result<()> {
    if labels.is_empty() {
        return Ok(());
    }
    let id_str = id.as_ref();
    let mut args = vec!["label", "add", id_str];
    for label in labels {
        args.extend(&["--label", label]);
    }
    self.run_bf(&args).await?;
    Ok(())
}
```

## Verification

### `bf labels` works correctly:
```bash
$ cd /home/coding/NEEDLE && /home/coding/bead-forge/target/debug/bf labels needle-xeh
failure-count:1

$ /home/coding/bead-forge/target/debug/bf labels needle-xeh --format json
["failure-count:1"]
```

### `bf label add` supports multiple labels:
```bash
$ /home/coding/bead-forge/target/debug/bf label add needle-qlz --label test-label1 --label test-label2
Added label 'test-label1' to needle-qlz
Added label 'test-label2' to needle-qlz
```

## Impact

- **Current**: Each label read spawns a `br/bf show` process (full DB scan)
- **Current**: Each label add spawns a separate `br/bf label add` process
- **Proposed**: Label reads use direct SELECT on labels table
- **Proposed**: Multiple labels added in single process spawn

## Files Referenced

- `/home/coding/NEEDLE/src/bead_store/mod.rs` - NEEDLE's BeadStore implementations
- `/home/coding/bead-forge/src/cli/mod.rs:1459-1471` - bf labels command
- `/home/coding/bead-forge/src/cli/mod.rs:521-531` - bf label add (batch support)
- `/home/coding/bead-forge/src/storage/sqlite.rs:831-833` - get_labels storage method
