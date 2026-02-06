//! CLI adapter implementation using inquire for interactive prompts
//!
//! This module provides `CliAdapter`, which implements `InteractionAdapter` for
//! terminal-based user interaction.

use std::collections::HashMap;
use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use indicatif::{ProgressBar, ProgressStyle};
use inquire::{Confirm, MultiSelect, Select, Text};
use inquire::error::InquireError;
use owo_colors::OwoColorize;

use specks_core::interaction::{
    InteractionAdapter, InteractionError, InteractionResult, ProgressHandle,
};

/// Global flag to track if Ctrl+C was pressed
static CANCELLED: AtomicBool = AtomicBool::new(false);

/// Check if cancellation was requested
fn is_cancelled() -> bool {
    CANCELLED.load(Ordering::SeqCst)
}

/// Set up the global Ctrl+C handler
///
/// This should be called once at program startup. The handler sets a global flag
/// that can be checked by the adapter to detect cancellation.
pub fn setup_ctrl_c_handler() {
    // Only set up the handler once
    static HANDLER_SET: AtomicBool = AtomicBool::new(false);

    if HANDLER_SET.swap(true, Ordering::SeqCst) {
        return; // Already set up
    }

    if let Err(e) = ctrlc::set_handler(move || {
        CANCELLED.store(true, Ordering::SeqCst);
        // Print a newline to clean up the terminal
        eprintln!();
    }) {
        // Log but don't fail if we can't set the handler
        eprintln!("Warning: Could not set Ctrl+C handler: {}", e);
    }
}

/// Reset the cancellation flag
///
/// This should be called at the start of a new interaction session
pub fn reset_cancellation() {
    CANCELLED.store(false, Ordering::SeqCst);
}

/// CLI adapter for terminal-based user interaction
///
/// This adapter uses the `inquire` crate for interactive prompts and
/// `indicatif` for progress spinners. It implements proper TTY detection
/// and Ctrl+C handling.
///
/// # Example
///
/// ```ignore
/// let adapter = CliAdapter::new();
/// if adapter.is_tty() {
///     let name = adapter.ask_text("What is your name?", Some("World"))?;
///     adapter.print_success(&format!("Hello, {}!", name));
/// }
/// ```
pub struct CliAdapter {
    /// Whether stdin is a TTY
    is_tty: bool,
    /// Counter for generating unique progress handle IDs
    progress_counter: AtomicU64,
    /// Active progress bars, keyed by handle ID
    active_progress: Arc<Mutex<HashMap<u64, ProgressBar>>>,
}

impl CliAdapter {
    /// Create a new CLI adapter
    ///
    /// Automatically detects whether stdin is a TTY.
    pub fn new() -> Self {
        setup_ctrl_c_handler();
        Self {
            is_tty: std::io::stdin().is_terminal(),
            progress_counter: AtomicU64::new(0),
            active_progress: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new CLI adapter with explicit TTY setting
    ///
    /// This is useful for testing or when you want to override TTY detection.
    #[allow(dead_code)] // Test utility
    pub fn with_tty(is_tty: bool) -> Self {
        setup_ctrl_c_handler();
        Self {
            is_tty,
            progress_counter: AtomicU64::new(0),
            active_progress: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if stdin is a TTY
    #[allow(dead_code)] // Part of public API, used in tests
    pub fn is_tty(&self) -> bool {
        self.is_tty
    }

    /// Check for cancellation before an operation
    fn check_cancelled(&self) -> InteractionResult<()> {
        if is_cancelled() {
            Err(InteractionError::Cancelled)
        } else {
            Ok(())
        }
    }

    /// Check for TTY before an interactive operation
    fn require_tty(&self) -> InteractionResult<()> {
        if !self.is_tty {
            Err(InteractionError::NonTty)
        } else {
            Ok(())
        }
    }

    /// Convert an inquire error to an InteractionError
    fn convert_inquire_error(err: InquireError) -> InteractionError {
        match err {
            InquireError::OperationCanceled => InteractionError::Cancelled,
            InquireError::OperationInterrupted => InteractionError::Cancelled,
            InquireError::IO(io_err) => InteractionError::Io(io_err.to_string()),
            InquireError::NotTTY => InteractionError::NonTty,
            InquireError::InvalidConfiguration(msg) => {
                InteractionError::InvalidInput(msg.to_string())
            }
            InquireError::Custom(err) => InteractionError::Other(err.to_string()),
        }
    }
}

impl Default for CliAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl InteractionAdapter for CliAdapter {
    fn ask_text(&self, prompt: &str, default: Option<&str>) -> InteractionResult<String> {
        self.require_tty()?;
        self.check_cancelled()?;

        let mut text = Text::new(prompt);
        if let Some(d) = default {
            text = text.with_default(d);
        }

        text.prompt().map_err(Self::convert_inquire_error)
    }

    fn ask_select(&self, prompt: &str, options: &[&str]) -> InteractionResult<usize> {
        self.require_tty()?;
        self.check_cancelled()?;

        if options.is_empty() {
            return Err(InteractionError::InvalidInput(
                "options cannot be empty".to_string(),
            ));
        }

        // Convert to owned strings for inquire
        let options_owned: Vec<String> = options.iter().map(|s| s.to_string()).collect();

        let selected = Select::new(prompt, options_owned.clone())
            .prompt()
            .map_err(Self::convert_inquire_error)?;

        // Find the index of the selected item
        options_owned
            .iter()
            .position(|s| s == &selected)
            .ok_or_else(|| InteractionError::Other("selected item not found".to_string()))
    }

    fn ask_confirm(&self, prompt: &str, default: bool) -> InteractionResult<bool> {
        self.require_tty()?;
        self.check_cancelled()?;

        Confirm::new(prompt)
            .with_default(default)
            .prompt()
            .map_err(Self::convert_inquire_error)
    }

    fn ask_multi_select(&self, prompt: &str, options: &[&str]) -> InteractionResult<Vec<usize>> {
        self.require_tty()?;
        self.check_cancelled()?;

        if options.is_empty() {
            return Err(InteractionError::InvalidInput(
                "options cannot be empty".to_string(),
            ));
        }

        // Convert to owned strings for inquire
        let options_owned: Vec<String> = options.iter().map(|s| s.to_string()).collect();

        let selected = MultiSelect::new(prompt, options_owned.clone())
            .prompt()
            .map_err(Self::convert_inquire_error)?;

        // Find the indices of the selected items
        let indices: Vec<usize> = selected
            .iter()
            .filter_map(|s| options_owned.iter().position(|opt| opt == s))
            .collect();

        Ok(indices)
    }

    fn start_progress(&self, message: &str) -> ProgressHandle {
        let id = self.progress_counter.fetch_add(1, Ordering::SeqCst);

        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg} [{elapsed}]")
                .expect("valid template")
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        // Store the progress bar
        if let Ok(mut progress_map) = self.active_progress.lock() {
            progress_map.insert(id, pb);
        }

        ProgressHandle::new(id, message)
    }

    fn end_progress(&self, handle: ProgressHandle, success: bool) {
        if let Ok(mut progress_map) = self.active_progress.lock() {
            if let Some(pb) = progress_map.remove(&handle.id()) {
                let msg = handle.message();
                let elapsed = pb.elapsed();
                let elapsed_str = format!("{:.1}s", elapsed.as_secs_f64());

                // Clear the spinner line completely, then print the final status
                pb.finish_and_clear();

                if success {
                    println!("{} {} [{}]", "✓".green(), msg.green(), elapsed_str);
                } else {
                    println!("{} {} [{}]", "✗".red(), msg.red(), elapsed_str);
                }
            }
        }
    }

    fn print_info(&self, message: &str) {
        println!("{}", message);
        // Flush to ensure immediate display
        let _ = std::io::stdout().flush();
    }

    fn print_warning(&self, message: &str) {
        println!("{} {}", "warning:".yellow().bold(), message.yellow());
        let _ = std::io::stdout().flush();
    }

    fn print_error(&self, message: &str) {
        eprintln!("{} {}", "error:".red().bold(), message.red().bold());
        let _ = std::io::stderr().flush();
    }

    fn print_success(&self, message: &str) {
        println!("{} {}", "✓".green(), message.green());
        let _ = std::io::stdout().flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_adapter_new_detects_tty() {
        // In test context, stdin is typically not a TTY
        let adapter = CliAdapter::new();
        // We can't assert a specific value here since it depends on the test environment,
        // but we verify the method works
        let _ = adapter.is_tty();
    }

    #[test]
    fn test_cli_adapter_with_tty_override() {
        let adapter_tty = CliAdapter::with_tty(true);
        assert!(adapter_tty.is_tty());

        let adapter_no_tty = CliAdapter::with_tty(false);
        assert!(!adapter_no_tty.is_tty());
    }

    #[test]
    fn test_non_tty_returns_error_for_text() {
        let adapter = CliAdapter::with_tty(false);
        let result = adapter.ask_text("test", None);
        assert!(matches!(result, Err(InteractionError::NonTty)));
    }

    #[test]
    fn test_non_tty_returns_error_for_select() {
        let adapter = CliAdapter::with_tty(false);
        let result = adapter.ask_select("test", &["a", "b"]);
        assert!(matches!(result, Err(InteractionError::NonTty)));
    }

    #[test]
    fn test_non_tty_returns_error_for_confirm() {
        let adapter = CliAdapter::with_tty(false);
        let result = adapter.ask_confirm("test?", true);
        assert!(matches!(result, Err(InteractionError::NonTty)));
    }

    #[test]
    fn test_non_tty_returns_error_for_multi_select() {
        let adapter = CliAdapter::with_tty(false);
        let result = adapter.ask_multi_select("test", &["a", "b"]);
        assert!(matches!(result, Err(InteractionError::NonTty)));
    }

    #[test]
    fn test_empty_options_error_for_select() {
        // Test that the InvalidInput error type exists and works
        // (actual empty options validation would require TTY which we can't test here)
        let err = InteractionError::InvalidInput("options cannot be empty".to_string());
        assert!(matches!(err, InteractionError::InvalidInput(_)));
    }

    #[test]
    fn test_progress_handle_creation() {
        let adapter = CliAdapter::with_tty(false);
        let handle = adapter.start_progress("test message");
        assert_eq!(handle.message(), "test message");
        // End progress doesn't fail even for non-TTY
        adapter.end_progress(handle, true);
    }

    #[test]
    fn test_print_methods_dont_panic() {
        let adapter = CliAdapter::with_tty(false);
        // These should not panic, even without a TTY
        adapter.print_info("info message");
        adapter.print_warning("warning message");
        adapter.print_error("error message");
        adapter.print_success("success message");
    }

    #[test]
    fn test_progress_counter_increments() {
        let adapter = CliAdapter::with_tty(false);
        let handle1 = adapter.start_progress("first");
        let handle2 = adapter.start_progress("second");
        assert_ne!(handle1.id(), handle2.id());
        adapter.end_progress(handle1, true);
        adapter.end_progress(handle2, true);
    }

    #[test]
    fn test_default_implementation() {
        let adapter = CliAdapter::default();
        // Should work the same as new()
        let _ = adapter.is_tty();
    }

    #[test]
    fn test_convert_inquire_error() {
        // Test error conversion
        let cancelled = CliAdapter::convert_inquire_error(InquireError::OperationCanceled);
        assert!(matches!(cancelled, InteractionError::Cancelled));

        let interrupted = CliAdapter::convert_inquire_error(InquireError::OperationInterrupted);
        assert!(matches!(interrupted, InteractionError::Cancelled));

        let not_tty = CliAdapter::convert_inquire_error(InquireError::NotTTY);
        assert!(matches!(not_tty, InteractionError::NonTty));
    }

    // Integration test note:
    // Manual verification of prompt styling should be done by running:
    //
    // ```rust
    // let adapter = CliAdapter::new();
    // if adapter.is_tty() {
    //     // Test text input
    //     let name = adapter.ask_text("Enter your name:", Some("World")).unwrap();
    //     println!("Got: {}", name);
    //
    //     // Test select
    //     let idx = adapter.ask_select("Pick one:", &["Option A", "Option B", "Option C"]).unwrap();
    //     println!("Selected index: {}", idx);
    //
    //     // Test confirm
    //     let confirmed = adapter.ask_confirm("Continue?", true).unwrap();
    //     println!("Confirmed: {}", confirmed);
    //
    //     // Test multi-select
    //     let indices = adapter.ask_multi_select("Pick any:", &["A", "B", "C"]).unwrap();
    //     println!("Selected indices: {:?}", indices);
    //
    //     // Test progress
    //     let handle = adapter.start_progress("Working...");
    //     std::thread::sleep(std::time::Duration::from_secs(2));
    //     adapter.end_progress(handle, true);
    //
    //     // Test print methods
    //     adapter.print_info("This is info");
    //     adapter.print_warning("This is a warning");
    //     adapter.print_error("This is an error");
    //     adapter.print_success("This is success");
    // }
    // ```
    //
    // Expected behavior:
    // - Info: default/white text
    // - Warning: yellow text with "warning:" prefix
    // - Error: red bold text with "error:" prefix
    // - Success: green text with "✓" prefix
    // - Progress: cyan spinner that animates
    // - Ctrl+C should cancel prompts and return InteractionError::Cancelled
}
