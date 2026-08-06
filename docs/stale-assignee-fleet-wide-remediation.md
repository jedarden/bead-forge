# Fleet-Wide Stale Assignee Remediation Guide

## Executive Summary

When NEEDLE workers crash, timeout, or abandon work, they leave beads stuck in `in_progress` status with non-empty `assignee` fields. These beads become invisible to the fleet because `bf claim` excludes beads with assignees from the candidate pool. This guide provides operational procedures for detecting and remediating stale assignees across multiple workspaces.

**Impact:** Stale assignees create a phantom "blocked work" condition where beads appear claimed but are not actively being worked on, reducing fleet throughput.

**Solution:** Clear stale assignees using `bf update --clear-assignee` to return beads to the discoverable pool.

---

## Understanding the Problem

### How Stale Assignees Occur

1. **Worker Crash**: NEEDLE worker process killed (OOM, segfault, SIGKILL)
2. **Timeout**: External 600-second timeout kills worker (exit code 124)
3. **Network Partition**: Worker loses connectivity to workspace storage
4. **Manual Interruption**: Worker manually terminated during bead execution
5. **Deployment Error**: Worker pool deployed with incorrect configuration

### Why It Matters

- **Invisibility**: Beads with non-null `assignee` are excluded from `bf ready` output
- **Fleet Efficiency**: Stale claims reduce available work for active workers
- **Incorrect Status**: Beads appear `in_progress` despite no active work
- **Cascade Effects**: Blocked beads downstream cannot start when upstream is stuck

### Detection Timeline

| Severity | Time Threshold | Action Required |
|----------|---------------|-----------------|
| Minor | > 30 min past claim TTL | Monitor, manual review if persistent |
| Moderate | > 2 hours | Automatic detection, manual remediation |
| Severe | > 24 hours | Immediate remediation, investigate root cause |
| Critical | > 72 hours | Emergency procedures, post-incident analysis |

---

## Discovery Methods

### Method 1: Per-Workspace Statistics

Check each workspace for stale assignees:

```bash
# Show assignee distribution (high assignee count = potential stale assignments)
cd /home/coding/FORGE
bf stats --by-assignee

cd /home/coding/NEEDLE  
bf stats --by-assignee

# Repeat for each workspace
```

**Interpretation:** Assignees with >5 beads or beads in `in_progress` > claim TTL are candidates for remediation.

### Method 2: Fleet-Wide Scan

Scan all workspaces for beads stuck in `in_progress`:

```bash
#!/bin/bash
# fleet-stale-scan.sh - Scan all workspaces for stale assignees

WORKSPACES=(
  "/home/coding/FORGE"
  "/home/coding/NEEDLE"
  "/home/coding/AgentScribe"
  "/home/coding/ARMOR"
  "/home/coding/SIGIL"
  "/home/coding/CLASP"
  "/home/coding/bead-forge"
)

for workspace in "${WORKSPACES[@]}"; do
  echo "=== $workspace ==="
  if [ -d "$workspace/.beads" ]; then
    bf -w "$workspace" list --status in_progress --format json | \
      jq -r '.[] | "\(.id)\t\(.title)\t\(.assignee)\t\(.updated_at)"'
  fi
done
```

**Output interpretation:**
- Beads with `updated_at` older than claim TTL (default 30 min) are stale
- Cross-reference assignees with active worker list from monitoring

### Method 3: Assignee-Specific Search

Find all beads assigned to a known-dead worker:

```bash
# Find beads for specific worker across all workspaces
for workspace in /home/coding/{FORGE,NEEDLE,AgentScribe,ARMOR,SIGIL,CLASP,bead-forge}; do
  if [ -d "$workspace/.beads" ]; then
    echo "=== $workspace ==="
    bf -w "$workspace" list --assignee "dead-worker-X" --format json | \
      jq -r '.[] | .id'
  fi
done
```

### Method 4: Automated Detection

Set up a cron job to detect stale assignees daily:

```cron
# /etc/cron.d/bf-stale-detect
# Detect stale assignees daily at 9 AM
0 9 * * * root /usr/local/bin/bf-stale-detect.sh
```

```bash
#!/bin/bash
# /usr/local/bin/bf-stale-detect.sh
# Alert if beads are stuck > 2 hours

STALE_THRESHOLD_HOURS=2
ALERT_EMAIL="ops@example.com"

for workspace in /home/coding/{FORGE,NEEDLE,AgentScribe,ARMOR,SIGIL,CLASP,bead-forge}; do
  if [ -d "$workspace/.beads" ]; then
    STALE=$(bf -w "$workspace" recent --status in_progress --time-period "${STALE_THRESHOLD_HOURS}h" --format json | jq -r '.[].id')
    if [ -n "$STALE" ]; then
      echo "WARNING: Stale assignees detected in $workspace:" | mail -s "Stale assignees alert" "$ALERT_EMAIL"
      echo "$STALE" | mail -s "Stale beads in $workspace" "$ALERT_EMAIL"
    fi
  fi
done
```

---

## Remediation Procedures

### Single Workspace Remediation

**Scenario:** One workspace has stale assignees from crashed workers.

```bash
cd /home/coding/FORGE

# Step 1: Identify stale beads
bf list --status in_progress --format json | \
  jq --arg ttl_min "$(date -d '30 minutes ago' -u +%Y-%m-%dT%H:%M:%SZ)" \
  'map(select(.updated_at < $ttl_min))' > /tmp/stale-beads.json

# Step 2: Review the list
cat /tmp/stale-beads.json | jq -r '.[] | [.id, .assignee, .updated_at] | @tsv'

# Step 3: Clear assignees for each stale bead
cat /tmp/stale-beads.json | jq -r '.[].id' | while read bead_id; do
  echo "Clearing assignee for $bead_id"
  bf update "$bead_id" --clear-assignee
  bf comment "$bead_id" "Cleared stale assignee after worker crash detection"
done

# Step 4: Verify remediation
bf ready --format json | jq -r '.[].id' | grep -Ff <(cat /tmp/stale-beads.json | jq -r '.[].id')
```

### Fleet-Wide Worker-Specific Remediation

**Scenario:** Specific worker (e.g., `worker-cluster-7`) is confirmed dead; clear all its claims across all workspaces.

```bash
#!/bin/bash
# fleet-clear-worker.sh - Clear all beads assigned to a specific worker

WORKER_ID="$1"
if [ -z "$WORKER_ID" ]; then
  echo "Usage: $0 <worker-id>"
  exit 1
fi

WORKSPACES=(
  "/home/coding/FORGE"
  "/home/coding/NEEDLE"
  "/home/coding/AgentScribe"
  "/home/coding/ARMOR"
  "/home/coding/SIGIL"
  "/home/coding/CLASP"
  "/home/coding/bead-forge"
)

echo "Scanning for beads assigned to $WORKER_ID..."
total_cleared=0

for workspace in "${WORKSPACES[@]}"; do
  if [ -d "$workspace/.beads" ]; then
    echo "=== $workspace ==="
    bead_ids=$(bf -w "$workspace" list --assignee "$WORKER_ID" --format json | jq -r '.[].id')
    
    if [ -n "$bead_ids" ]; then
      echo "$bead_ids" | while read bead_id; do
        echo "  Clearing: $bead_id"
        bf -w "$workspace" update "$bead_id" --clear-assignee
        bf -w "$workspace" comment "$bead_id" "Cleared stale assignee from decommissioned worker $WORKER_ID"
        ((total_cleared++))
      done
    else
      echo "  No beads found"
    fi
  fi
done

echo "=== Summary ==="
echo "Cleared assignees from $total_cleared beads across fleet"
echo "Worker $WORKER_ID cleared from all workspaces"
```

**Usage:**
```bash
./fleet-clear-worker.sh worker-cluster-7
```

### Emergency Fleet-Wide Clear

**⚠️ CRITICAL:** Only use when entire worker pool is confirmed dead (e.g., cluster-wide outage).

```bash
#!/bin/bash
# fleet-emergency-clear.sh - EMERGENCY ONLY: Clear ALL in_progress beads
# Use this when the entire worker pool is dead (cluster reboot, network partition)

echo "=== EMERGENCY FLEET-WIDE CLEAR ==="
echo "This will clear ALL in_progress beads across ALL workspaces"
read -p "Type 'EMERGENCY' to confirm: " confirmation

if [ "$confirmation" != "EMERGENCY" ]; then
  echo "Aborted"
  exit 1
fi

WORKSPACES=(
  "/home/coding/FORGE"
  "/home/coding/NEEDLE"
  "/home/coding/AgentScribe"
  "/home/coding/ARMOR"
  "/home/coding/SIGIL"
  "/home/coding/CLASP"
  "/home/coding/bead-forge"
)

total_cleared=0

for workspace in "${WORKSPACES[@]}"; do
  if [ -d "$workspace/.beads" ]; then
    echo "=== $workspace ==="
    bead_ids=$(bf -w "$workspace" list --status in_progress --format json | jq -r '.[].id')
    
    if [ -n "$bead_ids" ]; then
      count=$(echo "$bead_ids" | wc -l)
      echo "  Clearing $count beads..."
      
      echo "$bead_ids" | while read bead_id; do
        bf -w "$workspace" update "$bead_id" --clear-assignee
        bf -w "$workspace" comment "$bead_id" "Emergency reclamation: fleet-wide worker pool crash at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
        ((total_cleared++))
      done
    fi
  fi
done

echo "=== EMERGENCY CLEAR COMPLETE ==="
echo "Cleared $total_cleared beads across fleet"
echo "IMMEDIATE ACTION REQUIRED:"
echo "1. Restart worker pool"
echo "2. Monitor for resumed claiming"
echo "3. Conduct post-incident analysis"
```

### Doctor-Based Remediation

**Preferred method:** Use `bf doctor --reclaim-stale` for time-based reclamation:

```bash
# Single workspace - reclaim beads stuck > 2 hours
cd /home/coding/FORGE
bf doctor --reclaim-stale --ttl 120

# Fleet-wide - apply to all workspaces
for workspace in /home/coding/{FORGE,NEEDLE,AgentScribe,ARMOR,SIGIL,CLASP,bead-forge}; do
  if [ -d "$workspace/.beads" ]; then
    echo "=== $workspace ==="
    bf -w "$workspace" doctor --reclaim-stale --ttl 120
  fi
done
```

**What `--reclaim-stale` does:**
- Finds beads in `in_progress` status older than TTL
- Resets status to `open`
- Clears assignee to `NULL`
- Records reclamation event in audit log

---

## Verification Steps

### Post-Remediation Verification

After clearing stale assignees, verify the remediation was successful:

#### 1. Confirm Assignees Cleared

```bash
# Verify specific bead has NULL assignee
bf show <bead-id> --format json | jq '.assignee'
# Expected output: null

# Verify all previously stale beads are cleared
cat /tmp/stale-beads.json | jq -r '.[].id' | while read id; do
  assignee=$(bf show "$id" --format json | jq '.assignee')
  if [ "$assignee" != "null" ]; then
    echo "ERROR: $id still has assignee $assignee"
  fi
done
```

#### 2. Confirm Bead Discoverability

```bash
# Bead should appear in ready list
bf ready --format json | jq -r '.[].id' | grep -q "<bead-id>" && echo "✓ Discoverable" || echo "✗ Not discoverable"

# Test that it can be claimed (dry-run)
bf claim --assignee "test-worker" --dry-run | grep -q "<bead-id>" && echo "✓ Claimable" || echo "✗ Not claimable"
```

#### 3. Confirm Fleet Operations Resumed

```bash
# Monitor for new claims on remediated beads
bf log --since 5m | grep -E "CLAIMED|ASSIGNEE_CHANGED"

# Verify worker pool is actively claiming
bf recent --status in_progress --time-period 10m | grep -q "bf-" && echo "✓ Workers claiming" || echo "✗ No active claiming"
```

#### 4. Data Integrity Check

```bash
# Run doctor to ensure no corruption
for workspace in /home/coding/{FORGE,NEEDLE,AgentScribe,ARMOR,SIGIL,CLASP,bead-forge}; do
  if [ -d "$workspace/.beads" ]; then
    echo "=== $workspace ==="
    bf -w "$workspace" doctor || echo "  ⚠️ Doctor found issues"
  fi
done
```

### Continuous Monitoring

Set up ongoing monitoring to prevent accumulation of stale assignees:

```bash
#!/bin/bash
# monitor-stale-assignees.sh - Continuous monitoring loop

while true; do
  echo "=== Stale assignee check at $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="
  
  for workspace in /home/coding/{FORGE,NEEDLE,AgentScribe,ARMOR,SIGIL,CLASP,bead-forge}; do
    if [ -d "$workspace/.beads" ]; then
      stale_count=$(bf -w "$workspace" list --status in_progress --format json | \
        jq --arg ttl "$(date -d '2 hours ago' -u +%Y-%m-%dT%H:%M:%SZ)" \
        'map(select(.updated_at < $ttl)) | length')
      
      if [ "$stale_count" -gt 0 ]; then
        echo "⚠️  $workspace: $stale_count stale beads"
      fi
    fi
  done
  
  sleep 300  # Check every 5 minutes
done
```

---

## Prevention Strategies

### 1. Worker Shutdown Handling

Ensure workers properly clear assignees on shutdown:

```bash
#!/bin/bash
# NEEDLE worker shutdown handler

trap 'echo "Worker shutting down..."; clear_my_assignees; exit 0' EXIT INT TERM

clear_my_assignees() {
  local my_id="worker-$(hostname)-$$"
  echo "Clearing assignees for $my_id"
  
  for workspace in /home/coding/{FORGE,NEEDLE,AgentScribe}; do
    if [ -d "$workspace/.beads" ]; then
      bf -w "$workspace" list --assignee "$my_id" --format json | \
        jq -r '.[].id' | xargs -I {} bf -w "$workspace" update {} --clear-assignee
    fi
  done
}

# Main worker loop
while true; do
  bead_id=$(bf claim --assignee "worker-$(hostname)-$$" --json | jq -r '.bead_id')
  # ... work on bead ...
done
```

### 2. Claim TTL Configuration

Adjust claim TTL in `.beads/config.yaml` to match your worker timeout:

```yaml
# .beads/config.yaml
claim_ttl_minutes: 30  # Should match external worker timeout
```

**Relationship to external timeout:**
- NEEDLE workers have 600-second (10-minute) external timeout
- Set `claim_ttl_minutes` to 10-15 minutes to detect worker failures quickly
- Lower TTL = faster stale detection = more frequent reclaims

### 3. Health Check Integration

Integrate with worker health monitoring:

```bash
#!/bin/bash
# health-check-with-reclaim.sh - Periodic health check with stale reclamation

WORKER_ID="worker-$(hostname)-$$"
LAST_ACTIVITY_FILE="/tmp/worker-$$.last-activity"

# Update activity timestamp on each claim
update_activity() {
  date +%s > "$LAST_ACTIVITY_FILE"
}

# Health check runs periodically
health_check() {
  local now=$(date +%s)
  local last_activity=$(cat "$LAST_ACTIVITY_FILE" 2>/dev/null || echo 0)
  local inactive_seconds=$((now - last_activity))
  
  # If inactive > 20 minutes, clear own assignees (self-healing)
  if [ "$inactive_seconds" -gt 1200 ]; then
    echo "Health check: Inactive for ${inactive_seconds}s, clearing own assignees"
    clear_my_assignees
    return 1
  fi
  
  return 0
}

# Main worker loop with health checks
while true; do
  bead_id=$(bf claim --assignee "$WORKER_ID" --json | jq -r '.bead_id')
  update_activity
  
  # Work on bead with timeout handling
  # ...
  
  health_check || { echo "Health check failed, exiting"; exit 1; }
done
```

### 4. Monitoring Integration

Integrate stale assignee metrics with monitoring system:

```bash
#!/bin/bash
# prometheus-metrics.sh - Export stale assignee metrics

METRICS_FILE="/var/lib/node_exporter/textfile_collector/bf_stale_assignees.prom"

while true; do
  cat > "$METRICS_FILE.$$" << EOF
# HELP bf_stale_assignees Number of beads with stale assignees (> claim TTL)
# TYPE bf_stale_assignees gauge
EOF

  for workspace in /home/coding/{FORGE,NEEDLE,AgentScribe,ARMOR,SIGIL,CLASP,bead-forge}; do
    if [ -d "$workspace/.beads" ]; then
      workspace_name=$(basename "$workspace")
      stale_count=$(bf -w "$workspace" list --status in_progress --format json | \
        jq --arg ttl "$(date -d '30 minutes ago' -u +%Y-%m-%dT%H:%M:%SZ)" \
        'map(select(.updated_at < $ttl)) | length')
      
      echo "bf_stale_assignees{workspace=\"$workspace_name\"} $stale_count" >> "$METRICS_FILE.$$"
    fi
  done

  mv "$METRICS_FILE.$$" "$METRICS_FILE"
  sleep 60  # Update every minute
done
```

---

## Post-Incident Procedures

### Incident Documentation

After remediation, document the incident:

```bash
#!/bin/bash
# incident-report.sh - Generate incident report

REPORT_DIR="/var/log/bf-incidents"
mkdir -p "$REPORT_DIR"

INCIDENT_ID="$(date +%Y%m%d-%H%M%S)"
REPORT_FILE="$REPORT_DIR/incident-$INCIDENT_ID.md"

cat > "$REPORT_FILE" << EOF
# Stale Assignee Incident Report - $INCIDENT_ID

## Timeline
- **Detected:** $(date -u +%Y-%m-%dT%H:%M:%SZ)
- **Remediated:** $(date -u +%Y-%m-%dT%H:%M:%SZ)
- **Duration:** TBD

## Scope
EOF

for workspace in /home/coding/{FORGE,NEEDLE,AgentScribe,ARMOR,SIGIL,CLASP,bead-forge}; do
  if [ -d "$workspace/.beads" ]; then
    echo "### $(basename $workspace)" >> "$REPORT_FILE"
    bf -w "$workspace" log --since 24h | grep -E "CLAIMED|RECLAIMED|ASSIGNEE_CHANGED" >> "$REPORT_FILE"
  fi
done

cat >> "$REPORT_FILE" << EOF

## Root Cause Analysis
TBD - Investigate:
- Worker crash logs
- System resource exhaustion
- Network partitions
- Deployment errors

## Remediation Actions
EOF

echo "- Cleared stale assignees from N beads" >> "$REPORT_FILE"
echo "- Used: fleet-clear-worker.sh / emergency-clear procedure" >> "$REPORT_FILE"

cat >> "$REPORT_FILE" << EOF

## Prevention Measures
- Adjust claim_ttl_minutes
- Add worker shutdown handlers
- Improve monitoring alerts

## Follow-up Items
- [ ] Update claim TTL configuration
- [ ] Add worker health checks
- [ ] Improve alerting thresholds
EOF

echo "Incident report: $REPORT_FILE"
```

### Configuration Updates

Update configurations based on incident learnings:

```bash
# After incident, review and update claim TTL
# If workers are timing out frequently, reduce TTL

cd /home/coding/FORGE
bf config set claim_ttl_minutes 15  # Reduce from default 30

# Apply to all workspaces
for workspace in /home/coding/{FORGE,NEEDLE,AgentScribe,ARMOR,SIGIL,CLASP,bead-forge}; do
  if [ -d "$workspace/.beads" ]; then
    bf -w "$workspace" config set claim_ttl_minutes 15
  fi
done
```

---

## Troubleshooting

### Issue: Bead Still Not Discoverable After Clearing

**Symptoms:**
```bash
bf update bf-abc123 --clear-assignee
bf ready | grep bf-abc123  # No output
```

**Diagnosis:**
```bash
# Check if assignee actually cleared
bf show bf-abc123 --format json | jq '.assignee'  # Should be null

# Check if bead is blocked
bf dep tree bf-abc123  # Look for open blockers

# Check if bead status is correct
bf show bf-abc123 | grep Status  # Should be "open"
```

**Resolution:**
- If assignee not null: Retry `bf update --clear-assignee`
- If blocked: Clear or close blockers first
- If status not open: Use `bf reopen bf-abc123`

### Issue: Batch Clearing Fails Mid-Operation

**Symptoms:**
```bash
# Script clears some beads but fails partway through
./fleet-clear-worker.sh worker-X
# Error: "Database is locked" or "disk I/O error"
```

**Resolution:**
```bash
# Use atomic batch operations for safety
bf -w /home/coding/FORGE batch --json '[
  {"op":"update","id":"bf-1","clear_assignee":true},
  {"op":"update","id":"bf-2","clear_assignee":true},
  {"op":"update","id":"bf-3","clear_assignee":true}
]'

# Or use doctor reclaim-stale which is transaction-safe
bf doctor --reclaim-stale --ttl 120
```

### Issue: High Recurrence Rate

**Symptoms:**
Stale assignees reappear within hours of remediation.

**Diagnosis:**
```bash
# Check if workers are crashing
journalctl -u needle-worker -n 100 | grep -E "crash|OOM|killed"

# Check claim TTL vs worker timeout
bf config get claim_ttl_minutes
# External timeout is 600s (10 min)
# TTL should be 10-15 minutes

# Check worker deployment scaling
kubectl get pods -n needle | grep "CrashLoopBackOff\|Error"
```

**Resolution:**
- Fix worker crashes (resource limits, code bugs)
- Align claim TTL with worker timeout
- Fix deployment scaling issues

---

## API Reference

### Core Commands

| Command | Purpose | Example |
|---------|---------|---------|
| `bf update --clear-assignee` | Clear assignee from a bead | `bf update bf-abc123 --clear-assignee` |
| `bf list --assignee <id>` | Find beads assigned to worker | `bf list --assignee worker-X` |
| `bf list --status in_progress` | Find all in-progress beads | `bf list --status in_progress` |
| `bf doctor --reclaim-stale` | Reclaim beads past TTL | `bf doctor --reclaim-stale --ttl 120` |
| `bf ready` | List discoverable beads | `bf ready --format json` |
| `bf show` | Inspect a bead | `bf show bf-abc123 --format json` |

### Batch Operations

```bash
# Clear multiple assignees atomically
bf batch --json '[
  {"op": "update", "id": "bf-1", "clear_assignee": true},
  {"op": "comment", "id": "bf-1", "text": "Stale reclamation"},
  {"op": "update", "id": "bf-2", "clear_assignee": true}
]'

# Reopen multiple beads and clear assignees
bf batch --json '[
  {"op": "update", "id": "bf-1", "status": "open"},
  {"op": "update", "id": "bf-1", "clear_assignee": true},
  {"op": "update", "id": "bf-2", "status": "open"},
  {"op": "update", "id": "bf-2", "clear_assignee": true}
]'
```

---

## Related Documentation

- [Stale Assignee Workflow](stale-assignee-workflow.md) - Detailed workflow with acceptance criteria
- [CLI Reference](README.md#commands) - Complete command reference
- [Doctor Commands](README.md#maintenance--config) - Database health and repair
- [NEEDLE Integration](README.md#needle-integration) - Worker integration details

---

## Quick Reference Card

```bash
# ==================== DISCOVERY ====================
# Scan single workspace
bf stats --by-assignee
bf list --status in_progress

# Scan fleet-wide
for ws in /home/coding/{FORGE,NEEDLE,AgentScribe,ARMOR,SIGIL,CLASP,bead-forge}; do
  bf -w "$ws" stats --by-assignee
done

# ==================== REMEDIATION ====================
# Single bead
bf update <id> --clear-assignee

# Single worker (all workspaces)
./fleet-clear-worker.sh worker-X

# All stale > 2 hours (fleet-wide)
for ws in /home/coding/{FORGE,NEEDLE,AgentScribe,ARMOR,SIGIL,CLASP,bead-forge}; do
  bf -w "$ws" doctor --reclaim-stale --ttl 120
done

# Emergency (all in_progress)
./fleet-emergency-clear.sh  # ⚠️ EMERGENCY ONLY

# ==================== VERIFICATION ====================
# Check bead is discoverable
bf show <id> --format json | jq '.assignee'  # Should be null
bf ready --format json | jq '.[].id' | grep <id>  # Should find it

# Monitor fleet health
for ws in /home/coding/{FORGE,NEEDLE,AgentScribe,ARMOR,SIGIL,CLASP,bead-forge}; do
  echo "=== $ws ==="
  bf -w "$ws" doctor
done
```
