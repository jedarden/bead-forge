# Bead bf-2a7g0x: Update run_bf_claim signature

## Status: Already Implemented

The work requested in this bead was already completed in a previous implementation.

## Current State

The `run_bf_claim` function in `src/claim.rs` (lines 690-709) already has the exact signature requested:

```rust
pub fn run_bf_claim(
    tx: &Connection,
    worker: &str,
    claim_ttl_minutes: i64,
    now: DateTime<Utc>,
    model: Option<String>,
    harness: Option<String>,
    harness_version: Option<String>,
) -> Result<Option<ClaimResult>>
```

## Verification

- ✅ Function accepts model, harness, and harness_version parameters
- ✅ Parameters are properly typed as `Option<String>`
- ✅ Function compiles successfully (cargo build passes)
- ✅ No breaking changes to return type or core behavior
- ✅ Function properly constructs WorkerMetadata from the parameters
- ✅ Delegates to core claim() function correctly

## Conclusion

The bead's acceptance criteria are fully met by the existing implementation.
