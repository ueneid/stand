//! Encryption module for Stand.
//!
//! This module provides encryption and decryption functionality using the age library.
//! It supports X25519 key pairs for asymmetric encryption.

mod age_crypto;
pub mod keys;

pub use age_crypto::{decrypt_value, encrypt_value, is_encrypted};
pub use keys::{generate_key_pair, KeyPair};

use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Prefix for encrypted values in TOML configuration.
pub const ENCRYPTED_PREFIX: &str = "encrypted:";

/// Error types for cryptographic operations.
#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Failed to generate key pair: {0}")]
    KeyGenerationFailed(String),

    #[error("Failed to encrypt value: {0}")]
    EncryptionFailed(String),

    #[error("Failed to decrypt value: {0}")]
    DecryptionFailed(String),

    #[error("Invalid public key format: {0}")]
    InvalidPublicKey(String),

    #[error("Invalid private key format: {0}")]
    InvalidPrivateKey(String),

    #[error("No encryption key configured")]
    NoEncryptionKey,

    #[error("No private key available for decryption")]
    NoPrivateKey,

    #[error("Base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Decrypts all encrypted values in a HashMap.
///
/// This function checks each value in the HashMap, and if it's encrypted (starts with "encrypted:"),
/// it will be decrypted using the provided private key.
///
/// # Arguments
/// * `variables` - The HashMap of variable names to values
/// * `project_dir` - The project directory (used to locate .stand.keys file)
///
/// # Returns
/// A new HashMap with all encrypted values decrypted.
/// If no encrypted values are found, returns the original HashMap unchanged.
/// If encrypted values are found but no private key is available, returns an error.
pub fn decrypt_variables(
    variables: HashMap<String, String>,
    project_dir: &Path,
) -> Result<HashMap<String, String>, CryptoError> {
    // Check if any values are encrypted
    let has_encrypted = variables.values().any(|v| is_encrypted(v));
    if !has_encrypted {
        return Ok(variables);
    }

    // Load private key
    let private_key = load_private_key_for_decryption(project_dir)?;
    let identity = keys::parse_private_key(&private_key)?;

    // Decrypt all encrypted values
    let mut result = HashMap::new();
    for (key, value) in variables {
        if is_encrypted(&value) {
            let decrypted = decrypt_value(&value, &identity)?;
            result.insert(key, decrypted);
        } else {
            result.insert(key, value);
        }
    }

    Ok(result)
}

/// Load private key from environment variable or file.
fn load_private_key_for_decryption(project_dir: &Path) -> Result<String, CryptoError> {
    // First try environment variable
    if let Some(key) = keys::load_private_key_from_env() {
        return Ok(key);
    }

    // Then try .stand.keys file
    let keys_path = project_dir.join(".stand.keys");
    keys::load_private_key(&keys_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_encrypted_prefix_constant() {
        assert_eq!(ENCRYPTED_PREFIX, "encrypted:");
    }

    #[test]
    fn test_is_encrypted_detects_encrypted_values() {
        assert!(is_encrypted("encrypted:abc123"));
        assert!(is_encrypted("encrypted:"));
        assert!(!is_encrypted("plain text"));
        assert!(!is_encrypted(""));
        assert!(!is_encrypted("encrypt:abc")); // Wrong prefix
    }

    #[test]
    fn test_decrypt_variables_no_encrypted_values() {
        let dir = tempdir().unwrap();
        let mut variables = HashMap::new();
        variables.insert("KEY1".to_string(), "value1".to_string());
        variables.insert("KEY2".to_string(), "value2".to_string());

        // Should return the same values when nothing is encrypted
        let result = decrypt_variables(variables.clone(), dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), variables);
    }

    #[test]
    fn test_decrypt_variables_with_encrypted_values() {
        let dir = tempdir().unwrap();

        // Generate keys and save private key
        let key_pair = generate_key_pair();
        let keys_path = dir.path().join(".stand.keys");
        keys::save_private_key(&keys_path, &key_pair.private_key).unwrap();

        // Encrypt a value
        let recipient = key_pair.to_recipient().unwrap();
        let encrypted = encrypt_value("secret-value", &recipient).unwrap();

        let mut variables = HashMap::new();
        variables.insert("PLAIN_KEY".to_string(), "plain-value".to_string());
        variables.insert("SECRET_KEY".to_string(), encrypted);

        // Should decrypt the encrypted value
        let result = decrypt_variables(variables, dir.path());
        assert!(result.is_ok());

        let decrypted = result.unwrap();
        assert_eq!(decrypted.get("PLAIN_KEY"), Some(&"plain-value".to_string()));
        assert_eq!(
            decrypted.get("SECRET_KEY"),
            Some(&"secret-value".to_string())
        );
    }

    #[test]
    fn test_decrypt_variables_fails_without_private_key() {
        let dir = tempdir().unwrap();

        // Create a config file without keys
        fs::write(dir.path().join(".stand.toml"), "version = \"1.0\"").unwrap();

        let mut variables = HashMap::new();
        variables.insert("SECRET".to_string(), "encrypted:somedata".to_string());

        // Should fail because no private key is available
        let result = decrypt_variables(variables, dir.path());
        assert!(result.is_err());
    }
}
