// exec.rs command implementation

use crate::config::loader;
use crate::process::executor::CommandExecutor;
use anyhow::{anyhow, Result};
use std::path::Path;

/// Execute a command with the specified environment
pub fn execute_with_environment(
    project_path: &Path,
    env_name: &str,
    command: Vec<String>,
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
