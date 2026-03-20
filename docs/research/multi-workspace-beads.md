# Multi-Workspace Beads Pattern

## Observed Pattern

Unlike typical beads usage (one `.beads/` at repo root), this deployment uses multiple `.beads/` directories within a single git repository, scoped to subprojects:

```
ardenone-cluster/                          # one git repo
├── .beads/                                # repo-level beads (infra tasks)
├── containers/
│   ├── zai-proxy/.beads/                  # zai-proxy container beads
│   ├── claude-code-leaderboard/.beads/    # leaderboard beads
│   ├── options-aggregator/.beads/         # options beads
│   └── mcp-openai-search/.beads/         # MCP search beads
├── project/
│   ├── ibkr-mcp/.beads/                  # IBKR MCP project beads
│   ├── mission-control/.beads/           # mission control beads
│   ├── kalshi-improvement/.beads/        # kalshi improvement beads
│   ├── options-enrichment/.beads/        # options enrichment beads
│   └── native-ads-profiling/.beads/      # native ads beads
├── cluster-configuration/
│   └── apexalgo-iad/
│       └── native-ads-scraper/.beads/    # scraper-specific beads
└── research/
    └── bead-priority/.beads/             # research beads
```

14 independent `.beads/` directories, each with its own:
- `issues.jsonl` (independent JSONL log)
- `beads.db` (independent SQLite database)
- `config.yaml` (independent configuration)
- Issue prefix (scoped ID namespace)

## Why This Pattern Exists

A monorepo contains many logically independent subprojects. Each subproject has its own:
- Codebase scope (different languages, frameworks, concerns)
- Plan document (separate implementation plans)
- Worker assignment (NEEDLE workers target specific subproject directories)
- Issue lifecycle (a container's beads are independent of a project's beads)

Putting all beads in the repo root would:
- Mix concerns (zai-proxy bugs alongside native-ads features)
- Create a single contention point (all workers hit one SQLite DB)
- Lose locality (worker in `containers/zai-proxy/` has to navigate a flat list of 500+ beads)

## Implications for bead-forge

### 1. Discovery

bead-forge must discover `.beads/` directories by walking the filesystem, not assuming repo root. NEEDLE's explore strand already does this — it scans child directories for `.beads/` folders.

```
bf discover /home/coding/ardenone-cluster
→ Found 14 workspaces:
  /home/coding/ardenone-cluster/.beads/ (3 open)
  /home/coding/ardenone-cluster/containers/zai-proxy/.beads/ (6 open)
  /home/coding/ardenone-cluster/project/ibkr-mcp/.beads/ (1 open)
  ...
```

### 2. Server Mode — Multi-Workspace

The bead-forge server must manage multiple independent workspaces simultaneously. Each workspace has its own priority queue, but workers can claim from any workspace.

```
bf server --root /home/coding/ardenone-cluster
→ Serving 14 workspaces, 47 total open beads
```

The server watches for new `.beads/` directories appearing (a worker running `bf init` in a new subdirectory).

### 3. Cross-Workspace Claiming

A worker assigned to `containers/zai-proxy/` that exhausts its beads should be able to claim from `project/ibkr-mcp/` without restarting. The server handles this transparently:

```
bf claim --workspace containers/zai-proxy/
→ None available

bf claim --any
→ Claimed project/ibkr-mcp/ibkr-mcp-42 (next highest priority across all workspaces)
```

### 4. Workspace-Scoped vs Global Operations

Some operations are workspace-scoped, others are global:

| Operation | Scope | Example |
|-----------|-------|---------|
| `bf list` | Workspace (default) or global (`--all`) | `bf list` in zai-proxy/ shows only zai-proxy beads |
| `bf claim` | Workspace (default) or global (`--any`) | Atomic pop from one workspace's queue |
| `bf create` | Workspace | Creates bead in current .beads/ |
| `bf status` | Global | Fleet overview across all workspaces |
| `bf discover` | Global | Scan for .beads/ directories |

### 5. JSONL Remains Per-Workspace

Each `.beads/issues.jsonl` stays independent. The server indexes them all but doesn't merge them. This preserves:
- Git diff locality (changes to zai-proxy beads don't touch ibkr-mcp JSONL)
- Independent sync (each workspace can flush/import independently)
- Backward compatibility with `br` (which operates on one workspace at a time)

### 6. ID Namespace Isolation

Each workspace has its own `issue_prefix` in `config.yaml`. Bead IDs are unique within a workspace but may collide across workspaces (e.g., both could have `bd-123`). The server uses `{workspace_path}:{bead_id}` as the global key internally.

### 7. SQLite Contention Distribution

With 14 workspaces, write contention is naturally distributed across 14 SQLite databases. This is already better than one monolithic database. The bead-forge server adds coordination on top without replacing this natural sharding.

### 8. Worker Assignment Affinity

Workers should have workspace affinity (prefer beads in their assigned workspace) with fallback to global claiming. This maps to NEEDLE's current model:

```
needle run --workspace=/home/coding/ardenone-cluster/containers/zai-proxy
→ Worker claims from zai-proxy first
→ If exhausted, explores to sibling workspaces
→ If all exhausted, claims globally
```

bead-forge preserves this with:
```
bf claim --workspace containers/zai-proxy/ --fallback any
```

## Comparison with Single-Root Pattern

| Aspect | Single root | Multi-workspace |
|--------|-------------|-----------------|
| SQLite contention | One DB, all workers | Distributed across N DBs |
| ID namespace | One prefix | N independent prefixes |
| Git diff noise | Every bead change in one file | Scoped to subproject |
| Worker locality | Workers search flat list | Workers scoped to subdirectory |
| Discovery | Trivial (repo root) | Requires filesystem scan |
| Cross-project deps | Natural (same DB) | Requires cross-workspace references |
| Scale | Bottleneck at ~50+ workers | Naturally sharded |

## Open Question: Cross-Workspace Dependencies

Currently each workspace's dependency graph is isolated. A bead in `containers/zai-proxy/` cannot formally block a bead in `project/ibkr-mcp/`. In practice, these dependencies exist (a container change may be needed before a project feature works) but are tracked informally.

bead-forge could support cross-workspace dependencies with qualified IDs:
```
bf dependency add-blocker project/ibkr-mcp:ibkr-mcp-42 containers/zai-proxy:zai-proxy-7
```

This is a nice-to-have, not a launch requirement.
