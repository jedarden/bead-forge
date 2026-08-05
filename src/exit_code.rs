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
}
