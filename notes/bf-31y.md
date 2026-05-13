# bf-31y: rotate.rs Module Verification

## Status: ✅ ALREADY IMPLEMENTED

The `src/rotate.rs` module was already fully implemented when this bead was claimed.

## Implementation Verified

### Core Functionality
- `rotate(beads_dir, options)` - Main rotation function
- `RotateOptions` struct with age_days, max_size_mb, max_archives, dry_run
- `RotateResult` struct returning archived count, remaining count, archive path, deleted archives

### Algorithm (matches spec)
1. **Scan active JSONL** - Stream issues.jsonl and categorize beads by status/age
2. **Check size limit** - If issues.jsonl.1 exceeds rotate_max_size_mb, trigger shift
3. **Shift archives** - Sequential shift (.1 → .2, .2 → .3, etc.) via `shift_archives()`
4. **Append to archive** - Stream archived beads to issues.jsonl.1 (create or append)
5. **Rewrite active** - Atomic temp+rename of issues.jsonl with only active beads

### Helper Functions
- `should_archive()` - Checks if bead is closed/tombstone and older than threshold
- `shift_archives()` - Sequential archive rotation with oldest deletion
- `cleanup_old_archives()` - Delete archives beyond max_archives limit
- `find_bead_in_archives()` - Search active + archives for bead by ID
- `list_all_with_archives()` - List all beads across active + archives
- `list_archives()` - Get archive file metadata

### CLI Integration
- Command: `bf rotate [--days N] [--dry-run]`
- Config support via `RotateConfig` (rotate_age_days, rotate_max_size_mb, rotate_max_archives)
- Default: 30 days, 100MB max size, 10 archives max

### Tests
All 10 tests passing:
- test_should_archive_closed_old_bead
- test_should_not_archive_closed_recent_bead
- test_should_not_archive_open_bead
- test_rotate_creates_archive
- test_rotate_dry_run
- test_find_bead_in_archives
- test_list_all_with_archives
- test_cleanup_old_archives
- test_shift_archives
- test_shift_archives_with_deletion

### Build Status
- ✅ cargo build clean (no errors)
- ✅ All rotate tests pass
- ✅ CLI command functional
