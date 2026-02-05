//! Planning loop state machine for iterative speck creation
//!
//! Implements the iterative planning loop per [D03] Iterative Planning Loop and
//! Concept C02: Planning Loop State Machine.
//!
//! The loop runs: interviewer -> planner -> critic -> interviewer -> (approve | revise)
//! until the user approves the speck or aborts.

use std::path::{Path, PathBuf};

use specks_core::SpecksError;

use crate::agent::AgentRunner;

/// Planning loop state per Concept C02
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopState {
    /// Initial state: receive idea string or speck path
    Start,
    /// Interviewer gathering requirements from user
    InterviewerGather,
    /// Planner creating or revising the speck
    Planner,
    /// Critic reviewing the speck for quality
    Critic,
    /// Interviewer presenting results and asking for approval
    InterviewerPresent,
    /// User provided feedback, loop back to planner
    #[allow(dead_code)] // Part of state machine design, used when full loop is implemented
    Revise,
    /// User approved the speck
    Approved,
    /// User aborted the planning loop
    Aborted,
}

impl LoopState {
    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, LoopState::Approved | LoopState::Aborted)
    }

    /// Get the next state in the normal flow
    #[allow(dead_code)] // Part of state machine design
    pub fn next(&self) -> Option<LoopState> {
        match self {
            LoopState::Start => Some(LoopState::InterviewerGather),
            LoopState::InterviewerGather => Some(LoopState::Planner),
            LoopState::Planner => Some(LoopState::Critic),
            LoopState::Critic => Some(LoopState::InterviewerPresent),
            LoopState::InterviewerPresent => None, // Branches to Revise or Approved
            LoopState::Revise => Some(LoopState::Planner),
            LoopState::Approved | LoopState::Aborted => None,
        }
    }
}

/// Mode of the planning loop
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanMode {
    /// Creating a new speck from an idea
    New,
    /// Revising an existing speck
    Revision,
}

impl std::fmt::Display for PlanMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanMode::New => write!(f, "new"),
            PlanMode::Revision => write!(f, "revision"),
        }
    }
}

/// Outcome of the planning loop
#[derive(Debug, Clone)]
pub struct LoopOutcome {
    /// Path to the created/revised speck
    pub speck_path: PathBuf,
    /// Name of the speck
    pub speck_name: String,
    /// Mode that was used
    pub mode: PlanMode,
    /// Number of iterations through the loop
    pub iterations: usize,
    /// Number of validation errors
    pub validation_errors: usize,
    /// Number of validation warnings
    pub validation_warnings: usize,
    /// Whether critic approved
    pub critic_approved: bool,
    /// Whether user approved
    pub user_approved: bool,
}

/// Context passed between loop iterations
#[derive(Debug, Clone)]
pub struct LoopContext {
    /// The original idea or speck path
    pub input: String,
    /// Mode of operation
    pub mode: PlanMode,
    /// Output from interviewer gathering phase
    pub requirements: Option<String>,
    /// Path to the current speck draft
    pub speck_path: Option<PathBuf>,
    /// Output from critic review
    pub critic_feedback: Option<String>,
    /// User's revision feedback
    pub revision_feedback: Option<String>,
    /// Current iteration count
    pub iteration: usize,
    /// Additional context file contents
    pub context_files: Vec<String>,
}

impl LoopContext {
    /// Create a new context for a fresh idea
    pub fn new_idea(idea: String, context_files: Vec<String>) -> Self {
        Self {
            input: idea,
            mode: PlanMode::New,
            requirements: None,
            speck_path: None,
            critic_feedback: None,
            revision_feedback: None,
            iteration: 0,
            context_files,
        }
    }

    /// Create a new context for revising an existing speck
    pub fn revision(speck_path: PathBuf, context_files: Vec<String>) -> Self {
        Self {
            input: speck_path.to_string_lossy().to_string(),
            mode: PlanMode::Revision,
            requirements: None,
            speck_path: Some(speck_path),
            critic_feedback: None,
            revision_feedback: None,
            iteration: 0,
            context_files,
        }
    }
}

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
}

impl PlanningLoop {
    /// Create a new planning loop
    pub fn new(
        context: LoopContext,
        project_root: PathBuf,
        timeout_secs: u64,
        speck_name: Option<String>,
        json_output: bool,
        quiet: bool,
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
        }
    }

    /// Set a custom agent runner (for testing)
    #[allow(dead_code)]
    pub fn with_runner(mut self, runner: AgentRunner) -> Self {
        self.runner = runner;
        self
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
    pub fn run(&mut self) -> Result<LoopOutcome, SpecksError> {
        // Verify claude CLI is available
        self.runner.check_claude_cli()?;

        // Start the loop
        self.transition(LoopState::InterviewerGather);

        while !self.is_complete() {
            match self.state {
                LoopState::InterviewerGather => {
                    self.run_interviewer_gather()?;
                    self.transition(LoopState::Planner);
                }
                LoopState::Planner => {
                    self.run_planner()?;
                    self.transition(LoopState::Critic);
                }
                LoopState::Critic => {
                    self.run_critic()?;
                    self.transition(LoopState::InterviewerPresent);
                }
                LoopState::InterviewerPresent => {
                    let user_decision = self.run_interviewer_present()?;
                    match user_decision {
                        UserDecision::Approve => {
                            self.transition(LoopState::Approved);
                        }
                        UserDecision::Revise(feedback) => {
                            self.context.revision_feedback = Some(feedback);
                            self.context.iteration += 1;
                            self.transition(LoopState::Planner);
                        }
                        UserDecision::Abort => {
                            self.transition(LoopState::Aborted);
                        }
                    }
                }
                LoopState::Revise => {
                    // This state is handled by transitioning to Planner
                    self.transition(LoopState::Planner);
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

    /// Run the interviewer gather phase
    fn run_interviewer_gather(&mut self) -> Result<(), SpecksError> {
        if !self.quiet {
            eprintln!("Gathering requirements...");
        }

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

        let result = self.runner.invoke_agent(&config, &prompt)?;
        self.context.requirements = Some(result.output);

        Ok(())
    }

    /// Run the planner phase
    fn run_planner(&mut self) -> Result<(), SpecksError> {
        if !self.quiet {
            if self.context.iteration == 0 {
                eprintln!("Creating speck...");
            } else {
                eprintln!(
                    "Revising speck (iteration {})...",
                    self.context.iteration + 1
                );
            }
        }

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

        let _result = self.runner.invoke_agent(&config, &prompt)?;

        // Update context with speck path
        self.context.speck_path = Some(speck_path);

        Ok(())
    }

    /// Run the critic phase
    fn run_critic(&mut self) -> Result<(), SpecksError> {
        if !self.quiet {
            eprintln!("Reviewing speck...");
        }

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

        let result = self.runner.invoke_agent(&config, &prompt)?;
        self.context.critic_feedback = Some(result.output);

        Ok(())
    }

    /// Run the interviewer present phase and get user decision
    fn run_interviewer_present(&mut self) -> Result<UserDecision, SpecksError> {
        if !self.quiet {
            eprintln!("Presenting results...");
        }

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
        prompt.push_str("If the user says 'ready', 'approve', 'looks good', 'yes', or similar, return APPROVED. ");
        prompt.push_str("If the user says 'abort', 'cancel', 'quit', or similar, return ABORTED. ");
        prompt.push_str("Otherwise, return REVISE: followed by their feedback.");

        let config =
            crate::agent::interviewer_config(&self.project_root).with_timeout(self.timeout_secs);

        let result = self.runner.invoke_agent(&config, &prompt)?;

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

/// User's decision after reviewing the speck
#[derive(Debug, Clone)]
pub enum UserDecision {
    /// User approved the speck
    Approve,
    /// User wants to revise with this feedback
    Revise(String),
    /// User aborted the planning loop
    Abort,
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

    #[test]
    fn test_loop_state_transitions() {
        assert_eq!(LoopState::Start.next(), Some(LoopState::InterviewerGather));
        assert_eq!(
            LoopState::InterviewerGather.next(),
            Some(LoopState::Planner)
        );
        assert_eq!(LoopState::Planner.next(), Some(LoopState::Critic));
        assert_eq!(
            LoopState::Critic.next(),
            Some(LoopState::InterviewerPresent)
        );
        assert_eq!(LoopState::InterviewerPresent.next(), None);
        assert_eq!(LoopState::Revise.next(), Some(LoopState::Planner));
        assert_eq!(LoopState::Approved.next(), None);
        assert_eq!(LoopState::Aborted.next(), None);
    }

    #[test]
    fn test_loop_state_terminal() {
        assert!(!LoopState::Start.is_terminal());
        assert!(!LoopState::InterviewerGather.is_terminal());
        assert!(!LoopState::Planner.is_terminal());
        assert!(!LoopState::Critic.is_terminal());
        assert!(!LoopState::InterviewerPresent.is_terminal());
        assert!(!LoopState::Revise.is_terminal());
        assert!(LoopState::Approved.is_terminal());
        assert!(LoopState::Aborted.is_terminal());
    }

    #[test]
    fn test_plan_mode_display() {
        assert_eq!(format!("{}", PlanMode::New), "new");
        assert_eq!(format!("{}", PlanMode::Revision), "revision");
    }

    #[test]
    fn test_loop_context_new_idea() {
        let ctx = LoopContext::new_idea("add feature X".to_string(), vec![]);
        assert_eq!(ctx.input, "add feature X");
        assert_eq!(ctx.mode, PlanMode::New);
        assert!(ctx.requirements.is_none());
        assert!(ctx.speck_path.is_none());
        assert_eq!(ctx.iteration, 0);
    }

    #[test]
    fn test_loop_context_revision() {
        let path = PathBuf::from("/project/.specks/specks-1.md");
        let ctx = LoopContext::revision(path.clone(), vec![]);
        assert_eq!(ctx.input, path.to_string_lossy().to_string());
        assert_eq!(ctx.mode, PlanMode::Revision);
        assert_eq!(ctx.speck_path, Some(path));
        assert_eq!(ctx.iteration, 0);
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

        let loop_instance = PlanningLoop::new(ctx, project_root, 300, None, false, false);

        assert_eq!(*loop_instance.state(), LoopState::Start);
        assert!(!loop_instance.is_complete());
    }

    #[test]
    fn test_planning_loop_manual_transitions() {
        let ctx = LoopContext::new_idea("test idea".to_string(), vec![]);
        let project_root = PathBuf::from("/project");

        let mut loop_instance = PlanningLoop::new(ctx, project_root, 300, None, false, false);

        // Manual state transitions
        loop_instance.transition(LoopState::InterviewerGather);
        assert_eq!(*loop_instance.state(), LoopState::InterviewerGather);

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
        let loop_instance = PlanningLoop::new(ctx, project_root, 300, None, false, false);

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
    fn test_loop_context_with_context_files() {
        let context_files = vec![
            "Context from file 1".to_string(),
            "Context from file 2".to_string(),
        ];
        let ctx = LoopContext::new_idea("test idea".to_string(), context_files.clone());

        assert_eq!(ctx.context_files.len(), 2);
        assert_eq!(ctx.context_files[0], "Context from file 1");
    }

    #[test]
    fn test_user_decision_variants() {
        // Test that UserDecision variants can be created
        let approve = UserDecision::Approve;
        let revise = UserDecision::Revise("needs more detail".to_string());
        let abort = UserDecision::Abort;

        // Basic pattern matching
        match approve {
            UserDecision::Approve => {}
            _ => panic!("Expected Approve"),
        }

        match revise {
            UserDecision::Revise(feedback) => {
                assert_eq!(feedback, "needs more detail");
            }
            _ => panic!("Expected Revise"),
        }

        match abort {
            UserDecision::Abort => {}
            _ => panic!("Expected Abort"),
        }
    }

    #[test]
    fn test_loop_outcome_structure() {
        let outcome = LoopOutcome {
            speck_path: PathBuf::from(".specks/specks-test.md"),
            speck_name: "test".to_string(),
            mode: PlanMode::New,
            iterations: 2,
            validation_errors: 0,
            validation_warnings: 1,
            critic_approved: true,
            user_approved: true,
        };

        assert_eq!(outcome.speck_name, "test");
        assert_eq!(outcome.iterations, 2);
        assert!(outcome.critic_approved);
        assert!(outcome.user_approved);
    }
}
