# Bead bf-jch6e9: Test invalid type

## Summary

This is a **test/probe bead** — `bf-jch6e9` was created with a non-standard
`issue_type` of `invalid-type-xyz` to verify that bead-forge accepts and
round-trips arbitrary/invalid types. **Result: PASS.** bead-forge stores,
displays, and round-trips the custom type verbatim with no validation
rejection. No code change was required or warranted — the behavior is the
intended design.

## Why invalid types are accepted (by design)

`src/model.rs` defines `IssueType` with a catch-all `Custom(String)` variant:

```rust
pub enum IssueType {
    Task, Bug, Feature, Epic, Chore, Docs, Question,
    #[serde(untagged)]
    Custom(String),
}
```

`FromStr` for `IssueType` maps the seven canonical names and falls through to
`Custom(other.to_string())` for *anything* else — it never returns `Err`. This
mirrors upstream `br`/`beads_rust`'s permissive behavior. So `invalid-type-xyz`
becomes `IssueType::Custom("invalid-type-xyz")` and is stored as-is in the
`issue_type` column of `issues`.

## Verification (isolated temp workspace — no pollution of the real store)

Created a throwaway workspace with `bf -w <tmp> init` and ran the full round-trip:

| Step | Command | Result |
|------|---------|--------|
| 1 | `create --title "Probe" --type invalid-type-xyz --json` | Accepted, no validation error (id `bf-5r9`) |
| 2 | `show <id>` (human) | `Type: invalid-type-xyz` |
| 3 | `show <id> --json` | `"issue_type":"invalid-type-xyz"` |
| 4 | `list --type invalid-type-xyz --json` | 1 row, `issue_type = 'invalid-type-xyz'` |
| 5 | `sync --flush-only` → wipe `beads.db` → reopen | Type recovered verbatim after checkpoint round-trip |

The probe bead itself (`bf-jch6e9`) in the live store independently confirms
this: `bf show bf-jch6e9 --json` returns `"issue_type":"invalid-type-xyz"`.

## Conclusion

Invalid/custom issue types are handled correctly end-to-end (create → store →
show → list-by-type → JSONL checkpoint → re-import). No defect found; no
actionable code change. Closing as a verified test/probe bead.
