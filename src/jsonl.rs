use crate::error::Result;
use crate::model::Issue;
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

#[derive(Debug)]
pub struct ImportResult {
    pub imported: usize,
    pub updated: usize,
    pub skipped: usize,
}

#[derive(Debug)]
pub struct ExportResult {
    pub count: usize,
}

/// Result type for incremental_flush that includes warnings for failures.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FlushResult {
    /// Number of beads successfully flushed
    pub flushed: usize,
    /// Warnings about failures (non-empty if flush partially failed)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
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
    let mut issues = list_all()?;
    // Sort by ID for stable diffs
    issues.sort_by(|a, b| a.id.cmp(&b.id));
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

/// Query the dirty_issues table and return all bead_ids.
///
/// This reads the dirty_issues table into a Vec<String> using a prepared statement.
/// The IDs are sorted by marked_at timestamp (oldest first) to provide predictable ordering.
pub fn get_dirty_issue_ids(conn: &rusqlite::Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare_cached(
        "SELECT bead_id FROM dirty_issues ORDER BY marked_at ASC"
    )?;

    let mut rows = stmt.query([])?;
    let mut ids = Vec::new();
    while let Some(row) = rows.next()? {
        ids.push(row.get(0)?);
    }

    Ok(ids)
}

/// Incremental flush that only writes dirty beads to JSONL.
/// This is the main entry point for auto-flush functionality.
pub fn incremental_flush(storage: &crate::storage::sqlite::Storage, path: &Path) -> Result<FlushResult> {
    use crate::storage::sqlite::Storage;

    // Query dirty issue IDs using Storage's query_dirty_issues method
    let dirty_ids = storage.query_dirty_issues()?;

    // Early return if no dirty issues
    if dirty_ids.is_empty() {
        return Ok(FlushResult {
            flushed: 0,
            warnings: Vec::new(),
        });
    }

    // List dirty issues from database
    let list_dirty = || -> Result<Vec<crate::model::Issue>> {
        let conn = storage.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT i.id, i.content_hash, i.title, i.description, i.design, i.acceptance_criteria, i.notes,
                    i.status, i.priority, i.issue_type, i.assignee, i.owner, i.estimated_minutes,
                    i.created_at, i.created_by, i.updated_at, i.closed_at, i.close_reason,
                    i.closed_by_session, i.due_at, i.defer_until, i.external_ref, i.source_system,
                    i.source_repo, i.deleted_at, i.deleted_by, i.delete_reason, i.original_type,
                    i.compaction_level, i.compacted_at, i.compacted_at_commit, i.original_size,
                    i.sender, i.ephemeral, i.pinned, i.is_template,
                    GROUP_CONCAT(bl.label) AS labels
             FROM issues i
             INNER JOIN dirty_issues d ON i.id = d.bead_id
             LEFT JOIN bead_labels bl ON i.id = bl.bead_id
             GROUP BY i.id
             ORDER BY i.id",
        )?;
        let mut rows = stmt.query([])?;
        let mut issues = Vec::new();
        while let Some(row) = rows.next()? {
            issues.push(Storage::row_to_issue_conn(&conn, row)?);
        }
        drop(rows);
        drop(stmt);
        drop(conn);
        Ok(issues)
    };

    // Listing itself is unrecoverable if it fails (nothing to export), so it
    // still propagates via `?`; only the write step below degrades to a warning.
    let issues = list_dirty()?;

    // Export failures degrade to a warning with flushed=0 rather than
    // propagating: a transient write failure (e.g. destination path
    // temporarily unwritable) should not crash the caller, and dirty marks
    // must stay set so the next flush attempt retries these beads.
    let flushed = match export_jsonl_merge(path, &issues, &[]) {
        Ok(result) => result.count,
        Err(e) => {
            return Ok(FlushResult {
                flushed: 0,
                warnings: vec![format!("JSONL export failed: {e}")],
            });
        }
    };

    // The export itself succeeded, so clear dirty marks — but a failure here
    // must not be reported as an export failure (the beads did flush) and
    // must not silently look like success either, since the dirty marks
    // staying set means these beads will be re-exported next time.
    let mut warnings = Vec::new();
    {
        let conn = storage.conn.lock().unwrap();
        if let Err(e) = conn.execute("DELETE FROM dirty_issues", []) {
            warnings.push(format!("failed to clear dirty marks: {e}"));
        }
    }

    Ok(FlushResult { flushed, warnings })
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
        export_jsonl_merge(
            &path,
            &[issue("bf-a", "A"), issue("bf-b", "B"), issue("bf-c", "C")],
            &[],
        )
        .unwrap();
        let raw_before = std::fs::read_to_string(&path).unwrap();

        let mut changed = issue("bf-b", "B renamed");
        changed.priority = crate::model::Priority(0);
        let result = export_jsonl_merge(&path, &[changed], &[]).unwrap();
        assert_eq!(result.count, 1, "count reports upserts applied");

        assert_eq!(
            ids_in(&path),
            vec!["bf-a", "bf-b", "bf-c"],
            "all beads retained, sorted"
        );
        // bf-a and bf-c lines are untouched relative to the seed write.
        let line_a_before = raw_before.lines().find(|l| l.contains("\"bf-a\"")).unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(
            after.contains(line_a_before),
            "untouched bead line preserved verbatim"
        );
        assert!(after.contains("B renamed"), "dirty bead line replaced");
    }

    #[test]
    fn merge_removes_ids() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");
        export_jsonl_merge(
            &path,
            &[issue("bf-a", "A"), issue("bf-b", "B"), issue("bf-c", "C")],
            &[],
        )
        .unwrap();

        // Remove the middle bead; no upserts. count == 0 (no upserts).
        let result = export_jsonl_merge(&path, &[], &[String::from("bf-b")]).unwrap();
        assert_eq!(result.count, 0);
        assert_eq!(
            ids_in(&path),
            vec!["bf-a", "bf-c"],
            "removed id's line pruned"
        );
    }

    #[test]
    fn merge_no_op_on_missing_file_writes_nothing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");
        // No upserts, no removals, no existing file → never create an empty file.
        let result = export_jsonl_merge(&path, &[], &[String::from("bf-x")]).unwrap();
        assert_eq!(result.count, 0);
        assert!(
            !path.exists(),
            "a pure no-op must not create an empty JSONL file"
        );
    }

    #[test]
    fn merge_preserves_unparseable_orphan_lines() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");
        std::fs::write(
            &path,
            "{\"id\":\"bf-a\",\"title\":\"A\"}\nnot json at all\n",
        )
        .unwrap();

        // Merge a new bead; the foreign/hand-edited line must not be dropped.
        export_jsonl_merge(&path, &[issue("bf-z", "Z")], &[]).unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(
            after.contains("not json at all"),
            "orphan line must be preserved"
        );
        assert!(after.contains("\"bf-z\""), "new bead merged in");
    }

    #[test]
    fn labels_are_exported_to_jsonl() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create an issue with labels
        let mut issue = issue("bf-labels", "Test Labels");
        issue.labels = vec![
            "phase-1".to_string(),
            "storage".to_string(),
            "critical".to_string(),
        ];

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
        issue.labels = vec![
            "label1".to_string(),
            "label2".to_string(),
            "label3".to_string(),
        ];

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
        assert!(
            !contents.contains("\"labels\""),
            "empty labels should be skipped in JSON"
        );
    }

    #[test]
    fn debug_label_export_import() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create an issue with labels
        let mut issue = issue("bf-debug", "Debug Label Test");
        issue.labels = vec!["auto-flushed".to_string(), "test-label".to_string()];

        println!("Before export - issue.labels: {:?}", issue.labels);

        // Export to JSONL using merge (same as auto-flush)
        export_jsonl_merge(&path, &[issue.clone()], &[]).unwrap();

        // Read back the JSONL contents
        let contents = std::fs::read_to_string(&path).unwrap();
        println!("JSONL contents: {}", contents);

        // Parse the JSONL
        let parsed: Issue = serde_json::from_str(&contents.trim()).unwrap();
        println!("After import - parsed.labels: {:?}", parsed.labels);

        // Verify labels are preserved
        assert_eq!(parsed.labels.len(), 2, "Should have 2 labels");
        assert!(
            parsed.labels.contains(&"auto-flushed".to_string()),
            "Should contain 'auto-flushed' label"
        );
        assert!(
            parsed.labels.contains(&"test-label".to_string()),
            "Should contain 'test-label' label"
        );
    }

    #[test]
    fn export_jsonl_writes_multiple_beads_sorted() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create beads in random order
        let beads = vec![
            issue("bf-z", "Zebra"),
            issue("bf-a", "Apple"),
            issue("bf-m", "Middle"),
        ];

        // Export using export_jsonl
        let result = export_jsonl(&path, || Ok(beads.clone())).unwrap();
        assert_eq!(result.count, 3, "should export all 3 beads");

        // Verify file exists and has content
        assert!(path.exists(), "export file should exist");
        let contents = std::fs::read_to_string(&path).unwrap();

        // Verify output is sorted by ID (alphabetically)
        let ids = ids_in(&path);
        assert_eq!(
            ids,
            vec!["bf-a", "bf-m", "bf-z"],
            "output should be sorted by ID"
        );

        // Verify all beads are present and valid JSON
        let lines: Vec<&str> = contents.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 3, "should have 3 lines");

        for line in lines {
            let parsed: Issue = serde_json::from_str(line).unwrap();
            assert!(parsed.id == "bf-a" || parsed.id == "bf-m" || parsed.id == "bf-z");
        }
    }

    #[test]
    fn export_jsonl_atomic_temp_rename() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create beads
        let beads = vec![issue("bf-test", "Test Issue")];

        // Export using export_jsonl
        export_jsonl(&path, || Ok(beads.clone())).unwrap();

        // Verify atomic behavior: temp file should not exist after successful export
        let temp_path = path.with_extension("jsonl.tmp");
        assert!(
            !temp_path.exists(),
            "temp file should be cleaned up after atomic rename"
        );

        // Verify final file exists
        assert!(path.exists(), "final file should exist");

        // Verify content is correct
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(
            contents.contains("bf-test"),
            "final file should contain the bead"
        );
    }

    #[test]
    fn export_jsonl_dirty_only_exports_modified_beads() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // First, export all beads using export_jsonl
        let all_beads = vec![
            issue("bf-1", "First"),
            issue("bf-2", "Second"),
            issue("bf-3", "Third"),
        ];
        export_jsonl(&path, || Ok(all_beads.clone())).unwrap();
        let before_export = std::fs::read_to_string(&path).unwrap();

        // Now, export only dirty beads using export_jsonl_dirty
        let dirty_beads = vec![issue("bf-2", "Second Modified")];
        let mut clear_called = false;
        let result = export_jsonl_dirty(
            &path,
            || Ok(dirty_beads.clone()),
            || {
                clear_called = true;
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(result.count, 1, "should report 1 dirty bead exported");
        assert!(
            clear_called,
            "clear_dirty should be called after successful export"
        );

        // Verify final state: all beads present, only bf-2 modified
        let after_export = std::fs::read_to_string(&path).unwrap();
        let ids = ids_in(&path);
        assert_eq!(
            ids,
            vec!["bf-1", "bf-2", "bf-3"],
            "all beads should be present"
        );
        assert!(
            after_export.contains("Second Modified"),
            "modified bead should be updated"
        );
        assert!(
            !after_export.contains("Second\n"),
            "old version should be replaced"
        );

        // Verify other beads preserved byte-for-byte (surgical update)
        assert!(after_export.contains("First"), "bf-1 should be preserved");
        assert!(after_export.contains("Third"), "bf-3 should be preserved");
    }

    #[test]
    fn export_jsonl_dirty_no_op_when_no_dirty_beads() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create initial file
        let beads = vec![issue("bf-1", "Initial")];
        export_jsonl(&path, || Ok(beads.clone())).unwrap();
        let before = std::fs::read_to_string(&path).unwrap();

        // Export with no dirty beads
        let mut clear_called = false;
        let result = export_jsonl_dirty(
            &path,
            || Ok(vec![]), // no dirty beads
            || {
                clear_called = true;
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(result.count, 0, "should report 0 beads exported");
        assert!(
            !clear_called,
            "clear_dirty should NOT be called when no dirty beads"
        );

        // Verify file unchanged
        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            before, after,
            "file should be unchanged when no dirty beads"
        );
    }

    #[test]
    fn export_jsonl_dirty_atomic_behavior() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Export dirty beads
        let dirty_beads = vec![issue("bf-dirty", "Dirty Bead")];
        export_jsonl_dirty(&path, || Ok(dirty_beads.clone()), || Ok(())).unwrap();

        // Verify atomic behavior: temp file should not exist after successful export
        let temp_path = path.with_extension("jsonl.tmp");
        assert!(
            !temp_path.exists(),
            "temp file should be cleaned up after atomic rename"
        );

        // Verify final file exists
        assert!(path.exists(), "final file should exist");
    }

    #[test]
    fn export_jsonl_preserves_stable_ordering() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Export in reverse order
        let beads_reverse = vec![
            issue("bf-3", "Third"),
            issue("bf-2", "Second"),
            issue("bf-1", "First"),
        ];
        export_jsonl(&path, || Ok(beads_reverse.clone())).unwrap();

        // Verify sorted order after first export
        let ids = ids_in(&path);
        assert_eq!(
            ids,
            vec!["bf-1", "bf-2", "bf-3"],
            "beads should be sorted by ID"
        );

        // Export again in different order
        let beads_forward = vec![
            issue("bf-1", "First"),
            issue("bf-2", "Second"),
            issue("bf-3", "Third"),
        ];
        export_jsonl(&path, || Ok(beads_forward.clone())).unwrap();

        // Verify sorted order after second export (regardless of input order, output is sorted)
        let ids = ids_in(&path);
        assert_eq!(
            ids,
            vec!["bf-1", "bf-2", "bf-3"],
            "beads should be sorted by ID"
        );

        // Verify both exports contain the same IDs (regardless of timestamps)
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("bf-1"), "should contain bf-1");
        assert!(contents.contains("bf-2"), "should contain bf-2");
        assert!(contents.contains("bf-3"), "should contain bf-3");
    }

    #[test]
    fn export_jsonl_empty_list() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Export empty list
        let result = export_jsonl(&path, || Ok(vec![])).unwrap();
        assert_eq!(result.count, 0, "should report 0 beads");

        // File should still exist (empty file is valid)
        assert!(path.exists(), "file should exist even when empty");

        // But should have no content
        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents.trim(), "", "file should be empty");
    }

    // ==================== import_jsonl tests ====================

    #[test]
    fn import_jsonl_valid_multiple_beads() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create a JSONL file with multiple valid beads
        let jsonl_content = r#"{"id":"bf-001","title":"First bead","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}
{"id":"bf-002","title":"Second bead","status":"open","priority":1,"type":"bug","created_at":"2024-01-02T00:00:00Z","updated_at":"2024-01-02T00:00:00Z","source_repo":"test"}
{"id":"bf-003","title":"Third bead","status":"open","priority":0,"type":"feature","created_at":"2024-01-03T00:00:00Z","updated_at":"2024-01-03T00:00:00Z","source_repo":"test"}"#;
        std::fs::write(&path, jsonl_content).unwrap();

        // Track which beads were upserted
        let mut imported_ids = Vec::new();
        let mut upsert_called = Vec::new();

        let result = import_jsonl(&path, |issue| {
            upsert_called.push(issue.id.clone());
            // Simulate all beads as new
            imported_ids.push(issue.id.clone());
            Ok(UpsertResult::New)
        })
        .unwrap();

        assert_eq!(result.imported, 3, "should import 3 new beads");
        assert_eq!(result.updated, 0, "should not update any beads");
        assert_eq!(result.skipped, 0, "should not skip any beads");
        assert_eq!(
            upsert_called,
            vec!["bf-001", "bf-002", "bf-003"],
            "upsert should be called for each bead in order"
        );
    }

    #[test]
    fn import_jsonl_malformed_json_skip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create a JSONL file with malformed JSON lines
        let jsonl_content = r#"{"id":"bf-001","title":"Valid bead","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}
{"id":"bf-002","title":"Invalid JSON","status":"open","priority":1,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"
{"id":"bf-003","title":"Another valid","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}
not json at all
{"id":"bf-004","title":"Last valid","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}"#;
        std::fs::write(&path, jsonl_content).unwrap();

        let mut valid_count = 0;
        let result = import_jsonl(&path, |issue| {
            valid_count += 1;
            Ok(UpsertResult::New)
        });

        // Import should fail on malformed JSON
        assert!(
            result.is_err(),
            "import_jsonl should return error for malformed JSON"
        );

        // Even though it failed, some valid beads might have been processed before the error
        // This is the expected behavior - the function stops at the first error
    }

    #[test]
    fn import_jsonl_missing_required_field_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create a JSONL file with beads missing required fields
        // Missing: created_at, updated_at, source_repo
        let jsonl_content = r#"{"id":"bf-001","title":"Missing required fields","status":"open","priority":2,"type":"task"}"#;
        std::fs::write(&path, jsonl_content).unwrap();

        let result = import_jsonl(&path, |_issue| Ok(UpsertResult::New));

        // Should fail to deserialize due to missing required fields
        assert!(
            result.is_err(),
            "import_jsonl should return error when required fields are missing"
        );
        // Check error message without unwrap_err
        match result {
            Err(e) => {
                let err_msg = e.to_string();
                assert!(
                    err_msg.contains("created_at")
                        || err_msg.contains("missing field")
                        || err_msg.contains("missing"),
                    "error should mention the missing field: {}",
                    err_msg
                );
            }
            Ok(_) => panic!("Expected error but got Ok result"),
        }
    }

    #[test]
    fn import_jsonl_upsert_behavior_update_existing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create a JSONL file with beads
        let jsonl_content = r#"{"id":"bf-001","title":"Original title","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}
{"id":"bf-002","title":"Unchanged bead","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}"#;
        std::fs::write(&path, jsonl_content).unwrap();

        // Simulate a database with existing beads
        let mut existing_db = std::collections::HashMap::new();
        existing_db.insert("bf-001".to_string(), "Existing content for bf-001");
        existing_db.insert("bf-002".to_string(), "Existing content for bf-002");

        let result = import_jsonl(&path, |issue| {
            if existing_db.contains_key(&issue.id) {
                // Check if content changed
                let old_content = existing_db.get(&issue.id).unwrap();
                if old_content != &issue.title {
                    Ok(UpsertResult::Updated)
                } else {
                    Ok(UpsertResult::Unchanged)
                }
            } else {
                Ok(UpsertResult::New)
            }
        })
        .unwrap();

        // Both beads exist in "database"
        // bf-001 title changed (from "Existing content for bf-001" to "Original title")
        // bf-002 title changed (from "Existing content for bf-002" to "Unchanged bead")
        assert_eq!(result.imported, 0, "should not import new beads");
        assert_eq!(result.updated, 2, "should update both existing beads");
        assert_eq!(result.skipped, 0, "should not skip any beads");
    }

    #[test]
    fn import_jsonl_upsert_behavior_mixed() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create a JSONL file with multiple beads
        let jsonl_content = r#"{"id":"bf-001","title":"New bead","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}
{"id":"bf-002","title":"Update me","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}
{"id":"bf-003","title":"Keep same","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}"#;
        std::fs::write(&path, jsonl_content).unwrap();

        // Simulate a database with some existing beads
        let mut existing_db = std::collections::HashMap::new();
        existing_db.insert("bf-002".to_string(), "Old title for bf-002");
        existing_db.insert("bf-003".to_string(), "Keep same"); // Same title, should skip

        let result = import_jsonl(&path, |issue| {
            if let Some(existing_title) = existing_db.get(&issue.id) {
                if existing_title == &issue.title {
                    Ok(UpsertResult::Unchanged)
                } else {
                    Ok(UpsertResult::Updated)
                }
            } else {
                Ok(UpsertResult::New)
            }
        })
        .unwrap();

        assert_eq!(result.imported, 1, "should import 1 new bead (bf-001)");
        assert_eq!(result.updated, 1, "should update 1 bead (bf-002)");
        assert_eq!(result.skipped, 1, "should skip 1 bead (bf-003)");
    }

    #[test]
    fn import_jsonl_empty_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create an empty file
        std::fs::write(&path, "").unwrap();

        let mut call_count = 0;
        let result = import_jsonl(&path, |_issue| {
            call_count += 1;
            Ok(UpsertResult::New)
        })
        .unwrap();

        assert_eq!(call_count, 0, "upsert should not be called for empty file");
        assert_eq!(result.imported, 0, "should import 0 beads");
        assert_eq!(result.updated, 0, "should update 0 beads");
        assert_eq!(result.skipped, 0, "should skip 0 beads");
    }

    #[test]
    fn import_jsonl_upsert_propagates_errors() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create a JSONL file with valid beads
        let jsonl_content = r#"{"id":"bf-001","title":"Valid bead","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}"#;
        std::fs::write(&path, jsonl_content).unwrap();

        // Simulate an upsert function that fails
        let result = import_jsonl(&path, |_issue| {
            Err(anyhow::anyhow!("Database error").into())
        });

        assert!(
            result.is_err(),
            "import_jsonl should propagate upsert errors"
        );
        match result {
            Err(e) => {
                let err_msg = e.to_string();
                assert!(
                    err_msg.contains("Database error"),
                    "error message should contain the upsert error: {}",
                    err_msg
                );
            }
            Ok(_) => panic!("Expected error but got Ok result"),
        }
    }

    #[test]
    fn import_jsonl_single_bead() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create a JSONL file with a single bead
        let jsonl_content = r#"{"id":"bf-single","title":"Single bead","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}"#;
        std::fs::write(&path, jsonl_content).unwrap();

        let result = import_jsonl(&path, |issue| {
            assert_eq!(issue.id, "bf-single");
            assert_eq!(issue.title, "Single bead");
            Ok(UpsertResult::New)
        })
        .unwrap();

        assert_eq!(result.imported, 1, "should import 1 bead");
        assert_eq!(result.updated, 0, "should not update any beads");
        assert_eq!(result.skipped, 0, "should not skip any beads");
    }

    #[test]
    fn import_jsonl_with_extra_fields() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create a JSONL file with beads that have extra optional fields
        let jsonl_content = r#"{"id":"bf-extra","title":"Bead with extras","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test","description":"This is a description","assignee":"testuser","labels":["bug","critical"]}"#;
        std::fs::write(&path, jsonl_content).unwrap();

        let result = import_jsonl(&path, |issue| {
            assert_eq!(issue.id, "bf-extra");
            assert_eq!(issue.description, Some("This is a description".to_string()));
            assert_eq!(issue.assignee, Some("testuser".to_string()));
            assert!(issue.labels.contains(&"bug".to_string()));
            assert!(issue.labels.contains(&"critical".to_string()));
            Ok(UpsertResult::New)
        })
        .unwrap();

        assert_eq!(result.imported, 1, "should import bead with extra fields");
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn import_jsonl_with_only_whitespace() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create a JSONL file with only whitespace
        std::fs::write(&path, "   \n\n  \t  \n").unwrap();

        let result = import_jsonl(&path, |_issue| Ok(UpsertResult::New));

        // Should fail - import_jsonl doesn't skip blank lines, it tries to parse them
        assert!(
            result.is_err(),
            "import_jsonl should fail on whitespace-only lines (no valid JSON)"
        );
    }

    #[test]
    fn import_jsonl_with_blank_lines() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create a JSONL file with blank lines between beads
        let jsonl_content = r#"{"id":"bf-001","title":"First","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}

{"id":"bf-002","title":"Second","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}

{"id":"bf-003","title":"Third","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}"#;
        std::fs::write(&path, jsonl_content).unwrap();

        let result = import_jsonl(&path, |issue| {
            assert!(!issue.id.is_empty());
            Ok(UpsertResult::New)
        });

        // Should fail - blank lines cause parse errors
        assert!(
            result.is_err(),
            "import_jsonl should fail on blank lines (tries to parse them as JSON)"
        );
    }

    #[test]
    fn import_jsonl_with_comment_like_lines() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create a JSONL file with comment-like lines (not standard JSONL, but testing robustness)
        let jsonl_content = r#"# This is a comment
{"id":"bf-001","title":"First","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}
// Another comment style
{"id":"bf-002","title":"Second","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}
{"id":"bf-003","title":"Third","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}"#;
        std::fs::write(&path, jsonl_content).unwrap();

        let result = import_jsonl(&path, |issue| Ok(UpsertResult::New));

        // Comment-like lines will cause parse errors - this is expected behavior
        assert!(
            result.is_err(),
            "import_jsonl should fail on non-JSON comment-like lines"
        );
    }

    #[test]
    fn import_jsonl_unicode_characters() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create beads with various Unicode characters
        let jsonl_content = r#"{"id":"bf-unicode-emoji","title":"Test with emoji 🎉🚀","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test","description":"Testing emoji: 😀🎨🔥"}
{"id":"bf-unicode-cjk","title":"Test with CJK 中文日本語한국어","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test","description":"Testing CJK characters"}
{"id":"bf-unicode-arabic","title":"Test with Arabic العربية","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test","description":"Testing Arabic"}
{"id":"bf-unicode-cyrillic","title":"Test with Cyrillic русский","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test","description":"Testing Cyrillic"}
{"id":"bf-unicode-mixed","title":"Test mixed 🎨中文🚀عربي","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test","description":"Mixed script"}"#;
        std::fs::write(&path, jsonl_content).unwrap();

        let result = import_jsonl(&path, |issue| {
            // Verify Unicode characters are preserved
            match issue.id.as_str() {
                "bf-unicode-emoji" => {
                    assert!(issue.title.contains("🎉"));
                    assert!(issue.title.contains("🚀"));
                    assert!(issue.description.as_ref().unwrap().contains("😀"));
                }
                "bf-unicode-cjk" => {
                    assert!(issue.title.contains("中文"));
                    assert!(issue.title.contains("日本語"));
                    assert!(issue.title.contains("한국어"));
                }
                "bf-unicode-arabic" => {
                    assert!(issue.title.contains("العربية"));
                }
                "bf-unicode-cyrillic" => {
                    assert!(issue.title.contains("русский"));
                }
                "bf-unicode-mixed" => {
                    assert!(issue.title.contains("🎨"));
                    assert!(issue.title.contains("中文"));
                    assert!(issue.title.contains("🚀"));
                    assert!(issue.title.contains("عربي"));
                }
                _ => panic!("Unexpected bead ID: {}", issue.id),
            }
            Ok(UpsertResult::New)
        })
        .unwrap();

        assert_eq!(result.imported, 5, "should import all 5 Unicode beads");
    }

    #[test]
    fn import_jsonl_special_characters() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create beads with special characters that need escaping
        let jsonl_content = r#"{"id":"bf-special-quotes","title":"Test with \"quotes\" and 'apostrophes'","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}
{"id":"bf-special-backslash","title":"Test with backslash \\ and forwardslash /","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}
{"id":"bf-special-newlines","title":"Test with newlines\nand\ttabs","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}
{"id":"bf-special-unicode-escape","title":"Test with unicode escape ❤❤❤","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}"#;
        std::fs::write(&path, jsonl_content).unwrap();

        let result = import_jsonl(&path, |issue| {
            // Verify special characters are properly unescaped
            match issue.id.as_str() {
                "bf-special-quotes" => {
                    assert!(issue.title.contains("\"quotes\""));
                    assert!(issue.title.contains("'apostrophes'"));
                }
                "bf-special-backslash" => {
                    assert!(issue.title.contains("\\"));
                    assert!(issue.title.contains("/"));
                }
                "bf-special-newlines" => {
                    assert!(issue.title.contains('\n'));
                    assert!(issue.title.contains('\t'));
                }
                "bf-special-unicode-escape" => {
                    // Unicode escape sequences should be converted to actual characters
                    assert!(issue.title.contains('❤'));
                }
                _ => panic!("Unexpected bead ID: {}", issue.id),
            }
            Ok(UpsertResult::New)
        })
        .unwrap();

        assert_eq!(
            result.imported, 4,
            "should import all 4 special character beads"
        );
    }

    #[test]
    fn import_jsonl_very_long_description() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create a bead with a very long description (10KB)
        let long_description = "x".repeat(10_000);
        let jsonl_content = format!(
            r#"{{"id":"bf-long","title":"Long description","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test","description":"{}"}}"#,
            long_description
        );
        std::fs::write(&path, jsonl_content).unwrap();

        let result = import_jsonl(&path, |issue| {
            assert_eq!(issue.id, "bf-long");
            assert_eq!(issue.description, Some(long_description.clone()));
            Ok(UpsertResult::New)
        })
        .unwrap();

        assert_eq!(
            result.imported, 1,
            "should import bead with very long description"
        );
    }

    #[test]
    fn import_jsonl_very_long_title() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create a bead with a very long title (1KB - reasonable limit)
        let long_title = "y".repeat(1_000);
        let jsonl_content = format!(
            r#"{{"id":"bf-long-title","title":"{}","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}}"#,
            long_title
        );
        std::fs::write(&path, jsonl_content).unwrap();

        let result = import_jsonl(&path, |issue| {
            assert_eq!(issue.id, "bf-long-title");
            assert_eq!(issue.title, long_title);
            Ok(UpsertResult::New)
        })
        .unwrap();

        assert_eq!(
            result.imported, 1,
            "should import bead with very long title"
        );
    }

    #[test]
    fn export_jsonl_unicode_characters() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create beads with various Unicode characters
        let mut bead_emoji = issue("bf-emoji", "Emoji test 🎉🚀");
        bead_emoji.description = Some("Description with emoji: 😀🎨🔥".to_string());

        let mut bead_cjk = issue("bf-cjk", "CJK test 中文");
        bead_cjk.description = Some("Japanese 日本語 and Korean 한국어".to_string());

        let beads = vec![bead_emoji, bead_cjk];

        // Export to JSONL
        export_jsonl(&path, || Ok(beads.clone())).unwrap();

        // Read back and verify Unicode is preserved
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("🎉"));
        assert!(contents.contains("🚀"));
        assert!(contents.contains("😀"));
        assert!(contents.contains("中文"));
        assert!(contents.contains("日本語"));
        assert!(contents.contains("한국어"));

        // Verify it can be re-imported
        let reimport_result = import_jsonl(&path, |issue| {
            match issue.id.as_str() {
                "bf-emoji" => {
                    assert!(issue.title.contains("🎉"));
                    assert!(issue.description.as_ref().unwrap().contains("😀"));
                }
                "bf-cjk" => {
                    assert!(issue.title.contains("中文"));
                    assert!(issue.description.as_ref().unwrap().contains("日本語"));
                }
                _ => panic!("Unexpected bead ID: {}", issue.id),
            }
            Ok(UpsertResult::New)
        })
        .unwrap();

        assert_eq!(
            reimport_result.imported, 2,
            "should re-import both Unicode beads"
        );
    }

    #[test]
    fn export_jsonl_special_characters() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create beads with special characters that need proper escaping
        let mut bead_quotes = issue("bf-quotes", "Test \"quotes\"");
        bead_quotes.description = Some("Description with 'apostrophes' and \"quotes\"".to_string());

        let mut bead_newlines = issue("bf-newlines", "Newline test");
        bead_newlines.description = Some("Line 1\nLine 2\tTabbed".to_string());

        let beads = vec![bead_quotes, bead_newlines];

        // Export to JSONL
        export_jsonl(&path, || Ok(beads.clone())).unwrap();

        // Read back and verify special characters are properly escaped
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("\\\"")); // Escaped quotes
        assert!(contents.contains("\\n")); // Escaped newline
        assert!(contents.contains("\\t")); // Escaped tab

        // Verify it can be re-imported and characters are preserved
        let reimport_result = import_jsonl(&path, |issue| {
            match issue.id.as_str() {
                "bf-quotes" => {
                    assert!(issue.title.contains('"'));
                    assert!(issue.description.as_ref().unwrap().contains('\''));
                    assert!(issue.description.as_ref().unwrap().contains('"'));
                }
                "bf-newlines" => {
                    assert!(issue.description.as_ref().unwrap().contains('\n'));
                    assert!(issue.description.as_ref().unwrap().contains('\t'));
                }
                _ => panic!("Unexpected bead ID: {}", issue.id),
            }
            Ok(UpsertResult::New)
        })
        .unwrap();

        assert_eq!(
            reimport_result.imported, 2,
            "should re-import both special character beads"
        );
    }

    #[test]
    fn export_jsonl_very_long_fields() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create a bead with very long fields
        let long_title = "T".repeat(1_000);
        let long_description = "D".repeat(50_000); // 50KB description

        let mut bead = issue("bf-long", &long_title);
        bead.description = Some(long_description);

        let beads = vec![bead];

        // Export to JSONL
        let result = export_jsonl(&path, || Ok(beads.clone())).unwrap();
        assert_eq!(result.count, 1, "should export 1 bead with long fields");

        // Read back and verify long fields are preserved
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.len() > 50_000, "file should be large (>50KB)");

        // Verify it can be re-imported
        let reimport_result = import_jsonl(&path, |issue| {
            assert_eq!(issue.id, "bf-long");
            assert_eq!(issue.title.len(), 1_000);
            assert_eq!(issue.description.as_ref().unwrap().len(), 50_000);
            Ok(UpsertResult::New)
        })
        .unwrap();

        assert_eq!(
            reimport_result.imported, 1,
            "should re-import bead with long fields"
        );
    }

    #[test]
    fn export_jsonl_concurrent_scenarios() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create a shared counter and error vector
        let export_count = Arc::new(Mutex::new(0));
        let errors = Arc::new(Mutex::new(Vec::new()));

        // Spawn multiple threads attempting concurrent exports
        let mut handles = vec![];
        for i in 0..5 {
            let path_clone = path.clone();
            let count_clone = Arc::clone(&export_count);
            let errors_clone = Arc::clone(&errors);

            let handle = thread::spawn(move || {
                let bead = issue(
                    &format!("bf-concurrent-{}", i),
                    &format!("Concurrent bead {}", i),
                );
                let beads = vec![bead];

                match export_jsonl(&path_clone, || Ok(beads.clone())) {
                    Ok(_) => {
                        let mut count = count_clone.lock().unwrap();
                        *count += 1;
                    }
                    Err(e) => {
                        let mut err_vec = errors_clone.lock().unwrap();
                        err_vec.push(format!("Thread {} failed: {}", i, e));
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify results - some exports should succeed, some might fail due to concurrent writes
        let success_count = *export_count.lock().unwrap();
        let error_list = errors.lock().unwrap();

        // At least one export should succeed
        assert!(success_count > 0, "at least one export should succeed");

        // Verify final file is valid (can be imported)
        if path.exists() {
            let contents = std::fs::read_to_string(&path).unwrap();
            let has_content = !contents.trim().is_empty();
            if has_content {
                let reimport_result = import_jsonl(&path, |_issue| Ok(UpsertResult::New)).unwrap();
                // Should have at least one valid bead
                assert!(
                    reimport_result.imported >= 1,
                    "final file should contain at least one valid bead"
                );
            }
        }

        println!(
            "Concurrent exports: {} succeeded, {} failed",
            success_count,
            error_list.len()
        );
        if !error_list.is_empty() {
            println!("Errors: {:?}", error_list);
        }
    }

    #[test]
    fn export_jsonl_permission_error() {
        use std::fs;

        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("readonly.jsonl");

        // Create a parent directory with read-only permissions
        let parent_dir = tmp.path().join("readonly_dir");
        fs::create_dir(&parent_dir).unwrap();
        let readonly_path = parent_dir.join("issues.jsonl");

        // On Unix systems, remove write permissions from the directory
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&parent_dir).unwrap().permissions();
            perms.set_mode(0o444); // Read-only
            fs::set_permissions(&parent_dir, perms).unwrap();
        }

        // Attempt to export to the read-only directory
        let bead = issue("bf-perm-test", "Permission test");
        let beads = vec![bead];

        let result = export_jsonl(&readonly_path, || Ok(beads.clone()));

        #[cfg(unix)]
        {
            // Restore permissions for cleanup
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&parent_dir).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&parent_dir, perms).unwrap();
        }

        // root (uid 0, e.g. the bead-forge-build CI container) bypasses Unix
        // permission bits entirely, so the write can legitimately succeed
        // there even though it must fail for a non-privileged user. Detect
        // that case from the actual outcome rather than asserting a specific
        // uid API (keeps this dependency-free) and skip instead of failing.
        if result.is_ok() {
            eprintln!(
                "export_jsonl_permission_error: write to a 0o444 directory succeeded \
                 (likely running as root, which ignores permission bits) — skipping \
                 the permission-failure assertion rather than falsely failing"
            );
            return;
        }

        // Should fail with permission error
        assert!(
            result.is_err(),
            "export should fail when directory is read-only"
        );
    }

    #[test]
    fn export_jsonl_to_directory_path() {
        use std::fs;

        let tmp = tempfile::TempDir::new().unwrap();
        let dir_path = tmp.path().join("this_is_a_directory");
        fs::create_dir(&dir_path).unwrap();

        // Attempt to export to a directory path (not a file)
        let bead = issue("bf-dir-test", "Directory path test");
        let beads = vec![bead];

        let result = export_jsonl(&dir_path, || Ok(beads.clone()));

        // Should fail - can't write to a directory
        assert!(
            result.is_err(),
            "export should fail when path is a directory"
        );
    }

    #[test]
    fn import_jsonl_truncated_last_line() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create a JSONL file with a truncated last line
        let jsonl_content = r#"{"id":"bf-001","title":"Valid","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}
{"id":"bf-002","title":"Truncated","status":"open""#; // Missing closing brace and other fields
        std::fs::write(&path, jsonl_content).unwrap();

        let result = import_jsonl(&path, |issue| Ok(UpsertResult::New));

        // Should fail on truncated line
        assert!(result.is_err(), "import should fail on truncated JSON");
    }

    #[test]
    fn import_jsonl_with_bom() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create a JSONL file with UTF-8 BOM (Byte Order Mark)
        let mut data = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
        data.extend_from_slice(
            br#"{"id":"bf-bom","title":"BOM test","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":"test"}"#,
        );
        std::fs::write(&path, data).unwrap();

        let result = import_jsonl(&path, |issue| {
            assert_eq!(issue.id, "bf-bom");
            Ok(UpsertResult::New)
        });

        // The BOM might cause parsing issues depending on how serde handles it
        // This test documents current behavior
        if result.is_ok() {
            assert_eq!(
                result.unwrap().imported,
                1,
                "should handle BOM and import bead"
            );
        } else {
            // If it fails, that's also acceptable behavior - BOM is not standard in JSONL
            println!("BOM handling: import failed (this is acceptable)");
        }
    }

    #[test]
    fn export_jsonl_merge_preserves_unicode_comments() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Create initial file with a bead
        let mut initial_bead = issue("bf-1", "Initial");
        initial_bead.description = Some("Initial description 🎨".to_string());
        export_jsonl_merge(&path, &[initial_bead], &[]).unwrap();

        // Add a "comment" line manually (non-JSON line that gets preserved)
        let mut contents = std::fs::read_to_string(&path).unwrap();
        contents.push_str("# This is a Unicode comment: 中文 🚀 Comments are preserved\n");
        std::fs::write(&path, contents).unwrap();

        // Merge in a new bead
        let mut new_bead = issue("bf-2", "New bead");
        new_bead.description = Some("New description 🔥".to_string());
        export_jsonl_merge(&path, &[new_bead], &[]).unwrap();

        // Verify both beads and the comment line are preserved
        let final_contents = std::fs::read_to_string(&path).unwrap();
        assert!(final_contents.contains("bf-1"), "should contain first bead");
        assert!(
            final_contents.contains("bf-2"),
            "should contain second bead"
        );
        assert!(
            final_contents.contains("中文"),
            "should preserve Unicode in comment"
        );
        assert!(
            final_contents.contains("🚀"),
            "should preserve emoji in comment"
        );
        assert!(
            final_contents.contains("🔥"),
            "should preserve emoji in description"
        );
    }

    #[test]
    fn export_jsonl_empty_database() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Export from empty database (simulating list_all returning empty vec)
        let result = export_jsonl(&path, || Ok(vec![])).unwrap();
        assert_eq!(result.count, 0, "should report 0 beads from empty database");

        // File should still exist but be empty
        assert!(
            path.exists(),
            "file should exist even when database is empty"
        );
        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            contents.trim(),
            "",
            "file should be empty when database is empty"
        );
    }

    #[test]
    fn export_jsonl_merge_empty_to_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Merge with no upserts and no existing file
        let result = export_jsonl_merge(&path, &[], &[]).unwrap();
        assert_eq!(result.count, 0, "should report 0 upserts");
        assert!(
            !path.exists(),
            "should not create file when there's nothing to merge"
        );
    }

    // ==================== incremental_flush tests ====================

    #[test]
    fn get_dirty_issue_ids_returns_correct_ids() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("beads.db");

        // Create database and schema
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(crate::storage::schema::SCHEMA_SQL)
            .unwrap();

        // Initially no dirty issues
        let ids = get_dirty_issue_ids(&conn).unwrap();
        assert_eq!(ids.len(), 0, "should return empty vec when no dirty issues");

        // Create some issues first (required for foreign key constraint)
        for i in 1..=3 {
            let id = format!("bf-{}", i);
            conn.execute(
                "INSERT INTO issues (id, title, status, priority, issue_type, source_repo, created_at, updated_at)
                 VALUES (?1, ?2, 'open', 2, 'task', '.', datetime('now'), datetime('now'))",
                [id.clone(), format!("Bead {}", i)],
            )
            .unwrap();
        }

        // Mark some beads as dirty
        conn.execute(
            "INSERT INTO dirty_issues (bead_id) VALUES ('bf-1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO dirty_issues (bead_id) VALUES ('bf-2')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO dirty_issues (bead_id) VALUES ('bf-3')",
            [],
        )
        .unwrap();

        // Verify the query returns correct IDs
        let ids = get_dirty_issue_ids(&conn).unwrap();
        assert_eq!(ids.len(), 3, "should return 3 dirty issue IDs");

        // Verify the IDs are the ones we inserted
        assert!(ids.contains(&"bf-1".to_string()), "should contain bf-1");
        assert!(ids.contains(&"bf-2".to_string()), "should contain bf-2");
        assert!(ids.contains(&"bf-3".to_string()), "should contain bf-3");

        // Verify ordering by marked_at (insertion order)
        assert_eq!(ids[0], "bf-1", "first ID should be bf-1 (oldest)");
        assert_eq!(ids[1], "bf-2", "second ID should be bf-2");
        assert_eq!(ids[2], "bf-3", "third ID should be bf-3 (newest)");

        // Clear dirty marks and verify empty result
        conn.execute("DELETE FROM dirty_issues", []).unwrap();
        let ids = get_dirty_issue_ids(&conn).unwrap();
        assert_eq!(ids.len(), 0, "should return empty vec after clearing dirty marks");
    }

    #[test]
    fn incremental_flush_success_clears_dirty_marks() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("beads.db");
        let jsonl_path = tmp.path().join("issues.jsonl");

        // Create database and schema
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(crate::storage::schema::SCHEMA_SQL)
            .unwrap();

        // Create a bead and mark it as dirty
        conn.execute(
            "INSERT INTO issues (id, title, status, priority, issue_type, source_repo, created_at, updated_at)
             VALUES ('bf-1', 'Test', 'open', 2, 'task', '.', datetime('now'), datetime('now'))",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO dirty_issues (bead_id) VALUES ('bf-1')",
            [],
        )
        .unwrap();

        // Verify it's marked dirty
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM dirty_issues", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1, "should have 1 dirty issue");

        // Flush
        let storage_for_flush = crate::storage::sqlite::Storage::open(&db_path).unwrap();
        let result = incremental_flush(&storage_for_flush, &jsonl_path).unwrap();

        assert_eq!(result.flushed, 1, "should flush 1 bead");
        assert!(result.warnings.is_empty(), "should have no warnings on success");

        // Verify dirty marks are cleared
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM dirty_issues", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0, "dirty marks should be cleared after successful flush");

        // Verify JSONL file contains the bead
        assert!(jsonl_path.exists(), "JSONL file should exist");
        let contents = std::fs::read_to_string(&jsonl_path).unwrap();
        assert!(contents.contains("bf-1"), "JSONL should contain the flushed bead");
    }

    #[test]
    fn incremental_flush_no_dirty_issues_is_no_op() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("beads.db");
        let jsonl_path = tmp.path().join("issues.jsonl");

        // Create database with no dirty issues
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(crate::storage::schema::SCHEMA_SQL)
            .unwrap();

        // Flush with no dirty issues
        let storage_for_flush = crate::storage::sqlite::Storage::open(&db_path).unwrap();
        let result = incremental_flush(&storage_for_flush, &jsonl_path).unwrap();

        assert_eq!(result.flushed, 0, "should flush 0 beads");
        assert!(result.warnings.is_empty(), "should have no warnings");
        assert!(!jsonl_path.exists(), "should not create JSONL file when no dirty issues");
    }

    #[test]
    fn incremental_flush_only_writes_dirty_beads() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("beads.db");
        let jsonl_path = tmp.path().join("issues.jsonl");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(crate::storage::schema::SCHEMA_SQL)
            .unwrap();

        // Create multiple beads, mark only one as dirty
        for i in 1..=3 {
            let id = format!("bf-{}", i);
            conn.execute(
                "INSERT INTO issues (id, title, status, priority, issue_type, source_repo, created_at, updated_at)
                 VALUES (?1, ?2, 'open', 2, 'task', '.', datetime('now'), datetime('now'))",
                [id.clone(), format!("Bead {}", i)],
            )
            .unwrap();

            // Only mark bf-2 as dirty
            if i == 2 {
                conn.execute(
                    "INSERT INTO dirty_issues (bead_id) VALUES (?1)",
                    [id],
                )
                .unwrap();
            }
        }

        // Flush - only bf-2 should be written
        let storage_for_flush = crate::storage::sqlite::Storage::open(&db_path).unwrap();
        let result = incremental_flush(&storage_for_flush, &jsonl_path).unwrap();

        assert_eq!(result.flushed, 1, "should flush only 1 dirty bead");
        assert!(result.warnings.is_empty(), "should have no warnings");

        // Verify JSONL contains only the dirty bead
        let contents = std::fs::read_to_string(&jsonl_path).unwrap();
        assert!(contents.contains("bf-2"), "should contain dirty bead");
        assert!(!contents.contains("bf-1"), "should not contain non-dirty bead bf-1");
        assert!(!contents.contains("bf-3"), "should not contain non-dirty bead bf-3");

        // Verify only bf-2's dirty mark is cleared
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM dirty_issues", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0, "dirty marks should be cleared");
    }

    #[test]
    fn incremental_flush_includes_related_data() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("beads.db");
        let jsonl_path = tmp.path().join("issues.jsonl");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(crate::storage::schema::SCHEMA_SQL)
            .unwrap();

        // Create a bead with labels, dependencies, comments, events
        conn.execute(
            "INSERT INTO issues (id, title, status, priority, issue_type, source_repo, created_at, updated_at)
             VALUES ('bf-rel', 'Test Related', 'open', 2, 'task', '.', datetime('now'), datetime('now'))",
            [],
        )
        .unwrap();

        // Add labels
        conn.execute(
            "INSERT INTO bead_labels (bead_id, label) VALUES ('bf-rel', 'phase-1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO bead_labels (bead_id, label) VALUES ('bf-rel', 'critical')",
            [],
        )
        .unwrap();

        // Add dependency
        conn.execute(
            "INSERT INTO dependencies (issue_id, depends_on_id, type, created_at)
             VALUES ('bf-rel', 'bf-dep', 'blocks', datetime('now'))",
            [],
        )
        .unwrap();

        // Add comment
        conn.execute(
            "INSERT INTO comments (issue_id, author, text, created_at)
             VALUES ('bf-rel', 'test-user', 'Test comment', datetime('now'))",
            [],
        )
        .unwrap();

        // Add event
        conn.execute(
            "INSERT INTO events (issue_id, event_type, actor, created_at)
             VALUES ('bf-rel', 'created', 'test-user', datetime('now'))",
            [],
        )
        .unwrap();

        // Mark as dirty
        conn.execute(
            "INSERT INTO dirty_issues (bead_id) VALUES ('bf-rel')",
            [],
        )
        .unwrap();

        // Flush
        let storage_for_flush = crate::storage::sqlite::Storage::open(&db_path).unwrap();
        let result = incremental_flush(&storage_for_flush, &jsonl_path).unwrap();

        assert_eq!(result.flushed, 1, "should flush 1 bead");
        assert!(result.warnings.is_empty(), "should have no warnings");

        // Verify all related data is in JSONL
        let contents = std::fs::read_to_string(&jsonl_path).unwrap();
        assert!(contents.contains("phase-1"), "should include labels");
        assert!(contents.contains("critical"), "should include labels");
        assert!(contents.contains("bf-dep"), "should include dependencies");
        assert!(contents.contains("Test comment"), "should include comments");
        assert!(contents.contains("created"), "should include events");
    }

    #[test]
    fn incremental_flush_failure_preserves_dirty_marks() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("beads.db");
        let jsonl_path = tmp.path().join("issues.jsonl");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(crate::storage::schema::SCHEMA_SQL)
            .unwrap();

        // Create a bead and mark it as dirty
        conn.execute(
            "INSERT INTO issues (id, title, status, priority, issue_type, source_repo, created_at, updated_at)
             VALUES ('bf-fail', 'Test Failure', 'open', 2, 'task', '.', datetime('now'), datetime('now'))",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO dirty_issues (bead_id) VALUES ('bf-fail')",
            [],
        )
        .unwrap();

        // Make the path a directory to cause flush failure
        std::fs::create_dir(&jsonl_path).unwrap();

        // Flush should fail gracefully
        let storage_for_flush = crate::storage::sqlite::Storage::open(&db_path).unwrap();
        let result = incremental_flush(&storage_for_flush, &jsonl_path).unwrap();

        assert_eq!(result.flushed, 0, "should flush 0 beads on failure");
        assert!(!result.warnings.is_empty(), "should have warnings on failure");
        assert!(
            result.warnings[0].contains("failed"),
            "warning should mention failure"
        );

        // Verify dirty marks are NOT cleared (persist for retry)
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM dirty_issues", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1, "dirty marks should persist after failed flush");
    }

    #[test]
    fn incremental_flush_surgical_replacement() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("beads.db");
        let jsonl_path = tmp.path().join("issues.jsonl");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(crate::storage::schema::SCHEMA_SQL)
            .unwrap();

        // Create initial JSONL with 3 beads
        let mut initial_issues = vec![
            issue("bf-1", "First"),
            issue("bf-2", "Second"),
            issue("bf-3", "Third"),
        ];
        export_jsonl_merge(&jsonl_path, &initial_issues, &[]).unwrap();
        let before = std::fs::read_to_string(&jsonl_path).unwrap();

        // In the database, create the same 3 beads and mark bf-2 as dirty
        for issue in &initial_issues {
            conn.execute(
                "INSERT INTO issues (id, title, status, priority, issue_type, source_repo, created_at, updated_at)
                 VALUES (?1, ?2, 'open', 2, 'task', '.', datetime('now'), datetime('now'))",
                [issue.id.clone(), issue.title.clone()],
            )
            .unwrap();
        }

        // Update bf-2 in database and mark it dirty
        conn.execute(
            "UPDATE issues SET title = 'Second Modified' WHERE id = 'bf-2'",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO dirty_issues (bead_id) VALUES ('bf-2')",
            [],
        )
        .unwrap();

        // Flush - should only replace bf-2 line
        let storage_for_flush = crate::storage::sqlite::Storage::open(&db_path).unwrap();
        let result = incremental_flush(&storage_for_flush, &jsonl_path).unwrap();

        assert_eq!(result.flushed, 1, "should flush 1 bead");
        assert!(result.warnings.is_empty(), "should have no warnings");

        let after = std::fs::read_to_string(&jsonl_path).unwrap();

        // bf-1 and bf-3 lines should be preserved verbatim
        assert!(
            after.lines().any(|l| l.contains("bf-1") && l.contains("First")),
            "bf-1 line should be preserved"
        );
        assert!(
            after.lines().any(|l| l.contains("bf-3") && l.contains("Third")),
            "bf-3 line should be preserved"
        );

        // bf-2 line should be updated
        assert!(
            after.lines().any(|l| l.contains("bf-2") && l.contains("Second Modified")),
            "bf-2 line should be updated"
        );
        assert!(
            !after.lines().any(|l| l.contains("bf-2") && l.contains("Second\n")),
            "old bf-2 line should not exist"
        );
    }

    #[test]
    fn incremental_flush_multiple_dirty_beads() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("beads.db");
        let jsonl_path = tmp.path().join("issues.jsonl");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(crate::storage::schema::SCHEMA_SQL)
            .unwrap();

        // Create 5 beads, mark 3 as dirty
        for i in 1..=5 {
            let id = format!("bf-{}", i);
            conn.execute(
                "INSERT INTO issues (id, title, status, priority, issue_type, source_repo, created_at, updated_at)
                 VALUES (?1, ?2, 'open', 2, 'task', '.', datetime('now'), datetime('now'))",
                [id.clone(), format!("Bead {}", i)],
            )
            .unwrap();

            // Mark beads 2, 3, 4 as dirty
            if i >= 2 && i <= 4 {
                conn.execute(
                    "INSERT INTO dirty_issues (bead_id) VALUES (?1)",
                    [id],
                )
                .unwrap();
            }
        }

        // Flush - should flush 3 beads
        let storage_for_flush = crate::storage::sqlite::Storage::open(&db_path).unwrap();
        let result = incremental_flush(&storage_for_flush, &jsonl_path).unwrap();

        assert_eq!(result.flushed, 3, "should flush 3 dirty beads");
        assert!(result.warnings.is_empty(), "should have no warnings");

        // Verify JSONL contains all 3 dirty beads
        let contents = std::fs::read_to_string(&jsonl_path).unwrap();
        assert!(contents.contains("bf-2"), "should contain bf-2");
        assert!(contents.contains("bf-3"), "should contain bf-3");
        assert!(contents.contains("bf-4"), "should contain bf-4");
        assert!(!contents.contains("bf-1"), "should not contain non-dirty bf-1");
        assert!(!contents.contains("bf-5"), "should not contain non-dirty bf-5");
    }

    #[test]
    fn incremental_flush_warning_on_clear_dirty_failure() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("beads.db");
        let jsonl_path = tmp.path().join("issues.jsonl");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(crate::storage::schema::SCHEMA_SQL)
            .unwrap();

        // Create a bead and mark it as dirty
        conn.execute(
            "INSERT INTO issues (id, title, status, priority, issue_type, source_repo, created_at, updated_at)
             VALUES ('bf-warn', 'Test Warning', 'open', 2, 'task', '.', datetime('now'), datetime('now'))",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO dirty_issues (bead_id) VALUES ('bf-warn')",
            [],
        )
        .unwrap();

        // Block only DELETE against dirty_issues, via a trigger, rather than
        // dropping the table. incremental_flush reads dirty_issues (to list
        // what to export) before it clears it — dropping the table breaks
        // that read too, so the function never reaches the clear step this
        // test means to exercise. A DELETE-only trigger leaves the read path
        // intact and fails exactly the step under test.
        conn.execute_batch(
            "CREATE TRIGGER block_dirty_delete BEFORE DELETE ON dirty_issues
             BEGIN SELECT RAISE(ABORT, 'simulated clear_dirty failure'); END;",
        )
        .unwrap();

        // Flush should succeed export but warn about clear_dirty failure
        let storage_for_flush = crate::storage::sqlite::Storage::open(&db_path).unwrap();
        let result = incremental_flush(&storage_for_flush, &jsonl_path).unwrap();

        assert_eq!(result.flushed, 1, "should flush 1 bead");
        assert!(!result.warnings.is_empty(), "should have warnings");
        assert!(
            result.warnings[0].contains("clear dirty marks"),
            "warning should mention clear_dirty failure"
        );
    }

    #[test]
    fn incremental_flush_with_labels() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("beads.db");
        let jsonl_path = tmp.path().join("issues.jsonl");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(crate::storage::schema::SCHEMA_SQL)
            .unwrap();

        // Create bead with labels
        conn.execute(
            "INSERT INTO issues (id, title, status, priority, issue_type, source_repo, created_at, updated_at)
             VALUES ('bf-labels', 'Test Labels', 'open', 2, 'task', '.', datetime('now'), datetime('now'))",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO bead_labels (bead_id, label) VALUES ('bf-labels', 'phase-1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO bead_labels (bead_id, label) VALUES ('bf-labels', 'storage')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO dirty_issues (bead_id) VALUES ('bf-labels')",
            [],
        )
        .unwrap();

        // Flush
        let storage_for_flush = crate::storage::sqlite::Storage::open(&db_path).unwrap();
        let result = incremental_flush(&storage_for_flush, &jsonl_path).unwrap();

        assert_eq!(result.flushed, 1, "should flush 1 bead");
        assert!(result.warnings.is_empty(), "should have no warnings");

        // Verify labels are in JSONL
        let contents = std::fs::read_to_string(&jsonl_path).unwrap();
        assert!(contents.contains("phase-1"), "should include phase-1 label");
        assert!(contents.contains("storage"), "should include storage label");
    }

    #[test]
    fn incremental_flush_to_existing_jsonl_preserves_orphans() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("beads.db");
        let jsonl_path = tmp.path().join("issues.jsonl");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(crate::storage::schema::SCHEMA_SQL)
            .unwrap();

        // Create initial JSONL with an orphan line
        std::fs::write(
            &jsonl_path,
            "{\"id\":\"bf-old\",\"title\":\"Old bead\"}\nrandom orphan line\n",
        )
        .unwrap();

        // Create a bead and mark it as dirty
        conn.execute(
            "INSERT INTO issues (id, title, status, priority, issue_type, source_repo, created_at, updated_at)
             VALUES ('bf-new', 'New bead', 'open', 2, 'task', '.', datetime('now'), datetime('now'))",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO dirty_issues (bead_id) VALUES ('bf-new')",
            [],
        )
        .unwrap();

        // Flush
        let storage_for_flush = crate::storage::sqlite::Storage::open(&db_path).unwrap();
        let result = incremental_flush(&storage_for_flush, &jsonl_path).unwrap();

        assert_eq!(result.flushed, 1, "should flush 1 bead");
        assert!(result.warnings.is_empty(), "should have no warnings");

        // Verify orphan line is preserved
        let contents = std::fs::read_to_string(&jsonl_path).unwrap();
        assert!(contents.contains("random orphan line"), "orphan line should be preserved");
        assert!(contents.contains("bf-new"), "new bead should be present");
    }
}
