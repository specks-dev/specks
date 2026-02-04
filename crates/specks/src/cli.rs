//! CLI argument parsing with clap derive

use clap::{Parser, Subcommand};

use crate::commands::BeadsCommands;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Specks - From ideas to implementation via multi-agent orchestration
#[derive(Parser)]
#[command(name = "specks")]
#[command(version = VERSION)]
#[command(about = "From ideas to implementation via multi-agent orchestration")]
#[command(long_about = "Specks transforms ideas into working software through orchestrated LLM agents.\n\nA multi-agent suite (director, planner, critic, architect, implementer, monitor, reviewer, auditor, logger, committer) collaborates to create structured plans and execute them to completion.\n\nThe CLI provides utilities to validate, list, track progress, and integrate with beads for execution tracking.")]
pub struct Cli {
    /// Increase output verbosity
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Output in JSON format
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a specks project in current directory
    ///
    /// Creates .specks/ directory with skeleton template, config, and runs directory.
    /// Run this once in your project root to start using specks.
    #[command(long_about = "Initialize a specks project in current directory.\n\nCreates:\n  .specks/specks-skeleton.md  Template for new specks\n  .specks/config.toml         Project configuration\n  .specks/runs/               Agent run artifacts (gitignored)")]
    Init {
        /// Overwrite existing .specks directory
        #[arg(long)]
        force: bool,
    },

    /// Validate speck structure against format conventions
    ///
    /// Checks anchors, references, metadata, and step dependencies.
    #[command(long_about = "Validate speck structure against format conventions.\n\nChecks:\n  - Required metadata fields (Owner, Status, Last updated)\n  - Anchor format and uniqueness\n  - Reference validity ([D01], #step-0, etc.)\n  - Step dependency cycles\n  - Cross-reference consistency")]
    Validate {
        /// Speck file to validate (validates all if not specified)
        file: Option<String>,

        /// Enable strict validation mode (warnings become errors)
        #[arg(long)]
        strict: bool,
    },

    /// List all specks with summary information
    ///
    /// Shows each speck's name, status, and completion percentage.
    #[command(long_about = "List all specks with summary information.\n\nDisplays:\n  - Speck name (from filename)\n  - Status (draft, active, done)\n  - Progress (completed/total items)\n\nSpecks are found in .specks/ matching the naming pattern.")]
    List {
        /// Filter by status (draft, active, done)
        #[arg(long)]
        status: Option<String>,
    },

    /// Show detailed completion status for a speck
    ///
    /// Displays step-by-step progress with task and checkpoint counts.
    #[command(long_about = "Show detailed completion status for a speck.\n\nDisplays:\n  - Overall progress percentage\n  - Per-step completion (tasks, tests, checkpoints)\n  - Substep progress if present\n\nUse -v/--verbose to see individual task and checkpoint items.")]
    Status {
        /// Speck file to show status for
        file: String,

        /// Show individual task and checkpoint details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Beads integration commands
    ///
    /// Sync steps to beads, link beads, show status, pull completion.
    #[command(subcommand, long_about = "Beads integration for two-way sync between specks and work tracking.\n\nRequires:\n  - Beads CLI (bd) installed and in PATH\n  - Beads initialized (bd init creates .beads/)\n  - Network connectivity\n\nSubcommands:\n  sync   Create beads from speck steps, write IDs back\n  link   Manually link a step to an existing bead\n  status Show execution status (complete/ready/blocked)\n  pull   Update speck checkboxes from bead completion\n\nTypical workflow:\n  1. specks beads sync specks-1.md    # Create beads\n  2. bd close <bead-id>               # Complete work\n  3. specks beads pull specks-1.md    # Update checkboxes")]
    Beads(BeadsCommands),
}

/// Get the command args for use in the application
pub fn parse() -> Cli {
    Cli::parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}
