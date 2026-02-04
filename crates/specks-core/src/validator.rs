//! Validation logic and rules

use serde::{Deserialize, Serialize};

use crate::types::Speck;

/// Result of validating a speck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the speck is valid (no errors)
    pub valid: bool,
    /// List of validation issues
    pub issues: Vec<ValidationIssue>,
}

/// A single validation issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Error/warning code (e.g., "E001", "W001")
    pub code: String,
    /// Severity level
    pub severity: Severity,
    /// Human-readable message
    pub message: String,
    /// Line number (if applicable)
    pub line: Option<usize>,
    /// Anchor reference (if applicable)
    pub anchor: Option<String>,
}

/// Severity level for validation issues
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Must fix
    Error,
    /// Should fix
    Warning,
    /// Optional/informational
    Info,
}

/// Validate a parsed speck
pub fn validate_speck(_speck: &Speck) -> ValidationResult {
    // TODO: Implement in Step 2
    ValidationResult {
        valid: true,
        issues: vec![],
    }
}
