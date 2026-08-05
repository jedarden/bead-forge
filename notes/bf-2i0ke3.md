# bf-2i0ke3: Add inline code comments explaining clap multi-value patterns

## Summary

Added comprehensive inline documentation for clap v4 multi-value argument patterns throughout `src/cli/mod.rs`. Each `Vec<String>` or `Vec<PathBuf>` field now includes detailed comments explaining:

1. The clap pattern in use (repeated flags vs positional collection)
2. Usage examples
3. How clap v4's Append action works
4. What clap attributes do (`num_args`, `required`, etc.)
5. Gotchas and behavioral notes

## Fields Documented

1. **Create command** (`label: Vec<String>` - lines 88-100)
   - Pattern: Repeated long flags with default Append action
   - No `num_args`: empty Vec when omitted, one value per flag

2. **LabelCommands::Add** (`label: Vec<String>` - lines 962-973)
   - Pattern: Short/long flags with `num_args = 1..` and `required = true`
   - At least one value required, validated at parse time
   - Gotcha: `num_args` applies per occurrence, not total count

3. **LabelCommands::Remove** (`label: Vec<String>` - lines 983-994)
   - Same pattern as Add, with no-op behavior for non-existent labels

4. **CommentsCommands::Add** (`text: Vec<String>` - lines 1018-1029)
   - Pattern: Positional multi-value collection
   - Values joined with spaces in handler
   - Shell word splitting behavior explained

## Existing Documentation (Already Present)

The file already had excellent clap multi-value documentation for:
- `Claim.workspace_paths: Vec<PathBuf>` (lines 313-321)
- `Search.status: Vec<String>` (lines 592-600)
- `Search.type_: Vec<String>` (lines 604-611)
- `Search.label: Vec<String>` (lines 618-619)

## Acceptance Criteria Met

✅ Added comments to each label argument definition explaining the clap pattern
✅ Documented what each clap attribute does (value_parser, num_args, action, etc.)
✅ Included gotchas and adjustments needed
✅ Comments follow existing doc style in the file

## Files Changed

- `src/cli/mod.rs`: Added 4 comprehensive comment blocks for multi-value patterns

## Testing

The changes are purely documentation (comments), so no functional changes were made.
Syntax is valid Rust - comments are well-formed and compile cleanly.
