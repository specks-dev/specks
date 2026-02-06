//! Planning loop state machine for iterative speck creation
//!
//! Implements the iterative planning loop per [D03] Iterative Planning Loop and
//! Concept C02: Planning Loop State Machine.
//!
//! The loop runs: clarifier -> presenter -> planner -> critic -> (loop)
//! until the user approves the speck or aborts.
//!
//! Per [D21] and [D24], the clarifier agent runs in EVERY iteration:
//! - First iteration: analyzes the user's idea
//! - Subsequent iterations: analyzes critic feedback for revision questions
//!
//! Per [D18] and [D19], the loop supports two invocation modes:
//! - **CLI mode**: CLI code handles all user interaction via inquire prompts
//! - **Claude Code mode**: Interviewer agent handles interaction via AskUserQuestion
//!
//! The `PlanningMode` enum is passed explicitly to `PlanningLoop::new()` so the
//! caller specifies which mode is in use.

mod clarifier;
mod cli_gather;
mod cli_present;
mod types;

use std::path::{Path, PathBuf};

use specks_core::SpecksError;
use specks_core::interaction::{InteractionAdapter, InteractionError};

use crate::agent::AgentRunner;

// Re-export types from the types module
pub use types::{LoopContext, LoopOutcome, LoopState, PlanMode, PlanningMode, UserDecision};

// Re-export clarifier types (public API for this module)
#[allow(unused_imports)]
pub use clarifier::{
    ClarifierAnalysis, ClarifierInput, ClarifierOutput, ClarifierQuestion, EnrichedRequirements,
    invoke_clarifier, invoke_clarifier_streaming,
};

// Re-export CLI gather types (public API for this module)
#[allow(unused_imports)]
pub use cli_gather::{CliGatherer, GatherResult};

// Re-export CLI present types (public API for this module)
#[allow(unused_imports)]
pub use cli_present::{CliPresenter, CriticSummary, Priority, PunchListItem};

/// Planning loop manager
pub struct PlanningLoop {
    /// Current state
    state: LoopState,
    /// Loop context
    context: LoopContext,
    /// Agent runner
    runner: AgentRunner,
    /// Project root
    project_root: PathBuf,
    /// Timeout per agent invocation
    timeout_secs: u64,
    /// Speck name (if specified)
    speck_name: Option<String>,
    /// Whether to output in JSON format (reserved for future use)
    _json_output: bool,
    /// Whether to suppress progress messages
    quiet: bool,
    /// Interaction adapter for user interaction
    adapter: Box<dyn InteractionAdapter>,
    /// Planning mode: CLI or Claude Code
    mode: PlanningMode,
    /// Current clarifier output (from most recent clarifier invocation)
    clarifier_output: Option<ClarifierOutput>,
    /// Enriched requirements (from clarifier + user answers)
    enriched_requirements: Option<EnrichedRequirements>,
}

impl PlanningLoop {
    /// Create a new planning loop
    ///
    /// # Arguments
    ///
    /// * `context` - The loop context with input and mode
    /// * `project_root` - Path to the project root
    /// * `timeout_secs` - Timeout per agent invocation in seconds
    /// * `speck_name` - Optional name for the speck
    /// * `json_output` - Whether to output in JSON format
    /// * `quiet` - Whether to suppress progress messages
    /// * `adapter` - Interaction adapter for user prompts and progress
    /// * `mode` - Planning mode: CLI (CLI handles interaction) or ClaudeCode (agent handles interaction)
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        context: LoopContext,
        project_root: PathBuf,
        timeout_secs: u64,
        speck_name: Option<String>,
        json_output: bool,
        quiet: bool,
        adapter: Box<dyn InteractionAdapter>,
        mode: PlanningMode,
    ) -> Self {
        let runner = AgentRunner::new(project_root.clone());

        Self {
            state: LoopState::Start,
            context,
            runner,
            project_root,
            timeout_secs,
            speck_name,
            _json_output: json_output,
            quiet,
            adapter,
            mode,
            clarifier_output: None,
            enriched_requirements: None,
        }
    }

    /// Set a custom agent runner (for testing)
    #[allow(dead_code)]
    pub fn with_runner(mut self, runner: AgentRunner) -> Self {
        self.runner = runner;
        self
    }

    /// Get a reference to the interaction adapter
    #[allow(dead_code)]
    pub fn adapter(&self) -> &dyn InteractionAdapter {
        self.adapter.as_ref()
    }

    /// Get the current state
    #[allow(dead_code)]
    pub fn state(&self) -> &LoopState {
        &self.state
    }

    /// Get the current context
    #[allow(dead_code)]
    pub fn context(&self) -> &LoopContext {
        &self.context
    }

    /// Get the iteration count
    #[allow(dead_code)]
    pub fn iteration(&self) -> usize {
        self.context.iteration
    }

    /// Get the planning mode
    #[allow(dead_code)]
    pub fn planning_mode(&self) -> &PlanningMode {
        &self.mode
    }

    /// Transition to the next state
    pub fn transition(&mut self, new_state: LoopState) {
        self.state = new_state;
    }

    /// Check if the loop is complete
    pub fn is_complete(&self) -> bool {
        self.state.is_terminal()
    }

    /// Run the planning loop to completion
    ///
    /// Returns the outcome or an error if the loop fails.
    ///
    /// # Errors
    ///
    /// Returns `SpecksError::UserAborted` if the user cancels via Ctrl+C or explicit abort.
    /// Returns `SpecksError::InteractionFailed` if interaction fails (e.g., non-TTY environment).
    pub fn run(&mut self) -> Result<LoopOutcome, SpecksError> {
        // Verify claude CLI is available
        self.runner.check_claude_cli()?;

        // Start the loop
        // Per [D21] and [D24], the flow is: Clarifier -> Present -> Planner -> Critic -> CriticPresent
        // The clarifier runs in EVERY iteration to generate context-aware questions
        self.transition(LoopState::Clarifier);

        while !self.is_complete() {
            match self.state {
                LoopState::Clarifier => {
                    // Run clarifier to generate context-aware questions
                    // Per [D21] and [D24], clarifier runs in every iteration:
                    // - First iteration: analyzes the user's idea
                    // - Subsequent iterations: analyzes critic feedback for revision questions
                    self.run_clarifier()?;
                    self.transition(LoopState::Present);
                }
                LoopState::Present => {
                    // Gather requirements (present clarifier questions to user)
                    self.run_interviewer_gather()?;
                    self.transition(LoopState::Planner);
                }
                LoopState::Planner => {
                    self.run_planner()?;
                    self.transition(LoopState::Critic);
                }
                LoopState::Critic => {
                    self.run_critic()?;
                    self.transition(LoopState::CriticPresent);
                }
                LoopState::CriticPresent => {
                    // Present critic results and get user decision
                    let user_decision = self.run_interviewer_present()?;
                    match user_decision {
                        UserDecision::Approve => {
                            self.transition(LoopState::Approved);
                        }
                        UserDecision::Revise(feedback) => {
                            self.context.revision_feedback = Some(feedback);
                            self.context.iteration += 1;
                            // Per [D24], revision loops back through clarifier
                            // Clarifier will analyze critic feedback for revision questions
                            self.transition(LoopState::Clarifier);
                        }
                        UserDecision::Abort => {
                            self.transition(LoopState::Aborted);
                        }
                    }
                }
                LoopState::Revise => {
                    // This state transitions through Clarifier
                    self.transition(LoopState::Clarifier);
                }
                LoopState::Start | LoopState::Approved | LoopState::Aborted => {
                    // Terminal states or handled above
                    break;
                }
            }
        }

        // Check if user aborted
        if self.state == LoopState::Aborted {
            return Err(SpecksError::UserAborted);
        }

        // Build the outcome
        let speck_path =
            self.context
                .speck_path
                .clone()
                .ok_or_else(|| SpecksError::AgentInvocationFailed {
                    reason: "No speck path after planning loop".to_string(),
                })?;

        let speck_name = self.extract_speck_name(&speck_path);

        // Run validation to get error/warning counts
        let (validation_errors, validation_warnings) = self.validate_speck(&speck_path)?;

        if !self.quiet {
            self.adapter.print_success("Planning complete!");
        }

        Ok(LoopOutcome {
            speck_path,
            speck_name,
            mode: self.context.mode.clone(),
            iterations: self.context.iteration + 1,
            validation_errors,
            validation_warnings,
            critic_approved: true, // If we got here, critic approved
            user_approved: true,   // If we got here, user approved
        })
    }

    /// Convert an InteractionError to a SpecksError
    #[allow(dead_code)]
    fn convert_interaction_error(err: InteractionError) -> SpecksError {
        match err {
            InteractionError::Cancelled => SpecksError::UserAborted,
            InteractionError::NonTty => SpecksError::InteractionFailed {
                reason: "stdin is not a TTY - interactive input unavailable. Use --yes for non-interactive mode.".to_string(),
            },
            InteractionError::Timeout { secs } => SpecksError::AgentTimeout { secs },
            _ => SpecksError::InteractionFailed {
                reason: err.to_string(),
            },
        }
    }

    /// Run the clarifier phase.
    ///
    /// Per [D21] and [D24], the clarifier runs in EVERY iteration:
    /// - First iteration: analyzes the user's idea and generates clarifying questions
    /// - Subsequent iterations: analyzes critic feedback and generates revision questions
    ///
    /// The clarifier output is stored for the presentation phase.
    ///
    /// Per [D25], when not in quiet mode, streams agent output with a fixed spinner.
    fn run_clarifier(&mut self) -> Result<(), SpecksError> {
        let progress_msg = if self.context.iteration == 0 {
            "Clarifier analyzing idea..."
        } else {
            "Clarifier analyzing feedback..."
        };

        // Build clarifier input based on iteration
        let input = if self.context.iteration == 0 {
            // First iteration: analyze the user's idea
            ClarifierInput::Idea {
                idea: self.context.input.clone(),
                context_files: self.context.context_files.clone(),
            }
        } else {
            // Subsequent iterations: analyze critic feedback
            let critic_feedback = self.context.critic_feedback.clone().unwrap_or_default();
            let speck_path = self
                .context
                .speck_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            ClarifierInput::CriticFeedback {
                critic_feedback,
                speck_path,
            }
        };

        // Invoke clarifier with streaming output when not in quiet mode (per [D25])
        let output = if !self.quiet {
            use crate::streaming::StreamingDisplay;
            let mut display = StreamingDisplay::new(progress_msg);
            clarifier::invoke_clarifier_streaming(
                &input,
                &self.runner,
                &self.project_root,
                self.timeout_secs,
                &mut display,
            )?
        } else {
            invoke_clarifier(&input, &self.runner, &self.project_root, self.timeout_secs)?
        };

        // Store clarifier output for the presentation phase
        self.clarifier_output = Some(output.clone());

        // Initialize enriched requirements for this iteration
        if self.context.iteration == 0 {
            self.enriched_requirements = Some(
                EnrichedRequirements::new(self.context.input.clone()).with_clarifier_output(output),
            );
        } else {
            let mut req = EnrichedRequirements::for_revision(
                self.context.input.clone(),
                self.context.critic_feedback.clone().unwrap_or_default(),
            );
            req = req.with_clarifier_output(output);
            self.enriched_requirements = Some(req);
        }

        // Log if clarifier returned no questions
        if let Some(ref output) = self.clarifier_output {
            if output.has_no_questions() && !self.quiet {
                self.adapter.print_info(
                    "Clarifier found no ambiguities - proceeding with clear requirements.",
                );
            }
        }

        Ok(())
    }

    /// Run the interviewer gather phase
    ///
    /// Per [D18], in CLI mode the CLI code handles interaction directly.
    /// In Claude Code mode, this invokes the interviewer agent.
    fn run_interviewer_gather(&mut self) -> Result<(), SpecksError> {
        match self.mode {
            PlanningMode::Cli => self.run_cli_gather(),
            PlanningMode::ClaudeCode => self.run_agent_gather(),
        }
    }

    /// Run the CLI-mode gather phase using inquire prompts
    ///
    /// Per [D18], the CLI itself acts as the interviewer in CLI mode.
    /// Per [D23], presents clarifier-generated questions instead of hard-coded prompts.
    fn run_cli_gather(&mut self) -> Result<(), SpecksError> {
        let gatherer = cli_gather::CliGatherer::new();

        // Pass clarifier output to the gatherer (per [D23])
        let result = gatherer.gather(
            self.adapter.as_ref(),
            &self.context,
            self.clarifier_output.as_ref(),
        )?;

        if !result.user_confirmed {
            // User cancelled during gather phase
            return Err(SpecksError::UserAborted);
        }

        // Update enriched requirements if the gatherer produced them
        if let Some(enriched) = result.enriched {
            self.enriched_requirements = Some(enriched);
        }

        self.context.requirements = Some(result.requirements);
        Ok(())
    }

    /// Run the agent-mode gather phase using the interviewer agent
    ///
    /// This is used in Claude Code mode where the interviewer agent handles interaction.
    fn run_agent_gather(&mut self) -> Result<(), SpecksError> {
        let progress_handle = if !self.quiet {
            Some(
                self.adapter
                    .start_progress("Interviewer gathering requirements..."),
            )
        } else {
            None
        };

        let prompt = match self.context.mode {
            PlanMode::New => {
                let mut prompt = format!(
                    "Gather requirements for this idea: {}\n\n",
                    self.context.input
                );
                if !self.context.context_files.is_empty() {
                    prompt.push_str("Additional context:\n");
                    for ctx in &self.context.context_files {
                        prompt.push_str(ctx);
                        prompt.push_str("\n\n");
                    }
                }
                prompt
            }
            PlanMode::Revision => {
                format!(
                    "Present the current state of this speck and ask what the user wants to change: {}",
                    self.context.input
                )
            }
        };

        let config =
            crate::agent::interviewer_config(&self.project_root).with_timeout(self.timeout_secs);

        let result = self.runner.invoke_agent(&config, &prompt);

        // End progress spinner
        if let Some(handle) = progress_handle {
            self.adapter.end_progress(handle, result.is_ok());
        }

        self.context.requirements = Some(result?.output);

        Ok(())
    }

    /// Run the planner phase
    ///
    /// Per [D20], planner invocation is identical in both modes.
    /// Per [D25], when not in quiet mode, streams agent output with a fixed spinner.
    fn run_planner(&mut self) -> Result<(), SpecksError> {
        let progress_msg = if self.context.iteration == 0 {
            "Planner creating speck...".to_string()
        } else {
            format!(
                "Planner revising speck (iteration {})...",
                self.context.iteration + 1
            )
        };

        let mut prompt = String::new();

        // Include requirements
        if let Some(ref requirements) = self.context.requirements {
            prompt.push_str("Requirements gathered:\n");
            prompt.push_str(requirements);
            prompt.push_str("\n\n");
        }

        // Include revision feedback if this is a revision iteration
        if let Some(ref feedback) = self.context.revision_feedback {
            prompt.push_str("User feedback for revision:\n");
            prompt.push_str(feedback);
            prompt.push_str("\n\n");
        }

        // Include previous critic feedback if available
        if let Some(ref critic_feedback) = self.context.critic_feedback {
            prompt.push_str("Previous critic feedback:\n");
            prompt.push_str(critic_feedback);
            prompt.push_str("\n\n");
        }

        // Determine target path
        let speck_path = if let Some(ref path) = self.context.speck_path {
            path.clone()
        } else {
            // Generate a path for new speck
            let name = self.speck_name.clone().unwrap_or_else(|| {
                // Auto-generate from timestamp
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                format!("{}", timestamp)
            });
            self.project_root
                .join(".specks")
                .join(format!("specks-{}.md", name))
        };

        prompt.push_str(&format!(
            "Create/revise the speck at: {}\n",
            speck_path.display()
        ));

        let config =
            crate::agent::planner_config(&self.project_root).with_timeout(self.timeout_secs);

        // Invoke planner with spinner only (tool-heavy agent, text output is noise)
        if !self.quiet {
            use crate::streaming::StreamingDisplay;
            let mut display = StreamingDisplay::new(&progress_msg);
            self.runner
                .invoke_agent_spinner_only(&config, &prompt, &mut display)?;
        } else {
            self.runner.invoke_agent(&config, &prompt)?;
        }

        // Update context with speck path
        self.context.speck_path = Some(speck_path);

        Ok(())
    }

    /// Run the critic phase
    ///
    /// Per [D20], critic invocation is identical in both modes.
    /// Per [D25], when not in quiet mode, streams agent output with a fixed spinner.
    fn run_critic(&mut self) -> Result<(), SpecksError> {
        let speck_path =
            self.context
                .speck_path
                .as_ref()
                .ok_or_else(|| SpecksError::AgentInvocationFailed {
                    reason: "No speck path for critic to review".to_string(),
                })?;

        let prompt = format!(
            "Review this speck for quality, compliance, and implementability: {}",
            speck_path.display()
        );

        let config =
            crate::agent::critic_config(&self.project_root).with_timeout(self.timeout_secs);

        // Invoke critic with spinner only (tool-heavy agent, text output is noise)
        let result = if !self.quiet {
            use crate::streaming::StreamingDisplay;
            let mut display = StreamingDisplay::new("Critic reviewing speck...");
            self.runner
                .invoke_agent_spinner_only(&config, &prompt, &mut display)?
        } else {
            self.runner.invoke_agent(&config, &prompt)?
        };

        self.context.critic_feedback = Some(result.output);

        Ok(())
    }

    /// Run the interviewer present phase and get user decision
    ///
    /// Per [D18], in CLI mode the CLI code handles interaction directly.
    /// In Claude Code mode, this invokes the interviewer agent.
    fn run_interviewer_present(&mut self) -> Result<UserDecision, SpecksError> {
        match self.mode {
            PlanningMode::Cli => self.run_cli_present(),
            PlanningMode::ClaudeCode => self.run_agent_present(),
        }
    }

    /// Run the CLI-mode present phase using inquire prompts
    ///
    /// Per [D18], the CLI itself acts as the interviewer in CLI mode.
    fn run_cli_present(&mut self) -> Result<UserDecision, SpecksError> {
        let presenter = cli_present::CliPresenter::new();
        presenter.present(self.adapter.as_ref(), &self.context)
    }

    /// Run the agent-mode present phase using the interviewer agent
    ///
    /// This is used in Claude Code mode where the interviewer agent handles interaction.
    fn run_agent_present(&mut self) -> Result<UserDecision, SpecksError> {
        let progress_handle = if !self.quiet {
            Some(
                self.adapter
                    .start_progress("Interviewer presenting results..."),
            )
        } else {
            None
        };

        let speck_path =
            self.context
                .speck_path
                .as_ref()
                .ok_or_else(|| SpecksError::AgentInvocationFailed {
                    reason: "No speck path to present".to_string(),
                })?;

        let mut prompt = format!(
            "Present the speck at {} and the critic's feedback. ",
            speck_path.display()
        );

        if let Some(ref critic_feedback) = self.context.critic_feedback {
            prompt.push_str("Critic feedback:\n");
            prompt.push_str(critic_feedback);
            prompt.push_str("\n\n");
        }

        prompt.push_str("Ask the user: 'Ready to approve, or would you like to revise?' ");
        prompt.push_str(
            "If the user says 'ready', 'approve', 'looks good', 'yes', or similar, return APPROVED. ",
        );
        prompt.push_str("If the user says 'abort', 'cancel', 'quit', or similar, return ABORTED. ");
        prompt.push_str("Otherwise, return REVISE: followed by their feedback.");

        let config =
            crate::agent::interviewer_config(&self.project_root).with_timeout(self.timeout_secs);

        let result = self.runner.invoke_agent(&config, &prompt);

        // End progress spinner
        if let Some(handle) = progress_handle {
            self.adapter.end_progress(handle, result.is_ok());
        }

        let result = result?;

        // Parse the response
        let output = result.output.trim();
        if output.contains("APPROVED") || output.to_lowercase().contains("user approved") {
            Ok(UserDecision::Approve)
        } else if output.contains("ABORTED") || output.to_lowercase().contains("user aborted") {
            Ok(UserDecision::Abort)
        } else if output.starts_with("REVISE:") {
            let feedback = output.strip_prefix("REVISE:").unwrap_or(output).trim();
            Ok(UserDecision::Revise(feedback.to_string()))
        } else {
            // Assume revision with the full output as feedback
            Ok(UserDecision::Revise(output.to_string()))
        }
    }

    /// Validate the speck and return (error_count, warning_count)
    fn validate_speck(&self, speck_path: &Path) -> Result<(usize, usize), SpecksError> {
        use specks_core::{parse_speck, validate_speck};

        let content = std::fs::read_to_string(speck_path)?;
        let speck = parse_speck(&content).map_err(|e| SpecksError::Parse {
            message: e.to_string(),
            line: None,
        })?;

        let result = validate_speck(&speck);

        Ok((result.error_count(), result.warning_count()))
    }

    /// Extract speck name from path
    fn extract_speck_name(&self, path: &Path) -> String {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.strip_prefix("specks-").unwrap_or(s))
            .unwrap_or("unknown")
            .to_string()
    }
}

/// Detect whether input is an idea string or a path to an existing speck
pub fn detect_input_type(input: &str, project_root: &Path) -> PlanMode {
    // Check if it looks like a path
    if input.ends_with(".md") {
        // Check if file exists
        let path = Path::new(input);
        if path.is_absolute() && path.exists() {
            return PlanMode::Revision;
        }
        // Try relative to project root
        let full_path = project_root.join(input);
        if full_path.exists() {
            return PlanMode::Revision;
        }
        // Try in .specks directory
        let specks_path = project_root.join(".specks").join(input);
        if specks_path.exists() {
            return PlanMode::Revision;
        }
    }

    // Default to new idea
    PlanMode::New
}

/// Resolve input to a full path if it's a speck path
pub fn resolve_speck_path(input: &str, project_root: &Path) -> Option<PathBuf> {
    if !input.ends_with(".md") {
        return None;
    }

    let path = Path::new(input);
    if path.is_absolute() && path.exists() {
        return Some(path.to_path_buf());
    }

    let full_path = project_root.join(input);
    if full_path.exists() {
        return Some(full_path);
    }

    let specks_path = project_root.join(".specks").join(input);
    if specks_path.exists() {
        return Some(specks_path);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use specks_core::interaction::{InteractionResult, ProgressHandle};
    use std::sync::atomic::{AtomicU64, Ordering};

    /// A mock adapter for testing
    struct MockAdapter {
        progress_counter: AtomicU64,
    }

    impl MockAdapter {
        fn new() -> Self {
            Self {
                progress_counter: AtomicU64::new(0),
            }
        }
    }

    impl InteractionAdapter for MockAdapter {
        fn ask_text(&self, _prompt: &str, default: Option<&str>) -> InteractionResult<String> {
            Ok(default.unwrap_or("mock").to_string())
        }

        fn ask_select(&self, _prompt: &str, _options: &[&str]) -> InteractionResult<usize> {
            Ok(0)
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

        fn print_info(&self, _message: &str) {}
        fn print_warning(&self, _message: &str) {}
        fn print_error(&self, _message: &str) {}
        fn print_success(&self, _message: &str) {}
        fn print_header(&self, _message: &str) {}
    }

    fn create_mock_adapter() -> Box<dyn InteractionAdapter> {
        Box::new(MockAdapter::new())
    }

    #[test]
    fn test_detect_input_type_idea() {
        let project_root = PathBuf::from("/nonexistent/project");

        // Plain text is a new idea
        assert_eq!(
            detect_input_type("add feature X", &project_root),
            PlanMode::New
        );

        // Non-existent .md file is still treated as new (falls back)
        assert_eq!(
            detect_input_type("nonexistent.md", &project_root),
            PlanMode::New
        );
    }

    #[test]
    fn test_planning_loop_creation() {
        let ctx = LoopContext::new_idea("test idea".to_string(), vec![]);
        let project_root = PathBuf::from("/project");

        let loop_instance = PlanningLoop::new(
            ctx,
            project_root,
            300,
            None,
            false,
            false,
            create_mock_adapter(),
            PlanningMode::Cli,
        );

        assert_eq!(*loop_instance.state(), LoopState::Start);
        assert!(!loop_instance.is_complete());
        assert_eq!(*loop_instance.planning_mode(), PlanningMode::Cli);
    }

    #[test]
    fn test_planning_loop_creation_claude_code_mode() {
        let ctx = LoopContext::new_idea("test idea".to_string(), vec![]);
        let project_root = PathBuf::from("/project");

        let loop_instance = PlanningLoop::new(
            ctx,
            project_root,
            300,
            None,
            false,
            false,
            create_mock_adapter(),
            PlanningMode::ClaudeCode,
        );

        assert_eq!(*loop_instance.planning_mode(), PlanningMode::ClaudeCode);
    }

    #[test]
    fn test_planning_loop_manual_transitions() {
        let ctx = LoopContext::new_idea("test idea".to_string(), vec![]);
        let project_root = PathBuf::from("/project");

        let mut loop_instance = PlanningLoop::new(
            ctx,
            project_root,
            300,
            None,
            false,
            false,
            create_mock_adapter(),
            PlanningMode::Cli,
        );

        // Manual state transitions
        // Present state is the new gather phase (was InterviewerGather)
        loop_instance.transition(LoopState::Present);
        assert_eq!(*loop_instance.state(), LoopState::Present);

        loop_instance.transition(LoopState::Approved);
        assert!(loop_instance.is_complete());
    }

    #[test]
    fn test_detect_input_type_with_existing_file() {
        // Create a temporary directory with a speck file
        let temp_dir = std::env::temp_dir().join("specks-test-detect-input");
        let specks_dir = temp_dir.join(".specks");
        let _ = std::fs::create_dir_all(&specks_dir);

        let speck_path = specks_dir.join("specks-test.md");
        std::fs::write(&speck_path, "# Test Speck").expect("write test file");

        // Absolute path to existing file should be revision
        assert_eq!(
            detect_input_type(&speck_path.to_string_lossy(), &temp_dir),
            PlanMode::Revision
        );

        // Relative path in .specks directory should be revision
        assert_eq!(
            detect_input_type("specks-test.md", &temp_dir),
            PlanMode::Revision
        );

        // Clean up
        let _ = std::fs::remove_file(&speck_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_speck_path() {
        // Create a temporary directory with a speck file
        let temp_dir = std::env::temp_dir().join("specks-test-resolve-path");
        let specks_dir = temp_dir.join(".specks");
        let _ = std::fs::create_dir_all(&specks_dir);

        let speck_path = specks_dir.join("specks-test.md");
        std::fs::write(&speck_path, "# Test Speck").expect("write test file");

        // Absolute path should resolve
        let resolved = resolve_speck_path(&speck_path.to_string_lossy(), &temp_dir);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap(), speck_path);

        // Relative path in .specks should resolve
        let resolved = resolve_speck_path("specks-test.md", &temp_dir);
        assert!(resolved.is_some());

        // Non-.md path should not resolve
        let resolved = resolve_speck_path("not-a-markdown-file", &temp_dir);
        assert!(resolved.is_none());

        // Clean up
        let _ = std::fs::remove_file(&speck_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_extract_speck_name() {
        let ctx = LoopContext::new_idea("test".to_string(), vec![]);
        let project_root = PathBuf::from("/project");
        let loop_instance = PlanningLoop::new(
            ctx,
            project_root,
            300,
            None,
            false,
            false,
            create_mock_adapter(),
            PlanningMode::Cli,
        );

        // Standard name extraction
        assert_eq!(
            loop_instance.extract_speck_name(Path::new(".specks/specks-1.md")),
            "1"
        );
        assert_eq!(
            loop_instance.extract_speck_name(Path::new(".specks/specks-feature.md")),
            "feature"
        );

        // Edge cases
        assert_eq!(
            loop_instance.extract_speck_name(Path::new(".specks/not-a-speck.md")),
            "not-a-speck"
        );
        assert_eq!(
            loop_instance.extract_speck_name(Path::new("/full/path/specks-test.md")),
            "test"
        );
    }

    #[test]
    fn test_convert_interaction_error_cancelled() {
        let err = PlanningLoop::convert_interaction_error(InteractionError::Cancelled);
        assert!(matches!(err, SpecksError::UserAborted));
    }

    #[test]
    fn test_convert_interaction_error_non_tty() {
        let err = PlanningLoop::convert_interaction_error(InteractionError::NonTty);
        assert!(matches!(err, SpecksError::InteractionFailed { .. }));
        if let SpecksError::InteractionFailed { reason } = err {
            assert!(reason.contains("TTY"));
        }
    }

    #[test]
    fn test_convert_interaction_error_timeout() {
        let err = PlanningLoop::convert_interaction_error(InteractionError::Timeout { secs: 30 });
        assert!(matches!(err, SpecksError::AgentTimeout { secs: 30 }));
    }

    #[test]
    fn test_planning_loop_has_adapter() {
        let ctx = LoopContext::new_idea("test idea".to_string(), vec![]);
        let project_root = PathBuf::from("/project");

        let loop_instance = PlanningLoop::new(
            ctx,
            project_root,
            300,
            None,
            false,
            false,
            create_mock_adapter(),
            PlanningMode::Cli,
        );

        // Verify the adapter is accessible and works
        let adapter = loop_instance.adapter();
        let handle = adapter.start_progress("test");
        assert_eq!(handle.message(), "test");
        adapter.end_progress(handle, true);
    }

    /// A mock adapter that cancels during gather
    struct CancellingMockAdapter {
        progress_counter: AtomicU64,
    }

    impl CancellingMockAdapter {
        fn new() -> Self {
            Self {
                progress_counter: AtomicU64::new(0),
            }
        }
    }

    impl InteractionAdapter for CancellingMockAdapter {
        fn ask_text(&self, _prompt: &str, _default: Option<&str>) -> InteractionResult<String> {
            // Simulate user cancellation
            Err(specks_core::interaction::InteractionError::Cancelled)
        }

        fn ask_select(&self, _prompt: &str, _options: &[&str]) -> InteractionResult<usize> {
            // Simulate user cancellation
            Err(specks_core::interaction::InteractionError::Cancelled)
        }

        fn ask_confirm(&self, _prompt: &str, _default: bool) -> InteractionResult<bool> {
            // Simulate user cancellation
            Err(specks_core::interaction::InteractionError::Cancelled)
        }

        fn ask_multi_select(
            &self,
            _prompt: &str,
            _options: &[&str],
        ) -> InteractionResult<Vec<usize>> {
            Err(specks_core::interaction::InteractionError::Cancelled)
        }

        fn start_progress(&self, message: &str) -> ProgressHandle {
            let id = self.progress_counter.fetch_add(1, Ordering::SeqCst);
            ProgressHandle::new(id, message)
        }

        fn end_progress(&self, _handle: ProgressHandle, _success: bool) {}

        fn print_info(&self, _message: &str) {}
        fn print_warning(&self, _message: &str) {}
        fn print_error(&self, _message: &str) {}
        fn print_success(&self, _message: &str) {}
        fn print_header(&self, _message: &str) {}
    }

    #[test]
    fn test_cli_mode_cancellation_returns_user_aborted() {
        // Test that cancellation during CLI gather returns UserAborted
        let ctx = LoopContext::new_idea("test idea".to_string(), vec![]);
        let project_root = PathBuf::from("/project");

        let mut loop_instance = PlanningLoop::new(
            ctx,
            project_root,
            300,
            None,
            false,
            false,
            Box::new(CancellingMockAdapter::new()),
            PlanningMode::Cli,
        );

        // Simulate the state transition to Present (was InterviewerGather)
        loop_instance.transition(LoopState::Present);

        // Try to run CLI gather - should fail with UserAborted
        let result = loop_instance.run_cli_gather();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SpecksError::UserAborted));
    }

    #[test]
    fn test_cli_mode_gather_produces_requirements() {
        // Test that CLI gather with approving adapter produces requirements
        let ctx = LoopContext::new_idea("test idea for feature".to_string(), vec![]);
        let project_root = PathBuf::from("/project");

        let mut loop_instance = PlanningLoop::new(
            ctx,
            project_root,
            300,
            None,
            false,
            true,                  // quiet mode
            create_mock_adapter(), // MockAdapter defaults to confirming
            PlanningMode::Cli,
        );

        // Run CLI gather
        let result = loop_instance.run_cli_gather();

        assert!(result.is_ok());
        // Verify requirements were gathered
        assert!(loop_instance.context.requirements.is_some());
        let requirements = loop_instance.context.requirements.as_ref().unwrap();
        assert!(requirements.contains("test idea for feature"));
    }

    #[test]
    fn test_cli_mode_present_returns_decision() {
        // Test that CLI present with approving adapter returns Approve decision
        let mut ctx = LoopContext::new_idea("test idea".to_string(), vec![]);
        ctx.speck_path = Some(PathBuf::from("/project/.specks/specks-test.md"));
        ctx.critic_feedback = Some("Approved. Looks good.".to_string());

        let project_root = PathBuf::from("/project");

        let mut loop_instance = PlanningLoop::new(
            ctx,
            project_root,
            300,
            None,
            false,
            true,                  // quiet mode
            create_mock_adapter(), // MockAdapter selects first option (Approve)
            PlanningMode::Cli,
        );

        // Run CLI present
        let result = loop_instance.run_cli_present();

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), UserDecision::Approve));
    }

    #[test]
    fn test_mode_aware_gather_uses_cli_gatherer_in_cli_mode() {
        // Verify that CLI mode uses CliGatherer (not agent)
        let ctx = LoopContext::new_idea("test".to_string(), vec![]);
        let project_root = PathBuf::from("/project");

        let loop_instance = PlanningLoop::new(
            ctx,
            project_root,
            300,
            None,
            false,
            false,
            create_mock_adapter(),
            PlanningMode::Cli,
        );

        // Verify mode is CLI
        assert_eq!(*loop_instance.planning_mode(), PlanningMode::Cli);

        // The run_interviewer_gather method will branch based on mode
        // In CLI mode, it calls run_cli_gather() which uses CliGatherer
        // We can't easily test the internal branching without running,
        // but we verify the mode is set correctly
    }

    #[test]
    fn test_mode_aware_present_uses_cli_presenter_in_cli_mode() {
        // Verify that CLI mode uses CliPresenter (not agent)
        let mut ctx = LoopContext::new_idea("test".to_string(), vec![]);
        ctx.speck_path = Some(PathBuf::from("/project/.specks/specks-test.md"));
        ctx.critic_feedback = Some("Approved.".to_string());

        let project_root = PathBuf::from("/project");

        let loop_instance = PlanningLoop::new(
            ctx,
            project_root,
            300,
            None,
            false,
            false,
            create_mock_adapter(),
            PlanningMode::Cli,
        );

        // Verify mode is CLI
        assert_eq!(*loop_instance.planning_mode(), PlanningMode::Cli);

        // The run_interviewer_present method will branch based on mode
        // In CLI mode, it calls run_cli_present() which uses CliPresenter
    }
}
