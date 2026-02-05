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

/// Beads integration utilities
pub mod beads;

// Re-exports for convenience
pub use beads::{BeadStatus, BeadsCli, Issue, IssueDetails, is_valid_bead_id};
pub use config::{
    BeadsConfig, Config, NamingConfig, RESERVED_FILES, SpecksConfig, find_project_root,
    find_project_root_from, find_specks, is_reserved_file, speck_name_from_path,
};
pub use error::SpecksError;
pub use parser::parse_speck;
pub use types::{
    Anchor, BeadsHints, Checkpoint, CheckpointKind, Decision, Question, Speck, SpeckMetadata,
    SpeckStatus, Step, Substep,
};
pub use validator::{
    Severity, ValidationConfig, ValidationIssue, ValidationLevel, ValidationResult, validate_speck,
    validate_speck_with_config,
};
