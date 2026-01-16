//! Set command implementation.
//!
//! Sets a variable in the configuration file, optionally encrypting the value.

use std::fs;
use std::io;
use std::path::Path;

use colored::Colorize;

use crate::config::{loader, ConfigError};
use crate::crypto::encrypt_value;

/// Set a variable in the configuration file.
///
/// If `encrypt` is true, the value will be encrypted before storing.
/// If `value` is None and `encrypt` is true, prompts for password input.
pub fn set_variable(
    project_dir: &Path,
    environment: &str,
    key: &str,
    value: Option<String>,
    encrypt: bool,
) -> Result<(), SetCommandError> {
    // Load configuration
    let config_path = project_dir.join(".stand.toml");
    let config = loader::load_config_toml(project_dir)?;

    // Verify environment exists
    if !config.environments.contains_key(environment) {
        return Err(SetCommandError::EnvironmentNotFound(
            environment.to_string(),
        ));
    }

    // Get the value (prompt if not provided and encrypting)
    let plain_value = match value {
        Some(v) => v,
        None if encrypt => prompt_for_secret(key)?,
        None => return Err(SetCommandError::ValueRequired),
    };

    // Encrypt if requested
    let final_value = if encrypt {
        // Check if encryption is enabled
        let public_key = get_public_key(&config_path)?;
        let recipient = crate::crypto::keys::parse_public_key(&public_key)
            .map_err(|e| SetCommandError::Crypto(e.to_string()))?;
        encrypt_value(&plain_value, &recipient)
            .map_err(|e| SetCommandError::Crypto(e.to_string()))?
    } else {
        plain_value
    };

    // Update the TOML file
    update_toml_variable(&config_path, environment, key, &final_value)?;

    if encrypt {
        println!(
            "{} Set {} in [environments.{}] (encrypted)",
            "✓".green(),
            key,
            environment
        );
    } else {
        println!(
            "{} Set {} in [environments.{}]",
            "✓".green(),
            key,
            environment
        );
    }

    Ok(())
}

/// Prompts for a secret value without echoing to the terminal.
///
/// Uses rpassword to suppress input echo, preventing sensitive values
/// from being visible on screen.
fn prompt_for_secret(key: &str) -> Result<String, SetCommandError> {
    let prompt = format!("Enter value for {}: ", key);
    rpassword::prompt_password(prompt).map_err(SetCommandError::Io)
}

/// Get the public key from the configuration.
fn get_public_key(config_path: &Path) -> Result<String, SetCommandError> {
    let content = fs::read_to_string(config_path)?;

    // Parse TOML to find public_key
    let toml_value: toml::Value =
        toml::from_str(&content).map_err(|e| SetCommandError::TomlParse(e.to_string()))?;

    toml_value
        .get("encryption")
        .and_then(|e| e.get("public_key"))
        .and_then(|k| k.as_str())
        .map(|s| s.to_string())
        .ok_or(SetCommandError::EncryptionNotEnabled)
}

/// Update a variable in the TOML file.
///
/// Variables are stored directly in the environment section due to `#[serde(flatten)]`.
fn update_toml_variable(
    config_path: &Path,
    environment: &str,
    key: &str,
    value: &str,
) -> Result<(), SetCommandError> {
    let content = fs::read_to_string(config_path)?;

    // Parse and modify TOML
    let mut doc: toml::Value =
        toml::from_str(&content).map_err(|e| SetCommandError::TomlParse(e.to_string()))?;

    // Navigate to environments.<env>
    let env = doc
        .get_mut("environments")
        .and_then(|e| e.get_mut(environment))
        .ok_or_else(|| SetCommandError::EnvironmentNotFound(environment.to_string()))?;

    // Set the variable directly in the environment section (due to #[serde(flatten)])
    env.as_table_mut()
        .ok_or_else(|| SetCommandError::TomlParse("Environment is not a table".to_string()))?
        .insert(key.to_string(), toml::Value::String(value.to_string()));

    // Write back to file
    let new_content =
        toml::to_string_pretty(&doc).map_err(|e| SetCommandError::TomlSerialize(e.to_string()))?;
    fs::write(config_path, new_content)?;

    Ok(())
}

/// Error type for set command.
#[derive(Debug, thiserror::Error)]
pub enum SetCommandError {
    #[error("Environment not found: {0}")]
    EnvironmentNotFound(String),

    #[error("Value is required when not encrypting")]
    ValueRequired,

    #[error("Encryption is not enabled. Run 'stand encrypt enable' first.")]
    EncryptionNotEnabled,

    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("TOML parsing error: {0}")]
    TomlParse(String),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_set_variable_plain() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".stand.toml");

        fs::write(
            &config_path,
            r#"version = "1.0"

[environments.dev]
description = "Development"
"#,
        )
        .unwrap();

        let result = set_variable(
            dir.path(),
            "dev",
            "API_URL",
            Some("https://api.example.com".to_string()),
            false,
        );
        assert!(result.is_ok());

        // Verify the variable was set
        let updated_content = fs::read_to_string(&config_path).unwrap();
        assert!(updated_content.contains("API_URL"));
        assert!(updated_content.contains("https://api.example.com"));
    }

    #[test]
    fn test_set_variable_env_not_found() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".stand.toml");

        fs::write(
            &config_path,
            r#"version = "1.0"

[environments.dev]
description = "Development"
"#,
        )
        .unwrap();

        let result = set_variable(
            dir.path(),
            "prod",
            "API_KEY",
            Some("secret".to_string()),
            false,
        );
        assert!(matches!(
            result,
            Err(SetCommandError::EnvironmentNotFound(_))
        ));
    }

    #[test]
    fn test_set_variable_encrypt_no_key() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".stand.toml");

        fs::write(
            &config_path,
            r#"version = "1.0"

[environments.dev]
description = "Development"
"#,
        )
        .unwrap();

        let result = set_variable(
            dir.path(),
            "dev",
            "API_KEY",
            Some("secret".to_string()),
            true,
        );
        assert!(matches!(result, Err(SetCommandError::EncryptionNotEnabled)));
    }

    #[test]
    fn test_set_variable_encrypted_success() {
        let dir = tempdir().unwrap();
        let key_pair = crate::crypto::keys::generate_key_pair();

        let config_path = dir.path().join(".stand.toml");
        fs::write(
            &config_path,
            format!(
                r#"version = "1.0"

[encryption]
public_key = "{}"

[environments.dev]
description = "Development"
"#,
                key_pair.public_key
            ),
        )
        .unwrap();

        let result = set_variable(
            dir.path(),
            "dev",
            "API_KEY",
            Some("secret-value".to_string()),
            true,
        );
        assert!(result.is_ok());

        // Verify the value was encrypted
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(
            content.contains("encrypted:"),
            "Value should be encrypted in config file"
        );
        // The plain value should NOT appear in the file
        assert!(
            !content.contains("secret-value"),
            "Plain value should not appear in config file"
        );
    }
}
