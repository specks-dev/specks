//! CLI interaction module
//!
//! This module provides the `CliAdapter` implementation of the `InteractionAdapter` trait
//! for terminal-based user interaction using the inquire crate.

mod cli_adapter;

// Note: These are temporarily unused after planning_loop removal in Step 8.1
// They will be removed in Step 8.2 when the interaction module is deleted
#[allow(unused_imports)]
pub use cli_adapter::{CliAdapter, reset_cancellation};
