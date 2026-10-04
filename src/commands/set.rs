//! Set command implementation.
//!
//! Sets a variable in the configuration file, optionally encrypting the value.

use std::fs;
use std::io;
use std::path::Path;

use colored::Colorize;
use toml_edit::DocumentMut;

use crate::config::{loader, ConfigError};
use crate::crypto::{
    decrypt_single_value, encrypt_value, is_encrypted, CryptoError, DecryptPurpose,
};
use crate::environment::safety::{self, VariableRisk};
use crate::shell::{ReflectContext, ShellUpdate};

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
    // Validate the variable name before touching the configuration
    if !safety::is_valid_variable_name(key) {
        return Err(SetCommandError::InvalidVariableName(key.to_string()));
    }
    if safety::is_reserved_key(key) {
        return Err(SetCommandError::ReservedKey(key.to_string()));
    }
    match safety::classify_variable(key) {
        VariableRisk::Blocked => return Err(SetCommandError::BlockedVariable(key.to_string())),
        VariableRisk::Warn => eprintln!(
            "{} {} can change how programs behave or load code; make sure you trust this value",
            "warning:".yellow(),
            key
        ),
        VariableRisk::Safe => {}
    }

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
        warn_if_unpinned(project_dir, &public_key);
        let recipient = crate::crypto::keys::parse_public_key(&public_key)?;
        encrypt_value(&plain_value, &recipient)?
    } else {
        plain_value.clone()
    };

    // Reflection into the enclosing `stand shell`, if any
    let reflect = reflect_context_for(project_dir);
    let before = reflect
        .as_ref()
        .and_then(|ctx| effective_value(project_dir, &ctx.environment, key));

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

    if let Some(ctx) = reflect {
        let written = encrypt.then_some((final_value.as_str(), plain_value.as_str()));
        reflect_change(
            &ctx,
            project_dir,
            key,
            before,
            written,
            format!("stand set {} {}", ctx.environment, key),
        );
    }

    Ok(())
}

/// Warn when no local identity can decrypt what `--encrypt` is about to write.
fn warn_if_unpinned(project_dir: &Path, public_key: &str) {
    if project_dir.join(".stand.keys").exists() {
        return;
    }
    let has_identity = crate::crypto::store::config_dir()
        .map(|dir| {
            crate::crypto::store::identity_file_paths_in(&dir, public_key)
                .iter()
                .any(|p| p.exists())
        })
        .unwrap_or(false);
    if !has_identity {
        eprintln!(
            "{} the project's public key does not match any local identity, so this value may only be decryptable by someone else; check [encryption].public_key in .stand.toml",
            "warning:".yellow()
        );
    }
}

/// The reflect context if this process runs inside a `stand shell` of `project_dir`.
pub(crate) fn reflect_context_for(project_dir: &Path) -> Option<ReflectContext> {
    ReflectContext::from_env().filter(|ctx| ctx.applies_to(project_dir))
}

/// The effective value of `key` in `environment` (inheritance and interpolation applied).
pub(crate) fn effective_value(project_dir: &Path, environment: &str, key: &str) -> Option<String> {
    let config = loader::load_config_toml_with_inheritance(project_dir).ok()?;
    config
        .environments
        .get(environment)?
        .variables
        .get(key)
        .cloned()
}

/// Reflect the change of `key`'s effective value into the enclosing shell.
///
/// `written` is `(ciphertext, plaintext)` of the value this command just wrote,
/// which lets us skip an unlock. Failures only produce warnings: the
/// configuration was already written.
pub(crate) fn reflect_change(
    ctx: &ReflectContext,
    project_dir: &Path,
    key: &str,
    before: Option<String>,
    written: Option<(&str, &str)>,
    command: String,
) {
    let after = effective_value(project_dir, &ctx.environment, key);
    if before == after {
        return;
    }
    let update = match after {
        None => ShellUpdate::Unset {
            key: key.to_string(),
        },
        Some(v) if !is_encrypted(&v) => ShellUpdate::Set {
            key: key.to_string(),
            value: v,
        },
        Some(v) => match written {
            Some((cipher, plain)) if cipher == v => ShellUpdate::Set {
                key: key.to_string(),
                value: plain.to_string(),
            },
            _ => {
                let purpose = DecryptPurpose {
                    command,
                    environment: Some(ctx.environment.clone()),
                    secret_count: 1,
                };
                match decrypt_single_value(&v, project_dir, &purpose) {
                    Ok(plain) => ShellUpdate::Set {
                        key: key.to_string(),
                        value: plain,
                    },
                    Err(_) => {
                        eprintln!(
                            "{} the inherited encrypted value of {} could not be applied to this shell; restart the stand shell to load it",
                            "warning:".yellow(),
                            key
                        );
                        ShellUpdate::Unset {
                            key: key.to_string(),
                        }
                    }
                }
            }
        },
    };
    if let Err(e) = ctx.emit(&[update]) {
        eprintln!(
            "{} could not update the current shell: {}",
            "warning:".yellow(),
            e
        );
    }
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
    let doc: DocumentMut = content
        .parse()
        .map_err(|e: toml_edit::TomlError| SetCommandError::TomlParse(e.to_string()))?;

    doc.get("encryption")
        .and_then(|e| e.get("public_key"))
        .and_then(|k| k.as_str())
        .map(|s| s.to_string())
        .ok_or(SetCommandError::EncryptionNotEnabled)
}

/// Update a variable in the TOML file.
///
/// Uses toml_edit to preserve comments and formatting.
/// Variables are stored directly in the environment section due to `#[serde(flatten)]`.
fn update_toml_variable(
    config_path: &Path,
    environment: &str,
    key: &str,
    value: &str,
) -> Result<(), SetCommandError> {
    let content = fs::read_to_string(config_path)?;

    // Parse with toml_edit to preserve formatting
    let mut doc: DocumentMut = content
        .parse()
        .map_err(|e: toml_edit::TomlError| SetCommandError::TomlParse(e.to_string()))?;

    // Navigate to environments.<env>
    let env_table = doc
        .get_mut("environments")
        .and_then(|e| e.get_mut(environment))
        .and_then(|e| e.as_table_mut())
        .ok_or_else(|| SetCommandError::EnvironmentNotFound(environment.to_string()))?;

    // Set the variable directly in the environment section (due to #[serde(flatten)])
    env_table.insert(key, toml_edit::value(value));

    // Write back preserving formatting
    fs::write(config_path, doc.to_string())?;

    Ok(())
}

/// Error type for set command.
#[derive(Debug, thiserror::Error)]
pub enum SetCommandError {
    #[error("Environment not found: {0}")]
    EnvironmentNotFound(String),

    #[error("Invalid variable name: {0} (use letters, digits and underscores; do not start with a digit)")]
    InvalidVariableName(String),

    #[error(
        "'{0}' is an environment setting, not a variable. Edit .stand.toml directly to change it."
    )]
    ReservedKey(String),

    #[error("'{0}' is blocked: it can inject code into the shell that loads these variables")]
    BlockedVariable(String),

    #[error("Value is required when not encrypting")]
    ValueRequired,

    #[error("Encryption is not enabled. Run 'stand encrypt enable' first.")]
    EncryptionNotEnabled,

    #[error("Cryptographic error: {0}")]
    Crypto(#[from] CryptoError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("TOML parsing error: {0}")]
    TomlParse(String),

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

    fn write_basic(dir: &Path) {
        fs::write(
            dir.join(".stand.toml"),
            "version = \"1.0\"\n\n[environments.dev]\ndescription = \"Development\"\n",
        )
        .unwrap();
    }

    #[test]
    fn test_set_reserved_key_rejected() {
        let dir = tempdir().unwrap();
        write_basic(dir.path());
        let r = set_variable(dir.path(), "dev", "description", Some("x".into()), false);
        assert!(matches!(r, Err(SetCommandError::ReservedKey(_))));
    }

    #[test]
    fn test_set_blocked_variables_rejected() {
        let dir = tempdir().unwrap();
        write_basic(dir.path());
        for k in ["BASH_ENV", "LD_PRELOAD"] {
            let r = set_variable(dir.path(), "dev", k, Some("x".into()), false);
            assert!(matches!(r, Err(SetCommandError::BlockedVariable(_))), "{k}");
        }
        let content = fs::read_to_string(dir.path().join(".stand.toml")).unwrap();
        assert!(!content.contains("BASH_ENV"));
    }

    #[test]
    fn test_set_warned_variable_allowed() {
        let dir = tempdir().unwrap();
        write_basic(dir.path());
        let r = set_variable(dir.path(), "dev", "PATH", Some("/bin".into()), false);
        assert!(r.is_ok());
    }

    #[test]
    fn test_set_invalid_variable_name_rejected() {
        let dir = tempdir().unwrap();
        write_basic(dir.path());
        for k in ["1BAD", "A-B"] {
            let r = set_variable(dir.path(), "dev", k, Some("x".into()), false);
            assert!(
                matches!(r, Err(SetCommandError::InvalidVariableName(_))),
                "{k}"
            );
        }
    }
}
