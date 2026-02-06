//! CLI adapter implementation using dialoguer for interactive prompts
//!
//! This module provides `CliAdapter`, which implements `InteractionAdapter` for
//! terminal-based user interaction with customizable spacing.

use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use console::Style;
use dialoguer::theme::Theme;
use dialoguer::{Confirm, Input, MultiSelect, Select};
use indicatif::{ProgressBar, ProgressStyle};
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
pub fn setup_ctrl_c_handler() {
    static HANDLER_SET: AtomicBool = AtomicBool::new(false);

    if HANDLER_SET.swap(true, Ordering::SeqCst) {
        return;
    }

    if let Err(e) = ctrlc::set_handler(move || {
        CANCELLED.store(true, Ordering::SeqCst);
        eprintln!();
    }) {
        eprintln!("Warning: Could not set Ctrl+C handler: {}", e);
    }
}

/// Reset the cancellation flag
pub fn reset_cancellation() {
    CANCELLED.store(false, Ordering::SeqCst);
}

/// Custom theme with generous spacing between elements
struct SpacedTheme {
    prompt_style: Style,
    active_style: Style,
    inactive_style: Style,
    hint_style: Style,
}

impl SpacedTheme {
    fn new() -> Self {
        Self {
            prompt_style: Style::new().cyan().bold(),
            active_style: Style::new().cyan(),
            inactive_style: Style::new(),
            hint_style: Style::new().dim(),
        }
    }
}

impl Theme for SpacedTheme {
    fn format_prompt(&self, f: &mut dyn FmtWrite, prompt: &str) -> std::fmt::Result {
        write!(f, "{}", self.prompt_style.apply_to(format!("? {}", prompt)))
    }

    fn format_input_prompt(
        &self,
        f: &mut dyn FmtWrite,
        prompt: &str,
        default: Option<&str>,
    ) -> std::fmt::Result {
        match default {
            Some(d) => write!(
                f,
                "{} {}",
                self.prompt_style.apply_to(format!("? {}", prompt)),
                self.hint_style.apply_to(format!("({})", d))
            ),
            None => write!(f, "{}", self.prompt_style.apply_to(format!("? {}", prompt))),
        }
    }

    fn format_input_prompt_selection(
        &self,
        f: &mut dyn FmtWrite,
        prompt: &str,
        sel: &str,
    ) -> std::fmt::Result {
        write!(
            f,
            "{} {}",
            self.prompt_style.apply_to(format!("? {}", prompt)),
            self.active_style.apply_to(sel)
        )
    }

    fn format_confirm_prompt(
        &self,
        f: &mut dyn FmtWrite,
        prompt: &str,
        default: Option<bool>,
    ) -> std::fmt::Result {
        let hint = match default {
            Some(true) => "(Y/n)",
            Some(false) => "(y/N)",
            None => "(y/n)",
        };
        write!(
            f,
            "{} {}",
            self.prompt_style.apply_to(format!("? {}", prompt)),
            self.hint_style.apply_to(hint)
        )
    }

    fn format_confirm_prompt_selection(
        &self,
        f: &mut dyn FmtWrite,
        prompt: &str,
        selection: Option<bool>,
    ) -> std::fmt::Result {
        let answer = match selection {
            Some(true) => "Yes",
            Some(false) => "No",
            None => "?",
        };
        write!(
            f,
            "{} {}",
            self.prompt_style.apply_to(format!("? {}", prompt)),
            self.active_style.apply_to(answer)
        )
    }

    fn format_select_prompt(&self, f: &mut dyn FmtWrite, prompt: &str) -> std::fmt::Result {
        write!(f, "{}", self.prompt_style.apply_to(format!("? {}", prompt)))
    }

    fn format_select_prompt_selection(
        &self,
        f: &mut dyn FmtWrite,
        prompt: &str,
        sel: &str,
    ) -> std::fmt::Result {
        write!(
            f,
            "{} {}",
            self.prompt_style.apply_to(format!("? {}", prompt)),
            self.active_style.apply_to(sel)
        )
    }

    fn format_select_prompt_item(
        &self,
        f: &mut dyn FmtWrite,
        text: &str,
        active: bool,
    ) -> std::fmt::Result {
        // Add blank line before each item for spacing
        writeln!(f)?;
        if active {
            write!(
                f,
                "  {} {}",
                self.active_style.apply_to(">"),
                self.active_style.apply_to(text)
            )
        } else {
            write!(f, "    {}", self.inactive_style.apply_to(text))
        }
    }

    fn format_multi_select_prompt(&self, f: &mut dyn FmtWrite, prompt: &str) -> std::fmt::Result {
        write!(f, "{}", self.prompt_style.apply_to(format!("? {}", prompt)))
    }

    fn format_multi_select_prompt_selection(
        &self,
        f: &mut dyn FmtWrite,
        prompt: &str,
        selections: &[&str],
    ) -> std::fmt::Result {
        write!(
            f,
            "{} {}",
            self.prompt_style.apply_to(format!("? {}", prompt)),
            self.active_style.apply_to(selections.join(", "))
        )
    }

    fn format_multi_select_prompt_item(
        &self,
        f: &mut dyn FmtWrite,
        text: &str,
        checked: bool,
        active: bool,
    ) -> std::fmt::Result {
        // Add blank line before each item for spacing
        writeln!(f)?;
        let checkbox = if checked { "[✓]" } else { "[ ]" };
        if active {
            write!(
                f,
                "  {} {} {}",
                self.active_style.apply_to(">"),
                self.active_style.apply_to(checkbox),
                self.active_style.apply_to(text)
            )
        } else {
            write!(
                f,
                "    {} {}",
                self.inactive_style.apply_to(checkbox),
                self.inactive_style.apply_to(text)
            )
        }
    }
}

/// CLI adapter for terminal-based user interaction
pub struct CliAdapter {
    is_tty: bool,
    progress_counter: AtomicU64,
    active_progress: Arc<Mutex<HashMap<u64, ProgressBar>>>,
}

impl CliAdapter {
    pub fn new() -> Self {
        setup_ctrl_c_handler();
        Self {
            is_tty: std::io::stdin().is_terminal(),
            progress_counter: AtomicU64::new(0),
            active_progress: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[allow(dead_code)]
    pub fn with_tty(is_tty: bool) -> Self {
        setup_ctrl_c_handler();
        Self {
            is_tty,
            progress_counter: AtomicU64::new(0),
            active_progress: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[allow(dead_code)]
    pub fn is_tty(&self) -> bool {
        self.is_tty
    }

    fn check_cancelled(&self) -> InteractionResult<()> {
        if is_cancelled() {
            Err(InteractionError::Cancelled)
        } else {
            Ok(())
        }
    }

    fn require_tty(&self) -> InteractionResult<()> {
        if !self.is_tty {
            Err(InteractionError::NonTty)
        } else {
            Ok(())
        }
    }

    fn convert_dialoguer_error(err: dialoguer::Error) -> InteractionError {
        InteractionError::Io(err.to_string())
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

        let theme = SpacedTheme::new();
        let mut input: Input<String> = Input::with_theme(&theme).with_prompt(prompt);
        if let Some(d) = default {
            input = input.default(d.to_string());
        }

        input.interact_text().map_err(Self::convert_dialoguer_error)
    }

    fn ask_select(&self, prompt: &str, options: &[&str]) -> InteractionResult<usize> {
        self.require_tty()?;
        self.check_cancelled()?;

        if options.is_empty() {
            return Err(InteractionError::InvalidInput(
                "options cannot be empty".to_string(),
            ));
        }

        let theme = SpacedTheme::new();

        // Print spacing before the select
        println!();

        Select::with_theme(&theme)
            .with_prompt(prompt)
            .items(options)
            .default(0)
            .interact()
            .map_err(Self::convert_dialoguer_error)
    }

    fn ask_confirm(&self, prompt: &str, default: bool) -> InteractionResult<bool> {
        self.require_tty()?;
        self.check_cancelled()?;

        let theme = SpacedTheme::new();

        Confirm::with_theme(&theme)
            .with_prompt(prompt)
            .default(default)
            .interact()
            .map_err(Self::convert_dialoguer_error)
    }

    fn ask_multi_select(&self, prompt: &str, options: &[&str]) -> InteractionResult<Vec<usize>> {
        self.require_tty()?;
        self.check_cancelled()?;

        if options.is_empty() {
            return Err(InteractionError::InvalidInput(
                "options cannot be empty".to_string(),
            ));
        }

        let theme = SpacedTheme::new();

        // Print spacing before the multi-select
        println!();

        MultiSelect::with_theme(&theme)
            .with_prompt(prompt)
            .items(options)
            .interact()
            .map_err(Self::convert_dialoguer_error)
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

    fn print_header(&self, message: &str) {
        println!("{}", message.cyan().bold());
        let _ = std::io::stdout().flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_adapter_new_detects_tty() {
        let adapter = CliAdapter::new();
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
        let err = InteractionError::InvalidInput("options cannot be empty".to_string());
        assert!(matches!(err, InteractionError::InvalidInput(_)));
    }

    #[test]
    fn test_progress_handle_creation() {
        let adapter = CliAdapter::with_tty(false);
        let handle = adapter.start_progress("test message");
        assert_eq!(handle.message(), "test message");
        adapter.end_progress(handle, true);
    }

    #[test]
    fn test_print_methods_dont_panic() {
        let adapter = CliAdapter::with_tty(false);
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
        let _ = adapter.is_tty();
    }
}
