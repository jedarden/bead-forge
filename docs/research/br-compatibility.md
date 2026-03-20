# br CLI Compatibility Requirements

## Goal

bead-forge (`bf`) must be a drop-in replacement for `br` (beads_rust). Existing NEEDLE workers, scripts, and CLAUDE.md instructions that reference `br` should work with `bf` after a symlink or alias.

## CLI Interface Parity

### Commands that must be identical

| br command | bf equivalent | Notes |
|-----------|---------------|-------|
| `br create --title "..." --type task --priority N --description "..."` | Same | Creates bead, appends to JSONL |
| `br list [--status open] [--limit N] [--format json]` | Same | Query beads by status/type/priority |
| `br show <id>` | Same | Display bead details |
| `br update <id> --status <status> [--assignee <worker>]` | Same | Update bead fields |
| `br close <id>` | Same | Set status to closed |
| `br dependency add-blocker <parent> <child>` | Same | Wire dependency |
| `br dep tree <id>` | Same | Show dependency graph |
| `br sync --flush-only` | Same | Export DB to JSONL |
| `br sync --import-only` | Same | Import JSONL to DB |
| `br doctor --repair` | Same | Validate and repair |

### New commands (bead-forge only)

| bf command | Purpose |
|-----------|---------|
| `bf claim --workspace <path>` | Atomic claim — server pops next bead, no client-side race |
| `bf server [--bind <addr>] [--port <port>]` | Start coordination server |
| `bf server --status` | Show server status, connected workers, queue depths |
| `bf migrate` | Import existing .beads/ data into server |

## File Format Parity

### .beads/ directory structure (must be identical)

```
.beads/
├── issues.jsonl          # Append-only audit log (source of truth)
├── beads.db              # SQLite database (read cache, optional with server)
├── config.yaml           # Workspace configuration
├── metadata.json         # Workspace metadata
├── hooks/                # Lifecycle hooks
├── .br_history/          # JSONL snapshots
└── .br_recovery/         # Recovery backups
```

### JSONL format (must be identical)

Each line is a JSON object with at minimum:
```json
{
  "id": "nd-abc123",
  "title": "Fix the bug",
  "description": "...",
  "status": "open",
  "priority": 1,
  "issue_type": "task",
  "assignee": null,
  "created_at": "2026-03-20T04:00:00Z",
  "updated_at": "2026-03-20T04:00:00Z"
}
```

Status updates append new lines with the same `id` — last entry wins.

### config.yaml (must be compatible)

bead-forge reads the same config.yaml format. New server-specific fields are additive:

```yaml
# Existing br fields (unchanged)
issue_prefix: nd
default_priority: 2
default_type: task

strands:
  pluck: true
  weave: true
  # ...

# New bead-forge fields (optional)
server:
  enabled: true
  bind: "127.0.0.1"
  port: 7700
  socket: "/tmp/bead-forge.sock"  # Unix socket (local workers)
  tailscale: true                  # Accept connections over Tailscale
```

## Operational Modes

### Mode 1: Standalone (br-compatible)
No server running. `bf` operates identically to `br` — reads/writes SQLite + JSONL directly. Default mode.

### Mode 2: Server
`bf server` starts the coordination daemon. Workers detect the server (via socket file or config) and route claim/update operations through it. JSONL append still happens for auditability.

### Mode 3: Hybrid
Server handles claiming and status transitions. SQLite is maintained as a read cache for `bf list` and `bf show` queries. Both can operate simultaneously.

## NEEDLE Integration

NEEDLE's `src/strands/pluck.sh` calls `br` for:
1. `br list --format json` — find claimable beads
2. `br update <id> --status in_progress --assignee <worker>` — claim
3. `br close <id>` — complete
4. `br update <id> --status blocked` — block

With bead-forge:
1. Replace step 1+2 with `bf claim --workspace <path> --assignee <worker> --json` — atomic, no race
2. Steps 3+4 remain the same (`bf close`, `bf update`)

Minimal NEEDLE change: one line in pluck.sh to use `bf claim` instead of `br list` + `br update`.
