//! CLI interaction module
//!
//! This module provides the `CliAdapter` implementation of the `InteractionAdapter` trait
//! for terminal-based user interaction using the inquire crate.

mod cli_adapter;

pub use cli_adapter::{CliAdapter, reset_cancellation};
