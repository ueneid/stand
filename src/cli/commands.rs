use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "stand")]
#[command(about = "A CLI tool for explicit environment variable management")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize Stand in the current directory
    Init {
        /// Force initialization even if Stand is already initialized
        #[arg(short, long)]
        force: bool,
    },
    /// Start a subshell with the specified environment
    Shell {
        /// Environment name to activate
        environment: String,
    },
    /// Execute a command with the specified environment
    Exec {
        /// Environment name to use
        environment: String,
        /// Command to execute
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// List all available environments
    List,
    /// Show environment variables for an environment
    Show {
        /// Environment name
        environment: String,
        /// Show actual values instead of hiding them
        #[arg(short, long)]
        values: bool,
    },
    /// Switch the default environment
    Switch {
        /// Environment name to set as default
        environment: String,
    },
    /// Set a session variable
    Set {
        /// Variable name
        name: String,
        /// Variable value
        value: String,
    },
    /// Unset a variable
    Unset {
        /// Variable name
        name: String,
    },
    /// Validate the configuration
    Validate,
    /// Show the current active environment
    Current,
}
