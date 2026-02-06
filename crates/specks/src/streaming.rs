//! Simple inline spinner display
//!
//! Updates on the same line using carriage return - no cursor positioning needed.

use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use owo_colors::OwoColorize;

/// Braille spinner animation frames
const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Simple inline spinner display.
pub struct StreamingDisplay {
    /// Message shown in the spinner
    message: String,
    /// When the display was started
    start_time: Instant,
    /// Current spinner frame index
    spinner_frame: usize,
    /// Whether stdout is a TTY
    is_tty: bool,
    /// Cancellation flag
    cancelled: Arc<AtomicBool>,
    /// Whether display is active
    active: bool,
    /// Optional file stats to show (lines, bytes)
    file_stats: Option<(usize, usize)>,
    /// Count of content chunks received
    chunk_count: usize,
    /// Total bytes of content received
    content_bytes: usize,
    /// Current tool being used (if any)
    current_tool: Option<String>,
    /// Count of tools used
    tool_count: usize,
}

impl StreamingDisplay {
    /// Create a new streaming display
    pub fn new(message: &str) -> Self {
        let is_tty = std::io::stdout().is_terminal();

        Self {
            message: message.to_string(),
            start_time: Instant::now(),
            spinner_frame: 0,
            is_tty,
            cancelled: Arc::new(AtomicBool::new(false)),
            active: false,
            file_stats: None,
            chunk_count: 0,
            content_bytes: 0,
            current_tool: None,
            tool_count: 0,
        }
    }

    /// Start the display
    pub fn start(&mut self) {
        if !self.is_tty {
            println!("{}", self.message);
            return;
        }

        let mut stdout = std::io::stdout();

        // Hide cursor during spinner
        write!(stdout, "\x1b[?25l").ok();
        stdout.flush().ok();

        self.active = true;
        self.draw_spinner();
    }

    /// Write streaming content (tracks bytes but doesn't display)
    pub fn write_content(&mut self, content: &str) {
        if content.is_empty() {
            return;
        }
        self.chunk_count += 1;
        self.content_bytes += content.len();
    }

    /// Update the spinner animation
    pub fn update_spinner(&mut self) {
        if !self.is_tty || !self.active {
            return;
        }
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
        self.draw_spinner();
    }

    /// Update file stats to display (lines, bytes)
    pub fn update_file_stats(&mut self, lines: usize, bytes: usize) {
        self.file_stats = Some((lines, bytes));
    }

    /// Set the current tool being used
    pub fn set_current_tool(&mut self, tool: &str) {
        self.current_tool = Some(tool.to_string());
        self.tool_count += 1;
        self.draw_spinner();
    }

    /// Clear the current tool (tool finished)
    pub fn clear_current_tool(&mut self) {
        self.current_tool = None;
    }

    /// Format elapsed time as "Xm Ys" or "Xs"
    fn format_elapsed(&self) -> String {
        let secs = self.start_time.elapsed().as_secs();
        if secs >= 60 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}s", secs)
        }
    }

    /// Format bytes as "1.6m bytes", "1.6k bytes", or "847 bytes"
    fn format_bytes(bytes: usize) -> String {
        const KB: usize = 1024;
        const MB: usize = 1024 * 1024;

        if bytes >= MB {
            let mb = bytes as f64 / MB as f64;
            if mb >= 10.0 {
                format!("{:.0}m bytes", mb)
            } else {
                format!("{:.1}m bytes", mb)
            }
        } else if bytes >= KB {
            let kb = bytes as f64 / KB as f64;
            if kb >= 10.0 {
                format!("{:.0}k bytes", kb)
            } else {
                format!("{:.1}k bytes", kb)
            }
        } else {
            format!("{} bytes", bytes)
        }
    }

    /// Draw the spinner on the current line
    fn draw_spinner(&self) {
        if !self.is_tty {
            return;
        }

        let mut stdout = std::io::stdout();
        let frame = SPINNER_FRAMES[self.spinner_frame];
        let elapsed = self.format_elapsed();

        // Build spinner text with stats
        let spinner_text = if let Some((lines, bytes)) = self.file_stats {
            format!("{} {} [{}] ({} lines, {})", frame, self.message, elapsed, lines, Self::format_bytes(bytes))
        } else if let Some(ref tool) = self.current_tool {
            format!("{} {} [{}] (using {}...)", frame, self.message, elapsed, tool)
        } else if self.tool_count > 0 {
            format!("{} {} [{}] ({} tool calls)", frame, self.message, elapsed, self.tool_count)
        } else if self.chunk_count > 0 {
            format!("{} {} [{}] ({} streamed)", frame, self.message, elapsed, Self::format_bytes(self.content_bytes))
        } else {
            format!("{} {} [{}]", frame, self.message, elapsed)
        };

        // Carriage return, clear line, draw spinner
        write!(stdout, "\r\x1b[2K\x1b[36m{}\x1b[0m", spinner_text).ok();
        stdout.flush().ok();
    }

    /// Finish with success status
    pub fn finish_success(&mut self) {
        self.finish_with_status(true, None);
    }

    /// Finish with error status
    pub fn finish_error(&mut self, error_msg: &str) {
        self.finish_with_status(false, Some(error_msg));
    }

    fn finish_with_status(&mut self, success: bool, error_msg: Option<&str>) {
        let elapsed = self.format_elapsed();
        let summary = self.build_summary();

        if self.is_tty && self.active {
            let mut stdout = std::io::stdout();

            // Clear the spinner line and print final status
            write!(stdout, "\r\x1b[2K").ok();

            if success {
                if summary.is_empty() {
                    println!("{} {} [{}]", "✓".green(), self.message.green(), elapsed);
                } else {
                    println!("{} {} [{}] {}", "✓".green(), self.message.green(), elapsed, summary.dimmed());
                }
            } else {
                let err = error_msg.unwrap_or("error");
                println!("{} {} [{}]: {}", "✗".red(), self.message.red(), elapsed, err.red());
            }

            // Show cursor
            write!(stdout, "\x1b[?25h").ok();
            stdout.flush().ok();

            self.active = false;
        } else {
            if success {
                if summary.is_empty() {
                    println!("✓ {} [{}]", self.message, elapsed);
                } else {
                    println!("✓ {} [{}] {}", self.message, elapsed, summary);
                }
            } else {
                let err = error_msg.unwrap_or("error");
                println!("✗ {} [{}]: {}", self.message, elapsed, err);
            }
        }
    }

    /// Build a summary string of what was done
    fn build_summary(&self) -> String {
        let mut parts = Vec::new();

        if let Some((lines, bytes)) = self.file_stats {
            if bytes > 0 {
                parts.push(format!("wrote {} lines, {}", lines, Self::format_bytes(bytes)));
            }
        }

        if self.tool_count > 0 {
            parts.push(format!("{} tool calls", self.tool_count));
        }

        if self.file_stats.is_none() && self.content_bytes > 0 {
            parts.push(format!("{} streamed", Self::format_bytes(self.content_bytes)));
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!("({})", parts.join(", "))
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    #[allow(dead_code)]
    pub fn cancellation_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancelled)
    }

    #[allow(dead_code)]
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

impl Drop for StreamingDisplay {
    fn drop(&mut self) {
        if self.is_tty && self.active {
            let mut stdout = std::io::stdout();
            write!(stdout, "\x1b[?25h").ok();  // Show cursor
            write!(stdout, "\r\x1b[2K").ok();   // Clear line
            stdout.flush().ok();
        }
    }
}

#[allow(dead_code)]
pub type StreamCallback = Box<dyn FnMut(&str) + Send>;

#[allow(dead_code)]
#[derive(Default)]
pub struct StreamingConfig {
    pub on_line: Option<StreamCallback>,
    pub show_display: bool,
    pub spinner_message: String,
}

#[allow(dead_code)]
impl StreamingConfig {
    pub fn with_display(message: &str) -> Self {
        Self {
            on_line: None,
            show_display: true,
            spinner_message: message.to_string(),
        }
    }

    pub fn quiet() -> Self {
        Self {
            on_line: None,
            show_display: false,
            spinner_message: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_display_new() {
        let display = StreamingDisplay::new("Test message");
        assert_eq!(display.message, "Test message");
        assert!(!display.is_cancelled());
    }

    #[test]
    fn test_cancellation_flag() {
        let display = StreamingDisplay::new("Test");
        assert!(!display.is_cancelled());
        let flag = display.cancellation_flag();
        flag.store(true, Ordering::SeqCst);
        assert!(display.is_cancelled());
    }
}
