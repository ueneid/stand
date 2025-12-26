// shell.rs command implementation
//
// Start an interactive subshell with environment variables loaded.

use crate::config::loader;
use crate::config::types::NestedBehavior;
use crate::shell::{
    build_shell_environment, detect_user_shell, get_active_environment, is_stand_shell_active,
    spawn_shell,
};
use anyhow::{anyhow, Result};
use std::io::{self, IsTerminal, Write};
use std::path::Path;

/// Prompt user for confirmation before executing in a protected environment
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

/// Check if nesting is allowed based on configuration
fn check_nesting_allowed(behavior: Option<NestedBehavior>, current_env: &str) -> Result<bool> {
    let behavior = behavior.unwrap_or(NestedBehavior::Prevent);

    match behavior {
        NestedBehavior::Prevent => Err(anyhow!(
            "Already inside a Stand shell (environment: '{}').\n\
             Exit the current shell first, or use 'stand exec' for one-off commands.\n\
             Tip: Set nested_shell_behavior = \"allow\" in settings to permit nesting.",
            current_env
        )),
        NestedBehavior::Warn => {
            eprintln!(
                "Warning: Already inside a Stand shell (environment: '{}').\n\
                 Continuing with nested shell...",
                current_env
            );
            Ok(true)
        }
        NestedBehavior::Allow => Ok(true),
    }
}

/// Result of validating shell environment before spawning
#[derive(Debug)]
pub struct ValidatedShellEnv {
    /// Path to the shell executable
    pub shell_path: String,
    /// Environment variables to inject
    pub env_vars: std::collections::HashMap<String, String>,
    /// Name of the environment
    pub env_name: String,
}

/// Validate and prepare shell environment without spawning
///
/// This function performs all pre-spawn validation:
/// - Loads and validates configuration
/// - Checks for nesting
/// - Validates environment exists
/// - Handles confirmation prompts
///
/// Returns the validated environment ready for spawning, or an error.
pub fn validate_shell_environment(
    project_path: &Path,
    env_name: &str,
    skip_confirmation: bool,
) -> Result<ValidatedShellEnv> {
    // Load configuration with inheritance applied
    let config = loader::load_config_toml_with_inheritance(project_path)?;

    // Check if we're already inside a Stand shell
    if is_stand_shell_active() {
        let current_env = get_active_environment().unwrap_or_else(|| "unknown".to_string());
        check_nesting_allowed(config.settings.nested_shell_behavior, &current_env)?;
    }

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
        // Check if stdin is a terminal - fail fast in non-interactive environments
        if !io::stdin().is_terminal() {
            return Err(anyhow!(
                "Environment '{}' requires confirmation but stdin is not a terminal.\n\
                 Use -y or --yes to skip confirmation in non-interactive environments.",
                env_name
            ));
        }
        // Prompt user for confirmation
        if !prompt_confirmation(env_name)? {
            return Err(anyhow!(
                "Execution cancelled. Use -y or --yes to skip confirmation."
            ));
        }
    }

    // Get user's shell
    let shell_path = detect_user_shell();

    // Build environment with Stand markers
    let project_root = project_path
        .to_str()
        .ok_or_else(|| anyhow!("Invalid project path"))?;
    let mut shell_env = build_shell_environment(env.variables.clone(), env_name, project_root);

    // Add environment color for prompt customization
    if let Some(ref color) = env.color {
        shell_env.insert("STAND_ENV_COLOR".to_string(), color.clone());
    }

    Ok(ValidatedShellEnv {
        shell_path,
        env_vars: shell_env,
        env_name: env_name.to_string(),
    })
}

/// Start an interactive shell with the specified environment
///
/// # Arguments
/// * `project_path` - Path to the project directory containing .stand.toml
/// * `env_name` - Name of the environment to use
/// * `skip_confirmation` - If true, skip confirmation for environments with requires_confirmation=true
pub fn start_shell_with_environment(
    project_path: &Path,
    env_name: &str,
    skip_confirmation: bool,
) -> Result<i32> {
    let validated = validate_shell_environment(project_path, env_name, skip_confirmation)?;

    // Print info message
    eprintln!(
        "Starting shell with environment '{}'. Type 'exit' to return.",
        validated.env_name
    );

    // Spawn the shell
    spawn_shell(&validated.shell_path, validated.env_vars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_check_nesting_allowed_prevent_returns_error() {
        let result = check_nesting_allowed(Some(NestedBehavior::Prevent), "dev");
        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("Already inside a Stand shell"));
        assert!(error_msg.contains("dev"));
    }

    #[test]
    fn test_check_nesting_allowed_allow_returns_ok() {
        let result = check_nesting_allowed(Some(NestedBehavior::Allow), "dev");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_check_nesting_allowed_warn_returns_ok() {
        let result = check_nesting_allowed(Some(NestedBehavior::Warn), "dev");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_check_nesting_allowed_default_is_prevent() {
        let result = check_nesting_allowed(None, "dev");
        assert!(result.is_err());
    }

    // Tests below use validate_shell_environment to avoid spawning actual shells
    // which could hang in CI or non-interactive environments.

    #[test]
    #[serial]
    fn test_shell_nonexistent_environment() {
        // Ensure we're not in a Stand shell
        env::remove_var("STAND_ACTIVE");
        env::remove_var("STAND_ENVIRONMENT");

        let dir = tempdir().unwrap();
        let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[environments.dev]
description = "Development environment"
DATABASE_URL = "postgres://localhost:5432/dev"
"#;

        let config_path = dir.path().join(".stand.toml");
        fs::write(&config_path, config_content).unwrap();

        let result = validate_shell_environment(dir.path(), "nonexistent", false);

        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("Environment 'nonexistent' not found"));
        assert!(error_msg.contains("Available: dev"));
    }

    #[test]
    #[serial]
    fn test_shell_detects_nesting() {
        let dir = tempdir().unwrap();
        let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"
nested_shell_behavior = "prevent"

[environments.dev]
description = "Development environment"
"#;

        let config_path = dir.path().join(".stand.toml");
        fs::write(&config_path, config_content).unwrap();

        // Simulate being inside a Stand shell
        env::set_var("STAND_ACTIVE", "1");
        env::set_var("STAND_ENVIRONMENT", "production");

        let result = validate_shell_environment(dir.path(), "dev", false);

        // Clean up
        env::remove_var("STAND_ACTIVE");
        env::remove_var("STAND_ENVIRONMENT");

        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("Already inside a Stand shell"));
        assert!(error_msg.contains("production"));
    }

    #[test]
    #[serial]
    fn test_shell_allows_nesting_when_configured() {
        let dir = tempdir().unwrap();
        let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"
nested_shell_behavior = "allow"

[environments.dev]
description = "Development environment"
TEST_VAR = "test_value"
"#;

        let config_path = dir.path().join(".stand.toml");
        fs::write(&config_path, config_content).unwrap();

        // Simulate being inside a Stand shell
        env::set_var("STAND_ACTIVE", "1");
        env::set_var("STAND_ENVIRONMENT", "production");

        // Use validate_shell_environment to avoid spawning shell
        let result = validate_shell_environment(dir.path(), "dev", false);

        // Clean up
        env::remove_var("STAND_ACTIVE");
        env::remove_var("STAND_ENVIRONMENT");

        // Should succeed - nesting is allowed
        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated.env_name, "dev");
        assert!(validated.env_vars.contains_key("TEST_VAR"));
        assert!(validated.env_vars.contains_key("STAND_ACTIVE"));
    }

    #[test]
    #[serial]
    fn test_shell_requires_confirmation_non_tty() {
        // Ensure we're not in a Stand shell
        env::remove_var("STAND_ACTIVE");
        env::remove_var("STAND_ENVIRONMENT");

        let dir = tempdir().unwrap();
        let config_content = r#"
version = "2.0"

[settings]
default_environment = "prod"

[environments.prod]
description = "Production environment"
requires_confirmation = true
DATABASE_URL = "postgres://prod:5432/prod"
"#;

        let config_path = dir.path().join(".stand.toml");
        fs::write(&config_path, config_content).unwrap();

        // In test environment, stdin is not a TTY
        let result = validate_shell_environment(dir.path(), "prod", false);

        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("requires confirmation"));
        assert!(error_msg.contains("not a terminal"));
    }

    #[test]
    #[serial]
    fn test_shell_skips_confirmation_with_yes_flag() {
        // Ensure we're not in a Stand shell
        env::remove_var("STAND_ACTIVE");
        env::remove_var("STAND_ENVIRONMENT");

        let dir = tempdir().unwrap();
        let config_content = r#"
version = "2.0"

[settings]
default_environment = "prod"

[environments.prod]
description = "Production environment"
requires_confirmation = true
DATABASE_URL = "postgres://prod:5432/prod"
"#;

        let config_path = dir.path().join(".stand.toml");
        fs::write(&config_path, config_content).unwrap();

        // With skip_confirmation = true, should succeed
        let result = validate_shell_environment(dir.path(), "prod", true);

        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated.env_name, "prod");
    }
}
