// Shell spawner module
//
// Handles spawning interactive shell sessions with environment variables.

use crate::shell::detector::{get_shell_type, ShellType};
use crate::shell::prompt::get_prompt_env_vars;
use anyhow::Result;
use std::collections::HashMap;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

/// Environment variable names used by Stand
pub const STAND_ACTIVE: &str = "STAND_ACTIVE";
pub const STAND_ENVIRONMENT: &str = "STAND_ENVIRONMENT";
pub const STAND_PROJECT_ROOT: &str = "STAND_PROJECT_ROOT";

/// Build the complete environment for a Stand shell session
///
/// Combines:
/// 1. User-defined environment variables from config
/// 2. Stand marker variables (STAND_ACTIVE, STAND_ENVIRONMENT, STAND_PROJECT_ROOT)
/// 3. Prompt customization variables
pub fn build_shell_environment(
    user_env: HashMap<String, String>,
    env_name: &str,
    project_root: &str,
) -> HashMap<String, String> {
    let mut env = user_env;

    // Add Stand marker variables
    env.insert(STAND_ACTIVE.to_string(), "1".to_string());
    env.insert(STAND_ENVIRONMENT.to_string(), env_name.to_string());
    env.insert(STAND_PROJECT_ROOT.to_string(), project_root.to_string());

    // Add prompt customization variables
    let shell_type = get_shell_type();
    let prompt_vars = get_prompt_env_vars(&shell_type, env_name);
    for (key, value) in prompt_vars {
        env.insert(key, value);
    }

    env
}

/// Spawn an interactive shell with the given environment variables
///
/// # Arguments
/// * `shell_path` - Path to the shell executable (e.g., "/bin/bash")
/// * `env_vars` - Environment variables to inject into the shell
///
/// # Returns
/// The exit code of the shell process
pub fn spawn_shell(shell_path: &str, env_vars: HashMap<String, String>) -> Result<i32> {
    let shell_type = ShellType::from_path(shell_path);

    // Build shell arguments based on shell type
    let args = get_shell_args(&shell_type);

    let mut cmd = Command::new(shell_path);
    cmd.args(&args);

    // Add environment variables
    for (key, value) in &env_vars {
        cmd.env(key, value);
    }

    let status = cmd.status()?;

    // Return exit code, handling signal termination on Unix
    match status.code() {
        Some(code) => Ok(code),
        None => {
            #[cfg(unix)]
            {
                if let Some(signal) = status.signal() {
                    return Ok(128 + signal);
                }
            }
            Ok(1)
        }
    }
}

/// Get appropriate shell arguments for interactive mode
fn get_shell_args(shell_type: &ShellType) -> Vec<String> {
    match shell_type {
        ShellType::Fish => {
            // Fish doesn't use -i the same way, it's interactive by default
            vec![]
        }
        _ => {
            // bash, zsh, and others use -i for interactive mode
            vec!["-i".to_string()]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_shell_environment_includes_user_vars() {
        let mut user_env = HashMap::new();
        user_env.insert(
            "DATABASE_URL".to_string(),
            "postgres://localhost".to_string(),
        );
        user_env.insert("API_KEY".to_string(), "secret123".to_string());

        let result = build_shell_environment(user_env, "dev", "/home/user/project");

        assert_eq!(
            result.get("DATABASE_URL"),
            Some(&"postgres://localhost".to_string())
        );
        assert_eq!(result.get("API_KEY"), Some(&"secret123".to_string()));
    }

    #[test]
    fn test_build_shell_environment_includes_stand_markers() {
        let user_env = HashMap::new();
        let result = build_shell_environment(user_env, "production", "/var/www/app");

        assert_eq!(result.get(STAND_ACTIVE), Some(&"1".to_string()));
        assert_eq!(
            result.get(STAND_ENVIRONMENT),
            Some(&"production".to_string())
        );
        assert_eq!(
            result.get(STAND_PROJECT_ROOT),
            Some(&"/var/www/app".to_string())
        );
    }

    #[test]
    fn test_build_shell_environment_stand_markers_override_user_vars() {
        let mut user_env = HashMap::new();
        // User tries to set STAND_ACTIVE (should be overridden)
        user_env.insert(STAND_ACTIVE.to_string(), "0".to_string());

        let result = build_shell_environment(user_env, "dev", "/home/user/project");

        // Stand markers should override user-provided values
        assert_eq!(result.get(STAND_ACTIVE), Some(&"1".to_string()));
    }

    #[test]
    fn test_get_shell_args_bash() {
        let args = get_shell_args(&ShellType::Bash);
        assert_eq!(args, vec!["-i".to_string()]);
    }

    #[test]
    fn test_get_shell_args_zsh() {
        let args = get_shell_args(&ShellType::Zsh);
        assert_eq!(args, vec!["-i".to_string()]);
    }

    #[test]
    fn test_get_shell_args_fish() {
        let args = get_shell_args(&ShellType::Fish);
        assert!(args.is_empty());
    }

    #[test]
    fn test_get_shell_args_other() {
        let args = get_shell_args(&ShellType::Other("sh".to_string()));
        assert_eq!(args, vec!["-i".to_string()]);
    }
}
