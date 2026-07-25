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

#[derive(Debug)]
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
        assert!(parsed.labels.contains(&"auto-flushed".to_string()), "Should contain 'auto-flushed' label");
        assert!(parsed.labels.contains(&"test-label".to_string()), "Should contain 'test-label' label");
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
        assert_eq!(ids, vec!["bf-a", "bf-m", "bf-z"], "output should be sorted by ID");

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
        assert!(!temp_path.exists(), "temp file should be cleaned up after atomic rename");

        // Verify final file exists
        assert!(path.exists(), "final file should exist");

        // Verify content is correct
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("bf-test"), "final file should contain the bead");
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
        assert!(clear_called, "clear_dirty should be called after successful export");

        // Verify final state: all beads present, only bf-2 modified
        let after_export = std::fs::read_to_string(&path).unwrap();
        let ids = ids_in(&path);
        assert_eq!(ids, vec!["bf-1", "bf-2", "bf-3"], "all beads should be present");
        assert!(after_export.contains("Second Modified"), "modified bead should be updated");
        assert!(!after_export.contains("Second\n"), "old version should be replaced");

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
        assert!(!clear_called, "clear_dirty should NOT be called when no dirty beads");

        // Verify file unchanged
        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(before, after, "file should be unchanged when no dirty beads");
    }

    #[test]
    fn export_jsonl_dirty_atomic_behavior() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("issues.jsonl");

        // Export dirty beads
        let dirty_beads = vec![issue("bf-dirty", "Dirty Bead")];
        export_jsonl_dirty(
            &path,
            || Ok(dirty_beads.clone()),
            || Ok(()),
        )
        .unwrap();

        // Verify atomic behavior: temp file should not exist after successful export
        let temp_path = path.with_extension("jsonl.tmp");
        assert!(!temp_path.exists(), "temp file should be cleaned up after atomic rename");

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
        assert_eq!(ids, vec!["bf-1", "bf-2", "bf-3"], "beads should be sorted by ID");

        // Export again in different order
        let beads_forward = vec![
            issue("bf-1", "First"),
            issue("bf-2", "Second"),
            issue("bf-3", "Third"),
        ];
        export_jsonl(&path, || Ok(beads_forward.clone())).unwrap();

        // Verify sorted order after second export (regardless of input order, output is sorted)
        let ids = ids_in(&path);
        assert_eq!(ids, vec!["bf-1", "bf-2", "bf-3"], "beads should be sorted by ID");

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
        assert!(result.is_err(), "import_jsonl should return error for malformed JSON");

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
                    err_msg.contains("created_at") || err_msg.contains("missing field") || err_msg.contains("missing"),
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
            Err::<UpsertResult, anyhow::Error>(anyhow::anyhow!("Database error"))
        });

        assert!(result.is_err(), "import_jsonl should propagate upsert errors");
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
}
