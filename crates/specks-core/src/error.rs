//! Error types for specks operations

use thiserror::Error;

/// Core error type for specks operations
#[derive(Error, Debug)]
pub enum SpecksError {
    // === Structural errors (E001-E006) ===
    /// E001: Missing required section
    #[error("E001: Missing required section: {section}")]
    MissingSection { section: String, line: Option<usize> },

    /// E002: Missing or empty required metadata field
    #[error("E002: Missing or empty required metadata field: {field}")]
    MissingMetadataField { field: String, line: Option<usize> },

    /// E003: Invalid metadata Status value
    #[error("E003: Invalid metadata Status value: {value} (must be draft/active/done)")]
    InvalidStatus { value: String, line: Option<usize> },

    /// E004: Step missing References line
    #[error("E004: Step missing References line")]
    MissingReferences { step: String, line: Option<usize> },

    /// E005: Invalid anchor format
    #[error("E005: Invalid anchor format: {anchor}")]
    InvalidAnchor { anchor: String, line: Option<usize> },

    /// E006: Duplicate anchor
    #[error("E006: Duplicate anchor: {anchor}")]
    DuplicateAnchor {
        anchor: String,
        first_line: usize,
        second_line: usize,
    },

    // === Project errors (E009) ===
    /// E009: .specks directory not initialized
    #[error("E009: .specks directory not initialized")]
    NotInitialized,

    // === Dependency errors (E010-E011) ===
    /// E010: Dependency references non-existent step anchor
    #[error("E010: Dependency references non-existent step anchor: {anchor}")]
    InvalidDependency {
        anchor: String,
        step: String,
        line: Option<usize>,
    },

    /// E011: Circular dependency detected
    #[error("E011: Circular dependency detected: {cycle}")]
    CircularDependency { cycle: String },

    // === Beads errors (E012-E015) ===
    /// E012: Invalid bead ID format
    #[error("E012: Invalid bead ID format: {id}")]
    InvalidBeadId { id: String, line: Option<usize> },

    /// E013: Beads not initialized in project
    #[error("E013: Beads not initialized in project (run `bd init`)")]
    BeadsNotInitialized,

    /// E014: Beads Root bead does not exist
    #[error("E014: Beads Root bead does not exist: {id}")]
    BeadsRootNotFound { id: String },

    /// E015: Step bead does not exist
    #[error("E015: Step bead does not exist: {id} (step anchor: {anchor})")]
    StepBeadNotFound { id: String, anchor: String },

    // === IO and system errors ===
    /// File not found or unreadable
    #[error("file not found or unreadable: {0}")]
    FileNotFound(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error
    #[error("configuration error: {0}")]
    Config(String),

    /// Parse error (general markdown/structure issue)
    #[error("parse error: {message}")]
    Parse {
        message: String,
        line: Option<usize>,
    },

    /// Feature not implemented
    #[error("feature not implemented: {0}")]
    NotImplemented(String),

    /// Beads CLI not installed (exit code 5)
    #[error("beads CLI not installed or not found")]
    BeadsNotInstalled,

    /// Beads command failed
    #[error("beads command failed: {0}")]
    BeadsCommand(String),

    /// Step anchor not found
    #[error("step anchor not found: {0}")]
    StepAnchorNotFound(String),

    // === Agent errors (E019-E021) ===
    /// E019: Claude CLI not installed
    #[error("E019: Claude CLI not installed. Install Claude Code from https://claude.ai/download")]
    ClaudeCliNotInstalled,

    /// E020: Agent invocation failed
    #[error("E020: Agent invocation failed: {reason}")]
    AgentInvocationFailed { reason: String },

    /// E021: Agent timeout
    #[error("E021: Agent timeout after {secs} seconds")]
    AgentTimeout { secs: u64 },

    // === Planning errors (E023-E024) ===
    /// E023: Created speck has validation warnings
    #[error("E023: Created speck has validation warnings")]
    SpeckValidationWarnings { warning_count: usize },

    /// E024: User aborted planning loop
    #[error("E024: User aborted planning loop")]
    UserAborted,

    // === Execution errors (E022) ===
    /// E022: Monitor halted execution
    #[error("E022: Monitor halted execution: {reason}")]
    MonitorHalted { reason: String },

    // === Distribution errors (E025) ===
    /// E025: Skills not found in share directory
    #[error("E025: Skills not found in share directory: {path}")]
    SkillsNotFound { path: String },
}

impl SpecksError {
    /// Get the error code (e.g., "E001", "E002")
    pub fn code(&self) -> &'static str {
        match self {
            SpecksError::MissingSection { .. } => "E001",
            SpecksError::MissingMetadataField { .. } => "E002",
            SpecksError::InvalidStatus { .. } => "E003",
            SpecksError::MissingReferences { .. } => "E004",
            SpecksError::InvalidAnchor { .. } => "E005",
            SpecksError::DuplicateAnchor { .. } => "E006",
            SpecksError::NotInitialized => "E009",
            SpecksError::InvalidDependency { .. } => "E010",
            SpecksError::CircularDependency { .. } => "E011",
            SpecksError::InvalidBeadId { .. } => "E012",
            SpecksError::BeadsNotInitialized => "E013",
            SpecksError::BeadsRootNotFound { .. } => "E014",
            SpecksError::StepBeadNotFound { .. } => "E015",
            SpecksError::FileNotFound(_) => "E002", // Reuse for file errors
            SpecksError::Io(_) => "E002",
            SpecksError::Config(_) => "E004", // Config errors
            SpecksError::Parse { .. } => "E001",
            SpecksError::NotImplemented(_) => "E003", // Feature not implemented
            SpecksError::BeadsNotInstalled => "E005", // Beads CLI error
            SpecksError::BeadsCommand(_) => "E016",   // Beads command error
            SpecksError::StepAnchorNotFound(_) => "E017", // Step anchor not found
            SpecksError::ClaudeCliNotInstalled => "E019",
            SpecksError::AgentInvocationFailed { .. } => "E020",
            SpecksError::AgentTimeout { .. } => "E021",
            SpecksError::MonitorHalted { .. } => "E022",
            SpecksError::SpeckValidationWarnings { .. } => "E023",
            SpecksError::UserAborted => "E024",
            SpecksError::SkillsNotFound { .. } => "E025",
        }
    }

    /// Get the line number associated with this error, if any
    pub fn line(&self) -> Option<usize> {
        match self {
            SpecksError::MissingSection { line, .. } => *line,
            SpecksError::MissingMetadataField { line, .. } => *line,
            SpecksError::InvalidStatus { line, .. } => *line,
            SpecksError::MissingReferences { line, .. } => *line,
            SpecksError::InvalidAnchor { line, .. } => *line,
            SpecksError::DuplicateAnchor { second_line, .. } => Some(*second_line),
            SpecksError::InvalidDependency { line, .. } => *line,
            SpecksError::InvalidBeadId { line, .. } => *line,
            SpecksError::Parse { line, .. } => *line,
            _ => None,
        }
    }

    /// Get the exit code for this error type
    pub fn exit_code(&self) -> i32 {
        match self {
            SpecksError::MissingSection { .. }
            | SpecksError::MissingMetadataField { .. }
            | SpecksError::InvalidStatus { .. }
            | SpecksError::MissingReferences { .. }
            | SpecksError::InvalidAnchor { .. }
            | SpecksError::DuplicateAnchor { .. }
            | SpecksError::InvalidDependency { .. }
            | SpecksError::CircularDependency { .. }
            | SpecksError::InvalidBeadId { .. } => 1, // Validation errors

            SpecksError::FileNotFound(_) | SpecksError::Io(_) => 2, // File errors

            SpecksError::NotImplemented(_) => 3, // Feature not implemented

            SpecksError::Config(_) => 4, // Configuration error

            SpecksError::BeadsNotInstalled => 5, // Beads CLI not installed

            SpecksError::BeadsCommand(_) => 1, // Beads command error

            SpecksError::StepAnchorNotFound(_) => 2, // Step anchor not found

            SpecksError::NotInitialized => 9, // .specks not initialized

            SpecksError::BeadsNotInitialized
            | SpecksError::BeadsRootNotFound { .. }
            | SpecksError::StepBeadNotFound { .. } => 13, // Beads not initialized

            SpecksError::Parse { .. } => 1, // Parse errors are validation errors

            SpecksError::ClaudeCliNotInstalled => 6, // Claude CLI not installed

            SpecksError::AgentInvocationFailed { .. } | SpecksError::AgentTimeout { .. } => 1, // Agent errors

            SpecksError::MonitorHalted { .. } => 4, // Monitor halted execution

            SpecksError::SpeckValidationWarnings { .. } => 0, // Warnings are not failures

            SpecksError::UserAborted => 5, // User aborted planning loop

            SpecksError::SkillsNotFound { .. } => 7, // Skills not found
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        let err = SpecksError::MissingSection {
            section: "Plan Metadata".to_string(),
            line: Some(10),
        };
        assert_eq!(err.code(), "E001");
        assert_eq!(err.line(), Some(10));
        assert_eq!(err.exit_code(), 1);

        let err = SpecksError::NotInitialized;
        assert_eq!(err.code(), "E009");
        assert_eq!(err.exit_code(), 9);

        let err = SpecksError::BeadsNotInitialized;
        assert_eq!(err.code(), "E013");
        assert_eq!(err.exit_code(), 13);
    }

    #[test]
    fn test_error_display() {
        let err = SpecksError::InvalidStatus {
            value: "invalid".to_string(),
            line: Some(5),
        };
        assert_eq!(
            err.to_string(),
            "E003: Invalid metadata Status value: invalid (must be draft/active/done)"
        );
    }

    #[test]
    fn test_agent_error_codes() {
        let err = SpecksError::ClaudeCliNotInstalled;
        assert_eq!(err.code(), "E019");
        assert_eq!(err.exit_code(), 6);

        let err = SpecksError::AgentInvocationFailed {
            reason: "test failure".to_string(),
        };
        assert_eq!(err.code(), "E020");
        assert_eq!(err.exit_code(), 1);
        assert!(err.to_string().contains("test failure"));

        let err = SpecksError::AgentTimeout { secs: 300 };
        assert_eq!(err.code(), "E021");
        assert_eq!(err.exit_code(), 1);
        assert!(err.to_string().contains("300 seconds"));
    }

    #[test]
    fn test_planning_error_codes() {
        let err = SpecksError::SpeckValidationWarnings { warning_count: 3 };
        assert_eq!(err.code(), "E023");
        assert_eq!(err.exit_code(), 0); // Warnings don't cause failure
        assert!(err.to_string().contains("validation warnings"));

        let err = SpecksError::UserAborted;
        assert_eq!(err.code(), "E024");
        assert_eq!(err.exit_code(), 5);
        assert!(err.to_string().contains("aborted"));
    }

    #[test]
    fn test_skills_not_found_error() {
        let err = SpecksError::SkillsNotFound {
            path: "/some/path".to_string(),
        };
        assert_eq!(err.code(), "E025");
        assert_eq!(err.exit_code(), 7);
        assert!(err.to_string().contains("/some/path"));
        assert!(err.to_string().contains("Skills not found"));
    }

    #[test]
    fn test_monitor_halted_error() {
        let err = SpecksError::MonitorHalted {
            reason: "drift detected".to_string(),
        };
        assert_eq!(err.code(), "E022");
        assert_eq!(err.exit_code(), 4);
        assert!(err.to_string().contains("drift detected"));
        assert!(err.to_string().contains("Monitor halted"));
    }
}
