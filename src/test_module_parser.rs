use std::fs;
use std::path::Path;

/// Result of parsing a test module list file
#[derive(Debug)]
pub struct ParseResult {
    pub modules: Vec<String>,
    pub source_path: String,
}

/// Parse a test module list file into a clean Vec<String> of module names.
///
/// The file format is expected to be:
/// - One module per line
/// - Tab-separated format: "<number>\t<module_name>"
/// - Empty lines are ignored
/// - Lines with only whitespace are ignored
///
/// # Arguments
/// * `path` - Path to the file to parse
///
/// # Returns
/// * `Ok(ParseResult)` with the parsed module list and source path
/// * `Err(std::io::Error)` if the file cannot be read
///
/// # Examples
/// ```no_run
/// use bead_forge::test_module_parser;
/// let result = test_module_parser::parse_module_list(".beads/traces/bf-4kzs6h-first-batch.txt")
///     .expect("Failed to parse");
/// assert_eq!(result.modules.len(), 73);
/// ```
pub fn parse_module_list<P: AsRef<Path>>(path: P) -> Result<ParseResult, std::io::Error> {
    let path = path.as_ref();
    let source_path = path.display().to_string();

    // Handle file not found gracefully
    if !path.exists() {
        return Ok(ParseResult {
            modules: Vec::new(),
            source_path,
        });
    }

    let content = fs::read_to_string(path)?;

    let modules: Vec<String> = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty()) // Skip empty lines
        .map(|line| {
            // Handle tab-separated format: "number\tmodule_name"
            if let Some(tab_pos) = line.find('\t') {
                line.split_at(tab_pos + 1).1.trim()
            } else {
                line
            }
        })
        .map(|name| name.to_string())
        .collect();

    Ok(ParseResult { modules, source_path })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_module_list() {
        // Create a temporary test file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_modules.txt");
        let content = "1\tmodule_a\n2\tmodule_b\n\n3\tmodule_c\n  \n4\tmodule_d\n";
        fs::write(&test_file, content).unwrap();

        let result = parse_module_list(&test_file).expect("Failed to parse");

        assert_eq!(result.modules, vec!["module_a", "module_b", "module_c", "module_d"]);
        assert_eq!(result.source_path, test_file.display().to_string());

        // Cleanup
        fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_file_not_found() {
        let result = parse_module_list("/nonexistent/path.txt").expect("Should handle gracefully");
        assert_eq!(result.modules.len(), 0);
        assert_eq!(result.source_path, "/nonexistent/path.txt");
    }

    #[test]
    fn test_empty_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("empty_modules.txt");
        fs::write(&test_file, "").unwrap();

        let result = parse_module_list(&test_file).expect("Failed to parse");
        assert_eq!(result.modules.len(), 0);

        // Cleanup
        fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_whitespace_only_lines() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("whitespace_modules.txt");
        fs::write(&test_file, "   \n\t\n  \t  \n").unwrap();

        let result = parse_module_list(&test_file).expect("Failed to parse");
        assert_eq!(result.modules.len(), 0);

        // Cleanup
        fs::remove_file(&test_file).ok();
    }
}
