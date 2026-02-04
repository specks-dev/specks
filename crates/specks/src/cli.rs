//! CLI argument parsing with clap derive

use clap::{Parser, Subcommand};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Specks - Agent-centric technical specifications CLI
#[derive(Parser)]
#[command(name = "specks")]
#[command(version = VERSION)]
#[command(about = "Agent-centric technical specifications CLI")]
#[command(long_about = "Specks is a system for turning ideas into actionable technical specifications via LLM agents.\n\nThe specks-author agent creates comprehensive specifications following a defined format.\nThe CLI provides utilities to validate, list, and track completion of specks.")]
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
    Init {
        /// Overwrite existing .specks directory
        #[arg(long)]
        force: bool,
    },

    /// Validate speck structure against format conventions
    Validate {
        /// Speck file to validate (validates all if not specified)
        file: Option<String>,

        /// Enable strict validation mode
        #[arg(long)]
        strict: bool,
    },

    /// List all specks with summary information
    List {
        /// Filter by status (draft, active, done)
        #[arg(long)]
        status: Option<String>,
    },

    /// Show detailed completion status for a speck
    Status {
        /// Speck file to show status for
        file: String,

        /// Show individual task and checkpoint details
        #[arg(short, long)]
        verbose: bool,
    },
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
