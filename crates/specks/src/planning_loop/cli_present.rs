//! CLI-mode result presentation for the planning loop
//!
//! Per [D18], in CLI mode the CLI code handles all user interaction directly
//! using inquire prompts instead of invoking the interviewer agent.
//!
//! This module handles presenting the speck and critic feedback to the user,
//! building a punch list of issues, and collecting the user's decision.

use std::path::Path;

use specks_core::SpecksError;
use specks_core::interaction::{InteractionAdapter, InteractionError};

use super::types::{LoopContext, UserDecision};

/// Priority levels for punch list items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    /// High priority - blocking issues (displayed in red)
    High,
    /// Medium priority - should address (displayed in yellow)
    Medium,
    /// Low priority - suggestions (displayed in default color)
    Low,
}

impl Priority {
    /// Get display label for the priority
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            Priority::High => "HIGH",
            Priority::Medium => "MEDIUM",
            Priority::Low => "LOW",
        }
    }
}

/// An item in the punch list
#[derive(Debug, Clone)]
pub struct PunchListItem {
    /// Priority of the item
    pub priority: Priority,
    /// Description of the issue
    pub description: String,
}

impl PunchListItem {
    /// Create a new punch list item
    pub fn new(priority: Priority, description: impl Into<String>) -> Self {
        Self {
            priority,
            description: description.into(),
        }
    }
}

/// Result of parsing critic feedback
#[derive(Debug, Clone, Default)]
pub struct CriticSummary {
    /// Whether the critic approved the speck
    pub approved: bool,
    /// Overall assessment
    pub assessment: String,
    /// Punch list items extracted from feedback
    pub punch_list: Vec<PunchListItem>,
}

/// User decision options for the select prompt
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionOption {
    /// Approve this plan
    Approve,
    /// Revise with feedback
    Revise,
    /// Abort planning
    Abort,
}

impl DecisionOption {
    /// Get display label for the option
    pub fn label(&self) -> &'static str {
        match self {
            DecisionOption::Approve => "Approve this plan",
            DecisionOption::Revise => "Revise with feedback",
            DecisionOption::Abort => "Abort planning",
        }
    }

    /// Get all options as labels
    pub fn all_labels() -> Vec<&'static str> {
        vec![
            DecisionOption::Approve.label(),
            DecisionOption::Revise.label(),
            DecisionOption::Abort.label(),
        ]
    }

    /// Create from selection index
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => DecisionOption::Approve,
            1 => DecisionOption::Revise,
            _ => DecisionOption::Abort,
        }
    }
}

/// CLI presenter for result presentation in CLI mode
pub struct CliPresenter;

impl CliPresenter {
    /// Create a new CLI presenter
    pub fn new() -> Self {
        Self
    }

    /// Present the speck and critic feedback, then get user decision
    ///
    /// This is the CLI-mode equivalent of the interviewer agent's present phase.
    /// It uses the interaction adapter to display results and collect the decision.
    ///
    /// # Arguments
    /// * `adapter` - The interaction adapter for user prompts
    /// * `context` - The current loop context with speck path and critic feedback
    ///
    /// # Returns
    /// A `UserDecision` indicating whether to approve, revise, or abort
    pub fn present(
        &self,
        adapter: &dyn InteractionAdapter,
        context: &LoopContext,
    ) -> Result<UserDecision, SpecksError> {
        // Print speck path with success message
        if let Some(ref speck_path) = context.speck_path {
            adapter.print_success(&format!("\nSpeck created: {}", speck_path.display()));

            // Print speck summary (title, step count, scope)
            if let Some(summary) = self.read_speck_summary(speck_path) {
                adapter.print_info(&format!("Summary: {}", summary));
            }
        }

        // Parse and display critic feedback
        let critic_summary = if let Some(ref feedback) = context.critic_feedback {
            let summary = Self::parse_critic_feedback(feedback);
            self.display_critic_feedback(adapter, &summary);
            summary
        } else {
            adapter.print_warning("\nNo critic feedback available.");
            CriticSummary::default()
        };

        // Present punch list if there are issues
        if !critic_summary.punch_list.is_empty() {
            self.display_punch_list(adapter, &critic_summary.punch_list);
        }

        adapter.print_info(""); // Blank line before decision

        // Ask for user decision
        let decision = self.ask_decision(adapter, &critic_summary)?;

        Ok(decision)
    }

    /// Parse critic feedback into a structured summary
    ///
    /// Extracts approval status, assessment, and punch list items from the feedback.
    /// Filters out "thinking" lines like "I'll...", "Let me...", etc.
    ///
    /// This is a static method so it can be called without a `CliPresenter` instance.
    pub fn parse_critic_feedback(feedback: &str) -> CriticSummary {
        let mut summary = CriticSummary::default();

        // Check for approval indicators
        let lower_feedback = feedback.to_lowercase();
        summary.approved = lower_feedback.contains("approved")
            || lower_feedback.contains("ready for implementation")
            || lower_feedback.contains("looks good")
            || (lower_feedback.contains("no major issues")
                && !lower_feedback.contains("not approved"));

        // Find a meaningful assessment line (skip thinking lines)
        for line in feedback.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !Self::is_thinking_line(trimmed) {
                // Look for lines that look like summaries
                if trimmed.starts_with("##")
                    || trimmed.starts_with("**")
                    || trimmed.contains("Review")
                    || trimmed.contains("Summary")
                    || trimmed.contains("Approved")
                    || trimmed.contains("approved")
                {
                    summary.assessment = trimmed.trim_start_matches('#').trim().to_string();
                    break;
                }
            }
        }

        // If no good assessment found, use a default based on approval status
        if summary.assessment.is_empty() {
            summary.assessment = if summary.approved {
                "Plan approved".to_string()
            } else {
                "Plan needs revision".to_string()
            };
        }

        // Extract punch list items based on keywords and patterns
        for line in feedback.lines() {
            let trimmed = line.trim();

            // Skip empty lines and thinking lines
            if trimmed.is_empty() || Self::is_thinking_line(trimmed) {
                continue;
            }

            // Look for bullet points or numbered items
            if trimmed.starts_with('-')
                || trimmed.starts_with('*')
                || trimmed.starts_with("•")
                || trimmed.starts_with("✅")
                || trimmed.starts_with("✓")
                || trimmed
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
            {
                let item_text = trimmed
                    .trim_start_matches(|c: char| {
                        c == '-'
                            || c == '*'
                            || c == '•'
                            || c == '✅'
                            || c == '✓'
                            || c.is_ascii_digit()
                            || c == '.'
                            || c == ')'
                            || c.is_whitespace()
                    })
                    .trim();

                if item_text.is_empty() || Self::is_thinking_line(item_text) {
                    continue;
                }

                // Determine priority based on keywords
                let lower_item = item_text.to_lowercase();
                let priority = if lower_item.contains("critical")
                    || lower_item.contains("blocking")
                    || lower_item.contains("must fix")
                    || lower_item.contains("missing required")
                {
                    Priority::High
                } else if lower_item.contains("minor")
                    || lower_item.contains("optional")
                    || lower_item.contains("nice to have")
                    || lower_item.contains("could")
                {
                    Priority::Low
                } else {
                    // Default to medium
                    Priority::Medium
                };

                summary
                    .punch_list
                    .push(PunchListItem::new(priority, item_text));
            }
        }

        summary
    }

    /// Check if a line is "thinking" text that should be filtered out
    fn is_thinking_line(line: &str) -> bool {
        let lower = line.to_lowercase();
        lower.starts_with("i'll ")
            || lower.starts_with("i will ")
            || lower.starts_with("let me ")
            || lower.starts_with("now let me")
            || lower.starts_with("now i'll")
            || lower.starts_with("first,")
            || lower.starts_with("next,")
            || lower.contains("let me start")
            || lower.contains("let me check")
            || lower.contains("let me read")
            || lower.contains("let me create")
            || lower.contains("let me write")
            || lower.contains("let me identify")
            || lower.contains("i need to")
    }

    /// Display critic feedback
    fn display_critic_feedback(&self, adapter: &dyn InteractionAdapter, summary: &CriticSummary) {
        adapter.print_info("");

        // Show approval status with clear visual
        if summary.approved {
            adapter.print_success("✓ Critic: Plan approved");
        } else {
            adapter.print_warning("⚠ Critic: Revisions needed");
        }
    }

    /// Display punch list with priority indicators
    fn display_punch_list(&self, adapter: &dyn InteractionAdapter, items: &[PunchListItem]) {
        // Count by priority
        let high_count = items
            .iter()
            .filter(|i| i.priority == Priority::High)
            .count();
        let medium_count = items
            .iter()
            .filter(|i| i.priority == Priority::Medium)
            .count();
        let low_count = items.iter().filter(|i| i.priority == Priority::Low).count();

        // Only show if there are items
        if items.is_empty() {
            return;
        }

        adapter.print_info("");

        // Show high priority items first (blocking issues)
        if high_count > 0 {
            adapter.print_info("Must address:");
            for item in items.iter().filter(|i| i.priority == Priority::High) {
                adapter.print_error(&format!("  ✗ {}", item.description));
            }
        }

        // Show medium priority items
        if medium_count > 0 {
            adapter.print_info("Suggestions:");
            for item in items.iter().filter(|i| i.priority == Priority::Medium) {
                adapter.print_info(&format!("  • {}", item.description));
            }
        }

        // Show low priority items
        if low_count > 0 {
            adapter.print_info("Optional:");
            for item in items.iter().filter(|i| i.priority == Priority::Low) {
                adapter.print_info(&format!("  ○ {}", item.description));
            }
        }
    }

    /// Ask user for their decision
    fn ask_decision(
        &self,
        adapter: &dyn InteractionAdapter,
        summary: &CriticSummary,
    ) -> Result<UserDecision, SpecksError> {
        // Build the prompt based on critic summary
        let prompt = if summary.approved {
            "The critic approved this plan. What would you like to do?"
        } else {
            "The critic has suggestions. What would you like to do?"
        };

        let options = DecisionOption::all_labels();
        let option_refs: Vec<&str> = options.to_vec();

        let selected = adapter
            .ask_select(prompt, &option_refs)
            .map_err(Self::convert_interaction_error)?;

        let decision = DecisionOption::from_index(selected);

        match decision {
            DecisionOption::Approve => Ok(UserDecision::Approve),
            DecisionOption::Abort => Ok(UserDecision::Abort),
            DecisionOption::Revise => {
                // Ask for feedback
                let feedback = adapter
                    .ask_text("What changes would you like to make?", None)
                    .map_err(Self::convert_interaction_error)?;
                Ok(UserDecision::Revise(feedback))
            }
        }
    }

    /// Read a brief summary from a speck file
    ///
    /// Extracts the purpose or title from the speck.
    fn read_speck_summary(&self, path: &Path) -> Option<String> {
        let content = std::fs::read_to_string(path).ok()?;

        // Look for Purpose line
        for line in content.lines() {
            if line.starts_with("**Purpose:**") {
                let purpose = line.strip_prefix("**Purpose:**")?.trim();
                if !purpose.is_empty() {
                    // Truncate if too long
                    if purpose.len() > 80 {
                        return Some(format!("{}...", &purpose[..77]));
                    }
                    return Some(purpose.to_string());
                }
            }
        }

        // Fall back to first H1 heading
        for line in content.lines() {
            if line.starts_with("# ") {
                let heading = line.strip_prefix("# ")?.trim();
                if !heading.is_empty() {
                    return Some(heading.to_string());
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

impl Default for CliPresenter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specks_core::interaction::{InteractionResult, ProgressHandle};
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

    /// A configurable mock adapter for testing (thread-safe)
    struct ConfigurableMockAdapter {
        progress_counter: AtomicU64,
        /// Responses for ask_select calls (in order)
        select_responses: Mutex<Vec<usize>>,
        /// Current index for select responses
        select_index: AtomicUsize,
        /// Responses for ask_text calls (in order)
        text_responses: Mutex<Vec<String>>,
        /// Current index for text responses
        text_index: AtomicUsize,
        /// Printed messages for verification
        printed_messages: Mutex<Vec<(String, String)>>, // (type, message)
    }

    impl ConfigurableMockAdapter {
        fn new() -> Self {
            Self {
                progress_counter: AtomicU64::new(0),
                select_responses: Mutex::new(vec![]),
                select_index: AtomicUsize::new(0),
                text_responses: Mutex::new(vec![]),
                text_index: AtomicUsize::new(0),
                printed_messages: Mutex::new(vec![]),
            }
        }

        fn with_select_responses(self, responses: Vec<usize>) -> Self {
            *self.select_responses.lock().unwrap() = responses;
            self
        }

        fn with_text_responses(self, responses: Vec<String>) -> Self {
            *self.text_responses.lock().unwrap() = responses;
            self
        }

        fn get_printed_messages(&self) -> Vec<(String, String)> {
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
            Ok(default)
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
            self.printed_messages
                .lock()
                .unwrap()
                .push(("info".to_string(), message.to_string()));
        }
        fn print_warning(&self, message: &str) {
            self.printed_messages
                .lock()
                .unwrap()
                .push(("warning".to_string(), message.to_string()));
        }
        fn print_error(&self, message: &str) {
            self.printed_messages
                .lock()
                .unwrap()
                .push(("error".to_string(), message.to_string()));
        }
        fn print_success(&self, message: &str) {
            self.printed_messages
                .lock()
                .unwrap()
                .push(("success".to_string(), message.to_string()));
        }
        fn print_header(&self, message: &str) {
            self.printed_messages
                .lock()
                .unwrap()
                .push(("header".to_string(), message.to_string()));
        }
    }

    #[test]
    fn test_cli_presenter_approve_decision() {
        let adapter = ConfigurableMockAdapter::new().with_select_responses(vec![0]); // Approve

        let presenter = CliPresenter::new();
        let mut context = LoopContext::new_idea("test idea".to_string(), vec![]);
        context.speck_path = Some(PathBuf::from("/project/.specks/specks-test.md"));
        context.critic_feedback = Some("Approved. The plan looks good.".to_string());

        let result = presenter.present(&adapter, &context).unwrap();
        assert!(matches!(result, UserDecision::Approve));
    }

    #[test]
    fn test_cli_presenter_abort_decision() {
        let adapter = ConfigurableMockAdapter::new().with_select_responses(vec![2]); // Abort

        let presenter = CliPresenter::new();
        let mut context = LoopContext::new_idea("test idea".to_string(), vec![]);
        context.speck_path = Some(PathBuf::from("/project/.specks/specks-test.md"));
        context.critic_feedback = Some("Some feedback".to_string());

        let result = presenter.present(&adapter, &context).unwrap();
        assert!(matches!(result, UserDecision::Abort));
    }

    #[test]
    fn test_cli_presenter_revise_decision() {
        let adapter = ConfigurableMockAdapter::new()
            .with_select_responses(vec![1]) // Revise
            .with_text_responses(vec!["Add more error handling".to_string()]);

        let presenter = CliPresenter::new();
        let mut context = LoopContext::new_idea("test idea".to_string(), vec![]);
        context.speck_path = Some(PathBuf::from("/project/.specks/specks-test.md"));
        context.critic_feedback = Some("Consider adding error handling.".to_string());

        let result = presenter.present(&adapter, &context).unwrap();
        match result {
            UserDecision::Revise(feedback) => {
                assert_eq!(feedback, "Add more error handling");
            }
            _ => panic!("Expected Revise decision"),
        }
    }

    #[test]
    fn test_parse_critic_feedback_approved() {
        let feedback = "The plan has been approved. Ready for implementation.\n\n- Minor suggestion: consider adding logging";
        let summary = CliPresenter::parse_critic_feedback(feedback);

        assert!(summary.approved);
        assert!(summary.assessment.contains("approved"));
    }

    #[test]
    fn test_parse_critic_feedback_not_approved() {
        let feedback =
            "The plan needs revision.\n\n- Critical: missing error handling\n- Should add tests";
        let summary = CliPresenter::parse_critic_feedback(feedback);

        assert!(!summary.approved);
        assert_eq!(summary.punch_list.len(), 2);
    }

    #[test]
    fn test_parse_critic_feedback_extracts_punch_list() {
        let feedback = r#"Overall assessment looks good but needs work.

- Critical issue: missing required field
- Should consider adding validation
- Minor: optional logging could be added
* Another bullet point suggestion
1. Numbered item recommendation
"#;

        let summary = CliPresenter::parse_critic_feedback(feedback);

        // Should extract bullet and numbered items
        assert!(summary.punch_list.len() >= 4);

        // Check priority assignment
        let high_priority = summary
            .punch_list
            .iter()
            .filter(|i| i.priority == Priority::High)
            .count();
        assert!(
            high_priority >= 1,
            "Should have at least one high priority item"
        );

        let medium_priority = summary
            .punch_list
            .iter()
            .filter(|i| i.priority == Priority::Medium)
            .count();
        assert!(
            medium_priority >= 1,
            "Should have at least one medium priority item"
        );

        let low_priority = summary
            .punch_list
            .iter()
            .filter(|i| i.priority == Priority::Low)
            .count();
        assert!(
            low_priority >= 1,
            "Should have at least one low priority item"
        );
    }

    #[test]
    fn test_priority_labels() {
        assert_eq!(Priority::High.label(), "HIGH");
        assert_eq!(Priority::Medium.label(), "MEDIUM");
        assert_eq!(Priority::Low.label(), "LOW");
    }

    #[test]
    fn test_decision_option_from_index() {
        assert_eq!(DecisionOption::from_index(0), DecisionOption::Approve);
        assert_eq!(DecisionOption::from_index(1), DecisionOption::Revise);
        assert_eq!(DecisionOption::from_index(2), DecisionOption::Abort);
        assert_eq!(DecisionOption::from_index(99), DecisionOption::Abort); // Out of bounds
    }

    #[test]
    fn test_decision_option_labels() {
        let labels = DecisionOption::all_labels();
        assert_eq!(labels.len(), 3);
        assert!(labels[0].contains("Approve"));
        assert!(labels[1].contains("Revise"));
        assert!(labels[2].contains("Abort"));
    }

    #[test]
    fn test_punch_list_item_creation() {
        let item = PunchListItem::new(Priority::High, "Critical bug");
        assert_eq!(item.priority, Priority::High);
        assert_eq!(item.description, "Critical bug");
    }

    #[test]
    fn test_cli_presenter_displays_punch_list() {
        let adapter = ConfigurableMockAdapter::new().with_select_responses(vec![0]); // Approve

        let presenter = CliPresenter::new();
        let mut context = LoopContext::new_idea("test idea".to_string(), vec![]);
        context.speck_path = Some(PathBuf::from("/project/.specks/specks-test.md"));
        context.critic_feedback = Some(
            "Needs work.\n- Critical: missing auth\n- Should add tests\n- Minor: typo in comment"
                .to_string(),
        );

        let _ = presenter.present(&adapter, &context).unwrap();

        let messages = adapter.get_printed_messages();

        // Check that punch list was displayed
        let punch_list_header = messages.iter().any(|(_, msg)| msg.contains("Must address"));
        assert!(punch_list_header, "Should display punch list items");

        // Check that high priority items are printed as errors
        let has_error_message = messages.iter().any(|(t, _)| t == "error");
        assert!(
            has_error_message,
            "Should have error-level messages for high priority"
        );
    }

    #[test]
    fn test_read_speck_summary_with_purpose() {
        // Create a temp file with Purpose line
        let temp_dir = std::env::temp_dir().join("specks-test-present-summary");
        let _ = std::fs::create_dir_all(&temp_dir);
        let speck_path = temp_dir.join("test-speck.md");

        std::fs::write(
            &speck_path,
            "## Phase 1\n\n**Purpose:** Add authentication to the API.\n\n### Details\n",
        )
        .unwrap();

        let presenter = CliPresenter::new();
        let summary = presenter.read_speck_summary(&speck_path);

        assert!(summary.is_some());
        assert_eq!(summary.unwrap(), "Add authentication to the API.");

        // Clean up
        let _ = std::fs::remove_file(&speck_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_read_speck_summary_fallback_to_h1() {
        // Create a temp file without Purpose line
        let temp_dir = std::env::temp_dir().join("specks-test-present-summary-h1");
        let _ = std::fs::create_dir_all(&temp_dir);
        let speck_path = temp_dir.join("test-speck-h1.md");

        std::fs::write(&speck_path, "# Authentication Feature\n\nSome content.\n").unwrap();

        let presenter = CliPresenter::new();
        let summary = presenter.read_speck_summary(&speck_path);

        assert!(summary.is_some());
        assert_eq!(summary.unwrap(), "Authentication Feature");

        // Clean up
        let _ = std::fs::remove_file(&speck_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cli_presenter_no_critic_feedback() {
        let adapter = ConfigurableMockAdapter::new().with_select_responses(vec![0]); // Approve

        let presenter = CliPresenter::new();
        let mut context = LoopContext::new_idea("test idea".to_string(), vec![]);
        context.speck_path = Some(PathBuf::from("/project/.specks/specks-test.md"));
        // No critic_feedback set

        let result = presenter.present(&adapter, &context).unwrap();
        assert!(matches!(result, UserDecision::Approve));

        let messages = adapter.get_printed_messages();
        let has_warning = messages
            .iter()
            .any(|(t, msg)| t == "warning" && msg.contains("No critic feedback"));
        assert!(has_warning, "Should warn about missing critic feedback");
    }

    #[test]
    fn test_cli_presenter_default() {
        let presenter = CliPresenter::default();
        // Just verify it creates successfully
        let _ = presenter;
    }

    #[test]
    fn test_critic_summary_default() {
        let summary = CriticSummary::default();
        assert!(!summary.approved);
        assert!(summary.assessment.is_empty());
        assert!(summary.punch_list.is_empty());
    }
}
