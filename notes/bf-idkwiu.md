# NEEDLE Claim-Related and Metadata Tests Inventory

## Overview

This document catalogs all tests in NEEDLE that relate to claim operations and metadata flow (worker metadata, heartbeat data, telemetry events).

---

## Claim Operation Tests

### 1. Property-Based Claim Tests (`tests/property_tests.rs`)

| Test Function | Line | Validates |
|--------------|------|-----------|
| `claim_exclusion_filters_correctly` | ~227 | Exclusion set correctly filters out beads; after adding a bead ID to exclusion set, candidate list must not contain that bead |
| `claim_exclusion_set_no_inflation` | ~254 | Exclusion set grows correctly; adding N distinct IDs results in N exclusions with no duplicate inflation |

### 2. Real br CLI Integration Tests (`tests/real_br_integration_tests.rs`)

| Test Function | Line | Validates |
|--------------|------|-----------|
| `real_br_multi_worker_claiming_no_duplicates` | ~206 | Sequential claiming by 5 workers from shared bead list; verifies each worker skips already-claimed beads and picks unique bead |
| `real_br_all_beads_eventually_claimed` | ~267 | All beads across different priorities get claimed exactly once by sequential workers |
| `real_br_crashed_worker_bead_released_by_peer` | ~321 | Peer monitor detects stale heartbeat + dead PID, releases bead back to unassigned state |
| `test_concurrent_claim_exclusivity` | ~1523 | **Helper function**: N concurrent workers attempt to claim the same bead via flock serialization |
| `real_br_property_3_concurrent_claim_exclusivity_n2` | ~1608 | 2 workers racing to claim same bead; exactly 1 succeeds via flock |
| `real_br_property_3_concurrent_claim_exclusivity_n5` | ~1613 | 5 workers racing to claim same bead; exactly 1 succeeds via flock |
| `real_br_property_3_concurrent_claim_exclusivity_n20` | ~1618 | 20 workers racing to claim same bead; exactly 1 succeeds via flock |

### 3. Phase 2 Integration Tests (`tests/p2_integration_tests.rs`)

| Test Function | Line | Validates |
|--------------|------|-----------|
| `multi_worker_claiming_no_duplicates` | ~293 | 5 beads, 5 concurrent claimers with mock store; each bead claimed exactly once, no duplicates |
| `multi_worker_all_beads_eventually_claimed` | ~346 | 3 beads, 3 workers; all beads claimed by someone |
| `crashed_worker_bead_released_by_peer` | ~400 | Simulate crashed worker (stale heartbeat + dead PID); peer monitor detects and releases 1 bead |
| `flock_serializes_concurrent_claims_on_same_bead` | ~1080 | flock serializes concurrent claims on same bead |

### 4. Mock Store Implementation (`tests/p2_integration_tests.rs`)

| Component | Purpose |
|-----------|---------|
| `ConcurrentMockStore` | Thread-safe mock bead store for multi-worker tests; supports claim tracking and release counting |

---

## Worker/Metadata Tests

### 1. Heartbeat Metadata Fields (`tests/real_br_integration_tests.rs`)

Heartbeat metadata fields validated in tests:

| Field | Type | Description |
|-------|------|-------------|
| `worker_id` | String | Bare NATO name (e.g., "alpha", "foxtrot") |
| `qualified_id` | String | Fully-qualified identity: `{adapter}-{worker_id}` (e.g., "claude-code-glm-5-foxtrot") |
| `pid` | u32 | Worker process ID |
| `state` | WorkerState | Current worker state (Selecting, Executing, etc.) |
| `current_bead` | Option<BeadId> | Bead currently being processed |
| `workspace` | PathBuf | Path to workspace directory |
| `last_heartbeat` | DateTime<Utc> | Timestamp of last heartbeat |
| `started_at` | DateTime<Utc> | Worker start timestamp |
| `beads_processed` | u64 | Count of beads processed |
| `session` | String | Session identifier |
| `is_idle` | bool | Whether worker is idle |
| `current_task` | Option<String> | Current task identifier |
| **`model`** | **String** | **Model identifier (e.g., "claude", "test-model")** |

### 2. Worker Registry Metadata (`tests/real_br_integration_tests.rs`)

Registry worker entry fields:

| Field | Type | Description |
|-------|------|-------------|
| `workspace` | PathBuf | Path to workspace |
| `agent` | String | Agent adapter name (e.g., "test") |
| `model` | Option<String> | Model identifier |
| `provider` | Option<String> | Provider identifier (e.g., "anthropic") |
| `started_at` | DateTime<Utc> | Worker registration timestamp |
| `beads_processed` | u64 | Count of beads processed |

### 3. Metadata Usage in Tests

| Test | Metadata Validated |
|------|-------------------|
| `real_br_crashed_worker_bead_released_by_peer` | Heartbeat `model` field set to "test-model"; heartbeat serialization/deserialization |
| `real_br_mend_cleans_stale_claims_and_orphaned_locks` | Heartbeat with qualified_id, model, session fields; peer monitoring uses qualified_id |
| `real_br_provider_concurrency_limit_enforced` | WorkerEntry `provider` field; registry tracks provider-level concurrency |
| `otlp_integration_telemetry_end_to_end` | Telemetry events trace worker metadata through bead lifecycle |

---

## Telemetry/Event Tests

### 1. OTLP Integration Tests (`tests/otlp_integration.rs`)

| Test Function | Line | Validates |
|--------------|------|-----------|
| `otlp_integration_telemetry_end_to_end` | ~643 | End-to-end telemetry flow; validates events are NOT double-exported (spans vs logs) |
| `otlp_integration_drop_path` | ~966 | Graceful handling when OTLP collector unavailable |

Telemetry events validated:

| Event Type | Export Format | Description |
|-----------|---------------|-------------|
| `bead.claim.attempted` | **Span only** | Claim attempt - NOT exported as log |
| `agent.dispatched` | **Span only** | Agent dispatch - NOT exported as log |
| `strand.evaluated` | **Span only** | Strand evaluation - NOT exported as log |
| `bead.completed` | **Span only** | Bead completion - NOT exported as log |
| `heartbeat.emitted` | **Span event** | Heartbeat intra-span event - NOT exported as log |
| `worker.started` | Log | Worker started event |
| `worker.stopped` | Log | Worker stopped event |

### 2. Heartbeat Property Tests (`tests/property_tests.rs`)

| Test Function | Line | Validates |
|--------------|------|-----------|
| `heartbeat_fresh_is_never_stale` | ~277 | Fresh heartbeat emitted "now" is never stale for any positive TTL |
| `heartbeat_stale_past_ttl` | Property-based | Heartbeat is stale iff age exceeds TTL; monotonic staleness check |

### 3. Worker Metadata Threading Tests

**Note**: The following tests validate that worker metadata (model, provider, qualified_id) flows through:

- **Heartbeat serialization** - HeartbeatData contains `model` field
- **Registry registration** - WorkerEntry contains `agent`, `model`, `provider` fields  
- **Peer monitoring** - Peer monitor reads heartbeat metadata including `model`, `qualified_id`
- **Mend strand** - Uses heartbeat metadata to identify crashed workers
- **Telemetry events** - Worker metadata attached to telemetry spans/logs

---

## Workspace Fixtures

### Workspace States for Claim Testing (`tests/workspace_fixtures.rs`)

| Scenario | Purpose |
|----------|---------|
| `all_dead_workspaces` | No claimable candidates (all workspaces dead) |
| `all_alive_workspaces` | All workspaces have claimable candidates |
| `mixed_dead_alive_workspaces` | Mix of dead/alive workspaces for discovery |
| `CandidateState::Claimable` | Bead available to be claimed |
| `CandidateState::Assigned` | Bead already assigned to worker |
| `CandidateState::Excluded` | Bead excluded from consideration |

---

## Summary

**Total claim-related test functions identified: 13**
**Total metadata/telemetry test functions identified: 5+**

All tests validate:
1. **Claim exclusivity** - Exactly one worker can claim a given bead
2. **No duplicate claims** - Workers skip already-claimed beads via exclusion sets
3. **Concurrent claim serialization** - flock ensures atomic claim operations
4. **Stale claim cleanup** - Peer monitoring detects crashed workers via stale heartbeats
5. **Metadata threading** - Worker metadata (model, provider, qualified_id) flows through heartbeat → registry → peer monitoring → telemetry
6. **Telemetry correctness** - Events exported as spans, not double-exported as logs
