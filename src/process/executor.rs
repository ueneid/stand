// executor.rs module

use anyhow::Result;
use std::collections::HashMap;
use std::process::Command;

/// Executes commands with environment variables
pub struct CommandExecutor {
    command: String,
    args: Vec<String>,
    env_vars: HashMap<String, String>,
}

impl CommandExecutor {
    /// Create a new CommandExecutor with command and arguments
    pub fn new(command: String, args: Vec<String>) -> Self {
        Self {
            command,
            args,
            env_vars: HashMap::new(),
        }
    }

    /// Add environment variables to the executor
    pub fn with_env(mut self, env_vars: HashMap<String, String>) -> Self {
        self.env_vars = env_vars;
        self
    }

    /// Execute the command and return the exit code
    pub fn execute(self) -> Result<i32> {
        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args);

        // Add environment variables
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        let status = cmd.status()?;
        Ok(status.code().unwrap_or(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_simple_command() {
        // Test executing a simple echo command
        let executor = CommandExecutor::new("echo".to_string(), vec!["hello".to_string()]);
        let exit_code = executor.execute().unwrap();

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_command_success_exit_code() {
        // Test successful command returns exit code 0
        let executor = CommandExecutor::new(
            "sh".to_string(),
            vec!["-c".to_string(), "exit 0".to_string()],
        );
        let exit_code = executor.execute().unwrap();

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_command_failure_exit_code() {
        // Test failed command returns non-zero exit code
        let executor = CommandExecutor::new(
            "sh".to_string(),
            vec!["-c".to_string(), "exit 42".to_string()],
        );
        let exit_code = executor.execute().unwrap();

        assert_eq!(exit_code, 42);
    }

    #[test]
    fn test_execute_with_environment_variables() {
        // Test that environment variables are injected correctly
        let mut env_vars = HashMap::new();
        env_vars.insert("TEST_VAR".to_string(), "test_value".to_string());
        env_vars.insert("ANOTHER_VAR".to_string(), "another_value".to_string());

        let executor = CommandExecutor::new(
            "sh".to_string(),
            vec![
                "-c".to_string(),
                "test \"$TEST_VAR\" = \"test_value\" && test \"$ANOTHER_VAR\" = \"another_value\""
                    .to_string(),
            ],
        )
        .with_env(env_vars);

        let exit_code = executor.execute().unwrap();
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_with_multiple_arguments() {
        // Test command with multiple arguments
        let executor = CommandExecutor::new(
            "echo".to_string(),
            vec!["arg1".to_string(), "arg2".to_string(), "arg3".to_string()],
        );

        let exit_code = executor.execute().unwrap();
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_with_no_arguments() {
        // Test command with no arguments
        let executor = CommandExecutor::new("true".to_string(), vec![]);

        let exit_code = executor.execute().unwrap();
        assert_eq!(exit_code, 0);
    }
}
