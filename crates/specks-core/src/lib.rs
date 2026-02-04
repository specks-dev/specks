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
