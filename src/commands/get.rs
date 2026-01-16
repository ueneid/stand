//! Get command implementation.
//!
//! Retrieves a variable value from the configuration, decrypting if necessary.

use std::path::Path;

use crate::config::{loader, ConfigError};
use crate::crypto::{decrypt_value, is_encrypted};

/// Get a variable value from the configuration.
///
/// If the value is encrypted and a private key is available, it will be decrypted.
pub fn get_variable(
    project_dir: &Path,
    environment: &str,
    key: &str,
) -> Result<String, GetCommandError> {
    // Load configuration
    let config = loader::load_config_toml(project_dir)?;

    // Find the environment
    let env = config
        .environments
        .get(environment)
        .ok_or_else(|| GetCommandError::EnvironmentNotFound(environment.to_string()))?;

    // Find the variable
    let value = env
        .variables
        .get(key)
        .ok_or_else(|| GetCommandError::VariableNotFound(key.to_string()))?;

    // Decrypt if encrypted
    if is_encrypted(value) {
        let private_key = load_private_key(project_dir)?;
        let identity = crate::crypto::keys::parse_private_key(&private_key)
            .map_err(|e| GetCommandError::Crypto(e.to_string()))?;
        let decrypted =
            decrypt_value(value, &identity).map_err(|e| GetCommandError::Crypto(e.to_string()))?;
        Ok(decrypted)
    } else {
        Ok(value.clone())
    }
}

/// Load private key from environment variable or file.
fn load_private_key(project_dir: &Path) -> Result<String, GetCommandError> {
    // First try environment variable
    if let Some(key) = crate::crypto::keys::load_private_key_from_env() {
        return Ok(key);
    }

    // Then try .stand.keys file
    let keys_path = project_dir.join(".stand.keys");
    crate::crypto::keys::load_private_key(&keys_path)
        .map_err(|e| GetCommandError::PrivateKeyLoadFailed(e.to_string()))
}

/// Error type for get command.
#[derive(Debug, thiserror::Error)]
pub enum GetCommandError {
    #[error("Environment not found: {0}")]
    EnvironmentNotFound(String),

    #[error("Variable not found: {0}")]
    VariableNotFound(String),

    #[error(
        "Failed to load private key: {0}. Set STAND_PRIVATE_KEY or ensure .stand.keys exists."
    )]
    PrivateKeyLoadFailed(String),

    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_get_variable_plain() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".stand.toml");

        // Note: variables are flattened into the environment section
        fs::write(
            &config_path,
            r#"version = "1.0"

[environments.dev]
description = "Development"
API_URL = "https://api.example.com"
"#,
        )
        .unwrap();

        let result = get_variable(dir.path(), "dev", "API_URL");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://api.example.com");
    }

    #[test]
    fn test_get_variable_not_found() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".stand.toml");

        // Note: variables are flattened into the environment section
        fs::write(
            &config_path,
            r#"version = "1.0"

[environments.dev]
description = "Development"
API_URL = "https://api.example.com"
"#,
        )
        .unwrap();

        let result = get_variable(dir.path(), "dev", "NONEXISTENT");
        assert!(matches!(result, Err(GetCommandError::VariableNotFound(_))));
    }

    #[test]
    fn test_get_variable_env_not_found() {
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

        let result = get_variable(dir.path(), "prod", "API_KEY");
        assert!(matches!(
            result,
            Err(GetCommandError::EnvironmentNotFound(_))
        ));
    }

    #[test]
    fn test_get_variable_encrypted() {
        let dir = tempdir().unwrap();

        // Generate keys and save
        let key_pair = crate::crypto::keys::generate_key_pair();
        let keys_path = dir.path().join(".stand.keys");
        crate::crypto::keys::save_private_key(&keys_path, &key_pair.private_key).unwrap();

        // Encrypt a value
        let recipient = key_pair.to_recipient().unwrap();
        let encrypted = crate::crypto::encrypt_value("secret-api-key", &recipient).unwrap();

        // Write config with encrypted value
        let config_path = dir.path().join(".stand.toml");
        fs::write(
            &config_path,
            format!(
                r#"version = "1.0"

[encryption]
public_key = "{}"

[environments.dev]
description = "Development"
API_KEY = "{}"
"#,
                key_pair.public_key, encrypted
            ),
        )
        .unwrap();

        let result = get_variable(dir.path(), "dev", "API_KEY");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "secret-api-key");
    }
}
