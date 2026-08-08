//! Progress reporting module for long-running operations
//!
//! Provides progress bars, spinners, and step counters with support for:
//! - Terminal width detection and graceful fallback when not a TTY
//! - Quiet mode (--no-progress flag) for scripted environments
//! - Smooth animation and updates for progress indicators

use std::io::IsTerminal;

/// Global quiet mode flag - when true, all progress output is suppressed
static QUIET_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Enable or disable quiet mode globally
pub fn set_quiet_mode(quiet: bool) {
    QUIET_MODE.store(quiet, std::sync::atomic::Ordering::Relaxed);
}

/// Check if quiet mode is enabled
pub fn is_quiet_mode() -> bool {
    QUIET_MODE.load(std::sync::atomic::Ordering::Relaxed)
}

/// Check if we're running in a TTY environment
pub fn is_tty() -> bool {
    std::io::stderr().is_terminal()
}

/// Progress bar for batch operations
///
/// Shows a visual progress bar with current position and total count.
/// In quiet mode or non-TTY environments, falls back to minimal output.
pub struct ProgressBar {
    total: usize,
    current: usize,
    prefix: String,
    enabled: bool,
}

impl ProgressBar {
    /// Create a new progress bar with the given total and prefix text
    pub fn new(total: usize, prefix: &str) -> Self {
        let enabled = !is_quiet_mode() && is_tty();

        Self {
            total,
            current: 0,
            prefix: prefix.to_string(),
            enabled,
        }
    }

    /// Increment the progress by one
    pub fn inc(&mut self) {
        if self.current < self.total {
            self.current += 1;
            self.render();
        }
    }

    /// Set the current position directly
    pub fn set_position(&mut self, pos: usize) {
        self.current = pos.min(self.total);
        self.render();
    }

    /// Get the current position
    pub fn position(&self) -> usize {
        self.current
    }

    /// Get the total length
    pub fn length(&self) -> usize {
        self.total
    }

    /// Finish the progress bar with a completion message
    pub fn finish(self, message: Option<&str>) {
        if self.enabled {
            if let Some(msg) = message {
                eprintln!("{}: {}", self.prefix, msg);
            } else {
                eprintln!("{}: complete", self.prefix);
            }
        }
    }

    /// Render the progress bar
    fn render(&self) {
        if !self.enabled {
            return;
        }

        let percentage = if self.total > 0 {
            (self.current * 100) / self.total
        } else {
            100
        };

        // Simple progress bar with percentage
        eprint!("\r{}: [{}/{}] {}%",
            self.prefix,
            self.current,
            self.total,
            percentage
        );
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        if self.enabled {
            eprintln!(); // Clear the line
        }
    }
}

/// Spinner for network operations or indeterminate progress
///
/// Shows an animated spinner for operations without a known duration.
/// In quiet mode or non-TTY environments, falls back to minimal output.
pub struct Spinner {
    message: String,
    enabled: bool,
    started: bool,
}

impl Spinner {
    /// Create a new spinner with the given message
    pub fn new(message: &str) -> Self {
        let enabled = !is_quiet_mode() && is_tty();

        Self {
            message: message.to_string(),
            enabled,
            started: false,
        }
    }

    /// Start the spinner animation
    pub fn start(&mut self) {
        if !self.enabled || self.started {
            return;
        }

        self.started = true;
        self.render_frame();
    }

    /// Update the spinner message
    pub fn set_message(&mut self, message: &str) {
        self.message = message.to_string();
        if self.started {
            self.render_frame();
        }
    }

    /// Finish the spinner with a completion message
    pub fn finish(self, message: Option<&str>) {
        if self.enabled {
            if let Some(msg) = message {
                eprintln!("✓ {}", msg);
            } else {
                eprintln!("✓ {}", self.message);
            }
        }
    }

    /// Render a single frame of the spinner animation
    fn render_frame(&self) {
        if !self.enabled || !self.started {
            return;
        }

        // Simple spinner frames
        const FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let frame_idx = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() / 100) as usize % FRAMES.len();

        eprint!("\r{} {}", FRAMES[frame_idx], self.message);
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        if self.enabled && self.started {
            eprintln!(); // Clear the line
        }
    }
}

/// Step counter for multi-phase operations
///
/// Shows progress through discrete steps with descriptions.
/// In quiet mode or non-TTY environments, falls back to minimal output.
pub struct StepCounter {
    total_steps: usize,
    current_step: usize,
    operation_name: String,
    step_descriptions: Vec<String>,
    enabled: bool,
}

impl StepCounter {
    /// Create a new step counter with the given operation name and total steps
    pub fn new(operation_name: &str, total_steps: usize) -> Self {
        let enabled = !is_quiet_mode() && is_tty();

        Self {
            total_steps,
            current_step: 0,
            operation_name: operation_name.to_string(),
            step_descriptions: Vec::with_capacity(total_steps),
            enabled,
        }
    }

    /// Set description for a specific step (0-indexed)
    pub fn set_step_description(&mut self, step: usize, description: &str) {
        if step < self.total_steps {
            if step >= self.step_descriptions.len() {
                self.step_descriptions.resize(step + 1, String::new());
            }
            self.step_descriptions[step] = description.to_string();
        }
    }

    /// Advance to the next step
    pub fn advance(&mut self) {
        if self.current_step < self.total_steps {
            self.current_step += 1;
            self.render();
        }
    }

    /// Get the current step number
    pub fn current_step(&self) -> usize {
        self.current_step
    }

    /// Get the total number of steps
    pub fn total_steps(&self) -> usize {
        self.total_steps
    }

    /// Finish the step counter with a completion message
    pub fn finish(self, message: Option<&str>) {
        if self.enabled {
            if let Some(msg) = message {
                eprintln!("{}: {}", self.operation_name, msg);
            } else {
                eprintln!("{}: complete", self.operation_name);
            }
        }
    }

    /// Render the current step progress
    fn render(&self) {
        if !self.enabled {
            return;
        }

        let description = if self.current_step > 0 && self.current_step <= self.step_descriptions.len() {
            &self.step_descriptions[self.current_step - 1]
        } else {
            &String::new()
        };

        eprintln!("{} [{}/{}]: {}",
            self.operation_name,
            self.current_step,
            self.total_steps,
            description
        );
    }
}

/// Get terminal width if available
pub fn terminal_width() -> Option<usize> {
    // Try to get terminal width from standard terminal size APIs
    #[cfg(unix)]
    {
        use std::mem;
        unsafe {
            let mut winsize: libc::winsize = mem::zeroed();
            if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut winsize) == 0 {
                let width = winsize.ws_col as usize;
                if width > 0 {
                    return Some(width);
                }
            }
        }
    }

    // Fallback to environment variable
    if let Ok(width_str) = std::env::var("COLUMNS") {
        if let Ok(width) = width_str.parse::<usize>() {
            if width > 0 {
                return Some(width);
            }
        }
    }

    None
}

/// Check if terminal supports Unicode characters
pub fn supports_unicode() -> bool {
    // Check for common environment variables that indicate Unicode support
    if let Ok(lang) = std::env::var("LANG") {
        if lang.contains("UTF-8") || lang.contains("utf-8") {
            return true;
        }
    }

    // Default to assuming Unicode support on modern systems
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_creation() {
        let bar = ProgressBar::new(10, "test");
        assert_eq!(bar.length(), 10);
        assert_eq!(bar.position(), 0);
    }

    #[test]
    fn test_progress_bar_increment() {
        let mut bar = ProgressBar::new(5, "test");
        bar.inc();
        assert_eq!(bar.position(), 1);
        bar.inc();
        assert_eq!(bar.position(), 2);
    }

    #[test]
    fn test_progress_bar_set_position() {
        let mut bar = ProgressBar::new(10, "test");
        bar.set_position(5);
        assert_eq!(bar.position(), 5);

        // Setting beyond total should cap at total
        bar.set_position(15);
        assert_eq!(bar.position(), 10);
    }

    #[test]
    fn test_spinner_creation() {
        let spinner = Spinner::new("test message");
        assert_eq!(spinner.message, "test message");
    }

    #[test]
    fn test_spinner_set_message() {
        let mut spinner = Spinner::new("initial");
        spinner.set_message("updated message");
        assert_eq!(spinner.message, "updated message");
    }

    #[test]
    fn test_step_counter_creation() {
        let counter = StepCounter::new("test operation", 5);
        assert_eq!(counter.total_steps(), 5);
        assert_eq!(counter.current_step(), 0);
    }

    #[test]
    fn test_step_counter_advance() {
        let mut counter = StepCounter::new("test operation", 3);
        counter.advance();
        assert_eq!(counter.current_step(), 1);
        counter.advance();
        assert_eq!(counter.current_step(), 2);
    }

    #[test]
    fn test_step_counter_description() {
        let mut counter = StepCounter::new("test", 3);
        counter.set_step_description(0, "First step");
        counter.set_step_description(1, "Second step");
        counter.set_step_description(2, "Third step");

        assert_eq!(counter.step_descriptions.len(), 3);
        assert_eq!(counter.step_descriptions[0], "First step");
    }

    #[test]
    fn test_quiet_mode() {
        // Test quiet mode flag
        set_quiet_mode(true);
        assert!(is_quiet_mode());

        set_quiet_mode(false);
        assert!(!is_quiet_mode());
    }

    #[test]
    fn test_progress_bar_with_quiet_mode() {
        set_quiet_mode(true);
        let bar = ProgressBar::new(10, "test");
        // In quiet mode, the bar should be disabled
        assert!(!bar.enabled);

        set_quiet_mode(false);
    }

    #[test]
    fn test_spinner_with_quiet_mode() {
        set_quiet_mode(true);
        let spinner = Spinner::new("test");
        // In quiet mode, the spinner should be disabled
        assert!(!spinner.enabled);

        set_quiet_mode(false);
    }

    #[test]
    fn test_step_counter_with_quiet_mode() {
        set_quiet_mode(true);
        let counter = StepCounter::new("test", 5);
        // In quiet mode, the counter should be disabled
        assert!(!counter.enabled);

        set_quiet_mode(false);
    }
}
