# Verification: list and ready Commands Emit Envelope

## Date: 2026-07-23

## Acceptance Criteria Met

✅ **list --json outputs wrapped in envelope**
- Modified `cmd_list` in `src/cli/mod.rs` to wrap JSON output in envelope
- Output structure: `{version: 1, kind: "list", data: [...]}`

✅ **ready --json outputs wrapped in envelope**
- Already implemented (lines 1833-1836 in cmd_ready)
- Output structure: `{version: 1, kind: "ready", data: [...]}`

✅ **Stable envelope shape with metadata fields**
- Both commands output consistent envelope structure:
  - `version`: 1 (envelope version)
  - `kind`: "list" or "ready" (command identifier)
  - `data`: array of issue objects
  - `warning`: optional (present only when auto-flush fails)

## Changes Made

### Modified: src/cli/mod.rs

Updated `cmd_list` function to wrap JSON output in envelope (lines 1615-1629):

```rust
match output_format {
    OutputFormat::Json => {
        // Wrap in envelope with kind="list"
        let json_array = serde_json::to_string(&issues).unwrap_or_else(|_| "[]".to_string());
        println!("{}", formatter.format_with_envelope("list", &json_array));
    }
    _ => {
        print!("{}", formatter.format_issues(&issues));
    }
}
```

This matches the existing implementation in `cmd_ready` and ensures consistent envelope wrapping across both commands.

## Verification Commands

```bash
# Test list command JSON output
./target/debug/bf list --json --limit 1

# Test ready command JSON output
./target/debug/bf ready --json --limit 1

# Both should output:
# {
#   "version": 1,
#   "kind": "list" or "ready",
#   "data": [...]
# }
```

## Build Status

✅ Compiles cleanly: `cargo build` succeeds with no errors
