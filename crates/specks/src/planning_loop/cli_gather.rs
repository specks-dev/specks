//! CLI-mode requirement gathering for the planning loop
//!
//! Per [D18], in CLI mode the CLI code handles all user interaction directly
//! using inquire prompts instead of invoking the interviewer agent.
//!
//! Per [D23] and [D21], this module presents clarifier-generated questions
//! (context-aware and intelligent) instead of hard-coded generic prompts.

use std::collections::HashMap;
use std::path::Path;

use specks_core::interaction::{InteractionAdapter, InteractionError};
use specks_core::SpecksError;

use super::clarifier::{ClarifierOutput, EnrichedRequirements};
use super::types::{LoopContext, PlanMode};

/// Result of the CLI gather phase
#[derive(Debug, Clone)]
pub struct GatherResult {
    /// Formatted requirements string for the planner agent
    pub requirements: String,
    /// Whether the user confirmed they want to proceed
    pub user_confirmed: bool,
    /// Enriched requirements with clarifier analysis and user answers
    pub enriched: Option<EnrichedRequirements>,
}

/// CLI gatherer for requirement collection in CLI mode
///
/// Per [D23], the CLI presents clarifier-generated questions directly
/// instead of asking hard-coded generic questions.
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
    /// Per [D23], this presents clarifier-generated questions instead of hard-coded prompts.
    ///
    /// # Arguments
    /// * `adapter` - The interaction adapter for user prompts
    /// * `context` - The current loop context with input and mode
    /// * `clarifier_output` - Optional clarifier output with intelligent questions
    ///
    /// # Returns
    /// A `GatherResult` containing the formatted requirements and confirmation status
    pub fn gather(
        &self,
        adapter: &dyn InteractionAdapter,
        context: &LoopContext,
        clarifier_output: Option<&ClarifierOutput>,
    ) -> Result<GatherResult, SpecksError> {
        match context.mode {
            PlanMode::New => self.gather_new_idea(adapter, context, clarifier_output),
            PlanMode::Revision => self.gather_revision(adapter, context, clarifier_output),
        }
    }

    /// Gather requirements for a new idea
    ///
    /// Per [D23], presents clarifier-generated questions instead of hard-coded prompts.
    fn gather_new_idea(
        &self,
        adapter: &dyn InteractionAdapter,
        context: &LoopContext,
        clarifier_output: Option<&ClarifierOutput>,
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

        // Present clarifier analysis and questions (or handle empty case)
        let user_answers = if let Some(output) = clarifier_output {
            self.present_clarifier_output(adapter, output)?
        } else {
            // No clarifier output - proceed with minimal confirmation
            let confirmed = adapter
                .ask_confirm("No clarifier analysis available. Proceed anyway?", true)
                .map_err(Self::convert_interaction_error)?;
            if !confirmed {
                return Ok(GatherResult {
                    requirements: String::new(),
                    user_confirmed: false,
                    enriched: None,
                });
            }
            HashMap::new()
        };

        // Build enriched requirements
        let mut enriched = EnrichedRequirements::new(context.input.clone());
        if let Some(output) = clarifier_output {
            enriched = enriched.with_clarifier_output(output.clone());
        }
        for (question, answer) in &user_answers {
            enriched.add_answer(question, answer.clone());
        }

        // Ask for final confirmation
        let confirmed = adapter
            .ask_confirm("Create plan with these settings?", true)
            .map_err(Self::convert_interaction_error)?;

        if !confirmed {
            return Ok(GatherResult {
                requirements: String::new(),
                user_confirmed: false,
                enriched: None,
            });
        }

        // Use enriched requirements to generate planner prompt
        let requirements = enriched.to_planner_prompt();

        // Append context files if any
        let requirements = if !context.context_files.is_empty() {
            let mut req = requirements;
            req.push_str("## Additional Context\n\n");
            for ctx_file in &context.context_files {
                req.push_str(ctx_file);
                req.push_str("\n\n");
            }
            req
        } else {
            requirements
        };

        Ok(GatherResult {
            requirements,
            user_confirmed: true,
            enriched: Some(enriched),
        })
    }

    /// Gather requirements for revising an existing speck
    ///
    /// Per [D23], presents clarifier-generated questions about revision.
    fn gather_revision(
        &self,
        adapter: &dyn InteractionAdapter,
        context: &LoopContext,
        clarifier_output: Option<&ClarifierOutput>,
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

        // Present clarifier analysis and questions for revision
        let user_answers = if let Some(output) = clarifier_output {
            self.present_clarifier_output(adapter, output)?
        } else {
            // No clarifier output - ask what user wants to change
            let revision_description = adapter
                .ask_text("What would you like to change?", None)
                .map_err(Self::convert_interaction_error)?;

            adapter.print_info(&format!("\nRevision request: {}\n", revision_description));

            let mut answers = HashMap::new();
            answers.insert("revision_request".to_string(), revision_description);
            answers
        };

        // Confirm
        let confirmed = adapter
            .ask_confirm("Proceed with this revision?", true)
            .map_err(Self::convert_interaction_error)?;

        if !confirmed {
            return Ok(GatherResult {
                requirements: String::new(),
                user_confirmed: false,
                enriched: None,
            });
        }

        // Build enriched requirements for revision
        let critic_feedback = context.critic_feedback.clone().unwrap_or_default();
        let mut enriched = EnrichedRequirements::for_revision(context.input.clone(), critic_feedback);
        if let Some(output) = clarifier_output {
            enriched = enriched.with_clarifier_output(output.clone());
        }
        for (question, answer) in &user_answers {
            enriched.add_answer(question, answer.clone());
        }

        // Generate planner prompt from enriched requirements
        let requirements = enriched.to_planner_prompt();

        // Append context files if any
        let requirements = if !context.context_files.is_empty() {
            let mut req = requirements;
            req.push_str("## Additional Context\n\n");
            for ctx_file in &context.context_files {
                req.push_str(ctx_file);
                req.push_str("\n\n");
            }
            req
        } else {
            requirements
        };

        Ok(GatherResult {
            requirements,
            user_confirmed: true,
            enriched: Some(enriched),
        })
    }

    /// Present clarifier output and gather user answers
    ///
    /// Per [D23], displays analysis summary and asks clarifier-generated questions
    /// using inquire::Select for each question.
    fn present_clarifier_output(
        &self,
        adapter: &dyn InteractionAdapter,
        output: &ClarifierOutput,
    ) -> Result<HashMap<String, String>, SpecksError> {
        // Display analysis summary
        self.display_analysis_summary(adapter, output);

        // Handle empty questions case
        if output.has_no_questions() {
            adapter.print_info("✓ I understand what you want. Proceeding to create plan.\n");

            // Ask if user wants to add any additional context
            let add_context = adapter
                .ask_confirm("Would you like to add any additional context?", false)
                .map_err(Self::convert_interaction_error)?;

            if add_context {
                let extra_context = adapter
                    .ask_text("Enter additional context:", None)
                    .map_err(Self::convert_interaction_error)?;

                let mut answers = HashMap::new();
                if !extra_context.is_empty() {
                    answers.insert("additional_context".to_string(), extra_context);
                }
                return Ok(answers);
            }

            return Ok(HashMap::new());
        }

        // Present each question using inquire::Select with navigation
        let mut answers: Vec<Option<String>> = vec![None; output.questions.len()];
        let total_questions = output.questions.len();
        let mut current_idx = 0;

        // Question loop with back navigation
        while current_idx < total_questions {
            let question = &output.questions[current_idx];

            // Question with clean vertical spacing
            adapter.print_info("");
            adapter.print_info("");
            adapter.print_header(&format!("*** Question {} of {} ***", current_idx + 1, total_questions));
            adapter.print_info("");
            adapter.print_info(&question.why_asking);
            adapter.print_info("");

            // Build options list with default marked
            let mut options_with_default: Vec<String> = question
                .options
                .iter()
                .map(|opt| {
                    if opt == &question.default {
                        format!("{} (default)", opt)
                    } else {
                        opt.clone()
                    }
                })
                .collect();

            // Add "Go back" option if not on first question
            let go_back_idx = if current_idx > 0 {
                options_with_default.push("← Go back to previous question".to_string());
                Some(options_with_default.len() - 1)
            } else {
                None
            };

            let options_refs: Vec<&str> = options_with_default.iter().map(|s| s.as_str()).collect();

            let selected_index = adapter
                .ask_select(&question.question, &options_refs)
                .map_err(Self::convert_interaction_error)?;

            // Check if user selected "Go back"
            if Some(selected_index) == go_back_idx {
                current_idx -= 1;
                continue;
            }

            // Get the actual selected option (without "(default)" suffix)
            let selected_answer = question.options.get(selected_index)
                .cloned()
                .unwrap_or_else(|| question.default.clone());

            answers[current_idx] = Some(selected_answer);
            current_idx += 1;
        }

        // Show summary and get confirmation
        loop {
            adapter.print_info("");
            adapter.print_info("");
            adapter.print_header("*** Summary ***");
            adapter.print_info("");
            adapter.print_info("Your answers:");
            adapter.print_info("");

            for (idx, question) in output.questions.iter().enumerate() {
                let answer = answers[idx].as_deref().unwrap_or("(no answer)");
                // Truncate long questions for display
                let q_display = if question.question.len() > 50 {
                    format!("{}...", &question.question[..47])
                } else {
                    question.question.clone()
                };
                adapter.print_info(&format!("  Q{}. {}", idx + 1, q_display));
                adapter.print_info(&format!("      → {}", answer));
                adapter.print_info("");
            }

            // Show assumptions
            if !output.assumptions_if_no_answer.is_empty() {
                adapter.print_info("  Assumptions:");
                adapter.print_info("");
                for assumption in &output.assumptions_if_no_answer {
                    adapter.print_info(&format!("    • {}", assumption));
                }
                adapter.print_info("");
            }

            adapter.print_info("");

            // Build confirmation options
            let mut confirm_options = vec!["✓ Looks good, proceed".to_string()];
            for (idx, question) in output.questions.iter().enumerate() {
                let q_short = if question.question.len() > 40 {
                    format!("{}...", &question.question[..37])
                } else {
                    question.question.clone()
                };
                confirm_options.push(format!("← Revise Q{}: {}", idx + 1, q_short));
            }

            let confirm_refs: Vec<&str> = confirm_options.iter().map(|s| s.as_str()).collect();

            let selected = adapter
                .ask_select("Ready to proceed?", &confirm_refs)
                .map_err(Self::convert_interaction_error)?;

            if selected == 0 {
                // User confirmed
                break;
            } else {
                // User wants to revise a question
                let revise_idx = selected - 1;
                if revise_idx < output.questions.len() {
                    let question = &output.questions[revise_idx];

                    adapter.print_info("");
                    adapter.print_info("");
                    adapter.print_header(&format!("*** Revising Question {} ***", revise_idx + 1));
                    adapter.print_info("");
                    adapter.print_info(&question.why_asking);
                    adapter.print_info("");

                    let options_with_default: Vec<String> = question
                        .options
                        .iter()
                        .map(|opt| {
                            if opt == &question.default {
                                format!("{} (default)", opt)
                            } else {
                                opt.clone()
                            }
                        })
                        .collect();

                    let options_refs: Vec<&str> = options_with_default.iter().map(|s| s.as_str()).collect();

                    let selected_index = adapter
                        .ask_select(&question.question, &options_refs)
                        .map_err(Self::convert_interaction_error)?;

                    let selected_answer = question.options.get(selected_index)
                        .cloned()
                        .unwrap_or_else(|| question.default.clone());

                    answers[revise_idx] = Some(selected_answer);
                }
            }
        }

        // Convert to HashMap
        let mut result = HashMap::new();
        for (idx, question) in output.questions.iter().enumerate() {
            if let Some(answer) = &answers[idx] {
                result.insert(question.question.clone(), answer.clone());
            }
        }

        Ok(result)
    }

    /// Display the clarifier's analysis summary
    fn display_analysis_summary(&self, adapter: &dyn InteractionAdapter, output: &ClarifierOutput) {
        adapter.print_info("");
        adapter.print_header("*** Analysis ***");
        adapter.print_info("");

        // Show what clarifier understood
        if !output.analysis.understood_intent.is_empty() {
            adapter.print_info(&format!("I understand you want to: {}", output.analysis.understood_intent));
            adapter.print_info("");
        }

        // Show relevant context found
        if !output.analysis.relevant_context.is_empty() {
            adapter.print_info("Relevant context found:");
            adapter.print_info("");
            for ctx in &output.analysis.relevant_context {
                adapter.print_info(&format!("  • {}", ctx));
            }
            adapter.print_info("");
        }

        // Show identified ambiguities (if any)
        if !output.analysis.identified_ambiguities.is_empty() {
            adapter.print_info("Need to clarify:");
            adapter.print_info("");
            for ambiguity in &output.analysis.identified_ambiguities {
                adapter.print_info(&format!("  • {}", ambiguity));
            }
        }
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
    use super::super::clarifier::{ClarifierAnalysis, ClarifierQuestion};
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

    /// Create a sample clarifier output with questions
    fn sample_clarifier_output_with_questions() -> ClarifierOutput {
        ClarifierOutput {
            mode: "idea".to_string(),
            analysis: ClarifierAnalysis {
                understood_intent: "Create a caching layer for the application".to_string(),
                relevant_context: vec!["src/data.rs - existing data access".to_string()],
                identified_ambiguities: vec!["Cache storage type unclear".to_string()],
            },
            questions: vec![
                ClarifierQuestion {
                    question: "What type of cache should be used?".to_string(),
                    options: vec!["In-memory".to_string(), "Redis".to_string(), "File-based".to_string()],
                    why_asking: "Different cache types have different trade-offs".to_string(),
                    default: "In-memory".to_string(),
                },
                ClarifierQuestion {
                    question: "Should the cache have a TTL?".to_string(),
                    options: vec!["Yes, 5 minutes".to_string(), "Yes, 1 hour".to_string(), "No TTL".to_string()],
                    why_asking: "TTL affects data freshness vs performance".to_string(),
                    default: "Yes, 5 minutes".to_string(),
                },
            ],
            assumptions_if_no_answer: vec!["Will use in-memory cache with 5 minute TTL".to_string()],
        }
    }

    /// Create a sample clarifier output with no questions (detailed idea)
    fn sample_clarifier_output_empty() -> ClarifierOutput {
        ClarifierOutput {
            mode: "idea".to_string(),
            analysis: ClarifierAnalysis {
                understood_intent: "Add a CLI greeting command that prints Hello World".to_string(),
                relevant_context: vec!["src/cli.rs - existing CLI structure".to_string()],
                identified_ambiguities: vec![],
            },
            questions: vec![],
            assumptions_if_no_answer: vec![],
        }
    }

    #[test]
    fn test_cli_gatherer_with_clarifier_questions() {
        // Mock adapter: select first option for both questions, confirm final
        let adapter = ConfigurableMockAdapter::new()
            .with_select_responses(vec![0, 0]) // Select first option for each question
            .with_confirm_responses(vec![true]); // Confirm

        let gatherer = CliGatherer::new();
        let context = LoopContext::new_idea("add a caching layer".to_string(), vec![]);
        let clarifier_output = sample_clarifier_output_with_questions();

        let result = gatherer.gather(&adapter, &context, Some(&clarifier_output)).unwrap();

        assert!(result.user_confirmed);
        assert!(result.requirements.contains("add a caching layer"));
        // Should contain user answers from clarifier questions
        assert!(result.requirements.contains("In-memory"));
        assert!(result.enriched.is_some());
        let enriched = result.enriched.unwrap();
        assert_eq!(enriched.user_answers.len(), 2);
        assert_eq!(enriched.user_answers.get("What type of cache should be used?"), Some(&"In-memory".to_string()));
    }

    #[test]
    fn test_cli_gatherer_with_empty_clarifier_questions() {
        // Mock adapter: confirm "no additional context", then confirm final
        let adapter = ConfigurableMockAdapter::new()
            .with_confirm_responses(vec![false, true]); // No additional context, then confirm

        let gatherer = CliGatherer::new();
        let context = LoopContext::new_idea("add hello world command".to_string(), vec![]);
        let clarifier_output = sample_clarifier_output_empty();

        let result = gatherer.gather(&adapter, &context, Some(&clarifier_output)).unwrap();

        assert!(result.user_confirmed);
        assert!(result.requirements.contains("add hello world command"));
        // Should show clarifier understood intent
        assert!(result.enriched.is_some());
        let enriched = result.enriched.unwrap();
        assert!(enriched.user_answers.is_empty()); // No questions asked
    }

    #[test]
    fn test_cli_gatherer_empty_questions_with_additional_context() {
        // Mock adapter: say yes to additional context, provide it, then confirm
        let adapter = ConfigurableMockAdapter::new()
            .with_confirm_responses(vec![true, true]) // Yes to add context, then confirm
            .with_text_responses(vec!["Also support color output".to_string()]);

        let gatherer = CliGatherer::new();
        let context = LoopContext::new_idea("add hello command".to_string(), vec![]);
        let clarifier_output = sample_clarifier_output_empty();

        let result = gatherer.gather(&adapter, &context, Some(&clarifier_output)).unwrap();

        assert!(result.user_confirmed);
        assert!(result.enriched.is_some());
        let enriched = result.enriched.unwrap();
        assert_eq!(enriched.user_answers.get("additional_context"), Some(&"Also support color output".to_string()));
    }

    #[test]
    fn test_cli_gatherer_user_cancels_final_confirm() {
        // Mock adapter: answer questions but cancel final confirmation
        let adapter = ConfigurableMockAdapter::new()
            .with_select_responses(vec![0, 0])
            .with_confirm_responses(vec![false]); // Cancel final confirm

        let gatherer = CliGatherer::new();
        let context = LoopContext::new_idea("cancelled idea".to_string(), vec![]);
        let clarifier_output = sample_clarifier_output_with_questions();

        let result = gatherer.gather(&adapter, &context, Some(&clarifier_output)).unwrap();

        assert!(!result.user_confirmed);
        assert!(result.requirements.is_empty());
        assert!(result.enriched.is_none());
    }

    #[test]
    fn test_cli_gatherer_no_clarifier_output() {
        // Test behavior when no clarifier output is provided
        let adapter = ConfigurableMockAdapter::new()
            .with_confirm_responses(vec![true, true]); // Confirm proceed, confirm create

        let gatherer = CliGatherer::new();
        let context = LoopContext::new_idea("simple idea".to_string(), vec![]);

        let result = gatherer.gather(&adapter, &context, None).unwrap();

        assert!(result.user_confirmed);
        assert!(result.requirements.contains("simple idea"));
    }

    #[test]
    fn test_cli_gatherer_revision_mode_with_clarifier() {
        // Revision mode with clarifier questions about what to change
        let clarifier_output = ClarifierOutput {
            mode: "revision".to_string(),
            analysis: ClarifierAnalysis {
                understood_intent: "User wants to improve error handling".to_string(),
                relevant_context: vec![],
                identified_ambiguities: vec!["Which errors to handle?".to_string()],
            },
            questions: vec![ClarifierQuestion {
                question: "Which error types should be handled?".to_string(),
                options: vec!["All errors".to_string(), "Only network errors".to_string()],
                why_asking: "Determines scope of changes".to_string(),
                default: "All errors".to_string(),
            }],
            assumptions_if_no_answer: vec![],
        };

        let adapter = ConfigurableMockAdapter::new()
            .with_select_responses(vec![1]) // Select second option (network errors only)
            .with_confirm_responses(vec![true]); // Confirm

        let gatherer = CliGatherer::new();
        let context = LoopContext::revision(
            PathBuf::from("/project/.specks/specks-1.md"),
            vec![],
        );

        let result = gatherer.gather(&adapter, &context, Some(&clarifier_output)).unwrap();

        assert!(result.user_confirmed);
        assert!(result.enriched.is_some());
        let enriched = result.enriched.unwrap();
        assert_eq!(
            enriched.user_answers.get("Which error types should be handled?"),
            Some(&"Only network errors".to_string())
        );
    }

    #[test]
    fn test_cli_gatherer_revision_mode_no_clarifier() {
        // Revision mode without clarifier (fallback to asking what to change)
        let adapter = ConfigurableMockAdapter::new()
            .with_text_responses(vec!["fix the error handling".to_string()])
            .with_confirm_responses(vec![true]); // Confirm revision

        let gatherer = CliGatherer::new();
        let context = LoopContext::revision(
            PathBuf::from("/project/.specks/specks-1.md"),
            vec![],
        );

        let result = gatherer.gather(&adapter, &context, None).unwrap();

        assert!(result.user_confirmed);
        // The enriched requirements should have the revision request
        assert!(result.enriched.is_some());
        let enriched = result.enriched.unwrap();
        assert_eq!(
            enriched.user_answers.get("revision_request"),
            Some(&"fix the error handling".to_string())
        );
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

        let result = gatherer.gather(&adapter, &context, None).unwrap();

        assert!(!result.user_confirmed);
        assert!(result.requirements.is_empty());
    }

    #[test]
    fn test_cli_gatherer_with_context_files() {
        let adapter = ConfigurableMockAdapter::new()
            .with_confirm_responses(vec![false, true]); // No additional context, confirm

        let gatherer = CliGatherer::new();
        let context = LoopContext::new_idea(
            "feature with context".to_string(),
            vec!["--- file1.md ---\nsome context".to_string()],
        );
        let clarifier_output = sample_clarifier_output_empty();

        let result = gatherer.gather(&adapter, &context, Some(&clarifier_output)).unwrap();

        assert!(result.user_confirmed);
        assert!(result.requirements.contains("Additional Context"));
        assert!(result.requirements.contains("file1.md"));
    }

    #[test]
    fn test_display_analysis_summary_is_called() {
        let adapter = ConfigurableMockAdapter::new()
            .with_select_responses(vec![0])
            .with_confirm_responses(vec![true]);

        let gatherer = CliGatherer::new();
        let context = LoopContext::new_idea("test".to_string(), vec![]);
        let clarifier_output = sample_clarifier_output_with_questions();

        let _ = gatherer.gather(&adapter, &context, Some(&clarifier_output));

        // Verify analysis summary was displayed
        let messages = adapter.get_printed_messages();
        assert!(messages.iter().any(|m| m.contains("Clarifier Analysis")));
        assert!(messages.iter().any(|m| m.contains("Create a caching layer")));
    }

    #[test]
    fn test_answers_correctly_mapped_to_questions() {
        // Select second option for first question, first for second
        let adapter = ConfigurableMockAdapter::new()
            .with_select_responses(vec![1, 0]) // Redis, Yes 5 minutes
            .with_confirm_responses(vec![true]);

        let gatherer = CliGatherer::new();
        let context = LoopContext::new_idea("cache".to_string(), vec![]);
        let clarifier_output = sample_clarifier_output_with_questions();

        let result = gatherer.gather(&adapter, &context, Some(&clarifier_output)).unwrap();

        let enriched = result.enriched.unwrap();
        assert_eq!(
            enriched.user_answers.get("What type of cache should be used?"),
            Some(&"Redis".to_string())
        );
        assert_eq!(
            enriched.user_answers.get("Should the cache have a TTL?"),
            Some(&"Yes, 5 minutes".to_string())
        );
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
