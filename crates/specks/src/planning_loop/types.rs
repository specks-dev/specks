//! Types for the planning loop state machine
//!
//! These types are shared between CLI and Claude Code modes of the planning loop.
//!
//! Per [D21] and [D24], the clarifier agent runs in EVERY iteration:
//! - First iteration: analyzes the user's idea
//! - Subsequent iterations: analyzes critic feedback for revision questions
//!
//! Flow: Start -> Clarifier -> Present -> Planner -> Critic -> Present -> (Approve | loop)

use std::path::PathBuf;

/// Planning loop state per Concept C02 and [D21]/[D24]
///
/// The loop runs: clarifier -> presenter -> planner -> critic -> presenter -> (loop)
/// The clarifier runs in EVERY iteration, generating context-aware questions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopState {
    /// Initial state: receive idea string or speck path
    Start,
    /// Clarifier analyzing idea (first iteration) or critic feedback (subsequent)
    Clarifier,
    /// Presenter showing clarifier questions and gathering answers
    /// (CLI: inquire prompts, Claude Code: interviewer agent)
    Present,
    /// Planner creating or revising the speck
    Planner,
    /// Critic reviewing the speck for quality
    Critic,
    /// Presenter showing critic feedback and asking for approval
    /// (Same as Present state, but after critic review)
    CriticPresent,
    /// User provided feedback, loop back to clarifier with critic feedback
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
            LoopState::Start => Some(LoopState::Clarifier),
            LoopState::Clarifier => Some(LoopState::Present),
            LoopState::Present => Some(LoopState::Planner),
            LoopState::Planner => Some(LoopState::Critic),
            LoopState::Critic => Some(LoopState::CriticPresent),
            LoopState::CriticPresent => None, // Branches to Revise or Approved
            LoopState::Revise => Some(LoopState::Clarifier), // Loop back through clarifier
            LoopState::Approved | LoopState::Aborted => None,
        }
    }
}

/// Mode of the planning loop (new vs revision)
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

/// Invocation mode for the planning loop per [D19]
///
/// The caller explicitly specifies whether the loop is running in CLI mode
/// (where the CLI handles interaction) or Claude Code mode (where the
/// interviewer agent handles interaction).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanningMode {
    /// CLI mode: CLI code handles all user interaction via inquire prompts.
    /// The interviewer agent is NOT invoked in this mode.
    Cli,
    /// Claude Code mode: interviewer agent handles user interaction via AskUserQuestion.
    /// This mode is used when running via slash commands inside Claude Code.
    ClaudeCode,
}

impl std::fmt::Display for PlanningMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanningMode::Cli => write!(f, "cli"),
            PlanningMode::ClaudeCode => write!(f, "claude-code"),
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
    /// Mode that was used (new vs revision)
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
    /// Mode of operation (new vs revision)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_state_transitions() {
        // New flow per [D21] and [D24]: clarifier runs every iteration
        assert_eq!(LoopState::Start.next(), Some(LoopState::Clarifier));
        assert_eq!(LoopState::Clarifier.next(), Some(LoopState::Present));
        assert_eq!(LoopState::Present.next(), Some(LoopState::Planner));
        assert_eq!(LoopState::Planner.next(), Some(LoopState::Critic));
        assert_eq!(LoopState::Critic.next(), Some(LoopState::CriticPresent));
        assert_eq!(LoopState::CriticPresent.next(), None); // Branches to Revise or Approved
        assert_eq!(LoopState::Revise.next(), Some(LoopState::Clarifier)); // Loop back through clarifier
        assert_eq!(LoopState::Approved.next(), None);
        assert_eq!(LoopState::Aborted.next(), None);
    }

    #[test]
    fn test_loop_state_terminal() {
        assert!(!LoopState::Start.is_terminal());
        assert!(!LoopState::Clarifier.is_terminal());
        assert!(!LoopState::Present.is_terminal());
        assert!(!LoopState::Planner.is_terminal());
        assert!(!LoopState::Critic.is_terminal());
        assert!(!LoopState::CriticPresent.is_terminal());
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
    fn test_planning_mode_display() {
        assert_eq!(format!("{}", PlanningMode::Cli), "cli");
        assert_eq!(format!("{}", PlanningMode::ClaudeCode), "claude-code");
    }

    #[test]
    fn test_planning_mode_equality() {
        assert_eq!(PlanningMode::Cli, PlanningMode::Cli);
        assert_eq!(PlanningMode::ClaudeCode, PlanningMode::ClaudeCode);
        assert_ne!(PlanningMode::Cli, PlanningMode::ClaudeCode);
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
