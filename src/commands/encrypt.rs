//! Encryption management commands.

use std::fs;
use std::io::Write;
use std::path::Path;

use colored::Colorize;
use toml_edit::{DocumentMut, Item, Value};

use crate::crypto::{generate_key_pair, CryptoError, ENCRYPTED_PREFIX};

const KEYS_FILE: &str = ".stand.keys";
const CONFIG_FILE: &str = ".stand.toml";

/// Enable encryption for the project.
///
/// Generates a new key pair and adds the public key to .stand.toml.
pub fn enable_encryption(project_dir: &Path) -> Result<(), EncryptionCommandError> {
    let config_path = project_dir.join(CONFIG_FILE);
    let keys_path = project_dir.join(KEYS_FILE);

    // Check if config file exists
    if !config_path.exists() {
        return Err(EncryptionCommandError::ConfigNotFound);
    }

    // Parse config with toml_edit to preserve formatting
    let config_content = fs::read_to_string(&config_path)?;
    let mut doc: DocumentMut = config_content
        .parse()
        .map_err(|e| EncryptionCommandError::TomlParse(format!("{}", e)))?;

    // Check if encryption is already enabled
    if doc.get("encryption").is_some() {
        return Err(EncryptionCommandError::AlreadyEnabled);
    }

    // Generate key pair
    let key_pair = generate_key_pair();

    // Add [encryption] section to config using toml_edit
    let mut encryption_table = toml_edit::Table::new();
    encryption_table.insert("public_key", toml_edit::value(&key_pair.public_key));
    doc.insert("encryption", Item::Table(encryption_table));

    // Write back preserving formatting
    fs::write(&config_path, doc.to_string())?;

    // Save private key to .stand.keys
    crate::crypto::keys::save_private_key(&keys_path, &key_pair.private_key)
        .map_err(EncryptionCommandError::Crypto)?;

    // Add .stand.keys to .gitignore if not already present
    add_to_gitignore(project_dir, KEYS_FILE)?;

    println!("{} Generated key pair", "✓".green());
    println!(
        "{} Added [encryption] section to {}",
        "✓".green(),
        CONFIG_FILE
    );
    println!("{} Created {}", "✓".green(), KEYS_FILE);

    Ok(())
}

/// Disable encryption for the project.
///
/// Prompts for user confirmation, then decrypts all encrypted values
/// and removes encryption configuration. If the user declines, returns Ok(())
/// without making changes.
pub fn disable_encryption(project_dir: &Path) -> Result<(), EncryptionCommandError> {
    let config_path = project_dir.join(CONFIG_FILE);

    // Check if config file exists
    if !config_path.exists() {
        return Err(EncryptionCommandError::ConfigNotFound);
    }

    // Parse config with toml_edit to check encryption status
    let config_content = fs::read_to_string(&config_path)?;
    let doc: DocumentMut = config_content
        .parse()
        .map_err(|e| EncryptionCommandError::TomlParse(format!("{}", e)))?;

    // Check if encryption is enabled
    if doc.get("encryption").is_none() {
        return Err(EncryptionCommandError::NotEnabled);
    }

    // Prompt for confirmation
    println!(
        "{} This will decrypt all encrypted values and remove encryption.",
        "⚠".yellow()
    );
    print!("Continue? [y/N]: ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Aborted.");
        return Ok(());
    }

    // Perform the actual disable operation
    let result = disable_encryption_internal(project_dir)?;

    if result.decrypted_count > 0 {
        println!(
            "{} Decrypted {} value(s)",
            "✓".green(),
            result.decrypted_count
        );
    }
    println!("{} Removed [encryption] section", "✓".green());
    println!("{} Encryption disabled", "✓".green());

    Ok(())
}

/// Result of the disable_encryption_internal operation.
#[derive(Debug, Default)]
pub struct DisableEncryptionResult {
    /// Number of values successfully decrypted.
    pub decrypted_count: usize,
}

/// Internal function to disable encryption without user confirmation.
///
/// This function is separated to allow testing without interactive prompts.
pub fn disable_encryption_internal(
    project_dir: &Path,
) -> Result<DisableEncryptionResult, EncryptionCommandError> {
    let config_path = project_dir.join(CONFIG_FILE);
    let keys_path = project_dir.join(KEYS_FILE);

    // Parse config with toml_edit
    let config_content = fs::read_to_string(&config_path)?;
    let mut doc: DocumentMut = config_content
        .parse()
        .map_err(|e| EncryptionCommandError::TomlParse(format!("{}", e)))?;

    // Check if encryption is enabled
    if doc.get("encryption").is_none() {
        return Err(EncryptionCommandError::NotEnabled);
    }

    // Load private key
    let private_key = load_private_key_for_decryption(project_dir)?;
    let identity = crate::crypto::keys::parse_private_key(&private_key)
        .map_err(EncryptionCommandError::Crypto)?;

    let mut result = DisableEncryptionResult::default();

    // Find and decrypt all encrypted values in environments section
    if let Some(environments) = doc.get_mut("environments") {
        if let Some(env_table) = environments.as_table_mut() {
            for (_env_name, env_config) in env_table.iter_mut() {
                if let Some(env_tbl) = env_config.as_table_mut() {
                    for (key, value) in env_tbl.iter_mut() {
                        if let Some(val_str) = value.as_str() {
                            if val_str.starts_with(ENCRYPTED_PREFIX) {
                                let decrypted = crate::crypto::decrypt_value(val_str, &identity)
                                    .map_err(|e| EncryptionCommandError::DecryptionFailed {
                                        variable: key.to_string(),
                                        reason: e.to_string(),
                                    })?;
                                *value = Item::Value(Value::from(decrypted));
                                result.decrypted_count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    // Also check [common] section for encrypted values
    if let Some(common) = doc.get_mut("common") {
        if let Some(common_table) = common.as_table_mut() {
            for (key, value) in common_table.iter_mut() {
                if let Some(val_str) = value.as_str() {
                    if val_str.starts_with(ENCRYPTED_PREFIX) {
                        let decrypted =
                            crate::crypto::decrypt_value(val_str, &identity).map_err(|e| {
                                EncryptionCommandError::DecryptionFailed {
                                    variable: key.to_string(),
                                    reason: e.to_string(),
                                }
                            })?;
                        *value = Item::Value(Value::from(decrypted));
                        result.decrypted_count += 1;
                    }
                }
            }
        }
    }

    // Remove [encryption] section using toml_edit
    doc.remove("encryption");

    // Write back preserving formatting
    fs::write(&config_path, doc.to_string())?;

    // Remove .stand.keys file if it exists
    if keys_path.exists() {
        fs::remove_file(&keys_path)?;
    }

    Ok(result)
}

/// Adds a file to .gitignore if not already present.
fn add_to_gitignore(project_dir: &Path, filename: &str) -> Result<(), std::io::Error> {
    let gitignore_path = project_dir.join(".gitignore");

    if gitignore_path.exists() {
        let content = fs::read_to_string(&gitignore_path)?;
        if content.lines().any(|line| line.trim() == filename) {
            return Ok(()); // Already in .gitignore
        }
        // Append to existing .gitignore
        let mut file = fs::OpenOptions::new().append(true).open(&gitignore_path)?;
        std::io::Write::write_all(&mut file, format!("\n{}\n", filename).as_bytes())?;
    } else {
        // Create new .gitignore
        fs::write(&gitignore_path, format!("{}\n", filename))?;
    }

    println!("{} Added {} to .gitignore", "✓".green(), filename);
    Ok(())
}

/// Load private key from file or environment variable.
fn load_private_key_for_decryption(project_dir: &Path) -> Result<String, EncryptionCommandError> {
    // First try environment variable (may error on invalid UTF-8)
    match crate::crypto::keys::load_private_key_from_env() {
        Ok(Some(key)) => return Ok(key),
        Ok(None) => {} // Not set, try file
        Err(e) => return Err(EncryptionCommandError::Crypto(e)),
    }

    // Then try .stand.keys file
    let keys_path = project_dir.join(KEYS_FILE);
    crate::crypto::keys::load_private_key(&keys_path)
        .map_err(|e| EncryptionCommandError::PrivateKeyLoadFailed(e.to_string()))
}

/// Error type for encryption commands.
#[derive(Debug, thiserror::Error)]
pub enum EncryptionCommandError {
    #[error("Configuration file not found. Run 'stand init' first.")]
    ConfigNotFound,

    #[error("Encryption is already enabled for this project")]
    AlreadyEnabled,

    #[error("Encryption is not enabled for this project")]
    NotEnabled,

    #[error(
        "Failed to load private key: {0}. Set STAND_PRIVATE_KEY or ensure .stand.keys exists."
    )]
    PrivateKeyLoadFailed(String),

    #[error("Cryptographic error: {0}")]
    Crypto(#[from] CryptoError),

    #[error("TOML parsing error: {0}")]
    TomlParse(String),

    #[error("Failed to decrypt variable '{variable}': {reason}. All values must be decryptable to disable encryption.")]
    DecryptionFailed { variable: String, reason: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_enable_encryption_no_config() {
        let dir = tempdir().unwrap();
        let result = enable_encryption(dir.path());
        assert!(matches!(
            result,
            Err(EncryptionCommandError::ConfigNotFound)
        ));
    }

    #[test]
    fn test_enable_encryption_success() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".stand.toml");

        // Create minimal config
        fs::write(
            &config_path,
            r#"version = "1.0"

[environments.dev]
description = "Development"
"#,
        )
        .unwrap();

        let result = enable_encryption(dir.path());
        assert!(result.is_ok());

        // Check that [encryption] section was added
        let updated_config = fs::read_to_string(&config_path).unwrap();
        assert!(updated_config.contains("[encryption]"));
        assert!(updated_config.contains("public_key = \"age1"));

        // Check that .stand.keys was created
        let keys_path = dir.path().join(".stand.keys");
        assert!(keys_path.exists());
    }

    #[test]
    fn test_enable_encryption_already_enabled() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".stand.toml");

        // Create config with encryption already enabled
        fs::write(
            &config_path,
            r#"version = "1.0"

[encryption]
public_key = "age1test"

[environments.dev]
description = "Development"
"#,
        )
        .unwrap();

        let result = enable_encryption(dir.path());
        assert!(matches!(
            result,
            Err(EncryptionCommandError::AlreadyEnabled)
        ));
    }

    // === Issue 2: Tests for disable_encryption_internal ===

    #[test]
    fn test_disable_encryption_internal_decrypts_all_values() {
        let dir = tempdir().unwrap();

        // Generate keys
        let key_pair = crate::crypto::keys::generate_key_pair();
        let keys_path = dir.path().join(".stand.keys");
        crate::crypto::keys::save_private_key(&keys_path, &key_pair.private_key).unwrap();

        // Encrypt test values
        let recipient = key_pair.to_recipient().unwrap();
        let encrypted1 = crate::crypto::encrypt_value("secret1", &recipient).unwrap();
        let encrypted2 = crate::crypto::encrypt_value("secret2", &recipient).unwrap();

        // Create config with encrypted values
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
DB_PASSWORD = "{}"
"#,
                key_pair.public_key, encrypted1, encrypted2
            ),
        )
        .unwrap();

        // Disable encryption
        let result = disable_encryption_internal(dir.path());
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.decrypted_count, 2);

        // Verify the config was updated
        let updated_config = fs::read_to_string(&config_path).unwrap();
        assert!(!updated_config.contains("[encryption]"));
        assert!(!updated_config.contains("encrypted:"));
        assert!(updated_config.contains("API_KEY = \"secret1\""));
        assert!(updated_config.contains("DB_PASSWORD = \"secret2\""));

        // Verify .stand.keys was removed
        assert!(!keys_path.exists());
    }

    #[test]
    fn test_disable_encryption_internal_removes_encryption_section() {
        let dir = tempdir().unwrap();

        // Generate keys (no encrypted values, just testing section removal)
        let key_pair = crate::crypto::keys::generate_key_pair();
        let keys_path = dir.path().join(".stand.keys");
        crate::crypto::keys::save_private_key(&keys_path, &key_pair.private_key).unwrap();

        // Create config with encryption section but no encrypted values
        let config_path = dir.path().join(".stand.toml");
        fs::write(
            &config_path,
            format!(
                r#"version = "1.0"

[encryption]
public_key = "{}"

[environments.dev]
description = "Development"
PLAIN_VALUE = "not encrypted"
"#,
                key_pair.public_key
            ),
        )
        .unwrap();

        let result = disable_encryption_internal(dir.path());
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.decrypted_count, 0);

        // Verify [encryption] section was removed
        let updated_config = fs::read_to_string(&config_path).unwrap();
        assert!(!updated_config.contains("[encryption]"));
        assert!(!updated_config.contains("public_key"));

        // Verify other content is preserved
        assert!(updated_config.contains("PLAIN_VALUE = \"not encrypted\""));
    }

    #[test]
    fn test_disable_encryption_internal_removes_keys_file() {
        let dir = tempdir().unwrap();

        // Generate keys
        let key_pair = crate::crypto::keys::generate_key_pair();
        let keys_path = dir.path().join(".stand.keys");
        crate::crypto::keys::save_private_key(&keys_path, &key_pair.private_key).unwrap();
        assert!(keys_path.exists());

        // Create config
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

        let result = disable_encryption_internal(dir.path());
        assert!(result.is_ok());

        // Verify .stand.keys was deleted
        assert!(!keys_path.exists());
    }

    #[test]
    fn test_disable_encryption_internal_not_enabled() {
        let dir = tempdir().unwrap();

        // Create config WITHOUT encryption section
        let config_path = dir.path().join(".stand.toml");
        fs::write(
            &config_path,
            r#"version = "1.0"

[environments.dev]
description = "Development"
"#,
        )
        .unwrap();

        let result = disable_encryption_internal(dir.path());
        assert!(matches!(result, Err(EncryptionCommandError::NotEnabled)));
    }

    #[test]
    fn test_disable_encryption_internal_no_private_key() {
        let dir = tempdir().unwrap();

        // Create config with encryption enabled but NO .stand.keys file
        let config_path = dir.path().join(".stand.toml");
        fs::write(
            &config_path,
            r#"version = "1.0"

[encryption]
public_key = "age1test"

[environments.dev]
description = "Development"
SECRET = "encrypted:somedata"
"#,
        )
        .unwrap();

        // Note: No .stand.keys file created

        let result = disable_encryption_internal(dir.path());
        assert!(matches!(
            result,
            Err(EncryptionCommandError::PrivateKeyLoadFailed(_))
        ));
    }

    #[test]
    fn test_disable_encryption_internal_handles_common_section() {
        let dir = tempdir().unwrap();

        // Generate keys
        let key_pair = crate::crypto::keys::generate_key_pair();
        let keys_path = dir.path().join(".stand.keys");
        crate::crypto::keys::save_private_key(&keys_path, &key_pair.private_key).unwrap();

        // Encrypt test value
        let recipient = key_pair.to_recipient().unwrap();
        let encrypted = crate::crypto::encrypt_value("common-secret", &recipient).unwrap();

        // Create config with encrypted value in [common] section
        let config_path = dir.path().join(".stand.toml");
        fs::write(
            &config_path,
            format!(
                r#"version = "1.0"

[encryption]
public_key = "{}"

[common]
SHARED_SECRET = "{}"

[environments.dev]
description = "Development"
"#,
                key_pair.public_key, encrypted
            ),
        )
        .unwrap();

        let result = disable_encryption_internal(dir.path());
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.decrypted_count, 1);

        // Verify the common section was updated
        let updated_config = fs::read_to_string(&config_path).unwrap();
        assert!(updated_config.contains("SHARED_SECRET = \"common-secret\""));
    }

    #[test]
    fn test_disable_encryption_internal_fails_on_malformed_value() {
        let dir = tempdir().unwrap();

        // Generate keys
        let key_pair = crate::crypto::keys::generate_key_pair();
        let keys_path = dir.path().join(".stand.keys");
        crate::crypto::keys::save_private_key(&keys_path, &key_pair.private_key).unwrap();

        // Create config with a malformed encrypted value (not valid ciphertext)
        let config_path = dir.path().join(".stand.toml");
        let original_content = format!(
            r#"version = "1.0"

[encryption]
public_key = "{}"

[environments.dev]
description = "Development"
MALFORMED_SECRET = "encrypted:this-is-not-valid-ciphertext"
"#,
            key_pair.public_key
        );
        fs::write(&config_path, &original_content).unwrap();

        // Attempt to disable encryption - should fail
        let result = disable_encryption_internal(dir.path());
        assert!(matches!(
            result,
            Err(EncryptionCommandError::DecryptionFailed { .. })
        ));

        // Verify the config file was NOT modified (still contains encryption section)
        let config_after = fs::read_to_string(&config_path).unwrap();
        assert_eq!(config_after, original_content);

        // Verify .stand.keys was NOT deleted
        assert!(keys_path.exists());
    }
}
