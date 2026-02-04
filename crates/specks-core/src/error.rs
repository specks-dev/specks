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

    /// Beads CLI not installed
    #[error("beads CLI not installed or not found")]
    BeadsCliNotFound,
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
            SpecksError::BeadsCliNotFound => "E005",  // Beads CLI error
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

            SpecksError::BeadsCliNotFound => 5, // Beads CLI not installed

            SpecksError::NotInitialized => 9, // .specks not initialized

            SpecksError::BeadsNotInitialized
            | SpecksError::BeadsRootNotFound { .. }
            | SpecksError::StepBeadNotFound { .. } => 13, // Beads not initialized

            SpecksError::Parse { .. } => 1, // Parse errors are validation errors
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
}
