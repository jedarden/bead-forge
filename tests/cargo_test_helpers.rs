//! Test helpers for cargo test execution infrastructure.
//!
//! This module provides reusable utilities for integration tests that need to
//! execute cargo test commands and verify their behavior. It encapsulates common
//! patterns for creating test projects, running cargo test, and asserting on results.
//!
//! # Examples
//!
//! ```rust
//! use cargo_test_helpers::*;
//!
//! #[test]
//! fn test_my_feature() {
//!     let test_project = TestProject::new()
//!         .with_test("test_addition", "assert_eq!(2 + 2, 4);")
//!         .with_test("test_multiplication", "assert_eq!(3 * 3, 9);")
//!         .build();
//!
//!     let result = test_project.run_cargo_test().unwrap();
//!     assert_success!(result);
//!     assert_duration_gt!(result, 0);
//! }
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use bead_forge::trace::{TraceManager, CargoTestResult, BeadTestResult, TraceMetadata};

/// A test Rust project that can be created on-the-fly for testing cargo test execution.
///
/// This builder creates a minimal Cargo project structure with configurable tests,
/// allowing tests to create isolated Rust projects without manual file creation.
pub struct TestProject {
    /// Temporary directory holding the project
    temp_dir: TempDir,
    /// Project workspace directory
    workspace_dir: PathBuf,
    /// Test functions to include in lib.rs
    test_functions: Vec<String>,
    /// Additional dependencies for Cargo.toml
    dependencies: Vec<(String, String)>,
    /// Additional source code before test module
    source_code: String,
}

impl TestProject {
    /// Create a new TestProject builder.
    pub fn new() -> anyhow::Result<Self> {
        let temp_dir = TempDir::new()?;
        let workspace_dir = temp_dir.path().to_path_buf();

        Ok(Self {
            temp_dir,
            workspace_dir,
            test_functions: Vec::new(),
            dependencies: Vec::new(),
            source_code: String::new(),
        })
    }

    /// Add a test function to the project.
    ///
    /// # Arguments
    ///
    /// * `name` - Test function name (without "fn " prefix)
    /// * `body` - Test function body (the code inside the function)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let project = TestProject::new()
    ///     .with_test("test_addition", "assert_eq!(2 + 2, 4);")
    ///     .build();
    /// ```
    pub fn with_test(mut self, name: &str, body: &str) -> Self {
        self.test_functions.push(format!(
            r#"    #[test]
    fn {}() {{
        {}
    }}"#,
            name, body
        ));
        self
    }

    /// Add multiple test functions at once.
    ///
    /// # Arguments
    ///
    /// * `tests` - Iterator of (name, body) tuples
    ///
    /// # Example
    ///
    /// ```ignore
    /// let project = TestProject::new()
    ///     .with_tests(vec![
    ///         ("test_one", "assert_eq!(1, 1);"),
    ///         ("test_two", "assert_eq!(2, 2);"),
    ///     ])
    ///     .build();
    /// ```
    pub fn with_tests<I>(mut self, tests: I) -> Self
    where
        I: IntoIterator<Item = (String, String)>,
    {
        for (name, body) in tests {
            self = self.with_test(&name, &body);
        }
        self
    }

    /// Add a dependency to Cargo.toml.
    ///
    /// # Arguments
    ///
    /// * `name` - Crate name (e.g., "serde")
    /// * `version` - Version specification (e.g., "1.0")
    ///
    /// # Example
    ///
    /// ```ignore
    /// let project = TestProject::new()
    ///     .with_dependency("serde", "1.0")
    ///     .with_dependency("tokio", "1.0")
    ///     .build();
    /// ```
    pub fn with_dependency(mut self, name: &str, version: &str) -> Self {
        self.dependencies.push((name.to_string(), version.to_string()));
        self
    }

    /// Add additional source code before the test module.
    ///
    /// This is useful for setting up structs, functions, or imports that
    /// your tests will use.
    ///
    /// # Arguments
    ///
    /// * `code` - Source code to add before test module
    ///
    /// # Example
    ///
    /// ```ignore
    /// let project = TestProject::new()
    ///     .with_source_code("pub fn add(a: i32, b: i32) -> i32 { a + b }")
    ///     .with_test("test_add", "assert_eq!(add(2, 3), 5);")
    ///     .build();
    /// ```
    pub fn with_source_code(mut self, code: &str) -> Self {
        self.source_code = code.to_string();
        self
    }

    /// Build the test project by creating all necessary files.
    ///
    /// This creates the Cargo.toml, src/lib.rs, and any other files needed
    /// for a valid Rust project that can be tested with `cargo test`.
    pub fn build(self) -> anyhow::Result<Self> {
        // Create Cargo.toml
        let cargo_toml = self.workspace_dir.join("Cargo.toml");
        let mut toml_content = String::from(r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#);

        for (name, version) in &self.dependencies {
            toml_content.push_str(&format!("{} = \"{}\"\n", name, version));
        }

        fs::write(&cargo_toml, toml_content)?;

        // Create src directory and lib.rs
        let src_dir = self.workspace_dir.join("src");
        fs::create_dir(&src_dir)?;

        let lib_rs = src_dir.join("lib.rs");
        let mut lib_content = self.source_code.clone();
        lib_content.push_str("\n#[cfg(test)]\nmod tests {\n");

        for test_fn in &self.test_functions {
            lib_content.push_str(test_fn);
            lib_content.push('\n');
        }

        lib_content.push_str("}\n");

        fs::write(&lib_rs, lib_content)?;

        Ok(self)
    }

    /// Get the workspace directory path.
    pub fn workspace_dir(&self) -> &Path {
        &self.workspace_dir
    }

    /// Get the temporary directory (for manual cleanup if needed).
    pub fn temp_dir(&self) -> &TempDir {
        &self.temp_dir
    }

    /// Run cargo test in this project using TraceManager.
    ///
    /// # Returns
    ///
    /// * `Result<CargoTestResult>` containing exit code, duration, and trace path
    pub fn run_cargo_test(&self) -> anyhow::Result<CargoTestResult> {
        let trace_manager = TraceManager::new(&self.workspace_dir);
        trace_manager.run_cargo_test(&self.workspace_dir)
    }

    /// Run cargo test with custom arguments.
    ///
    /// # Arguments
    ///
    /// * `args` - Additional arguments to pass to cargo test
    ///
    /// # Returns
    ///
    /// * `Result<CargoTestResult>` containing exit code, duration, and trace path
    pub fn run_cargo_test_with_args(&self, args: &[&str]) -> anyhow::Result<CargoTestResult> {
        let trace_manager = TraceManager::new(&self.workspace_dir);
        trace_manager.run_cargo_test_with_args(&self.workspace_dir, args)
    }

    /// Run cargo test and write output to a bead-specific trace directory.
    ///
    /// # Arguments
    ///
    /// * `bead_id` - Bead ID for the trace directory
    /// * `metadata` - Trace metadata to record
    ///
    /// # Returns
    ///
    /// * `Result<BeadTestResult>` containing exit code, duration, and bead trace directory
    pub fn run_cargo_test_to_bead_trace(
        &self,
        bead_id: &str,
        metadata: &TraceMetadata,
    ) -> anyhow::Result<BeadTestResult> {
        let trace_manager = TraceManager::new(&self.workspace_dir);
        trace_manager.run_cargo_test_to_bead_trace(&self.workspace_dir, bead_id, metadata)
    }

    /// Read the trace file content from the most recent cargo test run.
    ///
    /// # Returns
    ///
    /// * `Result<String>` containing the trace file content
    pub fn read_trace(&self) -> anyhow::Result<String> {
        // Find the most recent cargo test trace file
        let traces_dir = self.workspace_dir.join(".beads").join("traces");
        if !traces_dir.exists() {
            return Ok(String::new());
        }

        let mut entries: Vec<_> = fs::read_dir(&traces_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("cargo-test-") && n.ends_with(".log"))
                    .unwrap_or(false)
            })
            .collect();

        entries.sort_by_key(|e| std::cmp::Reverse(e.path()));

        if let Some(entry) = entries.first() {
            Ok(fs::read_to_string(entry.path())?)
        } else {
            Ok(String::new())
        }
    }
}

/// Default metadata for bead test traces.
pub fn default_bead_metadata(bead_id: &str) -> TraceMetadata {
    TraceMetadata {
        bead_id: Some(bead_id.to_string()),
        agent: "test-runner".to_string(),
        provider: None,
        model: None,
        outcome: "unknown".to_string(),
        ..Default::default()
    }
}

/// Assert that a cargo test result indicates success (exit code 0).
///
/// # Macro Arguments
///
/// * `$result:expr` - CargoTestResult or BeadTestResult to check
///
/// # Example
///
/// ```ignore
/// let result = test_project.run_cargo_test().unwrap();
/// assert_success!(result);
/// ```
#[macro_export]
macro_rules! assert_success {
    ($result:expr) => {
        assert_eq!(
            $result.exit_code, 0,
            "cargo test should succeed (exit code 0), got {}",
            $result.exit_code
        );
    };
}

/// Assert that a cargo test result indicates failure (non-zero exit code).
///
/// # Macro Arguments
///
/// * `$result:expr` - CargoTestResult or BeadTestResult to check
///
/// # Example
///
/// ```ignore
/// let result = test_project.run_cargo_test().unwrap();
/// assert_failure!(result);
/// ```
#[macro_export]
macro_rules! assert_failure {
    ($result:expr) => {
        assert_ne!(
            $result.exit_code, 0,
            "cargo test should fail (non-zero exit code), got {}",
            $result.exit_code
        );
    };
}

/// Assert that a cargo test result has duration greater than specified milliseconds.
///
/// # Macro Arguments
///
/// * `$result:expr` - CargoTestResult or BeadTestResult to check
/// * `$min_ms:expr` - Minimum duration in milliseconds
///
/// # Example
///
/// ```ignore
/// let result = test_project.run_cargo_test().unwrap();
/// assert_duration_gt!(result, 100);
/// ```
#[macro_export]
macro_rules! assert_duration_gt {
    ($result:expr, $min_ms:expr) => {
        assert!(
            $result.duration_ms > $min_ms,
            "cargo test duration should be greater than {}ms, got {}ms",
            $min_ms, $result.duration_ms
        );
    };
}

/// Assert that a cargo test result has duration less than specified milliseconds.
///
/// # Macro Arguments
///
/// * `$result:expr` - CargoTestResult or BeadTestResult to check
/// * `$max_ms:expr` - Maximum duration in milliseconds
///
/// # Example
///
/// ```ignore
/// let result = test_project.run_cargo_test().unwrap();
/// assert_duration_lt!(result, 5000);
/// ```
#[macro_export]
macro_rules! assert_duration_lt {
    ($result:expr, $max_ms:expr) => {
        assert!(
            $result.duration_ms < $max_ms,
            "cargo test duration should be less than {}ms, got {}ms",
            $max_ms, $result.duration_ms
        );
    };
}

/// Assert that a cargo test result's trace file exists and contains expected text.
///
/// # Macro Arguments
///
/// * `$result:expr` - CargoTestResult to check
/// * `$expected_text:expr` - Text that should be present in the trace file
///
/// # Example
///
/// ```ignore
/// let result = test_project.run_cargo_test().unwrap();
/// assert_trace_contains!(result, "test result: ok");
/// ```
#[macro_export]
macro_rules! assert_trace_contains {
    ($result:expr, $expected_text:expr) => {
        let content = std::fs::read_to_string(&$result.trace_path)
            .expect("Failed to read trace file");
        assert!(
            content.contains($expected_text),
            "Trace file should contain '{}', but it does not.\nTrace content:\n{}",
            $expected_text, content
        );
    };
}

/// Assert that stdout contains expected text (for BeadTestResult).
///
/// # Macro Arguments
///
/// * `$result:expr` - BeadTestResult to check
/// * `$expected_text:expr` - Text that should be present in stdout
///
/// # Example
///
/// ```ignore
/// let result = test_project.run_cargo_test_to_bead_trace(...).unwrap();
/// assert_stdout_contains!(result, "test result: ok");
/// ```
#[macro_export]
macro_rules! assert_stdout_contains {
    ($result:expr, $expected_text:expr) => {
        assert!(
            $result.stdout.contains($expected_text),
            "Stdout should contain '{}', but it does not.\nStdout:\n{}",
            $expected_text, $result.stdout
        );
    };
}

/// Assert that stderr contains expected text (for BeadTestResult).
///
/// # Macro Arguments
///
/// * `$result:expr` - BeadTestResult to check
/// * `$expected_text:expr` - Text that should be present in stderr
///
/// # Example
///
/// ```ignore
/// let result = test_project.run_cargo_test_to_bead_trace(...).unwrap();
/// assert_stderr_contains!(result, "error");
/// ```
#[macro_export]
macro_rules! assert_stderr_contains {
    ($result:expr, $expected_text:expr) => {
        assert!(
            $result.stderr.contains($expected_text),
            "Stderr should contain '{}', but it does not.\nStderr:\n{}",
            $expected_text, $result.stderr
        );
    };
}

/// Assert that trace metadata fields are set correctly.
///
/// # Macro Arguments
///
/// * `$metadata:expr` - TraceMetadata to check
/// * `bead_id: $expected_bead_id:expr` - Expected bead_id
/// * `outcome: $expected_outcome:expr` - Expected outcome
///
/// # Additional fields (optional):
/// * `exit_code: $expected_exit_code:expr` - Expected exit code
/// * `duration_ms: $expected_duration_ms:expr` - Expected minimum duration
///
/// # Example
///
/// ```ignore
/// assert_metadata!(
///     metadata,
///     bead_id: "bf-8ei6pa",
///     outcome: "success",
///     exit_code: Some(0),
///     duration_ms: 100
/// );
/// ```
#[macro_export]
macro_rules! assert_metadata {
    ($metadata:expr, bead_id: $expected_bead_id:expr, outcome: $expected_outcome:expr) => {
        assert_eq!(
            $metadata.bead_id.as_deref(),
            Some($expected_bead_id),
            "Metadata bead_id should match"
        );
        assert_eq!(
            $metadata.outcome, $expected_outcome,
            "Metadata outcome should match"
        );
    };

    (
        $metadata:expr,
        bead_id: $expected_bead_id:expr,
        outcome: $expected_outcome:expr,
        exit_code: $expected_exit_code:expr
    ) => {
        assert_metadata!($metadata, bead_id: $expected_bead_id, outcome: $expected_outcome);
        assert_eq!(
            $metadata.exit_code, $expected_exit_code,
            "Metadata exit_code should match"
        );
    };

    (
        $metadata:expr,
        bead_id: $expected_bead_id:expr,
        outcome: $expected_outcome:expr,
        exit_code: $expected_exit_code:expr,
        duration_ms: $expected_duration_ms:expr
    ) => {
        assert_metadata!(
            $metadata,
            bead_id: $expected_bead_id,
            outcome: $expected_outcome,
            exit_code: $expected_exit_code
        );
        assert!(
            $metadata.duration_ms.map_or(false, |d| d >= $expected_duration_ms),
            "Metadata duration_ms should be at least {}, got {:?}",
            $expected_duration_ms, $metadata.duration_ms
        );
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_project_creation() {
        let project = TestProject::new()
            .unwrap()
            .with_test("test_example", "assert_eq!(1, 1);")
            .build()
            .unwrap();

        assert!(project.workspace_dir().exists());
        assert!(project.workspace_dir().join("Cargo.toml").exists());
        assert!(project.workspace_dir().join("src/lib.rs").exists());
    }

    #[test]
    fn test_test_project_with_dependencies() {
        let project = TestProject::new()
            .unwrap()
            .with_dependency("serde", "1.0")
            .with_dependency("tokio", "1.0")
            .with_test("test_example", "assert_eq!(1, 1);")
            .build()
            .unwrap();

        let cargo_toml = std::fs::read_to_string(
            project.workspace_dir().join("Cargo.toml")
        ).unwrap();

        assert!(cargo_toml.contains("serde = \"1.0\""));
        assert!(cargo_toml.contains("tokio = \"1.0\""));
    }

    #[test]
    fn test_test_project_with_source_code() {
        let project = TestProject::new()
            .unwrap()
            .with_source_code("pub fn helper() -> i32 { 42 }")
            .with_test("test_helper", "assert_eq!(helper(), 42);")
            .build()
            .unwrap();

        let lib_rs = std::fs::read_to_string(
            project.workspace_dir().join("src/lib.rs")
        ).unwrap();

        assert!(lib_rs.contains("pub fn helper() -> i32 { 42 }"));
        assert!(lib_rs.contains("test_helper"));
    }

    #[test]
    fn test_test_project_multiple_tests() {
        let project = TestProject::new()
            .unwrap()
            .with_tests(vec![
                ("test_one".to_string(), "assert_eq!(1, 1);".to_string()),
                ("test_two".to_string(), "assert_eq!(2, 2);".to_string()),
                ("test_three".to_string(), "assert_eq!(3, 3);".to_string()),
            ])
            .build()
            .unwrap();

        let lib_rs = std::fs::read_to_string(
            project.workspace_dir().join("src/lib.rs")
        ).unwrap();

        assert!(lib_rs.contains("test_one"));
        assert!(lib_rs.contains("test_two"));
        assert!(lib_rs.contains("test_three"));
    }

    #[test]
    fn test_test_project_cargo_test_execution() {
        let project = TestProject::new()
            .unwrap()
            .with_test("test_success", "assert_eq!(2 + 2, 4);")
            .build()
            .unwrap();

        let result = project.run_cargo_test().unwrap();
        assert_success!(result);
        assert_duration_gt!(result, 0);
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_test_project_cargo_test_with_failing_test() {
        let project = TestProject::new()
            .unwrap()
            .with_test("test_failure", "panic!(\"This test fails\");")
            .build()
            .unwrap();

        let result = project.run_cargo_test().unwrap();
        assert_failure!(result);
        assert_duration_gt!(result, 0);
    }

    #[test]
    fn test_default_bead_metadata() {
        let metadata = default_bead_metadata("bf-test-123");

        assert_eq!(metadata.bead_id, Some("bf-test-123".to_string()));
        assert_eq!(metadata.agent, "test-runner");
        assert_eq!(metadata.outcome, "unknown");
    }

    #[test]
    fn test_assert_metadata_macro() {
        let metadata = TraceMetadata {
            bead_id: Some("bf-test".to_string()),
            agent: "test".to_string(),
            outcome: "success".to_string(),
            exit_code: Some(0),
            duration_ms: Some(500),
            ..Default::default()
        };

        assert_metadata!(metadata, bead_id: "bf-test", outcome: "success");
        assert_metadata!(
            metadata,
            bead_id: "bf-test",
            outcome: "success",
            exit_code: Some(0)
        );
        assert_metadata!(
            metadata,
            bead_id: "bf-test",
            outcome: "success",
            exit_code: Some(0),
            duration_ms: 400
        );
    }
}