// Version display tests for bead-forge
// This test verifies that --version flag outputs the correct format
// and that the version matches Cargo.toml

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::fs;

    #[test]
    fn test_version_flag_output() {
        // Test that bf --version outputs the correct format
        let output = Command::new("bf")
            .arg("--version")
            .output()
            .expect("Failed to run 'bf --version'");

        // clap outputs version to stderr
        let version_text = String::from_utf8_lossy(&output.stderr);

        // Should contain "bf " followed by version number
        assert!(
            version_text.starts_with("bf "),
            "Version output should start with 'bf '. Got: {}",
            version_text
        );

        // Should be a valid semver format (major.minor.patch)
        let version_str = version_text.trim().strip_prefix("bf ").unwrap_or(&version_text);
        assert!(
            is_valid_semver(version_str),
            "Version should be valid semver. Got: {}",
            version_str
        );

        println!("Version output verified: {}", version_text.trim());
    }

    #[test]
    fn test_version_matches_cargo_toml() {
        // Test that bf --version matches the version in Cargo.toml
        let output = Command::new("bf")
            .arg("--version")
            .output()
            .expect("Failed to run 'bf --version'");

        let version_text = String::from_utf8_lossy(&output.stderr);
        let cli_version = version_text.trim().strip_prefix("bf ").unwrap_or(&version_text);

        // Read version from Cargo.toml
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
            "CLI version should match Cargo.toml version. CLI: {}, Cargo.toml: {}",
            cli_version, cargo_version
        );

        println!("Version match verified: {}", cli_version);
    }

    #[test]
    fn test_version_short_flag() {
        // Test that bf -V (short version flag) also works
        let output = Command::new("bf")
            .arg("-V")
            .output()
            .expect("Failed to run 'bf -V'");

        let version_text = String::from_utf8_lossy(&output.stderr);

        assert!(
            version_text.starts_with("bf "),
            "Short version flag should also output version. Got: {}",
            version_text
        );

        println!("Short version flag verified: {}", version_text.trim());
    }

    fn is_valid_semver(version: &str) -> bool {
        // Simple semver validation: major.minor.patch
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return false;
        }
        parts.iter().all(|part| {
            part.parse::<u32>().is_ok() || part.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '+')
        })
    }
}
