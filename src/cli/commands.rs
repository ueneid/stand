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
        /// Initialize with encryption enabled (same as `stand encrypt enable`, without Touch ID)
        #[arg(long)]
        encrypt: bool,
    },
    /// Start a subshell with the specified environment
    Shell {
        /// Environment name to activate
        environment: String,
        /// Skip confirmation prompt for environments that require it
        #[arg(short, long)]
        yes: bool,
        /// Shell to use (defaults to $SHELL)
        #[arg(long)]
        shell: Option<String>,
    },
    /// Execute a command with the specified environment
    Exec {
        /// Environment name to use
        environment: String,
        /// Skip confirmation prompt for environments that require it
        #[arg(short, long)]
        yes: bool,
        /// Command to execute
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// List all available environments
    List,
    /// Inspect environment variables defined for an environment
    Inspect {
        /// Environment name
        environment: String,
        /// Show actual values (default: show names only)
        #[arg(short, long)]
        values: bool,
    },
    /// Set a variable in the configuration file
    Set {
        /// Environment name
        environment: String,
        /// Variable name
        key: String,
        /// Variable value (if omitted with --encrypt, prompts for input)
        value: Option<String>,
        /// Encrypt the value before storing
        #[arg(short, long)]
        encrypt: bool,
    },
    /// Remove a variable from the configuration file
    Unset {
        /// Environment name
        environment: String,
        /// Variable name
        key: String,
    },
    /// Get a variable value from the configuration
    Get {
        /// Environment name
        environment: String,
        /// Variable name
        key: String,
    },
    /// Manage encryption settings
    #[command(subcommand)]
    Encrypt(EncryptCommands),
    /// Validate the configuration
    Validate,
    /// Show the current active environment
    Current,
    /// Show environment variables in the current Stand subshell
    Env {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show only Stand marker variables (STAND_*)
        #[arg(long, conflicts_with = "user_only")]
        stand_only: bool,
        /// Show only user-defined variables
        #[arg(long, conflicts_with = "stand_only")]
        user_only: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum EncryptCommands {
    /// Enable encryption for this project (generates a key protected by a passphrase)
    Enable {
        /// Also protect the key with Touch ID (needs `age-plugin-se`)
        #[arg(long)]
        touch_id: bool,
    },
    /// Disable encryption and decrypt all values
    Disable,
    /// Move a legacy `.stand.keys` into the passphrase-protected key store
    Migrate,
    /// Add Touch ID unlock to a project that already uses a passphrase
    AddTouchId,
}
