// exec.rs command implementation

use crate::config::loader;
use crate::process::executor::CommandExecutor;
use anyhow::{anyhow, Result};
use std::io::{self, Write};
use std::path::Path;

/// Prompt user for confirmation before executing in a protected environment
///
/// Returns true if the user confirms, false otherwise
fn prompt_confirmation(env_name: &str) -> Result<bool> {
    print!(
        "Environment '{}' requires confirmation.\nAre you sure you want to proceed? [y/N]: ",
        env_name
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let response = input.trim().to_lowercase();
    Ok(response == "y" || response == "yes")
}

/// Execute a command with the specified environment
///
/// # Arguments
/// * `project_path` - Path to the project directory containing .stand.toml
/// * `env_name` - Name of the environment to use
/// * `command` - Command and arguments to execute
/// * `skip_confirmation` - If true, skip confirmation for environments with requires_confirmation=true
pub fn execute_with_environment(
    project_path: &Path,
    env_name: &str,
    command: Vec<String>,
    skip_confirmation: bool,
) -> Result<i32> {
    // Load configuration with inheritance applied
    let config = loader::load_config_toml_with_inheritance(project_path)?;

    // Check if environment exists
    let env = config.environments.get(env_name).ok_or_else(|| {
        let mut available: Vec<_> = config.environments.keys().cloned().collect();
        available.sort();
        anyhow!(
            "Environment '{}' not found. Available: {}",
            env_name,
            available.join(", ")
        )
    })?;

    // Check if confirmation is required
    if env.requires_confirmation.unwrap_or(false) && !skip_confirmation {
        // Prompt user for confirmation
        if !prompt_confirmation(env_name)? {
            return Err(anyhow!(
                "Execution cancelled. Use -y or --yes to skip confirmation."
            ));
        }
    }

    // Validate command is not empty
    if command.is_empty() {
        return Err(anyhow!("Command cannot be empty"));
    }

    // Split command into program and arguments
    let program = command[0].clone();
    let args = command[1..].to_vec();

    // Execute command with environment variables
    let executor = CommandExecutor::new(program, args).with_env(env.variables.clone());

    executor.execute()
}
