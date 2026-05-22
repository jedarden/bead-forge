# bf-1leo: Annotation Filter Verification

The `bf list --annotation key=value` filter was already fully implemented at the time this bead was claimed.

## Implementation Verified

1. **CLI Argument**: `--annotation <ANNOTATION>` exists in `src/cli/mod.rs` List command
2. **Model**: `IssueFilter` struct has `annotation: Option<(String, String)>` field
3. **Parsing**: `cmd_list` correctly parses `key=value` format with error handling
4. **SQL Query**: `list_issues` joins `bead_annotations` table and filters by key/value
5. **All Mode**: `--all` mode also applies annotation filtering via `retain`

## Tests Run

```bash
# Valid format filtering
bf list --annotation metadata.source=git-reconstructed
# Returns: 39 beads with metadata.source=git-reconstructed

# Invalid format error handling
bf list --annotation invalidformat
# Error: Invalid annotation format. Use key=value

# Combined with limit
bf list --annotation metadata.source=git-reconstructed --limit 3
# Returns: 3 beads

# JSON output
bf list --annotation metadata.source=git-reconstructed --json
# Returns: JSON formatted results

# Non-existent annotation
bf list --annotation test=value
# Returns: (empty - no matches)
```

## Note

This is a `bf`-only feature - `br` does not have an `--annotation` filter in its list command.
`bf` is a strict superset of `br`, so this is acceptable.
