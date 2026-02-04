use clap::Parser;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "specks")]
#[command(version = VERSION)]
#[command(about = "Agent-centric technical specifications CLI")]
#[command(long_about = "Specks is a system for turning ideas into actionable technical specifications via LLM agents.")]
struct Cli {
    /// Increase output verbosity
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Output in JSON format
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
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
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init { force: _ }) => {
            println!("specks init: not yet implemented");
        }
        Some(Commands::Validate { file: _, strict: _ }) => {
            println!("specks validate: not yet implemented");
        }
        Some(Commands::List { status: _ }) => {
            println!("specks list: not yet implemented");
        }
        Some(Commands::Status { file: _ }) => {
            println!("specks status: not yet implemented");
        }
        None => {
            println!("specks v{VERSION}");
            println!("Use --help for usage information");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
