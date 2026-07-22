# bf-4hkcg — Test Epic Label Operations

Verified `bf label` subcommands operate correctly on an **epic**-type issue
(`bf-4hkcg` itself, type=epic) on the current tree (bf 0.3.0).

## What was tested (end-to-end)

| Step | Command | Result |
|------|---------|--------|
| Baseline | `br label list bf-4hkcg` | `epic-added-label` |
| Add (multi) | `br label add bf-4hkcg -l epic-label-two epic-label-three` | both added |
| List after add | `br label list bf-4hkcg` | 3 labels, sorted alphabetically |
| Add duplicate | `br label add bf-4hkcg -l epic-label-two` | idempotent — no duplicate entry |
| Remove | `br label remove bf-4hkcg -l epic-label-three` | removed |
| List after remove | `br label list bf-4hkcg` | `epic-added-label`, `epic-label-two` |
| Type integrity | `br show bf-4hkcg` | `Type: epic` preserved; labels persisted in `Labels:` line |

## Findings

- Label add/remove/list all work on epic-type issues with no special-casing.
- `-l/--label` accepts multiple values in one invocation.
- Adding an already-present label is idempotent (reports success, list unchanged).
- Labels are stored sorted and surface both in `label list` and `show`'s `Labels:` line.
- Adding labels does not alter the issue `Type`.

## Cleanup

Test labels (`epic-label-two`, `epic-label-three`) were removed; the epic was
restored to its baseline single label `epic-added-label`.

No source changes required — existing behavior verified as correct.
