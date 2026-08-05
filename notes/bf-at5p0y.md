# Bead bf-at5p0y: UpdateCommand CLI Structure

## Task
Add UpdateCommand structure to CLI for the `bf update` command.

## Status
**ALREADY IMPLEMENTED** - This bead confirms existing implementation rather than adding new code.

## Verification
The `Update` command is fully implemented in `src/cli/mod.rs`:

1. **Command Definition** (lines 167-230):
   - Enum variant: `Commands::Update`
   - Fields: id, title, status, priority, assignee, description, acceptance_criteria, notes, design, due_at
   - Advanced features: description-file, clear_assignee

2. **CLI Registration**:
   - Part of `#[derive(Subcommand)]` enum `Commands`
   - Registered under name "update"

3. **Routing** (lines 1255-1306):
   - Handled in `run()` function match statement
   - Calls `cmd_update()` handler function

4. **Handler Function** (lines 1916-1989):
   - `cmd_update()` with full implementation
   - Includes validation, storage updates, and auto-flush support

## Acceptance Criteria Met
- ✅ UpdateCommand struct exists with all required fields
- ✅ Command registered in CLI with name 'update'
- ✅ Proper attribute parsing with clap derive attributes
- ✅ Command routes to handler function (cmd_update)

## Implementation Notes
- Priority field is `Option<i32>` (correct numeric type) not `Option<String>`
- Implementation exceeds basic requirements with additional fields and features
- Includes proper error handling, validation, and JSON output support
