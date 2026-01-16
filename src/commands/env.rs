use crate::config::loader;
use crate::crypto::decrypt_variables;
use crate::shell::{get_active_environment, is_stand_shell_active};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::env;
use std::path::Path;

/// Options for controlling `stand env` command output format and filtering.
///
/// # Field Interactions
/// - `stand_only` and `user_only` are mutually exclusive (enforced by CLI)
/// - When both are `false`, both Stand markers and user variables are displayed
#[derive(Debug, Clone, Default)]
pub struct EnvOptions {
    /// Output in JSON format instead of plain text
    pub json: bool,
    /// Show only Stand marker variables (STAND_*)
    pub stand_only: bool,
    /// Show only user-defined environment variables
    pub user_only: bool,
}

/// Stand marker environment variable names used to identify and configure
/// Stand subshell sessions.
///
/// Note: These variables are set by the `shell` command when spawning a subshell.
/// If new marker variables are added to the shell spawning logic, they should
/// also be added here to be displayed by `stand env`.
const STAND_MARKER_VARS: &[&str] = &[
    "STAND_ACTIVE",
    "STAND_ENVIRONMENT",
    "STAND_PROJECT_ROOT",
    "STAND_ENV_COLOR",
    "STAND_PROMPT",
];

/// Get Stand marker variables from the current environment
fn get_stand_markers() -> HashMap<String, String> {
    let mut markers = HashMap::new();
    for var_name in STAND_MARKER_VARS {
        if let Ok(value) = env::var(var_name) {
            markers.insert(var_name.to_string(), value);
        }
    }
    markers
}

/// Get user-defined variables for the current environment (with decryption)
fn get_user_variables(project_path: &Path, env_name: &str) -> Result<HashMap<String, String>> {
    let config = loader::load_config_toml_with_inheritance(project_path)?;

    let env = config
        .environments
        .get(env_name)
        .ok_or_else(|| anyhow!("Environment '{}' not found in configuration", env_name))?;

    // Decrypt any encrypted values
    let decrypted = decrypt_variables(env.variables.clone(), project_path)
        .map_err(|e| anyhow!("Failed to decrypt variables: {}", e))?;

    Ok(decrypted)
}

/// Format output as plain text
fn format_plain(
    stand_markers: &HashMap<String, String>,
    user_vars: &HashMap<String, String>,
    options: &EnvOptions,
) -> String {
    let mut output = String::new();

    if !options.user_only && !stand_markers.is_empty() {
        output.push_str("# Stand Environment\n");
        let mut sorted_markers: Vec<_> = stand_markers.iter().collect();
        sorted_markers.sort_by_key(|(k, _)| *k);
        for (key, value) in sorted_markers {
            output.push_str(&format!("{}={}\n", key, value));
        }
    }

    if !options.stand_only && !user_vars.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str("# User Variables\n");
        let mut sorted_vars: Vec<_> = user_vars.iter().collect();
        sorted_vars.sort_by_key(|(k, _)| *k);
        for (key, value) in sorted_vars {
            output.push_str(&format!("{}={}\n", key, value));
        }
    }

    output
}

/// Format output as JSON
fn format_json(
    stand_markers: &HashMap<String, String>,
    user_vars: &HashMap<String, String>,
    options: &EnvOptions,
) -> Result<String> {
    use std::collections::BTreeMap;

    #[derive(serde::Serialize)]
    struct EnvOutput {
        #[serde(skip_serializing_if = "Option::is_none")]
        stand: Option<BTreeMap<String, String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<BTreeMap<String, String>>,
    }

    let stand = if options.user_only {
        None
    } else {
        Some(
            stand_markers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
    };

    let user = if options.stand_only {
        None
    } else {
        Some(
            user_vars
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
    };

    let output = EnvOutput { stand, user };
    Ok(serde_json::to_string_pretty(&output)?)
}

/// Display environment variables for the current Stand subshell session.
///
/// This function retrieves and formats both Stand marker variables (STAND_*)
/// and user-defined variables from the configuration for display.
///
/// # Arguments
///
/// * `project_path` - Path to the project root containing `.stand.toml`
/// * `options` - Output formatting and filtering options
///
/// # Returns
///
/// Formatted string containing environment variables (plain text or JSON)
///
/// # Errors
///
/// - Returns an error if not currently inside a Stand subshell
/// - Returns an error if STAND_ENVIRONMENT is not set (should not happen in valid session)
/// - JSON serialization errors are propagated when using JSON output format
pub fn show_env(project_path: &Path, options: EnvOptions) -> Result<String> {
    // Check if we're inside a Stand subshell
    if !is_stand_shell_active() {
        return Err(anyhow!(
            "Not inside a Stand subshell.\n\
             Use 'stand shell <environment>' to start a subshell first."
        ));
    }

    // Get current environment name
    let env_name = get_active_environment().ok_or_else(|| {
        anyhow!("STAND_ENVIRONMENT is not set. This should not happen inside a Stand subshell.")
    })?;

    // Get Stand markers
    let stand_markers = get_stand_markers();

    // Get user variables from config
    let user_vars = if options.stand_only {
        HashMap::new()
    } else {
        match get_user_variables(project_path, &env_name) {
            Ok(vars) => vars,
            Err(e) => {
                eprintln!("Warning: Could not load user-defined variables: {}", e);
                HashMap::new()
            }
        }
    };

    // Format output
    if options.json {
        format_json(&stand_markers, &user_vars, &options)
    } else {
        Ok(format_plain(&stand_markers, &user_vars, &options))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_get_stand_markers_empty() {
        // When no STAND_* vars are set, should return empty
        env::remove_var("STAND_ACTIVE");
        env::remove_var("STAND_ENVIRONMENT");
        let markers = get_stand_markers();
        assert!(markers.is_empty() || !markers.contains_key("STAND_ACTIVE"));
    }

    #[test]
    #[serial]
    fn test_get_stand_markers_with_vars() {
        env::set_var("STAND_ACTIVE", "1");
        env::set_var("STAND_ENVIRONMENT", "dev");

        let markers = get_stand_markers();

        assert_eq!(markers.get("STAND_ACTIVE"), Some(&"1".to_string()));
        assert_eq!(markers.get("STAND_ENVIRONMENT"), Some(&"dev".to_string()));

        env::remove_var("STAND_ACTIVE");
        env::remove_var("STAND_ENVIRONMENT");
    }

    #[test]
    #[serial]
    fn test_show_env_not_in_subshell() {
        env::remove_var("STAND_ACTIVE");
        env::remove_var("STAND_ENVIRONMENT");

        let dir = tempdir().unwrap();
        let result = show_env(dir.path(), EnvOptions::default());

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Not inside a Stand subshell"));
    }

    #[test]
    #[serial]
    fn test_show_env_stand_active_but_no_environment() {
        // STAND_ACTIVE is set but STAND_ENVIRONMENT is not - abnormal state
        env::set_var("STAND_ACTIVE", "1");
        env::remove_var("STAND_ENVIRONMENT");

        let dir = tempdir().unwrap();
        let result = show_env(dir.path(), EnvOptions::default());

        env::remove_var("STAND_ACTIVE");

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("STAND_ENVIRONMENT is not set"));
    }

    #[test]
    #[serial]
    fn test_show_env_in_subshell() {
        env::set_var("STAND_ACTIVE", "1");
        env::set_var("STAND_ENVIRONMENT", "dev");
        env::set_var("STAND_PROJECT_ROOT", "/test/path");

        let dir = tempdir().unwrap();
        let config_content = r#"
version = "2.0"


[environments.dev]
description = "Development"
DATABASE_URL = "postgres://localhost/dev"
API_KEY = "dev-key"
"#;
        fs::write(dir.path().join(".stand.toml"), config_content).unwrap();

        let result = show_env(dir.path(), EnvOptions::default());

        env::remove_var("STAND_ACTIVE");
        env::remove_var("STAND_ENVIRONMENT");
        env::remove_var("STAND_PROJECT_ROOT");

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("STAND_ACTIVE=1"));
        assert!(output.contains("STAND_ENVIRONMENT=dev"));
        assert!(output.contains("DATABASE_URL=postgres://localhost/dev"));
    }

    #[test]
    #[serial]
    fn test_show_env_stand_only() {
        env::set_var("STAND_ACTIVE", "1");
        env::set_var("STAND_ENVIRONMENT", "dev");

        let dir = tempdir().unwrap();
        let config_content = r#"
version = "2.0"


[environments.dev]
description = "Development"
DATABASE_URL = "postgres://localhost/dev"
"#;
        fs::write(dir.path().join(".stand.toml"), config_content).unwrap();

        let options = EnvOptions {
            stand_only: true,
            ..Default::default()
        };
        let result = show_env(dir.path(), options);

        env::remove_var("STAND_ACTIVE");
        env::remove_var("STAND_ENVIRONMENT");

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("STAND_ACTIVE"));
        assert!(!output.contains("DATABASE_URL"));
    }

    #[test]
    #[serial]
    fn test_show_env_user_only() {
        env::set_var("STAND_ACTIVE", "1");
        env::set_var("STAND_ENVIRONMENT", "dev");

        let dir = tempdir().unwrap();
        let config_content = r#"
version = "2.0"


[environments.dev]
description = "Development"
DATABASE_URL = "postgres://localhost/dev"
"#;
        fs::write(dir.path().join(".stand.toml"), config_content).unwrap();

        let options = EnvOptions {
            user_only: true,
            ..Default::default()
        };
        let result = show_env(dir.path(), options);

        env::remove_var("STAND_ACTIVE");
        env::remove_var("STAND_ENVIRONMENT");

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.contains("STAND_ACTIVE"));
        assert!(output.contains("DATABASE_URL"));
    }

    #[test]
    #[serial]
    fn test_show_env_json_output() {
        env::set_var("STAND_ACTIVE", "1");
        env::set_var("STAND_ENVIRONMENT", "dev");

        let dir = tempdir().unwrap();
        let config_content = r#"
version = "2.0"


[environments.dev]
description = "Development"
API_KEY = "test-key"
"#;
        fs::write(dir.path().join(".stand.toml"), config_content).unwrap();

        let options = EnvOptions {
            json: true,
            ..Default::default()
        };
        let result = show_env(dir.path(), options);

        env::remove_var("STAND_ACTIVE");
        env::remove_var("STAND_ENVIRONMENT");

        assert!(result.is_ok());
        let output = result.unwrap();
        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed.get("stand").is_some());
        assert!(parsed.get("user").is_some());
    }

    #[test]
    fn test_format_plain_output() {
        let mut stand_markers = HashMap::new();
        stand_markers.insert("STAND_ACTIVE".to_string(), "1".to_string());
        stand_markers.insert("STAND_ENVIRONMENT".to_string(), "dev".to_string());

        let mut user_vars = HashMap::new();
        user_vars.insert("API_KEY".to_string(), "secret".to_string());

        let output = format_plain(&stand_markers, &user_vars, &EnvOptions::default());

        assert!(output.contains("# Stand Environment"));
        assert!(output.contains("STAND_ACTIVE=1"));
        assert!(output.contains("# User Variables"));
        assert!(output.contains("API_KEY=secret"));
    }
}
