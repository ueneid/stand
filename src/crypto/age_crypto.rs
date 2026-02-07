//! Age-based encryption and decryption.
//!
//! Provides functions to encrypt and decrypt values using the age library.

use std::io::{Read, Write};

use age::x25519::{Identity, Recipient};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

use super::{CryptoError, ENCRYPTED_PREFIX};

/// Checks if a value is encrypted (has the encrypted: prefix).
pub fn is_encrypted(value: &str) -> bool {
    value.starts_with(ENCRYPTED_PREFIX)
}

/// Encrypts a plaintext value with the given public key.
///
/// Returns the encrypted value with the "encrypted:" prefix.
///
/// # Errors
/// Returns `CryptoError::EncryptionFailed` if encryption fails.
pub fn encrypt_value(plaintext: &str, recipient: &Recipient) -> Result<String, CryptoError> {
    let encryptor = age::Encryptor::with_recipients(vec![Box::new(recipient.clone())])
        .ok_or_else(|| CryptoError::EncryptionFailed("Failed to create encryptor".to_string()))?;

    let mut encrypted = vec![];
    let mut writer = encryptor
        .wrap_output(&mut encrypted)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    writer
        .write_all(plaintext.as_bytes())
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    writer
        .finish()
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    let encoded = BASE64.encode(&encrypted);
    Ok(format!("{}{}", ENCRYPTED_PREFIX, encoded))
}

/// Decrypts an encrypted value with the given private key.
///
/// The value should have the "encrypted:" prefix.
/// Returns the decrypted plaintext.
pub fn decrypt_value(encrypted_value: &str, identity: &Identity) -> Result<String, CryptoError> {
    let encoded = encrypted_value
        .strip_prefix(ENCRYPTED_PREFIX)
        .ok_or_else(|| CryptoError::DecryptionFailed("Missing encrypted: prefix".to_string()))?;

    if encoded.is_empty() {
        return Err(CryptoError::DecryptionFailed(
            "Encrypted value is empty after prefix".to_string(),
        ));
    }

    let encrypted = BASE64.decode(encoded).map_err(|e| {
        CryptoError::DecryptionFailed(format!("Invalid base64 encoding in encrypted value: {}", e))
    })?;

    let decryptor = match age::Decryptor::new(&encrypted[..])
        .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?
    {
        age::Decryptor::Recipients(d) => d,
        _ => {
            return Err(CryptoError::DecryptionFailed(
                "Unexpected decryptor type".to_string(),
            ))
        }
    };

    let mut decrypted = vec![];
    let mut reader = decryptor
        .decrypt(std::iter::once(identity as &dyn age::Identity))
        .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    reader
        .read_to_end(&mut decrypted)
        .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    String::from_utf8(decrypted)
        .map_err(|e| CryptoError::DecryptionFailed(format!("Invalid UTF-8: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keys::generate_key_pair;

    #[test]
    fn test_encrypt_and_decrypt_roundtrip() {
        let key_pair = generate_key_pair();
        let recipient = key_pair.to_recipient().unwrap();
        let identity = key_pair.to_identity().unwrap();

        let plaintext = "my-secret-api-key-12345";
        let encrypted = encrypt_value(plaintext, &recipient).unwrap();

        // Should have encrypted prefix
        assert!(encrypted.starts_with(ENCRYPTED_PREFIX));

        // Should be able to decrypt back
        let decrypted = decrypt_value(&encrypted, &identity).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_empty_string() {
        let key_pair = generate_key_pair();
        let recipient = key_pair.to_recipient().unwrap();
        let identity = key_pair.to_identity().unwrap();

        let plaintext = "";
        let encrypted = encrypt_value(plaintext, &recipient).unwrap();
        let decrypted = decrypt_value(&encrypted, &identity).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_unicode_string() {
        let key_pair = generate_key_pair();
        let recipient = key_pair.to_recipient().unwrap();
        let identity = key_pair.to_identity().unwrap();

        let plaintext = "こんにちは世界 🔐";
        let encrypted = encrypt_value(plaintext, &recipient).unwrap();
        let decrypted = decrypt_value(&encrypted, &identity).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_with_wrong_key_fails() {
        let key_pair1 = generate_key_pair();
        let key_pair2 = generate_key_pair();

        let recipient1 = key_pair1.to_recipient().unwrap();
        let identity2 = key_pair2.to_identity().unwrap();

        let plaintext = "secret";
        let encrypted = encrypt_value(plaintext, &recipient1).unwrap();

        // Decrypting with wrong key should fail
        let result = decrypt_value(&encrypted, &identity2);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_without_prefix_fails() {
        let key_pair = generate_key_pair();
        let identity = key_pair.to_identity().unwrap();

        let result = decrypt_value("not-encrypted-value", &identity);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_invalid_base64_fails() {
        let key_pair = generate_key_pair();
        let identity = key_pair.to_identity().unwrap();

        let result = decrypt_value("encrypted:not-valid-base64!!!", &identity);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_empty_after_prefix_fails() {
        let key_pair = generate_key_pair();
        let identity = key_pair.to_identity().unwrap();
        let result = decrypt_value("encrypted:", &identity);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("empty"),
            "Error should mention 'empty', got: {}",
            err_msg
        );
    }
}
