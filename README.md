# bead-forge

Drop-in replacement for [beads_rust](https://github.com/dicklesworthstone/beads_rust) (`br`) with built-in server mode for concurrent multi-worker coordination.

## Problem

`br` uses SQLite for bead storage. SQLite serializes writes, causing contention when multiple NEEDLE workers claim beads from the same workspace simultaneously. With 11+ workers, the thundering herd problem wastes cycles on claim retries and database busy errors.

## Solution

`bead-forge` (`bf`) provides two modes:

- **Standalone**: Identical to `br` — SQLite + JSONL, fully backward compatible
- **Server**: Built-in coordination server with atomic claiming — no races, no retries, zero contention at any worker count

## Status

Research phase. See [docs/research/](docs/research/) for analysis.
