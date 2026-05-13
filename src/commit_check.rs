//! Git pre-commit hook for scanning staged .beads/ changes for secrets.
//!
//! Scans staged changes to .beads/ files (config.yaml, metadata.json, issues.jsonl)
//! for secret patterns using the same scanner as write-time checks.

use crate::secrets::{SecretMatch, SecretScanner};
use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

/// Result of scanning staged changes for secrets.
#[derive(Debug)]
pub struct ScanResult {
    /// Files that were scanned
    pub files_scanned: Vec<String>,
    /// Secret matches found: (file_path, line_number, line_content, secret_match)
    pub secrets_found: Vec<(String, usize, String, SecretMatch)>,
}

/// Scan staged changes to .beads/ files for secrets.
pub fn scan_staged_beads(beads_dir: &Path) -> Result<ScanResult> {
    let scanner = SecretScanner::new()?;

    // Get the diff of staged changes for .beads/ files
    let output = Command::new("git")
        .args(["diff", "--cached", "--unified=0", "--", ".beads/"])
        .output()
        .map_err(|e| anyhow!("Failed to run git diff: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("does not have any commits yet") {
            // Repo has no commits yet - check if .beads/ files exist in index
            let ls_output = Command::new("git")
                .args(["ls-files", "--cached", ".beads/"])
                .output()
                .map_err(|e| anyhow!("Failed to run git ls-files: {}", e))?;

            if ls_output.status.success() {
                let files = String::from_utf8_lossy(&ls_output.stdout);
                let files: Vec<&str> = files.lines().collect();

                if files.is_empty() {
                    // No .beads/ files staged
                    return Ok(ScanResult {
                        files_scanned: vec![],
                        secrets_found: vec![],
                    });
                }

                // Read the staged content of each file and scan
                return scan_staged_files(&scanner, &files, beads_dir);
            }
        }
        return Err(anyhow!("git diff failed: {}", stderr));
    }

    let diff = String::from_utf8_lossy(&output.stdout);

    // Parse the diff to extract changed lines
    let (files_scanned, secrets_found) = parse_diff_and_scan(&scanner, &diff, beads_dir)?;

    Ok(ScanResult {
        files_scanned,
        secrets_found,
    })
}

/// Parse git diff and scan added lines for secrets.
fn parse_diff_and_scan(
    scanner: &SecretScanner,
    diff: &str,
    beads_dir: &Path,
) -> Result<(Vec<String>, Vec<(String, usize, String, SecretMatch)>)> {
    let mut files_scanned = Vec::new();
    let mut secrets_found = Vec::new();
    let mut seen = HashSet::new();

    let mut current_file: Option<String> = None;
    let mut current_line: usize = 0;

    for line in diff.lines() {
        if line.starts_with("+++ ") {
            // New file path
            let path = line[4..].trim().to_string();
            // Strip "a/" or "b/" prefix if present
            let path = path
                .strip_prefix("a/")
                .or_else(|| path.strip_prefix("b/"))
                .unwrap_or(&path);
            // Strip trailing tab with timestamp if present
            let path = path.split('\t').next().unwrap_or(path);
            current_file = Some(path.to_string());
            current_line = 0;
        } else if line.starts_with("@@ ") {
            // Hunk header - extract new line number
            // Format: @@ -old_start,old_count +new_start,new_count @@
            if let Some(rest) = line.strip_prefix("@@ ") {
                if let Some(new_part) = rest.split(" +").nth(1) {
                    if let Some(line_str) = new_part.split(',').next() {
                        if let Ok(line_num) = line_str.parse::<usize>() {
                            current_line = line_num;
                        }
                    }
                }
            }
        } else if line.starts_with('+') && !line.starts_with("+++") {
            // Added line
            if let Some(ref file) = current_file {
                let content = &line[1..];
                let matches = scanner.scan_string(content);

                for m in matches {
                    // Deduplicate by (file, line, matched_text)
                    let key = (file.clone(), current_line, m.matched_text.clone());
                    if seen.insert(key) {
                        secrets_found.push((file.clone(), current_line, content.to_string(), m));
                    }
                }
            }
            current_line += 1;
        } else if !line.starts_with('-') && !line.starts_with(' ') {
            // Context line or other - increment line count for context
            if !line.starts_with("@@") && !line.starts_with("diff") && !line.starts_with("index") {
                current_line += 1;
            }
        }

        // Track scanned files
        if let Some(ref file) = current_file {
            if !files_scanned.contains(file) {
                files_scanned.push(file.clone());
            }
        }
    }

    Ok((files_scanned, secrets_found))
}

/// Scan files that are newly added to the index (no commits yet).
fn scan_staged_files(
    scanner: &SecretScanner,
    files: &[&str],
    beads_dir: &Path,
) -> Result<ScanResult> {
    let mut secrets_found = Vec::new();
    let mut seen = HashSet::new();

    for file in files {
        // Get the staged content
        let output = Command::new("git")
            .args(["show", ":0", file])
            .output()
            .map_err(|e| anyhow!("Failed to run git show: {}", e))?;

        if !output.status.success() {
            continue;
        }

        let content = String::from_utf8_lossy(&output.stdout);

        // Scan each line
        for (line_num, line) in content.lines().enumerate() {
            let matches = scanner.scan_string(line);
            for m in matches {
                let key = (file.to_string(), line_num + 1, m.matched_text.clone());
                if seen.insert(key) {
                    secrets_found.push((file.to_string(), line_num + 1, line.to_string(), m));
                }
            }
        }
    }

    Ok(ScanResult {
        files_scanned: files.iter().map(|s| s.to_string()).collect(),
        secrets_found,
    })
}

/// Format scan results for display.
pub fn format_scan_results(result: &ScanResult) -> String {
    if result.secrets_found.is_empty() {
        return String::new();
    }

    let mut output = String::from("Secrets detected in staged .beads/ changes:\n");

    for (file, line_num, line_content, secret_match) in &result.secrets_found {
        output.push_str(&format!("\n  {}:{}\n", file, line_num));
        output.push_str(&format!("    Pattern: {}\n", secret_match.pattern_name));
        output.push_str(&format!("    Matched: {}\n", secret_match.matched_text));
        output.push_str(&format!("    Line: {}\n", line_content.trim()));
    }

    output
        .push_str("\nCommit rejected. Remove secrets or add to allowlist in .beads/config.yaml:\n");
    output.push_str("  secret_protection:\n");
    output.push_str("    allowlist:\n");
    output.push_str("      - \"pattern_regex\"\n");

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_diff_simple() {
        let scanner = SecretScanner::new().unwrap();
        let diff = "diff --git a/.beads/config.yaml b/.beads/config.yaml\n\
            index 123..456 789\n\
            --- a/.beads/config.yaml\n\
            +++ b/.beads/config.yaml\n\
            @@ -1 +1 @@\n\
            -old_value: test\n\
            +api_key: AKIAIOSFODNN7EXAMPLE\n";

        let (files, secrets) = parse_diff_and_scan(&scanner, diff, Path::new(".beads")).unwrap();

        assert!(!secrets.is_empty());
        assert_eq!(files.len(), 1);
        assert!(files[0].contains("config.yaml"));
        assert!(secrets[0].3.pattern_name.contains("AWS"));
    }

    #[test]
    fn test_parse_diff_no_secrets() {
        let scanner = SecretScanner::new().unwrap();
        let diff = "diff --git a/.beads/config.yaml b/.beads/config.yaml\n\
            index 123..456 789\n\
            --- a/.beads/config.yaml\n\
            +++ b/.beads/config.yaml\n\
            @@ -1 +1 @@\n\
            -old_value: test\n\
            +new_value: safe\n";

        let (files, secrets) = parse_diff_and_scan(&scanner, diff, Path::new(".beads")).unwrap();

        assert!(secrets.is_empty());
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_parse_diff_empty() {
        let scanner = SecretScanner::new().unwrap();
        let diff = "";

        let (files, secrets) = parse_diff_and_scan(&scanner, diff, Path::new(".beads")).unwrap();

        assert!(secrets.is_empty());
        assert!(files.is_empty());
    }

    #[test]
    fn test_format_scan_results() {
        let result = ScanResult {
            files_scanned: vec![".beads/config.yaml".to_string()],
            secrets_found: vec![(
                ".beads/config.yaml".to_string(),
                5,
                "api_key: AKIAIOSFODNN7EXAMPLE".to_string(),
                SecretMatch {
                    pattern_name: "AWS Access Key (AKIA)".to_string(),
                    matched_text: "AKIAIOSFODNN7EXAMPLE".to_string(),
                },
            )],
        };

        let formatted = format_scan_results(&result);

        assert!(formatted.contains("Secrets detected"));
        assert!(formatted.contains("AWS Access Key"));
        assert!(formatted.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(formatted.contains("Commit rejected"));
    }
}
