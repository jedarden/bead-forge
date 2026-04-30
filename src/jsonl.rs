use crate::model::Issue;
use anyhow::Result;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

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
    F: FnMut(&Issue) -> Result<bool>,
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
            true => result.imported += 1,
            false => result.updated += 1,
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

    Ok(ExportResult { count: issues.len() })
}

pub fn export_jsonl_dirty<F1, F2>(path: &Path, mut list_dirty: F1, mut clear_dirty: F2) -> Result<ExportResult>
where
    F1: FnMut() -> Result<Vec<Issue>>,
    F2: FnMut() -> Result<()>,
{
    let issues = list_dirty()?;
    if issues.is_empty() {
        return Ok(ExportResult { count: 0 });
    }

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
    clear_dirty()?;

    Ok(ExportResult {
        count: issues.len(),
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
