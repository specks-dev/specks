//! JSON output formatting per Spec S05

use serde::{Deserialize, Serialize};
use specks_core::{ValidationIssue, Severity};

const SCHEMA_VERSION: &str = "1";

/// JSON response envelope per Spec S05
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonResponse<T> {
    /// Schema version for forward compatibility
    pub schema_version: String,
    /// Command that generated this response
    pub command: String,
    /// Status: "ok" or "error"
    pub status: String,
    /// Command-specific payload
    pub data: T,
    /// Validation issues, warnings, etc.
    pub issues: Vec<JsonIssue>,
}

impl<T> JsonResponse<T> {
    /// Create a successful response
    pub fn ok(command: &str, data: T) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            command: command.to_string(),
            status: "ok".to_string(),
            data,
            issues: vec![],
        }
    }

    /// Create a successful response with issues
    pub fn ok_with_issues(command: &str, data: T, issues: Vec<JsonIssue>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            command: command.to_string(),
            status: "ok".to_string(),
            data,
            issues,
        }
    }

    /// Create an error response
    pub fn error(command: &str, data: T, issues: Vec<JsonIssue>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            command: command.to_string(),
            status: "error".to_string(),
            data,
            issues,
        }
    }
}

/// Issue object structure per Spec S05
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonIssue {
    /// Error/warning code (e.g., "E001")
    pub code: String,
    /// Severity level
    pub severity: String,
    /// Human-readable message
    pub message: String,
    /// Project-root-relative file path using forward slashes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Line number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// Anchor reference (always starts with # if present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
}

impl From<&ValidationIssue> for JsonIssue {
    fn from(issue: &ValidationIssue) -> Self {
        let severity = match issue.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        };
        Self {
            code: issue.code.clone(),
            severity: severity.to_string(),
            message: issue.message.clone(),
            file: None, // Set by the caller with proper path
            line: issue.line,
            anchor: issue.anchor.clone(),
        }
    }
}

impl JsonIssue {
    /// Set the file path
    pub fn with_file(mut self, file: &str) -> Self {
        self.file = Some(file.to_string());
        self
    }
}

/// Data payload for init command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitData {
    /// Path to the created directory
    pub path: String,
    /// Files created
    pub files_created: Vec<String>,
}

/// Data payload for validate command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateData {
    /// Validated files
    pub files: Vec<ValidatedFile>,
}

/// A validated file entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedFile {
    /// Project-root-relative path
    pub path: String,
    /// Whether the file is valid (no errors)
    pub valid: bool,
    /// Number of errors
    pub error_count: usize,
    /// Number of warnings
    pub warning_count: usize,
}

/// Data payload for list command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListData {
    /// List of specks
    pub specks: Vec<SpeckSummary>,
}

/// Summary of a speck for list command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeckSummary {
    /// Name without prefix/extension
    pub name: String,
    /// Status from metadata
    pub status: String,
    /// Progress (done/total checkboxes)
    pub progress: Progress,
    /// Last updated date
    pub updated: String,
}

/// Progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    /// Number of completed items
    pub done: usize,
    /// Total number of items
    pub total: usize,
}

/// Data payload for status command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusData {
    /// Speck name
    pub name: String,
    /// Status from metadata
    pub status: String,
    /// Overall progress
    pub progress: Progress,
    /// Step-by-step status
    pub steps: Vec<StepStatus>,
}

/// Status of a single step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepStatus {
    /// Step title
    pub title: String,
    /// Step anchor (with #)
    pub anchor: String,
    /// Number of completed items
    pub done: usize,
    /// Total number of items
    pub total: usize,
    /// Substeps (if any)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub substeps: Vec<SubstepStatus>,
}

/// Status of a substep
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstepStatus {
    /// Substep title
    pub title: String,
    /// Substep anchor (with #)
    pub anchor: String,
    /// Number of completed items
    pub done: usize,
    /// Total number of items
    pub total: usize,
}
