// exec.rs command implementation

use crate::config::loader;
use crate::process::executor::CommandExecutor;
use anyhow::{anyhow, Result};
use std::path::Path;

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
        return Err(anyhow!(
            "Environment '{}' requires confirmation. Use -y or --yes to proceed.",
            env_name
        ));
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
