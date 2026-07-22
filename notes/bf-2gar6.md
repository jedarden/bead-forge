# bf-2gar6 — Docs bead

Self-directed `docs` bead (no description/criteria given). Audited the user-facing
command reference in `docs/README.md` against the actual CLI implementation
(`src/cli/mod.rs` + `bf --help`) and corrected every discrepancy.

## Findings — reference was out of sync with the implementation

The **Commands** block was missing commands and described several with wrong syntax.

**Missing commands** (implemented but undocumented): `reopen`, `delete`, `ready`,
`count`, `mitosis`, `comments`, `schema`, `config`, `recent`, `labels`.

**Wrong syntax** (would fail if copy-pasted):
- `bf annotate <id> key=value [--remove key]` → `annotate` is a subcommand group:
  `bf annotate set <id> <key> <value>` (also `get`/`remove`/`list`/`clear`).
- `bf dep add-blocker <blocker> <blockee>` → `bf dep add <blocker> --blocks <blocked> [-t <type>]`.
- `bf label <id> add <label>` → subcommand comes first: `bf label add --label <l> [<l>...] <id>`.
- `bf doctor [--check] ... [--ttl <duration>]` → there is **no** `--check` flag
  (plain `bf doctor` runs the health check), and `--ttl` takes **minutes**, not a duration.

## Changes made (`docs/README.md`)

1. Rewrote the **Commands** section: all 30 implemented commands, grouped
   (Lifecycle / Claiming & concurrency / Dependencies & structure / Labels, comments,
   annotations / Query & history / Maintenance & config), each with verified flags.
2. Fixed the **Extensible Annotations** example (`bf annotate set ...` instead of `key=value`).
3. Replaced three nonexistent `bf doctor --check` references with the correct no-flag
   `bf doctor` health check (verification section + the "what `bf migrate` does" step).

All command syntax was cross-checked against `bf <cmd> --help` output.

## Out of scope (left intentionally)

`docs/plan/plan.md` is the historical implementation-plan / design document. It uses
placeholder forms (`fg-` prefix, `--db` flag, `dep add-blocker`, `annotate key=value`)
that reflect design intent at planning time rather than the shipped CLI. Correcting it
would be a broad rewrite of a design artifact and obscure its history, so it was left
as-is. The same applies to the `docs/research/*` ecosystem notes.
