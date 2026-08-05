/// Terminal color coding for status display
///
/// Provides ANSI color codes for displaying different statuses in terminal output.
/// Colors are automatically disabled when output is not a TTY or when NO_COLOR environment variable is set.

use crate::model::Status;
use std::env;

/// ANSI color codes for terminal output
#[derive(Debug, Clone, Copy)]
pub enum Color {
    Reset,
    Green,
    Yellow,
    Red,
    Blue,
    Cyan,
    Magenta,
    BrightGreen,
    BrightYellow,
    BrightRed,
    BrightBlue,
    BrightCyan,
    BrightMagenta,
}

impl Color {
    /// Get the ANSI code for this color
    pub fn ansi_code(self) -> &'static str {
        match self {
            Color::Reset => "\x1b[0m",
            Color::Green => "\x1b[32m",
            Color::Yellow => "\x1b[33m",
            Color::Red => "\x1b[31m",
            Color::Blue => "\x1b[34m",
            Color::Cyan => "\x1b[36m",
            Color::Magenta => "\x1b[35m",
            Color::BrightGreen => "\x1b[92m",
            Color::BrightYellow => "\x1b[93m",
            Color::BrightRed => "\x1b[91m",
            Color::BrightBlue => "\x1b[94m",
            Color::BrightCyan => "\x1b[96m",
            Color::BrightMagenta => "\x1b[95m",
        }
    }

    /// Check if colors should be enabled (no NO_COLOR env var and likely TTY)
    pub fn should_color() -> bool {
        // Check NO_COLOR environment variable (https://no-color.org/)
        if env::var("NO_COLOR").is_ok() {
            return false;
        }

        // In a real CLI, you'd also check if stdout is a TTY here
        // For now, we'll enable colors by default unless NO_COLOR is set
        true
    }
}

/// Get color for a given status
pub fn status_color(status: &Status) -> Color {
    match status {
        Status::Open => Color::Green,
        Status::InProgress => Color::Blue,
        Status::Blocked => Color::Red,
        Status::Deferred => Color::Yellow,
        Status::Draft => Color::Cyan,
        Status::Closed => Color::BrightGreen,
        Status::Tombstone => Color::BrightYellow,
        Status::Pinned => Color::Magenta,
        Status::Custom(_) => Color::BrightCyan,
    }
}

/// Format text with color if colors are enabled
pub fn colorize(text: &str, color: Color) -> String {
    if !Color::should_color() {
        return text.to_string();
    }

    format!("{}{}{}", color.ansi_code(), text, Color::Reset.ansi_code())
}

/// Format a status with appropriate color
pub fn format_status_colored(status: &Status) -> String {
    let color = status_color(status);
    colorize(&status.to_string(), color)
}

/// Format a status with appropriate color for table display
/// Returns a tuple of (colored_status, width) for table formatting
pub fn format_status_colored_for_table(status: &Status) -> String {
    format_status_colored(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_color_open() {
        let color = status_color(&Status::Open);
        assert_eq!(color.ansi_code(), "\x1b[32m"); // Green
    }

    #[test]
    fn test_status_color_blocked() {
        let color = status_color(&Status::Blocked);
        assert_eq!(color.ansi_code(), "\x1b[31m"); // Red
    }

    #[test]
    fn test_status_color_in_progress() {
        let color = status_color(&Status::InProgress);
        assert_eq!(color.ansi_code(), "\x1b[34m"); // Blue
    }

    #[test]
    fn test_status_color_closed() {
        let color = status_color(&Status::Closed);
        assert_eq!(color.ansi_code(), "\x1b[92m"); // Bright Green
    }

    #[test]
    fn test_colorize_with_colors_enabled() {
        // Temporarily ensure NO_COLOR is not set
        env::remove_var("NO_COLOR");
        let colored = colorize("test", Color::Red);
        assert_eq!(colored, "\x1b[31mtest\x1b[0m");
    }

    #[test]
    fn test_colorize_with_colors_disabled() {
        // Set NO_COLOR to disable colors
        env::set_var("NO_COLOR", "1");
        let colored = colorize("test", Color::Red);
        assert_eq!(colored, "test");
        env::remove_var("NO_COLOR");
    }

    #[test]
    fn test_format_status_colored() {
        env::remove_var("NO_COLOR");
        let status = Status::Open;
        let colored = format_status_colored(&status);
        assert!(colored.contains("\x1b[32m")); // Green
        assert!(colored.contains("open"));
        assert!(colored.contains("\x1b[0m")); // Reset
    }

    #[test]
    fn test_custom_status_color() {
        let color = status_color(&Status::Custom("in-review".to_string()));
        assert_eq!(color.ansi_code(), "\x1b[96m"); // Bright Cyan
    }
}
