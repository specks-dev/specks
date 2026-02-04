//! specks-core: Core library for parsing, validation, and types
//!
//! This crate provides the foundational types and logic for the specks system.

/// Core error types for specks operations
pub mod error;

/// Configuration handling
pub mod config;

/// Core data types (Speck, Step, Checkpoint, etc.)
pub mod types;

/// Speck file parsing
pub mod parser;

/// Validation logic and rules
pub mod validator;

// Re-exports for convenience
pub use error::SpecksError;
pub use parser::parse_speck;
pub use types::{
    Anchor, BeadsHints, Checkpoint, CheckpointKind, Decision, Question, Speck, SpeckMetadata,
    SpeckStatus, Step, Substep,
};
pub use validator::{
    validate_speck, validate_speck_with_config, Severity, ValidationConfig, ValidationIssue,
    ValidationLevel, ValidationResult,
};
