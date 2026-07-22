use crate::model::Issue;
use anyhow::Result;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

/// Result of upserting an issue during import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpsertResult {
    /// Issue was newly created
    New,
    /// Issue was updated (content changed)
    Updated,
    /// Issue was skipped (content unchanged)
    Unchanged,
}

pub struct ImportResult {
    pub imported: usize,
    pub updated: usize,
    pub skipped: usize,
}

pub struct ExportResult {
    pub count: usize,
}

pub fn stream_issues(path: &Path) -> Result<impl Iterator<Item = Result<Issue>>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    Ok(reader.lines().map(move |line| {
        let line = line?;
        serde_json::from_str::<Issue>(&line).map_err(Into::into)
    }))
}

pub fn import_jsonl<F>(path: &Path, mut upsert: F) -> Result<ImportResult>
where
    F: FnMut(&Issue) -> Result<UpsertResult>,
{
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut result = ImportResult {
        imported: 0,
        updated: 0,
        skipped: 0,
    };

    for line in reader.lines() {
        let line = line?;
        let issue: Issue = serde_json::from_str(&line)?;
        match upsert(&issue)? {
            UpsertResult::New => result.imported += 1,
            UpsertResult::Updated => result.updated += 1,
            UpsertResult::Unchanged => result.skipped += 1,
        }
    }

    Ok(result)
}

pub fn export_jsonl<F>(path: &Path, mut list_all: F) -> Result<ExportResult>
where
    F: FnMut() -> Result<Vec<Issue>>,
{
    let issues = list_all()?;
    let temp_path = path.with_extension("jsonl.tmp");

    {
        let file = File::create(&temp_path)?;
        let mut writer = BufWriter::new(file);

        for issue in &issues {
            serde_json::to_writer(&mut writer, issue)?;
            writer.write_all(b"\n")?;
        }

        writer.flush()?;
    }

    std::fs::rename(&temp_path, path)?;

    Ok(ExportResult {
        count: issues.len(),
    })
}

/// Surgically merge `upserts` (and apply `removals`) into an existing JSONL file.
///
/// This is the primitive behind incremental auto-flush (Phase 7.1): instead of
/// rewriting the whole file from the SQLite store, it reads the current
/// `issues.jsonl`, keeps every untouched line **byte-for-byte**, replaces (or
/// inserts) the line for each `upsert` bead, drops the line for each `removal`
/// id, and writes the result back atomically (temp + rename), sorted by id for
/// stable diffs — the same ordering `export_jsonl` (full flush) produces.
///
/// Lines that fail to parse (no recoverable `id`) are preserved verbatim and
/// appended after the sorted beads so a hand-edited or foreign line is never
/// silently dropped.
///
/// Returns the number of `upserts` applied (matching the dirty-bead count the
/// callers report). A pure no-op — no upserts, no removals, and no existing
/// file — writes nothing and returns 0.
pub fn export_jsonl_merge(
    path: &Path,
    upserts: &[Issue],
    removals: &[String],
) -> Result<ExportResult> {
    let file_exists = path.exists();
    if upserts.is_empty() && removals.is_empty() && !file_exists {
        return Ok(ExportResult { count: 0 });
    }

    // Index existing lines by id (value = the original raw line, preserved
    // verbatim). Unparseable lines are kept aside and re-appended.
    use std::collections::BTreeMap;
    let mut by_id: BTreeMap<String, String> = BTreeMap::new();
    let mut orphan_lines: Vec<String> = Vec::new();
    if file_exists {
        // A read error here (e.g. the path is a directory) propagates so the
        // caller can surface it as a flush failure rather than silently losing
        // the file's contents.
        let contents = std::fs::read_to_string(path)?;
        for line in contents.lines() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<serde_json::Value>(line)
                .ok()
                .and_then(|v| v.get("id").and_then(|i| i.as_str()).map(String::from))
            {
                Some(id) => {
                    by_id.insert(id, line.to_string());
                }
                None => orphan_lines.push(line.to_string()),
            }
        }
    }

    // Apply removals first, then upserts (an upsert of a just-removed id wins).
    for id in removals {
        by_id.remove(id);
    }
    for issue in upserts {
        by_id.insert(issue.id.clone(), serde_json::to_string(issue)?);
    }

    let temp_path = path.with_extension("jsonl.tmp");
    {
        let file = File::create(&temp_path)?;
        let mut writer = BufWriter::new(file);
        for line in by_id.values() {
            writer.write_all(line.as_bytes())?;
            writer.write_all(b"\n")?;
        }
        for line in &orphan_lines {
            writer.write_all(line.as_bytes())?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
    }
    std::fs::rename(&temp_path, path)?;

    Ok(ExportResult {
        count: upserts.len(),
    })
}

/// Incrementally flush the dirty beads into JSONL via [`export_jsonl_merge`].
///
/// Only the dirty beads' lines are rewritten; every other bead already in the
/// file is preserved (surgical line replacement, not a full rewrite — see
/// Phase 7.1 in `docs/plan/plan.md`). `clear_dirty` runs only after the write
/// commits, so a failed flush leaves the dirty marks intact for recovery.
///
/// A no-op (nothing dirty) returns early without touching the file.
pub fn export_jsonl_dirty<F1, F2>(
    path: &Path,
    mut list_dirty: F1,
    mut clear_dirty: F2,
) -> Result<ExportResult>
where
    F1: FnMut() -> Result<Vec<Issue>>,
    F2: FnMut() -> Result<()>,
{
    let issues = list_dirty()?;
    if issues.is_empty() {
        return Ok(ExportResult { count: 0 });
    }

    let result = export_jsonl_merge(path, &issues, &[])?;
    clear_dirty()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_stream_issues() {
        let jsonl = r#"{"id":"bf-test","title":"Test","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}
{"id":"bf-test2","title":"Test2","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}"#;

        let cursor = Cursor::new(jsonl);
        let reader = BufReader::new(cursor);

        let count = reader
            .lines()
            .filter_map(|line| line.ok())
            .filter_map(|line| serde_json::from_str::<Issue>(&line).ok())
            .count();

        assert_eq!(count, 2);
    }
}
