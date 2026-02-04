//! Error types for specks operations

use thiserror::Error;

/// Core error type for specks operations
#[derive(Error, Debug)]
pub enum SpecksError {
    /// .specks directory not initialized
    #[error(".specks directory not initialized")]
    NotInitialized,

    /// File not found or unreadable
    #[error("file not found or unreadable: {0}")]
    FileNotFound(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error
    #[error("configuration error: {0}")]
    Config(String),
}
