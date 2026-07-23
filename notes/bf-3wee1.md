# bf-3wee1: CLI Arguments for Description and Acceptance-Criteria

## Task
Add --description, --description-file, and --acceptance-criteria arguments to bf update command.

## Finding
All three CLI arguments **already exist** in the codebase and were implemented in a prior change:

### Existing Implementation (src/cli/mod.rs)

1. **`--description <text>`** (line 177-178)
   ```rust
   /// New description
   #[arg(long)]
   description: Option<String>,
   ```

2. **`--description-file <path>`** (line 182-184)
   ```rust
   /// Read the new description from a file. Useful for long or multiline
   /// bodies that are awkward to pass on the shell. Conflicts with
   /// --description (which wins for short inline text).
   #[arg(long, conflicts_with = "description")]
   description_file: Option<PathBuf>,
   ```

3. **`--acceptance-criteria <text>`** (line 187-188)
   ```rust
   /// New acceptance criteria
   #[arg(long)]
   acceptance_criteria: Option<String>,
   ```

### Wiring
- Arguments are defined in the `Update` command struct (lines 149-201)
- Arguments flow to `cmd_update` function (lines 1666-1678)
- `cmd_update` passes them to `storage.update_issue` via `IssueChanges`
- Help text properly documents all options (verified with `bf update --help`)

### Status
✅ All acceptance criteria met:
- CLI accepts `--description <text>` for inline description
- CLI accepts `--description-file <path>` to read from file
- CLI accepts `--acceptance-criteria <text>` for inline criteria
- Arguments are parsed and wired through the handler
- CLI help text documents these options
- `cargo build` compiles cleanly

No code changes required — this bead's work was already completed.
