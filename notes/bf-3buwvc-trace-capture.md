# Trace File Output Configuration

**Bead:** bf-3buwvc  
**Date:** 2026-07-25  
**Status:** ✅ Complete

## Trace Directory Structure

```
.beads/traces/
├── bf-{bead_id}-{timestamp}/
│   ├── metadata.json       # Structured metadata about the test run
│   ├── cargo-test.log     # Full cargo test output
│   ├── stdout.txt         # Standard output (JSONL format)
│   └── stderr.txt         # Standard error output
```

## Trace Directory Naming Convention

- **Format:** `bf-{bead_id}-{timestamp}`
- **Timestamp format:** `YYYYMMDD-HHMMSS` (UTC)
- **Example:** `bf-3buwvc-20260725-043050`

## Trace Directory Permissions

- **Permissions:** `775` (drwxrwxr-x)
- **Owner:** `coding:users`
- **Location:** `.beads/traces/` (relative to workspace root)
- **Write access:** ✅ Verified writable

## metadata.json Structure

```json
{
  "bead_id": "bf-3buwvc",
  "agent": "claude-code-glm-4.7-h1-bforge",
  "provider": "zai",
  "model": "glm-4.7",
  "exit_code": 0,
  "outcome": "success|failed",
  "duration_ms": 420,
  "input_tokens": null,
  "output_tokens": null,
  "cost_usd": null,
  "captured_at": "2026-07-25T04:30:50.123456789Z",
  "trace_format": "cargo_test",
  "pruned": false,
  "template_version": "1.0"
}
```

## Standard Cargo Test Capture Command

```bash
#!/bin/bash
# Trace file capture for cargo test execution

BEAD_ID="bf-{bead_id}"
TIMESTAMP=$(date -u +"%Y%m%d-%H%M%S")
TRACE_DIR=".beads/traces/${BEAD_ID}-${TIMESTAMP}"
mkdir -p "$TRACE_DIR"

# Start timer
START_TIME=$(date +%s%3N)

# Create initial metadata.json
cat > "$TRACE_DIR/metadata.json" <<EOF
{
  "bead_id": "$BEAD_ID",
  "agent": "claude-code-glm-4.7-h1-bforge",
  "provider": "zai",
  "model": "glm-4.7",
  "exit_code": null,
  "outcome": "pending",
  "duration_ms": null,
  "input_tokens": null,
  "output_tokens": null,
  "cost_usd": null,
  "captured_at": "$(date -u +"%Y-%m-%dT%H:%M:%S.%NZ")",
  "trace_format": "cargo_test",
  "pruned": false,
  "template_version": "1.0"
}
EOF

# Run cargo test with output capture
cargo test --no-fail-fast --message-format short > "$TRACE_DIR/cargo-test.log" 2>&1
EXIT_CODE=$?

# End timer
END_TIME=$(date +%s%3N)
DURATION_MS=$((END_TIME - START_TIME))

# Update metadata with results
OUTCOME=$([ $EXIT_CODE -eq 0 ] && echo "success" || echo "failed")
jq --arg exit_code "$EXIT_CODE" \
   --arg outcome "$OUTCOME" \
   --arg duration_ms "$DURATION_MS" \
   '.exit_code = ($exit_code | tonumber) | .outcome = $outcome | .duration_ms = ($duration_ms | tonumber)' \
   "$TRACE_DIR/metadata.json" > "$TRACE_DIR/metadata.json.tmp" && \
   mv "$TRACE_DIR/metadata.json.tmp" "$TRACE_DIR/metadata.json"

echo "✅ Test capture completed: $TRACE_DIR"
echo "Exit code: $EXIT_CODE, Outcome: $OUTCOME, Duration: ${DURATION_MS}ms"
```

## Quick One-Liner Version

```bash
BEAD_ID="bf-3buwvc" && TS=$(date -u +"%Y%m%d-%H%M%S") && TRACE=".beads/traces/${BEAD_ID}-${TS}" && mkdir -p "$TRACE" && START=$(date +%s%3N) && cargo test --no-fail-fast > "$TRACE/cargo-test.log" 2>&1 && EXIT=$? && DURATION=$(($(date +%s%3N)-START)) && echo "{\"bead_id\":\"$BEAD_ID\",\"exit_code\":$EXIT,\"outcome\":\"$([ $EXIT -eq 0 ] && echo success || echo failed)\",\"duration_ms\":$DURATION,\"captured_at\":\"$(date -u -Iseconds)\"}" | jq '.' > "$TRACE/metadata.json" && echo "Captured to: $TRACE"
```

## Exit Code Meanings

- **0:** All tests passed
- **101:** Tests failed or compilation errors
- **1/Other:** Other errors (timeout, signal, etc.)

## Usage Examples

### Standard test capture
```bash
bash -c 'BEAD_ID="bf-3buwvc" && TS=$(date -u +"%Y%m%d-%H%M%S") && TRACE=".beads/traces/${BEAD_ID}-${TS}" && mkdir -p "$TRACE" && cargo test --no-fail-fast > "$TRACE/cargo-test.log" 2>&1 && echo "{\"bead_id\":\"$BEAD_ID\",\"captured_at\":\"$(date -u -Iseconds)\"}" | jq . > "$TRACE/metadata.json"'
```

### With custom cargo test options
```bash
cargo test --verbose --no-fail-fast -- --test-threads=1 > .beads/traces/bf-custom-$(date -u +"%Y%m%d-%H%M%S")/cargo-test.log 2>&1
```

## Verification

✅ Trace directory permissions verified (775)  
✅ Directory creation tested  
✅ Cargo test output capture verified  
✅ Metadata.json generation tested  
✅ Exit code and duration tracking confirmed  

## Related Files

- **Trace directory:** `.beads/traces/`
- **Example trace:** `.beads/traces/bf-4ohbrj/` (existing)
- **Test traces created:** 
  - `.beads/traces/bf-3buwvc-test-20260725-043028/`
  - `.beads/traces/bf-3buwvc-final-20260725-043050/`
