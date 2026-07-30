# bf-4iqxz: Confirm no active NEEDLE workers hold the uncommitted .beads/ state

## Role of this bead
Child of the catch-up umbrella **bf-33zhy** ("One-time catch-up: flush and commit
bead-forge's own uncommitted .beads/ state"). bf-33zhy explicitly requires, before
any `bf sync --flush-only` / commit of `.beads/`, that a sibling confirm no
concurrently-running NEEDLE worker is actively mid-session on any bead reflected in
the uncommitted state. bf-4iqxz owns ONLY that confirmation. It does not flush,
repair, or commit `.beads/`.

## Method
Read-only liveness checks (no `.beads/` mutation except the required comment on the
parent). Signals used, in increasing directness:

1. `bf list --status in_progress` + `bf show <id>` — enumerate in-progress beads and
   their assignees.
2. `ps` — needle supervisor processes (`needle run --workspace /home/coding/bead-forge`).
3. `.needle/logs/needle-claude-code-glm-5-*.stderr.log` — last `DISPATCHING→EXECUTING`
   transition per worker and the assigned `needle.agent.pid`.
4. **`/proc/<pid>/cwd` + `ps -o etimes`** — definitive: is the dispatched claude agent
   process still alive with `cwd=/home/coding/bead-forge`, and for how long?

## Result: the tree is NOT quiet — do NOT flush/commit yet
Confirmation (the hoped-for "no active workers") **FAILED**. As of **2026-07-22
~10:03 EDT**, four live claude agent processes have `cwd=/home/coding/bead-forge`;
three are siblings other than this run:

| Worker (identifier) | Live claude PID | Elapsed | Bead(s) held | Also holds |
|---------------------|-----------------|---------|--------------|------------|
| victor              | 3136296         | ~9.3 min | bf-5y3cj (dispatched) | bf-5wku |
| whiskey             | 3151115         | ~5.8 min | bf-4waen (dispatched) | bf-j7w7 |
| cgov-polish (claude-print-opus) | 3171418 | ~3.3 min | bf-gj673 (dispatched) | — |
| uniform (THIS run)  | 3186487         | ~1.4 min | bf-4iqxz       | — |

Each sibling's needle log tail shows `state transition from=DISPATCHING to=EXECUTING`
for its bead with **no subsequent EXECUTING→COMPLETED/SUCCEEDED/FAILED transition**,
and the corresponding agent PID is still alive with non-trivial elapsed time. That is
an unambiguous "actively mid-session" signal.

## In-progress beads in the uncommitted state (enumerated)
`bf list --status in_progress` returned six; the five that are NOT this bead:

- **bf-gj673** (P1, assignee claude-print-opus-cgov-polish) — assignee-clearing gap umbrella. **ACTIVE (cgov-polish).**
- **bf-5y3cj** (P2, assignee claude-code-glm-5-victor) — bf-checkpoint.sh throttle/push/parity (FINAL slice of bf-3cu1k). **ACTIVE (victor).**
- **bf-5wku** (P2, assignee claude-code-glm-5-victor) — formatter for search/claim. Held by **ACTIVE (victor).**
- **bf-4waen** (P3, assignee claude-code-glm-5-whiskey) — port orphan-file scenarios to tests. **ACTIVE (whiskey).**
- **bf-j7w7** (P2, assignee claude-code-glm-5-whiskey) — claim JSON → formatter. Held by **ACTIVE (whiskey).**

The in-progress set in the uncommitted `issues.jsonl` therefore overlaps exactly with
live sessions. Flushing (`bf sync --flush-only`, which rewrites `issues.jsonl` from the
db) or `git commit`-ing `.beads/` now would interleave with active db writers and
capture in-flight trace dirs / the shared scratch file mid-write — a violation of the
shared-workspace operating rule. **Holding off is the correct call.**

## Recommendation to the next child of bf-33zhy
1. **Re-check liveness, do not assume stale data.** Run (from `/home/coding/bead-forge`):
   ```bash
   # any live claude agent in this tree? (empty = quiet)
   for pid in $(pgrep -f '/home/coding/.local/bin/claude'); do
     [ "$(readlink /proc/$pid/cwd 2>/dev/null)" = /home/coding/bead-forge ] && echo "HOT: pid $pid $(ps -o etimes= -p $pid)s"
   done
   ```
   Cross-check: `.needle/logs/needle-claude-code-glm-5-*.stderr.log` — a worker is idle
   only if its last dispatch has a matching `EXECUTING→COMPLETED/…` transition and no
   fresh dispatch since.
2. Only when the above is empty for **all** bead-forge workers, proceed with
   `bf sync --flush-only` → review diff → commit legitimate remainder → rebase onto
   `origin/main` → push.
3. **Commit single paths only** (`notes/`, specific trace dirs) — never `git add -A` /
   `git commit -a` (shared-workspace race rule; another worker's in-flight files may be
   swept in).
4. Observe: `.needle-predispatch-sha` shows as `M` purely because every worker rewrites
   it on each dispatch (`git rev-parse HEAD > .needle-predispatch-sha`). It is
   non-deterministic per-dispatch scratch; treat it as noise, not meaningful uncommitted
   state, and do not commit it (consider `.gitignore`-ing it separately).

## What this run changed
- No source changes. No `.beads/` flush/repair/commit of state.
- One db comment recorded on parent **bf-33zhy** (`bf comments add`) carrying this
  finding, so the next child proceeds safely. (A comment is a normal atomic INSERT —
  the same shared-db op every worker performs; it does not rewrite `issues.jsonl`.)

## Self-check: did this bead meet acceptance?
- Enumerate in-progress beads in uncommitted state: **yes** (5 siblings listed above).
- Confirm none actively mid-session: **no — 3 are active**; reported faithfully rather
  than declared safe. The task is to *confirm safety before flush*; the honest
  confirmation is "not safe yet," which is the protective outcome the operating rule
  wants.
- Record on parent bf-33zhy so the next child proceeds safely: **yes** (comment added).
