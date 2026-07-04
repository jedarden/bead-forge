//! Close bead functionality.

use crate::storage::Storage;
use anyhow::Result;
use std::path::Path;

/// Close a bead with the given reason.
///
/// This function:
/// - Transitions the bead's status to 'closed'
/// - Sets closed_at timestamp to current time
/// - Sets close_reason field (defaults to 'Completed' if not provided)
/// - Marks bead as dirty in SQLite
/// - Uses with_immediate_transaction for atomicity
///
/// # Arguments
/// * `db_path` - Path to the SQLite database
/// * `id` - Bead ID to close
/// * `reason` - Close reason (will be "Completed" if empty)
/// * `actor` - Actor performing the close (e.g., "cli", worker ID)
///
/// # Errors
/// Returns error if:
/// - Bead not found
/// - Bead already closed
/// - Database operation fails
pub fn close_bead(db_path: &Path, id: &str, reason: &str, actor: &str) -> Result<()> {
    let storage = Storage::open(db_path)?;
    storage.close_issue(id, reason, actor)?;
    Ok(())
}
