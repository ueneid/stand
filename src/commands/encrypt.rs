//! Encryption management commands.

use std::fs;
use std::path::Path;

use colored::Colorize;

use crate::crypto::{generate_key_pair, CryptoError};

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

    // Check if encryption is already enabled
    let config_content = fs::read_to_string(&config_path)?;
    if config_content.contains("[encryption]") {
        return Err(EncryptionCommandError::AlreadyEnabled);
    }

    // Generate key pair
    let key_pair = generate_key_pair();

    // Add [encryption] section to config
    let encryption_section = format!("\n[encryption]\npublic_key = \"{}\"\n", key_pair.public_key);
    let updated_config = format!("{}{}", config_content, encryption_section);
    fs::write(&config_path, updated_config)?;

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
    println!("{} Created {} (add to .gitignore)", "✓".green(), KEYS_FILE);

    Ok(())
}

/// Disable encryption for the project.
///
/// Decrypts all encrypted values and removes encryption configuration.
pub fn disable_encryption(project_dir: &Path) -> Result<(), EncryptionCommandError> {
    let config_path = project_dir.join(CONFIG_FILE);
    let keys_path = project_dir.join(KEYS_FILE);

    // Check if config file exists
    if !config_path.exists() {
        return Err(EncryptionCommandError::ConfigNotFound);
    }

    // Check if encryption is enabled
    let config_content = fs::read_to_string(&config_path)?;
    if !config_content.contains("[encryption]") {
        return Err(EncryptionCommandError::NotEnabled);
    }

    // Prompt for confirmation
    println!(
        "{} This will decrypt all encrypted values and remove encryption.",
        "⚠".yellow()
    );
    print!("Continue? [y/N]: ");
    std::io::Write::flush(&mut std::io::stdout())?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Aborted.");
        return Ok(());
    }

    // Load private key
    let private_key = load_private_key_for_decryption(project_dir)?;
    let identity = crate::crypto::keys::parse_private_key(&private_key)
        .map_err(EncryptionCommandError::Crypto)?;

    // Find and decrypt all encrypted values
    // We collect all encrypted values first, then replace them to avoid
    // issues with string indices changing during replacement.
    let mut decrypted_count = 0;
    let mut replacements: Vec<(String, String)> = Vec::new();

    // Find all encrypted values in the config
    let mut search_pos = 0;
    while let Some(relative_start) = config_content[search_pos..].find("\"encrypted:") {
        let start = search_pos + relative_start;
        let value_start = start + 1; // Skip the opening quote
        if let Some(end) = config_content[value_start..].find('"') {
            let encrypted_value = &config_content[value_start..value_start + end];
            match crate::crypto::decrypt_value(encrypted_value, &identity) {
                Ok(decrypted) => {
                    replacements.push((encrypted_value.to_string(), decrypted));
                    decrypted_count += 1;
                }
                Err(e) => {
                    return Err(EncryptionCommandError::Crypto(e));
                }
            }
            // Move past this encrypted value to find the next one
            search_pos = value_start + end + 1;
        } else {
            break;
        }
    }

    // Apply all replacements to the config
    let mut updated_config = config_content.clone();
    for (encrypted, decrypted) in replacements {
        // Replace the encrypted value with the decrypted value (including quotes)
        updated_config =
            updated_config.replace(&format!("\"{}\"", encrypted), &format!("\"{}\"", decrypted));
    }

    // Remove [encryption] section
    // This is a simple implementation - a proper TOML parser would be better
    if let Some(start) = updated_config.find("\n[encryption]") {
        // Find the next section or end of file
        let section_content = &updated_config[start + 1..];
        let next_section = section_content[13..] // Skip "[encryption]\n"
            .find("\n[")
            .map(|pos| start + 1 + 13 + pos)
            .unwrap_or(updated_config.len());
        updated_config = format!(
            "{}{}",
            &updated_config[..start],
            &updated_config[next_section..]
        );
    }

    fs::write(&config_path, updated_config)?;

    // Remove .stand.keys file if it exists
    if keys_path.exists() {
        fs::remove_file(&keys_path)?;
    }

    if decrypted_count > 0 {
        println!("{} Decrypted {} value(s)", "✓".green(), decrypted_count);
    }
    println!("{} Removed [encryption] section", "✓".green());
    println!("{} Encryption disabled", "✓".green());

    Ok(())
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
    // First try environment variable
    if let Some(key) = crate::crypto::keys::load_private_key_from_env() {
        return Ok(key);
    }

    // Then try .stand.keys file
    let keys_path = project_dir.join(KEYS_FILE);
    crate::crypto::keys::load_private_key(&keys_path)
        .map_err(|_| EncryptionCommandError::NoPrivateKey)
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

    #[error("Private key not found. Set STAND_PRIVATE_KEY or ensure .stand.keys exists.")]
    NoPrivateKey,

    #[error("Cryptographic error: {0}")]
    Crypto(#[from] CryptoError),

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
}
