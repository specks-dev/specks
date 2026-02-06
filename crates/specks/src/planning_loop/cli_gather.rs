//! CLI-mode requirement gathering for the planning loop
//!
//! Per [D18], in CLI mode the CLI code handles all user interaction directly
//! using inquire prompts instead of invoking the interviewer agent.

use std::path::Path;

use specks_core::interaction::{InteractionAdapter, InteractionError};
use specks_core::SpecksError;

use super::types::{LoopContext, PlanMode};

/// Result of the CLI gather phase
#[derive(Debug, Clone)]
pub struct GatherResult {
    /// Formatted requirements string for the planner agent
    pub requirements: String,
    /// Whether the user confirmed they want to proceed
    pub user_confirmed: bool,
}

/// Scope selection options for new ideas
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Full implementation with all bells and whistles
    Full,
    /// Minimal viable implementation
    Minimal,
    /// Custom scope (user will describe)
    Custom,
}

impl Scope {
    /// Get display label for the scope option
    pub fn label(&self) -> &'static str {
        match self {
            Scope::Full => "Full (complete implementation with all features)",
            Scope::Minimal => "Minimal (just the core functionality)",
            Scope::Custom => "Custom (I'll describe the scope)",
        }
    }

    /// Get all scope options as labels
    pub fn all_labels() -> Vec<&'static str> {
        vec![
            Scope::Full.label(),
            Scope::Minimal.label(),
            Scope::Custom.label(),
        ]
    }

    /// Create from selection index
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Scope::Full,
            1 => Scope::Minimal,
            _ => Scope::Custom,
        }
    }

    /// Get a brief description for the requirements string
    pub fn description(&self) -> &'static str {
        match self {
            Scope::Full => "full implementation",
            Scope::Minimal => "minimal viable implementation",
            Scope::Custom => "custom scope",
        }
    }
}

/// CLI gatherer for requirement collection in CLI mode
pub struct CliGatherer;

impl CliGatherer {
    /// Create a new CLI gatherer
    pub fn new() -> Self {
        Self
    }

    /// Gather requirements from the user via CLI prompts
    ///
    /// This is the CLI-mode equivalent of the interviewer agent's gather phase.
    /// It uses the interaction adapter to present prompts and collect responses.
    ///
    /// # Arguments
    /// * `adapter` - The interaction adapter for user prompts
    /// * `context` - The current loop context with input and mode
    ///
    /// # Returns
    /// A `GatherResult` containing the formatted requirements and confirmation status
    pub fn gather(
        &self,
        adapter: &dyn InteractionAdapter,
        context: &LoopContext,
    ) -> Result<GatherResult, SpecksError> {
        match context.mode {
            PlanMode::New => self.gather_new_idea(adapter, context),
            PlanMode::Revision => self.gather_revision(adapter, context),
        }
    }

    /// Gather requirements for a new idea
    fn gather_new_idea(
        &self,
        adapter: &dyn InteractionAdapter,
        context: &LoopContext,
    ) -> Result<GatherResult, SpecksError> {
        // Display the idea
        adapter.print_info(&format!("\nIdea: {}\n", context.input));

        // If there are context files, mention them
        if !context.context_files.is_empty() {
            adapter.print_info(&format!(
                "Additional context: {} file(s) provided\n",
                context.context_files.len()
            ));
        }

        // Ask about scope
        let scope_labels = Scope::all_labels();
        let scope_refs: Vec<&str> = scope_labels.iter().map(|s| *s).collect();
        let scope_index = adapter
            .ask_select("What scope should this feature have?", &scope_refs)
            .map_err(Self::convert_interaction_error)?;
        let scope = Scope::from_index(scope_index);

        // If custom scope, ask for description
        let custom_scope_description = if scope == Scope::Custom {
            let description = adapter
                .ask_text("Describe the scope you have in mind:", None)
                .map_err(Self::convert_interaction_error)?;
            Some(description)
        } else {
            None
        };

        // Ask about tests
        let include_tests = adapter
            .ask_confirm("Should this include tests?", true)
            .map_err(Self::convert_interaction_error)?;

        // Build a summary
        let scope_text = match &custom_scope_description {
            Some(desc) => format!("Custom scope: {}", desc),
            None => format!("Scope: {}", scope.description()),
        };

        let tests_text = if include_tests {
            "Including tests"
        } else {
            "No tests"
        };

        adapter.print_info(&format!("\nPlan summary:"));
        adapter.print_info(&format!("  - {}", scope_text));
        adapter.print_info(&format!("  - {}", tests_text));
        adapter.print_info("");

        // Ask for confirmation
        let confirmed = adapter
            .ask_confirm("Create plan with these settings?", true)
            .map_err(Self::convert_interaction_error)?;

        if !confirmed {
            return Ok(GatherResult {
                requirements: String::new(),
                user_confirmed: false,
            });
        }

        // Format requirements for the planner
        let mut requirements = String::new();

        requirements.push_str("## Idea\n\n");
        requirements.push_str(&context.input);
        requirements.push_str("\n\n");

        requirements.push_str("## Requirements\n\n");
        requirements.push_str(&format!("- **Scope**: {}\n", scope.description()));
        if let Some(ref desc) = custom_scope_description {
            requirements.push_str(&format!("  - Custom description: {}\n", desc));
        }
        requirements.push_str(&format!(
            "- **Tests**: {}\n",
            if include_tests { "Yes" } else { "No" }
        ));
        requirements.push_str("\n");

        // Include context files if any
        if !context.context_files.is_empty() {
            requirements.push_str("## Additional Context\n\n");
            for ctx_file in &context.context_files {
                requirements.push_str(ctx_file);
                requirements.push_str("\n\n");
            }
        }

        Ok(GatherResult {
            requirements,
            user_confirmed: true,
        })
    }

    /// Gather requirements for revising an existing speck
    fn gather_revision(
        &self,
        adapter: &dyn InteractionAdapter,
        context: &LoopContext,
    ) -> Result<GatherResult, SpecksError> {
        // Display the speck being revised
        adapter.print_info(&format!("\nRevising speck: {}\n", context.input));

        // Try to read and display a summary of the current speck
        if let Some(ref speck_path) = context.speck_path {
            if let Some(summary) = self.read_speck_summary(speck_path) {
                adapter.print_info("Current speck summary:");
                adapter.print_info(&format!("  {}\n", summary));
            }
        }

        // Ask what the user wants to change
        let revision_description = adapter
            .ask_text("What would you like to change?", None)
            .map_err(Self::convert_interaction_error)?;

        // Confirm
        adapter.print_info(&format!("\nRevision request: {}\n", revision_description));
        let confirmed = adapter
            .ask_confirm("Proceed with this revision?", true)
            .map_err(Self::convert_interaction_error)?;

        if !confirmed {
            return Ok(GatherResult {
                requirements: String::new(),
                user_confirmed: false,
            });
        }

        // Format requirements for the planner
        let mut requirements = String::new();

        requirements.push_str("## Revision Request\n\n");
        requirements.push_str(&format!("Revising speck: {}\n\n", context.input));
        requirements.push_str("### Changes Requested\n\n");
        requirements.push_str(&revision_description);
        requirements.push_str("\n\n");

        // Include context files if any
        if !context.context_files.is_empty() {
            requirements.push_str("## Additional Context\n\n");
            for ctx_file in &context.context_files {
                requirements.push_str(ctx_file);
                requirements.push_str("\n\n");
            }
        }

        Ok(GatherResult {
            requirements,
            user_confirmed: true,
        })
    }

    /// Read a brief summary from a speck file
    ///
    /// Extracts the purpose or first meaningful content from the speck.
    fn read_speck_summary(&self, path: &Path) -> Option<String> {
        let content = std::fs::read_to_string(path).ok()?;

        // Look for Purpose line
        for line in content.lines() {
            if line.starts_with("**Purpose:**") {
                let purpose = line.strip_prefix("**Purpose:**")?.trim();
                if !purpose.is_empty() {
                    // Truncate if too long
                    if purpose.len() > 100 {
                        return Some(format!("{}...", &purpose[..97]));
                    }
                    return Some(purpose.to_string());
                }
            }
        }

        // Fall back to first H2 heading
        for line in content.lines() {
            if line.starts_with("## ") {
                let heading = line.strip_prefix("## ")?.trim();
                if !heading.is_empty() {
                    return Some(heading.to_string());
                }
            }
        }

        None
    }

    /// Convert an InteractionError to a SpecksError
    fn convert_interaction_error(err: InteractionError) -> SpecksError {
        match err {
            InteractionError::Cancelled => SpecksError::UserAborted,
            InteractionError::NonTty => SpecksError::InteractionFailed {
                reason: "stdin is not a TTY - interactive input unavailable".to_string(),
            },
            InteractionError::Timeout { secs } => SpecksError::AgentTimeout { secs },
            _ => SpecksError::InteractionFailed {
                reason: err.to_string(),
            },
        }
    }
}

impl Default for CliGatherer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specks_core::interaction::{InteractionResult, ProgressHandle};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::sync::Mutex;

    /// A configurable mock adapter for testing (thread-safe)
    struct ConfigurableMockAdapter {
        progress_counter: AtomicU64,
        /// Responses for ask_select calls (in order)
        select_responses: Mutex<Vec<usize>>,
        /// Current index for select responses
        select_index: AtomicUsize,
        /// Responses for ask_confirm calls (in order)
        confirm_responses: Mutex<Vec<bool>>,
        /// Current index for confirm responses
        confirm_index: AtomicUsize,
        /// Responses for ask_text calls (in order)
        text_responses: Mutex<Vec<String>>,
        /// Current index for text responses
        text_index: AtomicUsize,
        /// Printed messages for verification
        printed_messages: Mutex<Vec<String>>,
    }

    impl ConfigurableMockAdapter {
        fn new() -> Self {
            Self {
                progress_counter: AtomicU64::new(0),
                select_responses: Mutex::new(vec![]),
                select_index: AtomicUsize::new(0),
                confirm_responses: Mutex::new(vec![]),
                confirm_index: AtomicUsize::new(0),
                text_responses: Mutex::new(vec![]),
                text_index: AtomicUsize::new(0),
                printed_messages: Mutex::new(vec![]),
            }
        }

        fn with_select_responses(self, responses: Vec<usize>) -> Self {
            *self.select_responses.lock().unwrap() = responses;
            self
        }

        fn with_confirm_responses(self, responses: Vec<bool>) -> Self {
            *self.confirm_responses.lock().unwrap() = responses;
            self
        }

        fn with_text_responses(self, responses: Vec<String>) -> Self {
            *self.text_responses.lock().unwrap() = responses;
            self
        }

        #[allow(dead_code)]
        fn get_printed_messages(&self) -> Vec<String> {
            self.printed_messages.lock().unwrap().clone()
        }
    }

    impl InteractionAdapter for ConfigurableMockAdapter {
        fn ask_text(&self, _prompt: &str, default: Option<&str>) -> InteractionResult<String> {
            let responses = self.text_responses.lock().unwrap();
            let index = self.text_index.fetch_add(1, Ordering::SeqCst);
            if index < responses.len() {
                Ok(responses[index].clone())
            } else {
                Ok(default.unwrap_or("mock text").to_string())
            }
        }

        fn ask_select(&self, _prompt: &str, _options: &[&str]) -> InteractionResult<usize> {
            let responses = self.select_responses.lock().unwrap();
            let index = self.select_index.fetch_add(1, Ordering::SeqCst);
            if index < responses.len() {
                Ok(responses[index])
            } else {
                Ok(0)
            }
        }

        fn ask_confirm(&self, _prompt: &str, default: bool) -> InteractionResult<bool> {
            let responses = self.confirm_responses.lock().unwrap();
            let index = self.confirm_index.fetch_add(1, Ordering::SeqCst);
            if index < responses.len() {
                Ok(responses[index])
            } else {
                Ok(default)
            }
        }

        fn ask_multi_select(
            &self,
            _prompt: &str,
            _options: &[&str],
        ) -> InteractionResult<Vec<usize>> {
            Ok(vec![0])
        }

        fn start_progress(&self, message: &str) -> ProgressHandle {
            let id = self.progress_counter.fetch_add(1, Ordering::SeqCst);
            ProgressHandle::new(id, message)
        }

        fn end_progress(&self, _handle: ProgressHandle, _success: bool) {}

        fn print_info(&self, message: &str) {
            self.printed_messages.lock().unwrap().push(message.to_string());
        }
        fn print_warning(&self, message: &str) {
            self.printed_messages.lock().unwrap().push(message.to_string());
        }
        fn print_error(&self, message: &str) {
            self.printed_messages.lock().unwrap().push(message.to_string());
        }
        fn print_success(&self, message: &str) {
            self.printed_messages.lock().unwrap().push(message.to_string());
        }
    }

    #[test]
    fn test_cli_gatherer_new_idea_full_scope_with_tests() {
        let adapter = ConfigurableMockAdapter::new()
            .with_select_responses(vec![0]) // Full scope
            .with_confirm_responses(vec![true, true]); // Include tests, confirm

        let gatherer = CliGatherer::new();
        let context = LoopContext::new_idea("add a caching layer".to_string(), vec![]);

        let result = gatherer.gather(&adapter, &context).unwrap();

        assert!(result.user_confirmed);
        assert!(result.requirements.contains("add a caching layer"));
        assert!(result.requirements.contains("full implementation"));
        assert!(result.requirements.contains("Tests**: Yes"));
    }

    #[test]
    fn test_cli_gatherer_new_idea_minimal_scope_no_tests() {
        let adapter = ConfigurableMockAdapter::new()
            .with_select_responses(vec![1]) // Minimal scope
            .with_confirm_responses(vec![false, true]); // No tests, confirm

        let gatherer = CliGatherer::new();
        let context = LoopContext::new_idea("simple feature".to_string(), vec![]);

        let result = gatherer.gather(&adapter, &context).unwrap();

        assert!(result.user_confirmed);
        assert!(result.requirements.contains("minimal viable implementation"));
        assert!(result.requirements.contains("Tests**: No"));
    }

    #[test]
    fn test_cli_gatherer_new_idea_custom_scope() {
        let adapter = ConfigurableMockAdapter::new()
            .with_select_responses(vec![2]) // Custom scope
            .with_text_responses(vec!["only the API endpoints".to_string()])
            .with_confirm_responses(vec![true, true]); // Include tests, confirm

        let gatherer = CliGatherer::new();
        let context = LoopContext::new_idea("new API".to_string(), vec![]);

        let result = gatherer.gather(&adapter, &context).unwrap();

        assert!(result.user_confirmed);
        assert!(result.requirements.contains("custom scope"));
        assert!(result.requirements.contains("only the API endpoints"));
    }

    #[test]
    fn test_cli_gatherer_new_idea_user_cancels() {
        let adapter = ConfigurableMockAdapter::new()
            .with_select_responses(vec![0]) // Full scope
            .with_confirm_responses(vec![true, false]); // Include tests, but cancel confirmation

        let gatherer = CliGatherer::new();
        let context = LoopContext::new_idea("cancelled idea".to_string(), vec![]);

        let result = gatherer.gather(&adapter, &context).unwrap();

        assert!(!result.user_confirmed);
        assert!(result.requirements.is_empty());
    }

    #[test]
    fn test_cli_gatherer_revision_mode() {
        let adapter = ConfigurableMockAdapter::new()
            .with_text_responses(vec!["fix the error handling".to_string()])
            .with_confirm_responses(vec![true]); // Confirm revision

        let gatherer = CliGatherer::new();
        let context = LoopContext::revision(
            PathBuf::from("/project/.specks/specks-1.md"),
            vec![],
        );

        let result = gatherer.gather(&adapter, &context).unwrap();

        assert!(result.user_confirmed);
        assert!(result.requirements.contains("Revision Request"));
        assert!(result.requirements.contains("fix the error handling"));
    }

    #[test]
    fn test_cli_gatherer_revision_mode_user_cancels() {
        let adapter = ConfigurableMockAdapter::new()
            .with_text_responses(vec!["some changes".to_string()])
            .with_confirm_responses(vec![false]); // Cancel revision

        let gatherer = CliGatherer::new();
        let context = LoopContext::revision(
            PathBuf::from("/project/.specks/specks-1.md"),
            vec![],
        );

        let result = gatherer.gather(&adapter, &context).unwrap();

        assert!(!result.user_confirmed);
        assert!(result.requirements.is_empty());
    }

    #[test]
    fn test_cli_gatherer_with_context_files() {
        let adapter = ConfigurableMockAdapter::new()
            .with_select_responses(vec![0])
            .with_confirm_responses(vec![true, true]);

        let gatherer = CliGatherer::new();
        let context = LoopContext::new_idea(
            "feature with context".to_string(),
            vec!["--- file1.md ---\nsome context".to_string()],
        );

        let result = gatherer.gather(&adapter, &context).unwrap();

        assert!(result.user_confirmed);
        assert!(result.requirements.contains("Additional Context"));
        assert!(result.requirements.contains("file1.md"));
    }

    #[test]
    fn test_scope_from_index() {
        assert_eq!(Scope::from_index(0), Scope::Full);
        assert_eq!(Scope::from_index(1), Scope::Minimal);
        assert_eq!(Scope::from_index(2), Scope::Custom);
        assert_eq!(Scope::from_index(99), Scope::Custom); // Out of bounds defaults to Custom
    }

    #[test]
    fn test_scope_labels() {
        let labels = Scope::all_labels();
        assert_eq!(labels.len(), 3);
        assert!(labels[0].contains("Full"));
        assert!(labels[1].contains("Minimal"));
        assert!(labels[2].contains("Custom"));
    }

    #[test]
    fn test_read_speck_summary_with_purpose() {
        // Create a temp file with Purpose line
        let temp_dir = std::env::temp_dir().join("specks-test-gather-summary");
        let _ = std::fs::create_dir_all(&temp_dir);
        let speck_path = temp_dir.join("test-speck.md");

        std::fs::write(
            &speck_path,
            "## Phase 1\n\n**Purpose:** Add authentication to the API.\n\n### Details\n",
        )
        .unwrap();

        let gatherer = CliGatherer::new();
        let summary = gatherer.read_speck_summary(&speck_path);

        assert!(summary.is_some());
        assert_eq!(summary.unwrap(), "Add authentication to the API.");

        // Clean up
        let _ = std::fs::remove_file(&speck_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_read_speck_summary_fallback_to_h2() {
        // Create a temp file without Purpose line
        let temp_dir = std::env::temp_dir().join("specks-test-gather-summary-h2");
        let _ = std::fs::create_dir_all(&temp_dir);
        let speck_path = temp_dir.join("test-speck-h2.md");

        std::fs::write(&speck_path, "# Main Title\n\n## Authentication Feature\n\nSome content.\n")
            .unwrap();

        let gatherer = CliGatherer::new();
        let summary = gatherer.read_speck_summary(&speck_path);

        assert!(summary.is_some());
        assert_eq!(summary.unwrap(), "Authentication Feature");

        // Clean up
        let _ = std::fs::remove_file(&speck_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_read_speck_summary_nonexistent_file() {
        let gatherer = CliGatherer::new();
        let summary = gatherer.read_speck_summary(Path::new("/nonexistent/path/speck.md"));
        assert!(summary.is_none());
    }

    #[test]
    fn test_cli_gatherer_default() {
        let gatherer = CliGatherer::default();
        // Just verify it creates successfully
        let _ = gatherer;
    }
}
