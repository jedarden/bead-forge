// Version display tests for bead-forge
// This test verifies that --version flag outputs the correct format
// and that the version matches Cargo.toml

use std::process::Command;
use std::fs;

#[cfg(test)]
mod tests {
    use super::*;

    fn get_bf_binary_path() -> std::path::PathBuf {
        // During cargo test, the CARGO_BIN_EXE_bf environment variable points to the built binary
        if let Ok(bin_path) = std::env::var("CARGO_BIN_EXE_bf") {
            return std::path::PathBuf::from(bin_path);
        }

        // Fallback: try to find bf in PATH
        if let Ok(output) = Command::new("which").arg("bf").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout);
                return std::path::PathBuf::from(path.trim());
            }
        }

        // Another fallback: use cargo build to get the path
        std::path::PathBuf::from("bf")
    }

    #[test]
    fn test_version_flag_output() {
        // Test that bf --version outputs the correct format
        let bf_path = get_bf_binary_path();
        let output = Command::new(&bf_path)
            .arg("--version")
            .output()
            .expect("Failed to run 'bf --version'");

        // The implementation outputs to stdout (via println!)
        let version_text = String::from_utf8_lossy(&output.stdout);

        // Should contain "bf " followed by version number
        assert!(
            version_text.starts_with("bf "),
            "Version output should start with 'bf '. Got: '{}'",
            version_text
        );

        // Should be a valid semver format (major.minor.patch)
        let version_str = version_text.trim().strip_prefix("bf ").unwrap_or(&version_text);
        assert!(
            is_valid_semver(version_str),
            "Version should be valid semver. Got: '{}'",
            version_str
        );

        println!("Version output verified: {}", version_text.trim());
    }

    #[test]
    fn test_version_matches_cargo_toml() {
        // Test that bf --version matches the version in Cargo.toml
        let bf_path = get_bf_binary_path();
        let output = Command::new(&bf_path)
            .arg("--version")
            .output()
            .expect("Failed to run 'bf --version'");

        let version_text = String::from_utf8_lossy(&output.stdout);
        let cli_version = version_text.trim().strip_prefix("bf ").unwrap_or(&version_text);

        // Read version from Cargo.toml in the project root
        // The test runs from the project root during cargo test
        let cargo_toml_path = std::path::Path::new("Cargo.toml");
        let cargo_toml_content = fs::read_to_string(cargo_toml_path)
            .expect("Failed to read Cargo.toml");

        // Parse version from Cargo.toml
        let cargo_version = cargo_toml_content
            .lines()
            .find(|line| line.starts_with("version = "))
            .and_then(|line| {
                line.split('=')
                    .nth(1)
                    .map(|v| v.trim().trim_matches('"'))
            })
            .expect("Could not find version in Cargo.toml");

        assert_eq!(
            cli_version, cargo_version,
            "CLI version should match Cargo.toml version. CLI: '{}', Cargo.toml: '{}'",
            cli_version, cargo_version
        );

        println!("Version match verified: {}", cli_version);
    }

    #[test]
    fn test_version_short_flag() {
        // Test that bf -V (short version flag) also works
        let bf_path = get_bf_binary_path();
        let output = Command::new(&bf_path)
            .arg("-V")
            .output()
            .expect("Failed to run 'bf -V'");

        let version_text = String::from_utf8_lossy(&output.stdout);

        assert!(
            version_text.starts_with("bf "),
            "Short version flag should also output version. Got: '{}'",
            version_text
        );

        println!("Short version flag verified: {}", version_text.trim());
    }

    #[test]
    fn test_version_exit_code() {
        // Test that bf --version exits with success
        let bf_path = get_bf_binary_path();
        let output = Command::new(&bf_path)
            .arg("--version")
            .output()
            .expect("Failed to run 'bf --version'");

        assert!(
            output.status.success(),
            "Version command should exit with success code"
        );

        println!("Version exit code verified: success");
    }

    fn is_valid_semver(version: &str) -> bool {
        // Simple semver validation: major.minor.patch
        // Allows for pre-release and build metadata (e.g., "1.0.0-alpha" or "1.0.0+build")
        let base_version = version.split('+').next().unwrap_or(version);
        let parts: Vec<&str> = base_version.split('.').collect();
        if parts.len() != 3 {
            return false;
        }
        // Check that major, minor are numeric, patch can have pre-release identifier
        parts[0].parse::<u32>().is_ok() &&
        parts[1].parse::<u32>().is_ok() &&
        (parts[2].parse::<u32>().is_ok() || parts[2].split('-').next().unwrap().parse::<u32>().is_ok())
    }
}
