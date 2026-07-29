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

    // Build a map of dirty issues by ID for O(1) lookup
    let dirty_map: std::collections::HashMap<String, Issue> = issues
        .into_iter()
        .map(|issue| (issue.id.clone(), issue))
        .collect();

    let temp_path = path.with_extension("jsonl.tmp");

    // Read existing JSONL and perform surgical line replacement
    let input_file = File::open(path)?;
    let reader = BufReader::new(input_file);
    let output_file = File::create(&temp_path)?;
    let mut writer = BufWriter::new(output_file);

    let mut replaced_count = 0;

    for line_result in reader.lines() {
        let line = line_result?;
        if let Ok(existing_issue) = serde_json::from_str::<Issue>(&line) {
            if let Some(dirty_issue) = dirty_map.get(&existing_issue.id) {
                // Replace this line with the dirty issue
                serde_json::to_writer(&mut writer, dirty_issue)?;
                writer.write_all(b"\n")?;
                replaced_count += 1;
            } else {
                // Keep existing line
                writer.write_all(line.as_bytes())?;
                writer.write_all(b"\n")?;
            }
        } else {
            // Line is malformed - keep it as-is
            writer.write_all(line.as_bytes())?;
            writer.write_all(b"\n")?;
        }
    }

    // Append any dirty issues that weren't in the file (newly created issues)
    for (id, issue) in &dirty_map {
        // Check if we already replaced this issue
        let was_replaced = {
            let input_file = File::open(path)?;
            let reader = BufReader::new(input_file);
            reader.lines().any(|l| {
                l.ok().and_then(|line| serde_json::from_str::<Issue>(&line).ok())
                    .map(|existing| existing.id == *id)
                    .unwrap_or(false)
            })
        };

        if !was_replaced {
            serde_json::to_writer(&mut writer, issue)?;
            writer.write_all(b"\n")?;
            replaced_count += 1;
        }
    }

    writer.flush()?;
    drop(writer);

    std::fs::rename(&temp_path, path)?;
    clear_dirty()?;

    Ok(ExportResult {
        count: replaced_count,
    })
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
