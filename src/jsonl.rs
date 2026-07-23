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
    // Nothing to add and no file to prune: never create an empty JSONL. (A
    // removal against a nonexistent file is a no-op.)
    if upserts.is_empty() && !file_exists {
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
///
/// # Rotation interplay invariant (plan §7.1 Open Question — RESOLVED)
///
/// `path` MUST be the **active** file named by `metadata.jsonl_export`
/// (`issues.jsonl`), never a rotated archive (`issues.jsonl.1`, …). The sole
/// caller ([`crate::sync::flush_dirty`]) resolves `path` that way, and rotated
/// archives are owned exclusively by [`crate::rotate::rotate`]. Passing an
/// archive path here would corrupt rotation; it is prevented by construction
/// (single resolution site), not by a runtime guard.
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

    fn issue(id: &str, title: &str) -> Issue {
        Issue::new(id.to_string(), title.to_string(), ".".to_string())
    }

    fn ids_in(path: &Path) -> Vec<String> {
        std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                serde_json::from_str::<serde_json::Value>(l).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn merge_upserts_dirty_and_preserves_untouched() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Seed three beads, then merge in only one changed bead: the other two
        // must survive byte-for-byte (surgical, not full rewrite).
        export_jsonl_merge(&path, &[issue("bf-a", "A"), issue("bf-b", "B"), issue("bf-c", "C")], &[])
            .unwrap();
        let raw_before = std::fs::read_to_string(&path).unwrap();

        let mut changed = issue("bf-b", "B renamed");
        changed.priority = crate::model::Priority(0);
        let result = export_jsonl_merge(&path, &[changed], &[]).unwrap();
        assert_eq!(result.count, 1, "count reports upserts applied");

        assert_eq!(ids_in(&path), vec!["bf-a", "bf-b", "bf-c"], "all beads retained, sorted");
        // bf-a and bf-c lines are untouched relative to the seed write.
        let line_a_before = raw_before.lines().find(|l| l.contains("\"bf-a\"")).unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(after.contains(line_a_before), "untouched bead line preserved verbatim");
        assert!(after.contains("B renamed"), "dirty bead line replaced");
    }

    #[test]
    fn merge_removes_ids() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");
        export_jsonl_merge(&path, &[issue("bf-a", "A"), issue("bf-b", "B"), issue("bf-c", "C")], &[])
            .unwrap();

        // Remove the middle bead; no upserts. count == 0 (no upserts).
        let result = export_jsonl_merge(&path, &[], &[String::from("bf-b")]).unwrap();
        assert_eq!(result.count, 0);
        assert_eq!(ids_in(&path), vec!["bf-a", "bf-c"], "removed id's line pruned");
    }

    #[test]
    fn merge_no_op_on_missing_file_writes_nothing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");
        // No upserts, no removals, no existing file → never create an empty file.
        let result = export_jsonl_merge(&path, &[], &[String::from("bf-x")]).unwrap();
        assert_eq!(result.count, 0);
        assert!(!path.exists(), "a pure no-op must not create an empty JSONL file");
    }

    #[test]
    fn merge_preserves_unparseable_orphan_lines() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");
        std::fs::write(&path, "{\"id\":\"bf-a\",\"title\":\"A\"}\nnot json at all\n").unwrap();

        // Merge a new bead; the foreign/hand-edited line must not be dropped.
        export_jsonl_merge(&path, &[issue("bf-z", "Z")], &[]).unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(after.contains("not json at all"), "orphan line must be preserved");
        assert!(after.contains("\"bf-z\""), "new bead merged in");
    }

    #[test]
    fn labels_are_exported_to_jsonl() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create an issue with labels
        let mut issue = issue("bf-labels", "Test Labels");
        issue.labels = vec!["phase-1".to_string(), "storage".to_string(), "critical".to_string()];

        // Export to JSONL
        export_jsonl_merge(&path, &[issue.clone()], &[]).unwrap();

        // Read back and verify labels are present
        let contents = std::fs::read_to_string(&path).unwrap();
        let parsed: Issue = serde_json::from_str(&contents.trim()).unwrap();
        assert_eq!(parsed.labels, vec!["phase-1", "storage", "critical"]);
    }

    #[test]
    fn labels_roundtrip_through_jsonl() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create an issue with labels
        let mut issue = issue("bf-roundtrip", "Roundtrip Test");
        issue.labels = vec!["label1".to_string(), "label2".to_string(), "label3".to_string()];

        // Export to JSONL
        export_jsonl(&path, || Ok(vec![issue.clone()])).unwrap();

        // Read and parse
        let contents = std::fs::read_to_string(&path).unwrap();
        let parsed: Issue = serde_json::from_str(&contents.trim()).unwrap();

        // Verify labels survived the roundtrip
        assert_eq!(parsed.labels, vec!["label1", "label2", "label3"]);
    }

    #[test]
    fn empty_labels_array_skipped_in_jsonl() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create an issue without labels
        let issue = issue("bf-nolabels", "No Labels");

        // Export to JSONL
        export_jsonl(&path, || Ok(vec![issue.clone()])).unwrap();

        // Read back and verify empty labels array is skipped
        let contents = std::fs::read_to_string(&path).unwrap();
        // Empty arrays should be skipped due to skip_serializing_if
        assert!(!contents.contains("\"labels\""), "empty labels should be skipped in JSON");
    }
}
