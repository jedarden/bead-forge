//! Exit code handling for bead-forge commands.
//!
//! Provides structured exit status representation and logging utilities.

use std::fmt;

/// Represents different types of process termination.
///
/// This enum distinguishes between normal exit codes and signal termination,
/// providing a more structured representation than a raw integer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitCode {
    /// Normal process exit with a specific code.
    Code(i32),
    /// Process was terminated by a signal (e.g., SIGTERM, SIGKILL).
    Signal(String),
    /// No exit code available (process status unknown).
    None,
}

impl fmt::Display for ExitCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExitCode::Code(code) => write!(f, "{}", code),
            ExitCode::Signal(signal) => write!(f, "{}", signal),
            ExitCode::None => write!(f, "none"),
        }
    }
}

/// Exit status for CLI commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    /// Command succeeded.
    Success = 0,
    /// General failure.
    Failure = 1,
    /// Usage error (invalid arguments, missing required args, etc.).
    Usage = 2,
    /// Database error or corruption.
    Database = 3,
    /// File I/O error.
    Io = 4,
    /// Validation error.
    Validation = 5,
    /// Conflict or concurrency error.
    Conflict = 6,
}

impl fmt::Display for ExitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExitStatus::Success => write!(f, "success"),
            ExitStatus::Failure => write!(f, "failure"),
            ExitStatus::Usage => write!(f, "usage error"),
            ExitStatus::Database => write!(f, "database error"),
            ExitStatus::Io => write!(f, "I/O error"),
            ExitStatus::Validation => write!(f, "validation error"),
            ExitStatus::Conflict => write!(f, "conflict"),
        }
    }
}

impl From<std::io::Error> for ExitStatus {
    fn from(_err: std::io::Error) -> Self {
        ExitStatus::Io
    }
}

impl ExitStatus {
    /// Get the numeric exit code.
    pub fn code(self) -> i32 {
        self as i32
    }

    /// Check if the exit status represents success.
    pub fn is_success(self) -> bool {
        self == ExitStatus::Success
    }

    /// Create an exit status from a numeric code.
    ///
    /// Unknown codes are mapped to `ExitStatus::Failure`.
    pub fn from_code(code: i32) -> Self {
        match code {
            0 => ExitStatus::Success,
            2 => ExitStatus::Usage,
            3 => ExitStatus::Database,
            4 => ExitStatus::Io,
            5 => ExitStatus::Validation,
            6 => ExitStatus::Conflict,
            _ => ExitStatus::Failure,
        }
    }
}

/// Format an exit code for logging purposes.
///
/// Converts a numeric exit code into a human-readable string
/// suitable for log output.
///
/// # Arguments
///
/// * `code` - The numeric exit code
///
/// # Returns
///
/// A string describing the exit code
pub fn format_exit_code_to_log(code: i32) -> String {
    let status = ExitStatus::from_code(code);
    format!("Exit code {}: {}", code, status)
}

/// Format an ExitCode for display with separator.
///
/// Converts an ExitCode enum variant into a formatted string with clear
/// separator lines, suitable for appending to log file content.
///
/// # Arguments
///
/// * `code` - Optional ExitCode to format
///
/// # Returns
///
/// A formatted string with the exit code or signal information
///
/// # Examples
///
/// ```
/// use bead_forge::exit_code::{ExitCode, format_exit_code};
///
/// let code = ExitCode::Code(0);
/// assert_eq!(format_exit_code(Some(code)), "=== Exit Code: 0 ===");
///
/// let signal = ExitCode::Signal("SIGTERM".to_string());
/// assert_eq!(format_exit_code(Some(signal)), "=== Signal: SIGTERM ===");
///
/// assert_eq!(format_exit_code(None), "=== Exit Code: (none) ===");
/// ```
pub fn format_exit_code(code: Option<ExitCode>) -> String {
    match code {
        Some(ExitCode::Code(n)) => format!("=== Exit Code: {} ===", n),
        Some(ExitCode::Signal(signal)) => format!("=== Signal: {} ===", signal),
        Some(ExitCode::None) => "=== Exit Code: (none) ===".to_string(),
        None => "=== Exit Code: (none) ===".to_string(),
    }
}

/// Process termination information.
///
/// Represents how a process terminated, either by exit code or signal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessTermination {
    /// Process exited with a specific code.
    ExitCode(i32),
    /// Process was terminated by a signal.
    Signal(String),
    /// Process termination status is unknown.
    Unknown,
}

impl ProcessTermination {
    /// Create a `ProcessTermination` from an optional exit code.
    ///
    /// Maps common signal exit codes (128+N) to their signal names.
    /// Unknown codes are treated as exit codes.
    pub fn from_code(code: Option<i32>) -> Self {
        let code = match code {
            Some(c) if c >= 0 => c,
            _ => return ProcessTermination::Unknown,
        };

        // Map signal codes (128 + signal number)
        let signal_names = vec![
            (129, "SIGHUP"),
            (130, "SIGINT"),
            (131, "SIGQUIT"),
            (132, "SIGILL"),
            (133, "SIGTRAP"),
            (134, "SIGABRT"),
            (135, "SIGBUS"),
            (136, "SIGFPE"),
            (137, "SIGKILL"),
            (138, "SIGUSR1"),
            (139, "SIGSEGV"),
            (140, "SIGUSR2"),
            (141, "SIGPIPE"),
            (142, "SIGALRM"),
            (143, "SIGTERM"),
            (144, "SIGSTKFLT"),
            (145, "SIGCHLD"),
            (146, "SIGCONT"),
            (147, "SIGSTOP"),
            (148, "SIGTSTP"),
            (149, "SIGTTIN"),
            (150, "SIGTTOU"),
        ];

        if let Some((_, signal)) = signal_names.iter().find(|(c, _)| *c == code) {
            return ProcessTermination::Signal(signal.to_string());
        }

        ProcessTermination::ExitCode(code)
    }

    /// Format the termination information for display.
    pub fn format(&self) -> String {
        match self {
            ProcessTermination::ExitCode(code) => format!("=== Exit Code: {} ===", code),
            ProcessTermination::Signal(signal) => format!("=== Signal: {} ===", signal),
            ProcessTermination::Unknown => "=== Exit Code: unknown ===".to_string(),
        }
    }
}

/// Append exit code information to a log.
///
/// Adds a formatted exit code or signal line to the end of a log string.
///
/// # Arguments
///
/// * `log` - The original log content
/// * `code` - The optional exit code
///
/// # Returns
///
/// The log content with exit code information appended
pub fn append_exit_code_to_log(log: &str, code: Option<i32>) -> String {
    let termination = ProcessTermination::from_code(code);
    format!("{}\n{}\n", log, termination.format())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for ExitCode enum

    #[test]
    fn test_exit_code_code_variant() {
        let code = ExitCode::Code(0);
        assert_eq!(code, ExitCode::Code(0));
    }

    #[test]
    fn test_exit_code_signal_variant() {
        let signal = ExitCode::Signal("SIGTERM".to_string());
        assert_eq!(signal, ExitCode::Signal("SIGTERM".to_string()));
    }

    #[test]
    fn test_exit_code_none_variant() {
        let none = ExitCode::None;
        assert_eq!(none, ExitCode::None);
    }

    #[test]
    fn test_exit_code_equality() {
        let code1 = ExitCode::Code(42);
        let code2 = ExitCode::Code(42);
        let code3 = ExitCode::Code(99);

        assert_eq!(code1, code2);
        assert_ne!(code1, code3);
    }

    #[test]
    fn test_exit_code_display_code() {
        let code = ExitCode::Code(0);
        assert_eq!(format!("{}", code), "0");

        let code = ExitCode::Code(1);
        assert_eq!(format!("{}", code), "1");

        let code = ExitCode::Code(255);
        assert_eq!(format!("{}", code), "255");
    }

    #[test]
    fn test_exit_code_display_signal() {
        let signal = ExitCode::Signal("SIGTERM".to_string());
        assert_eq!(format!("{}", signal), "SIGTERM");

        let signal = ExitCode::Signal("SIGKILL".to_string());
        assert_eq!(format!("{}", signal), "SIGKILL");

        let signal = ExitCode::Signal("SIGINT".to_string());
        assert_eq!(format!("{}", signal), "SIGINT");
    }

    #[test]
    fn test_exit_code_display_none() {
        let none = ExitCode::None;
        assert_eq!(format!("{}", none), "none");
    }

    #[test]
    fn test_exit_code_clone() {
        let code = ExitCode::Code(42);
        let cloned = code.clone();
        assert_eq!(code, cloned);

        let signal = ExitCode::Signal("SIGTERM".to_string());
        let cloned_signal = signal.clone();
        assert_eq!(signal, cloned_signal);

        let none = ExitCode::None;
        let cloned_none = none.clone();
        assert_eq!(none, cloned_none);
    }

    #[test]
    fn test_exit_code_debug_formatting() {
        let code = ExitCode::Code(42);
        assert!(format!("{:?}", code).contains("Code"));

        let signal = ExitCode::Signal("SIGTERM".to_string());
        assert!(format!("{:?}", signal).contains("Signal"));

        let none = ExitCode::None;
        assert!(format!("{:?}", none).contains("None"));
    }

    // Tests for ExitStatus enum

    #[test]
    fn test_exit_status_codes() {
        assert_eq!(ExitStatus::Success.code(), 0);
        assert_eq!(ExitStatus::Failure.code(), 1);
        assert_eq!(ExitStatus::Usage.code(), 2);
        assert_eq!(ExitStatus::Database.code(), 3);
        assert_eq!(ExitStatus::Io.code(), 4);
        assert_eq!(ExitStatus::Validation.code(), 5);
        assert_eq!(ExitStatus::Conflict.code(), 6);
    }

    #[test]
    fn test_exit_status_is_success() {
        assert!(ExitStatus::Success.is_success());
        assert!(!ExitStatus::Failure.is_success());
    }

    #[test]
    fn test_exit_status_from_code() {
        assert_eq!(ExitStatus::from_code(0), ExitStatus::Success);
        assert_eq!(ExitStatus::from_code(2), ExitStatus::Usage);
        assert_eq!(ExitStatus::from_code(3), ExitStatus::Database);
        assert_eq!(ExitStatus::from_code(4), ExitStatus::Io);
        assert_eq!(ExitStatus::from_code(5), ExitStatus::Validation);
        assert_eq!(ExitStatus::from_code(6), ExitStatus::Conflict);
        assert_eq!(ExitStatus::from_code(1), ExitStatus::Failure);
        assert_eq!(ExitStatus::from_code(99), ExitStatus::Failure);
    }

    #[test]
    fn test_format_exit_code_to_log() {
        assert_eq!(
            format_exit_code_to_log(0),
            "Exit code 0: success"
        );
        assert_eq!(
            format_exit_code_to_log(1),
            "Exit code 1: failure"
        );
        assert_eq!(
            format_exit_code_to_log(2),
            "Exit code 2: usage error"
        );
        assert_eq!(
            format_exit_code_to_log(99),
            "Exit code 99: failure"
        );
    }

    #[test]
    fn test_exit_status_display() {
        assert_eq!(format!("{}", ExitStatus::Success), "success");
        assert_eq!(format!("{}", ExitStatus::Failure), "failure");
        assert_eq!(format!("{}", ExitStatus::Usage), "usage error");
        assert_eq!(format!("{}", ExitStatus::Database), "database error");
        assert_eq!(format!("{}", ExitStatus::Io), "I/O error");
        assert_eq!(format!("{}", ExitStatus::Validation), "validation error");
        assert_eq!(format!("{}", ExitStatus::Conflict), "conflict");
    }

    // Tests for ProcessTermination and append_exit_code_to_log

    #[test]
    fn test_process_termination_from_code_zero() {
        let term = ProcessTermination::from_code(Some(0));
        assert_eq!(term, ProcessTermination::ExitCode(0));
    }

    #[test]
    fn test_process_termination_from_code_positive() {
        let term = ProcessTermination::from_code(Some(1));
        assert_eq!(term, ProcessTermination::ExitCode(1));
    }

    #[test]
    fn test_process_termination_from_code_sigint() {
        let term = ProcessTermination::from_code(Some(130));
        assert_eq!(term, ProcessTermination::Signal("SIGINT".to_string()));
    }

    #[test]
    fn test_process_termination_from_code_sigkill() {
        let term = ProcessTermination::from_code(Some(137));
        assert_eq!(term, ProcessTermination::Signal("SIGKILL".to_string()));
    }

    #[test]
    fn test_process_termination_from_code_sigterm() {
        let term = ProcessTermination::from_code(Some(143));
        assert_eq!(term, ProcessTermination::Signal("SIGTERM".to_string()));
    }

    #[test]
    fn test_process_termination_from_code_none() {
        let term = ProcessTermination::from_code(None);
        assert_eq!(term, ProcessTermination::Unknown);
    }

    #[test]
    fn test_process_termination_from_code_negative() {
        let term = ProcessTermination::from_code(Some(-1));
        assert_eq!(term, ProcessTermination::Unknown);
    }

    #[test]
    fn test_process_termination_from_code_unknown_signal() {
        // Code 200 is not in the signal mapping, should be treated as exit code
        let term = ProcessTermination::from_code(Some(200));
        assert_eq!(term, ProcessTermination::ExitCode(200));
    }

    #[test]
    fn test_process_termination_format_exit_code() {
        assert_eq!(
            ProcessTermination::ExitCode(0).format(),
            "=== Exit Code: 0 ==="
        );
        assert_eq!(
            ProcessTermination::ExitCode(1).format(),
            "=== Exit Code: 1 ==="
        );
        assert_eq!(
            ProcessTermination::ExitCode(255).format(),
            "=== Exit Code: 255 ==="
        );
    }

    #[test]
    fn test_process_termination_format_signal() {
        assert_eq!(
            ProcessTermination::Signal("SIGTERM".to_string()).format(),
            "=== Signal: SIGTERM ==="
        );
        assert_eq!(
            ProcessTermination::Signal("SIGKILL".to_string()).format(),
            "=== Signal: SIGKILL ==="
        );
        assert_eq!(
            ProcessTermination::Signal("SIGINT".to_string()).format(),
            "=== Signal: SIGINT ==="
        );
    }

    #[test]
    fn test_process_termination_format_unknown() {
        assert_eq!(
            ProcessTermination::Unknown.format(),
            "=== Exit Code: unknown ==="
        );
    }

    #[test]
    fn test_append_exit_code_to_log_with_code() {
        let log = "Test output\nSome more logs";
        let result = append_exit_code_to_log(log, Some(42));

        assert!(result.contains("Test output"));
        assert!(result.contains("Some more logs"));
        assert!(result.contains("=== Exit Code: 42 ==="));
        // Ensure the exit code is on a new line
        assert!(result.contains("\n=== Exit Code: 42 ===\n"));
    }

    #[test]
    fn test_append_exit_code_to_log_with_signal() {
        let log = "Test output";
        let result = append_exit_code_to_log(log, Some(143));

        assert!(result.contains("Test output"));
        assert!(result.contains("=== Signal: SIGTERM ==="));
    }

    #[test]
    fn test_append_exit_code_to_log_with_none() {
        let log = "Test output";
        let result = append_exit_code_to_log(log, None);

        assert!(result.contains("Test output"));
        assert!(result.contains("=== Exit Code: unknown ==="));
    }

    #[test]
    fn test_append_exit_code_to_log_empty_content() {
        let log = "";
        let result = append_exit_code_to_log(log, Some(0));

        assert!(result.starts_with("\n=== Exit Code: 0 ===\n"));
    }

    #[test]
    fn test_process_termination_all_signal_codes() {
        // Test all known signal codes
        let signal_cases = vec![
            (129, "SIGHUP"),
            (130, "SIGINT"),
            (131, "SIGQUIT"),
            (132, "SIGILL"),
            (133, "SIGTRAP"),
            (134, "SIGABRT"),
            (135, "SIGBUS"),
            (136, "SIGFPE"),
            (137, "SIGKILL"),
            (138, "SIGUSR1"),
            (139, "SIGSEGV"),
            (140, "SIGUSR2"),
            (141, "SIGPIPE"),
            (142, "SIGALRM"),
            (143, "SIGTERM"),
            (144, "SIGSTKFLT"),
            (145, "SIGCHLD"),
            (146, "SIGCONT"),
            (147, "SIGSTOP"),
            (148, "SIGTSTP"),
            (149, "SIGTTIN"),
            (150, "SIGTTOU"),
        ];

        for (code, expected_signal) in signal_cases {
            let term = ProcessTermination::from_code(Some(code));
            assert_eq!(
                term,
                ProcessTermination::Signal(expected_signal.to_string()),
                "Failed for code {}: expected {}, got {:?}",
                code,
                expected_signal,
                term
            );
        }
    }

    #[test]
    fn test_append_exit_code_preserves_content() {
        let log = "Line 1\nLine 2\nLine 3";
        let result = append_exit_code_to_log(log, Some(0));

        // Ensure all original lines are preserved
        assert!(result.contains("Line 1"));
        assert!(result.contains("Line 2"));
        assert!(result.contains("Line 3"));
    }

    #[test]
    fn test_append_exit_code_multiple_calls() {
        let log = "Original content";
        let first = append_exit_code_to_log(log, Some(1));
        let second = append_exit_code_to_log(&first, Some(2));

        // Should have both exit codes appended
        assert!(second.contains("=== Exit Code: 1 ==="));
        assert!(second.contains("=== Exit Code: 2 ==="));
    }

    // Tests for format_exit_code function

    #[test]
    fn test_format_exit_code_code_variant() {
        let code = ExitCode::Code(0);
        assert_eq!(format_exit_code(Some(code)), "=== Exit Code: 0 ===");

        let code = ExitCode::Code(1);
        assert_eq!(format_exit_code(Some(code)), "=== Exit Code: 1 ===");

        let code = ExitCode::Code(42);
        assert_eq!(format_exit_code(Some(code)), "=== Exit Code: 42 ===");

        let code = ExitCode::Code(255);
        assert_eq!(format_exit_code(Some(code)), "=== Exit Code: 255 ===");
    }

    #[test]
    fn test_format_exit_code_signal_variant() {
        let signal = ExitCode::Signal("SIGTERM".to_string());
        assert_eq!(format_exit_code(Some(signal)), "=== Signal: SIGTERM ===");

        let signal = ExitCode::Signal("SIGKILL".to_string());
        assert_eq!(format_exit_code(Some(signal)), "=== Signal: SIGKILL ===");

        let signal = ExitCode::Signal("SIGINT".to_string());
        assert_eq!(format_exit_code(Some(signal)), "=== Signal: SIGINT ===");
    }

    #[test]
    fn test_format_exit_code_none_case() {
        // Test with ExitCode::None variant
        let none = ExitCode::None;
        assert_eq!(format_exit_code(Some(none)), "=== Exit Code: (none) ===");

        // Test with Option::None
        assert_eq!(format_exit_code(None), "=== Exit Code: (none) ===");
    }

    #[test]
    fn test_format_exit_code_separator_format() {
        // Verify exactly 3 equals signs on each side
        let result = format_exit_code(Some(ExitCode::Code(0)));
        assert!(result.starts_with("==="));
        assert!(result.ends_with("==="));
        assert!(!result.starts_with("===="));
        assert!(!result.ends_with("===="));
    }

    #[test]
    fn test_format_exit_code_negative_code() {
        let code = ExitCode::Code(-1);
        assert_eq!(format_exit_code(Some(code)), "=== Exit Code: -1 ===");
    }

    // Tests for normal exit codes (0, 1, 127, 255)

    #[test]
    fn test_normal_exit_code_zero() {
        // Test exit code 0 (success)
        let code = ExitCode::Code(0);
        assert_eq!(format!("{}", code), "0");
        assert_eq!(format_exit_code(Some(code)), "=== Exit Code: 0 ===");

        let term = ProcessTermination::from_code(Some(0));
        assert_eq!(term, ProcessTermination::ExitCode(0));
        assert_eq!(term.format(), "=== Exit Code: 0 ===");

        assert_eq!(format_exit_code_to_log(0), "Exit code 0: success");
        assert_eq!(ExitStatus::from_code(0), ExitStatus::Success);
    }

    #[test]
    fn test_normal_exit_code_one() {
        // Test exit code 1 (general failure)
        let code = ExitCode::Code(1);
        assert_eq!(format!("{}", code), "1");
        assert_eq!(format_exit_code(Some(code)), "=== Exit Code: 1 ===");

        let term = ProcessTermination::from_code(Some(1));
        assert_eq!(term, ProcessTermination::ExitCode(1));
        assert_eq!(term.format(), "=== Exit Code: 1 ===");

        assert_eq!(format_exit_code_to_log(1), "Exit code 1: failure");
        assert_eq!(ExitStatus::from_code(1), ExitStatus::Failure);
    }

    #[test]
    fn test_normal_exit_code_127() {
        // Test exit code 127 (command not found)
        let code = ExitCode::Code(127);
        assert_eq!(format!("{}", code), "127");
        assert_eq!(format_exit_code(Some(code)), "=== Exit Code: 127 ===");

        let term = ProcessTermination::from_code(Some(127));
        assert_eq!(term, ProcessTermination::ExitCode(127));
        assert_eq!(term.format(), "=== Exit Code: 127 ===");

        // 127 is not a special exit code in ExitStatus, so it maps to Failure
        assert_eq!(format_exit_code_to_log(127), "Exit code 127: failure");
        assert_eq!(ExitStatus::from_code(127), ExitStatus::Failure);
    }

    #[test]
    fn test_normal_exit_code_255() {
        // Test exit code 255 (exit code out of range)
        let code = ExitCode::Code(255);
        assert_eq!(format!("{}", code), "255");
        assert_eq!(format_exit_code(Some(code)), "=== Exit Code: 255 ===");

        let term = ProcessTermination::from_code(Some(255));
        assert_eq!(term, ProcessTermination::ExitCode(255));
        assert_eq!(term.format(), "=== Exit Code: 255 ===");

        // 255 is not a special exit code in ExitStatus, so it maps to Failure
        assert_eq!(format_exit_code_to_log(255), "Exit code 255: failure");
        assert_eq!(ExitStatus::from_code(255), ExitStatus::Failure);
    }

    #[test]
    fn test_normal_exit_codes_formatting_consistency() {
        // Test that all normal exit codes format consistently
        let normal_codes = [0, 1, 127, 255];

        for code in normal_codes {
            let exit_code = ExitCode::Code(code);
            let formatted = format_exit_code(Some(exit_code.clone()));

            // All should format with === prefix/suffix
            assert!(formatted.starts_with("=== Exit Code:"));
            assert!(formatted.ends_with("==="));

            // All should contain the numeric code
            assert!(formatted.contains(&format!("{}", code)));

            // ProcessTermination should format identically
            let term = ProcessTermination::from_code(Some(code));
            assert_eq!(term.format(), formatted);
        }
    }

    // Tests for signal exit code variations

    #[test]
    fn test_signal_sigterm_exit_code_formatting() {
        // Test SIGTERM exit code (128+15=143)
        let code = 143;

        // Test ProcessTermination correctly identifies SIGTERM
        let term = ProcessTermination::from_code(Some(code));
        assert_eq!(term, ProcessTermination::Signal("SIGTERM".to_string()));

        // Test formatting produces "=== Signal: SIGTERM ==="
        let formatted = term.format();
        assert_eq!(formatted, "=== Signal: SIGTERM ===");

        // Test format_exit_code produces correct signal output
        let exit_code = ExitCode::Signal("SIGTERM".to_string());
        let exit_formatted = format_exit_code(Some(exit_code));
        assert_eq!(exit_formatted, "=== Signal: SIGTERM ===");

        // Test append_exit_code_to_log with SIGTERM
        let log = "Process output";
        let result = append_exit_code_to_log(log, Some(code));
        assert!(result.contains("=== Signal: SIGTERM ==="));
    }

    #[test]
    fn test_signal_sigkill_exit_code_formatting() {
        // Test SIGKILL exit code (128+9=137)
        let code = 137;

        // Test ProcessTermination correctly identifies SIGKILL
        let term = ProcessTermination::from_code(Some(code));
        assert_eq!(term, ProcessTermination::Signal("SIGKILL".to_string()));

        // Test formatting produces "=== Signal: SIGKILL ==="
        let formatted = term.format();
        assert_eq!(formatted, "=== Signal: SIGKILL ===");

        // Test format_exit_code produces correct signal output
        let exit_code = ExitCode::Signal("SIGKILL".to_string());
        let exit_formatted = format_exit_code(Some(exit_code));
        assert_eq!(exit_formatted, "=== Signal: SIGKILL ===");

        // Test append_exit_code_to_log with SIGKILL
        let log = "Process output";
        let result = append_exit_code_to_log(log, Some(code));
        assert!(result.contains("=== Signal: SIGKILL ==="));
    }

    #[test]
    fn test_signal_sigint_exit_code_formatting() {
        // Test SIGINT exit code (128+2=130)
        let code = 130;

        // Test ProcessTermination correctly identifies SIGINT
        let term = ProcessTermination::from_code(Some(code));
        assert_eq!(term, ProcessTermination::Signal("SIGINT".to_string()));

        // Test formatting produces "=== Signal: SIGINT ==="
        let formatted = term.format();
        assert_eq!(formatted, "=== Signal: SIGINT ===");

        // Test format_exit_code produces correct signal output
        let exit_code = ExitCode::Signal("SIGINT".to_string());
        let exit_formatted = format_exit_code(Some(exit_code));
        assert_eq!(exit_formatted, "=== Signal: SIGINT ===");

        // Test append_exit_code_to_log with SIGINT
        let log = "Process output";
        let result = append_exit_code_to_log(log, Some(code));
        assert!(result.contains("=== Signal: SIGINT ==="));
    }

    #[test]
    fn test_signal_exit_codes_formatting_consistency() {
        // Test that all signal exit codes format consistently with "=== Signal: XXX ==="
        let signal_cases = vec![
            (143, "SIGTERM"),
            (137, "SIGKILL"),
            (130, "SIGINT"),
        ];

        for (code, signal_name) in signal_cases {
            let term = ProcessTermination::from_code(Some(code));
            let formatted = term.format();

            // All should start with "=== Signal:" and end with "==="
            assert!(formatted.starts_with("=== Signal:"));
            assert!(formatted.ends_with("==="));

            // All should contain the signal name
            assert!(formatted.contains(signal_name));

            // Should use the exact format "=== Signal: NAME ==="
            let expected = format!("=== Signal: {} ===", signal_name);
            assert_eq!(formatted, expected);
        }
    }

    // Comprehensive edge case tests for None/missing exit codes

    #[test]
    fn test_none_exit_code_formatting_comprehensive() {
        // Test that ExitCode::None formats correctly
        let none = ExitCode::None;
        assert_eq!(format!("{}", none), "none");

        // Test format_exit_code with ExitCode::None
        assert_eq!(format_exit_code(Some(none.clone())), "=== Exit Code: (none) ===");

        // Test format_exit_code with Option::None
        assert_eq!(format_exit_code(None), "=== Exit Code: (none) ===");

        // Verify consistency between Some(ExitCode::None) and None
        assert_eq!(
            format_exit_code(Some(ExitCode::None)),
            format_exit_code(None)
        );
    }

    #[test]
    fn test_missing_exit_code_in_process_termination() {
        // Test ProcessTermination::from_code with None
        let term = ProcessTermination::from_code(None);
        assert_eq!(term, ProcessTermination::Unknown);

        // Verify formatting of Unknown termination
        assert_eq!(term.format(), "=== Exit Code: unknown ===");

        // Test that negative codes also produce Unknown
        let negative_term = ProcessTermination::from_code(Some(-1));
        assert_eq!(negative_term, ProcessTermination::Unknown);
        assert_eq!(negative_term.format(), "=== Exit Code: unknown ===");
    }

    #[test]
    fn test_none_exit_code_with_empty_log() {
        // Test appending None exit code to empty log
        let empty_log = "";
        let result = append_exit_code_to_log(empty_log, None);

        assert!(result.contains("=== Exit Code: unknown ==="));
        assert_eq!(result, "\n=== Exit Code: unknown ===\n");
    }

    #[test]
    fn test_none_exit_code_with_multiline_log() {
        // Test appending None exit code to multiline content
        let multiline_log = "Line 1\nLine 2\nLine 3";
        let result = append_exit_code_to_log(multiline_log, None);

        // Verify original content is preserved
        assert!(result.contains("Line 1"));
        assert!(result.contains("Line 2"));
        assert!(result.contains("Line 3"));

        // Verify unknown exit code is appended
        assert!(result.contains("=== Exit Code: unknown ==="));
    }

    #[test]
    fn test_none_exit_code_formatting_consistency() {
        // Test that all None-like cases format consistently
        let cases = vec![
            format_exit_code(None),
            format_exit_code(Some(ExitCode::None)),
            ProcessTermination::Unknown.format(),
        ];

        // All should contain "(none)" or "unknown"
        for case in cases {
            assert!(case.contains("(none)") || case.contains("unknown"));
        }
    }

    #[test]
    fn test_none_exit_code_equality_comparisons() {
        // Test ExitCode::None equality
        let none1 = ExitCode::None;
        let none2 = ExitCode::None;
        assert_eq!(none1, none2);

        // Test ProcessTermination::Unknown equality
        let unknown1 = ProcessTermination::Unknown;
        let unknown2 = ProcessTermination::Unknown;
        assert_eq!(unknown1, unknown2);

        // Test that different None representations are not equal
        assert_ne!(format_exit_code(None), format_exit_code(Some(ExitCode::Code(0))));
    }

    #[test]
    fn test_none_exit_code_with_special_characters() {
        // Test None exit code with logs containing special characters
        let special_log = "Log with émojis 🎉 and spëcial çharacters";
        let result = append_exit_code_to_log(special_log, None);

        assert!(result.contains("émojis"));
        assert!(result.contains("🎉"));
        assert!(result.contains("=== Exit Code: unknown ==="));
    }

    #[test]
    fn test_none_exit_code_multiple_appends() {
        // Test multiple consecutive None exit codes
        let log = "Original";
        let first = append_exit_code_to_log(log, None);
        let second = append_exit_code_to_log(&first, None);

        // Should have two "unknown" exit codes
        let count = second.matches("=== Exit Code: unknown ===").count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_none_exit_code_between_valid_codes() {
        // Test None exit code sandwiched between valid codes
        let log = "Process output";
        let with_none = append_exit_code_to_log(log, None);
        let with_first = append_exit_code_to_log(&with_none, Some(0));
        let with_second = append_exit_code_to_log(&with_first, Some(1));

        assert!(with_second.contains("=== Exit Code: unknown ==="));
        assert!(with_second.contains("=== Exit Code: 0 ==="));
        assert!(with_second.contains("=== Exit Code: 1 ==="));
    }

    #[test]
    fn test_none_exit_code_debug_display() {
        // Test Debug trait for ExitCode::None
        let none = ExitCode::None;
        let debug = format!("{:?}", none);
        assert!(debug.contains("None"));

        // Test Debug trait for ProcessTermination::Unknown
        let unknown = ProcessTermination::Unknown;
        let debug = format!("{:?}", unknown);
        assert!(debug.contains("Unknown"));
    }

    #[test]
    fn test_none_exit_code_clone_behavior() {
        // Test that ExitCode::None clones correctly
        let none = ExitCode::None;
        let cloned = none.clone();
        assert_eq!(none, cloned);

        // Test that ProcessTermination::Unknown clones correctly
        let unknown = ProcessTermination::Unknown;
        let cloned_unknown = unknown.clone();
        assert_eq!(unknown, cloned_unknown);
    }

    #[test]
    fn test_none_exit_code_with_whitespace() {
        // Test None exit code with logs containing leading/trailing whitespace
        let whitespace_log = "  \n  Line with spaces  \n  ";
        let result = append_exit_code_to_log(whitespace_log, None);

        assert!(result.contains("=== Exit Code: unknown ==="));
        assert!(result.contains("Line with spaces"));
    }

    #[test]
    fn test_none_exit_code_format_structure() {
        // Test that None exit code formatting maintains correct structure
        let result = format_exit_code(None);

        // Should have exactly 3 equals at start and end
        assert!(result.starts_with("==="));
        assert!(result.ends_with("==="));

        // Should not have 4 equals at start or end
        assert!(!result.starts_with("===="));
        assert!(!result.ends_with("===="));

        // Should contain "(none)" substring
        assert!(result.contains("(none)"));
    }

    #[test]
    fn test_none_vs_zero_exit_code_distinction() {
        // Test that None exit code is distinctly different from zero exit code
        let none_result = format_exit_code(None);
        let zero_result = format_exit_code(Some(ExitCode::Code(0)));

        assert_ne!(none_result, zero_result);
        assert!(none_result.contains("(none)"));
        assert!(zero_result.contains("0"));
        assert!(!zero_result.contains("(none)"));
    }

    #[test]
    fn test_none_exit_code_graceful_handling() {
        // Test that None exit code is handled gracefully across all functions

        // format_exit_code should not panic with None
        let formatted = format_exit_code(None);
        assert!(!formatted.is_empty());

        // ProcessTermination::from_code should not panic with None
        let term = ProcessTermination::from_code(None);
        assert_eq!(term, ProcessTermination::Unknown);

        // append_exit_code_to_log should not panic with None
        let log = "Test log";
        let result = append_exit_code_to_log(log, None);
        assert!(!result.is_empty());
        assert!(result.contains(log));
    }

    #[test]
    fn test_exit_code_none_display_format() {
        // Test Display trait implementation for ExitCode::None
        let none = ExitCode::None;
        let display = format!("{}", none);

        // Should be lowercase "none"
        assert_eq!(display, "none");
        assert!(!display.contains("None")); // Not "None" with capital N
        assert!(!display.contains("NONE")); // Not "NONE" in all caps
    }

    // Comprehensive separator formatting tests for exact equals count

    #[test]
    fn test_separator_exact_count_format_exit_code() {
        // Test that format_exit_code produces exactly 3 equals signs on each side
        let test_cases = vec![
            (Some(ExitCode::Code(0)), "=== Exit Code: 0 ==="),
            (Some(ExitCode::Code(1)), "=== Exit Code: 1 ==="),
            (Some(ExitCode::Code(42)), "=== Exit Code: 42 ==="),
            (Some(ExitCode::Code(255)), "=== Exit Code: 255 ==="),
            (Some(ExitCode::Signal("SIGTERM".to_string())), "=== Signal: SIGTERM ==="),
            (Some(ExitCode::Signal("SIGKILL".to_string())), "=== Signal: SIGKILL ==="),
            (Some(ExitCode::None), "=== Exit Code: (none) ==="),
        ];

        for (input, expected) in test_cases {
            let result = format_exit_code(input.clone());
            assert_eq!(result, expected, "format_exit_code({:?}) should produce exact format", input);

            // Verify exactly 3 equals at start
            assert!(result.starts_with("==="), "Should start with exactly 3 equals");
            assert!(!result.starts_with("===="), "Should not start with 4 equals");

            // Verify exactly 3 equals at end
            assert!(result.ends_with("==="), "Should end with exactly 3 equals");
            assert!(!result.ends_with("===="), "Should not end with 4 equals");

            // Count equals signs at the start
            let start_equals = result.chars().take_while(|&c| c == '=').count();
            assert_eq!(start_equals, 3, "Should have exactly 3 equals at start, got {}", start_equals);

            // Count equals signs at the end
            let end_equals = result.chars().rev().take_while(|&c| c == '=').count();
            assert_eq!(end_equals, 3, "Should have exactly 3 equals at end, got {}", end_equals);
        }

        // Test None case separately since it doesn't need cloning
        let result = format_exit_code(None);
        assert_eq!(result, "=== Exit Code: (none) ===");
        assert!(result.starts_with("==="));
        assert!(result.ends_with("==="));
    }

    #[test]
    fn test_separator_exact_count_process_termination() {
        // Test that ProcessTermination::format produces exactly 3 equals signs on each side
        let test_cases = vec![
            (ProcessTermination::ExitCode(0), "=== Exit Code: 0 ==="),
            (ProcessTermination::ExitCode(1), "=== Exit Code: 1 ==="),
            (ProcessTermination::Signal("SIGTERM".to_string()), "=== Signal: SIGTERM ==="),
            (ProcessTermination::Signal("SIGKILL".to_string()), "=== Signal: SIGKILL ==="),
            (ProcessTermination::Unknown, "=== Exit Code: unknown ==="),
        ];

        for (input, expected) in test_cases {
            let result = input.format();
            assert_eq!(result, expected, "ProcessTermination::format() should produce exact format");

            // Verify exactly 3 equals at start
            assert!(result.starts_with("==="), "Should start with exactly 3 equals");
            assert!(!result.starts_with("===="), "Should not start with 4 equals");

            // Verify exactly 3 equals at end
            assert!(result.ends_with("==="), "Should end with exactly 3 equals");
            assert!(!result.ends_with("===="), "Should not end with 4 equals");

            // Count equals signs at the start
            let start_equals = result.chars().take_while(|&c| c == '=').count();
            assert_eq!(start_equals, 3, "Should have exactly 3 equals at start, got {}", start_equals);

            // Count equals signs at the end
            let end_equals = result.chars().rev().take_while(|&c| c == '=').count();
            assert_eq!(end_equals, 3, "Should have exactly 3 equals at end, got {}", end_equals);
        }
    }

    #[test]
    fn test_separator_positioning_in_output() {
        // Test that separators are positioned correctly within the full output string

        // Test format_exit_code positioning
        let formatted = format_exit_code(Some(ExitCode::Code(42)));
        let lines: Vec<&str> = formatted.split('\n').collect();
        assert_eq!(lines.len(), 1, "format_exit_code should produce a single line");

        // Verify separator at the beginning
        assert!(lines[0].starts_with("==="), "Separator should be at the start of the line");

        // Verify separator at the end
        assert!(lines[0].ends_with("==="), "Separator should be at the end of the line");

        // Verify content between separators
        assert!(lines[0].contains("Exit Code: 42"), "Content should be between separators");
    }

    #[test]
    fn test_separator_in_append_exit_code_to_log() {
        // Test separator positioning when appended to log content
        let log = "Line 1\nLine 2\nLine 3";
        let result = append_exit_code_to_log(log, Some(0));

        let lines: Vec<&str> = result.split('\n').collect();

        // Find the exit code line
        let exit_code_line = lines.iter().find(|line| line.contains("Exit Code:"));
        assert!(exit_code_line.is_some(), "Should contain an exit code line");

        let exit_line = exit_code_line.unwrap();

        // Verify exact equals count in the appended line
        assert!(exit_line.starts_with("==="), "Appended line should start with 3 equals");
        assert!(exit_line.ends_with("==="), "Appended line should end with 3 equals");
        assert!(!exit_line.starts_with("===="), "Appended line should not start with 4 equals");
        assert!(!exit_line.ends_with("===="), "Appended line should not end with 4 equals");
    }

    #[test]
    fn test_separator_format_matches_specification() {
        // Test that all separator formatting matches the exact specification: === content ===

        let specifications = vec![
            ("=== Exit Code: 0 ===", Some(ExitCode::Code(0))),
            ("=== Exit Code: 1 ===", Some(ExitCode::Code(1))),
            ("=== Signal: SIGTERM ===", Some(ExitCode::Signal("SIGTERM".to_string()))),
            ("=== Exit Code: (none) ===", Some(ExitCode::None)),
        ];

        for (expected_spec, input) in specifications {
            let result = format_exit_code(input.clone());
            assert_eq!(result, expected_spec, "Separator format must match specification exactly");
        }

        // Test None case separately
        let result = format_exit_code(None);
        assert_eq!(result, "=== Exit Code: (none) ===");
    }

    #[test]
    fn test_separator_no_extra_equals_in_middle() {
        // Test that equals signs only appear at the start and end, not in the middle

        let result = format_exit_code(Some(ExitCode::Code(42)));

        // Remove the leading and trailing ===
        let middle = &result[3..result.len() - 3];

        // The middle part should not contain any equals signs
        assert!(!middle.contains('='), "Middle content should not contain equals signs");
    }

    #[test]
    fn test_separator_consistency_across_all_variants() {
        // Test that all ExitCode and ProcessTermination variants use consistent separator format

        let exit_code_cases = vec![
            format_exit_code(Some(ExitCode::Code(0))),
            format_exit_code(Some(ExitCode::Code(1))),
            format_exit_code(Some(ExitCode::Signal("SIGTERM".to_string()))),
            format_exit_code(Some(ExitCode::None)),
            format_exit_code(None),
        ];

        for formatted in exit_code_cases {
            // All should have exactly 3 equals at start
            let start_equals = formatted.chars().take_while(|&c| c == '=').count();
            assert_eq!(start_equals, 3, "All variants must have exactly 3 equals at start");

            // All should have exactly 3 equals at end
            let end_equals = formatted.chars().rev().take_while(|&c| c == '=').count();
            assert_eq!(end_equals, 3, "All variants must have exactly 3 equals at end");

            // All should match the pattern === content ===
            assert!(formatted.starts_with("===") && formatted.ends_with("==="));
        }
    }

    #[test]
    fn test_separator_with_various_content_lengths() {
        // Test separator formatting with various content lengths

        // Short content
        let short = format_exit_code(Some(ExitCode::Code(0)));
        assert!(short.starts_with("===") && short.ends_with("==="));

        // Medium content
        let medium = format_exit_code(Some(ExitCode::Signal("SIGTERM".to_string())));
        assert!(medium.starts_with("===") && medium.ends_with("==="));

        // Long content (via ProcessTermination with potentially long signal names)
        let long = ProcessTermination::Signal("SIGSTKFLT".to_string()).format();
        assert!(long.starts_with("===") && long.ends_with("==="));

        // All should maintain exactly 3 equals regardless of content length
        for formatted in vec![short, medium, long] {
            let start_equals = formatted.chars().take_while(|&c| c == '=').count();
            let end_equals = formatted.chars().rev().take_while(|&c| c == '=').count();
            assert_eq!(start_equals, 3, "Equals count should be independent of content length");
            assert_eq!(end_equals, 3, "Equals count should be independent of content length");
        }
    }
}
