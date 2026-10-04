//! Encryption module for Stand.
//!
//! This module provides encryption and decryption functionality using the age library.
//! It supports X25519 key pairs for asymmetric encryption. Values stay encrypted to the
//! project X25519 key; the private key itself is stored wrapped (passphrase and optionally
//! Touch ID) and every decryption goes through the unlock gate in [`unlock`].

mod age_crypto;
pub mod enable;
pub mod keys;
pub mod store;
#[cfg(test)]
pub(crate) mod testing;
pub mod touch_id;
pub mod tty;
pub mod ui;
pub mod unlock;

pub use age_crypto::{decrypt_value, encrypt_value, is_encrypted};
pub use keys::{generate_key_pair, KeyPair};
pub use touch_id::{RealSecureEnclave, SeKeyMaterial, SecureEnclaveBackend};
pub use tty::TtyUi;
pub use ui::{DecryptPurpose, UnlockUi};
pub use unlock::{
    unlock_project_identity, unlock_project_identity_exact_with,
    unlock_project_identity_verified_with, unlock_project_identity_with, Binding, UnlockContext,
    UnlockEnv, Unlocked, VerifiedProject,
};

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

    #[error("Failed to decrypt variable '{variable}': {reason}")]
    DecryptionFailedForVariable { variable: String, reason: String },

    #[error("Invalid public key format: {0}")]
    InvalidPublicKey(String),

    #[error("Invalid private key format: {0}")]
    InvalidPrivateKey(String),

    #[error("No encryption key configured")]
    NoEncryptionKey,

    #[error("No private key available for decryption: {hint}")]
    NoPrivateKey { hint: String },

    #[error("STAND_PRIVATE_KEY is only used when CI=true or STAND_KEY_SOURCE=env. Never export it in your interactive shell profile.")]
    EnvKeyNotAllowed,

    #[error("Decryption requires an interactive terminal (could not open /dev/tty); refusing to decrypt. In CI, set CI=true and STAND_PRIVATE_KEY.")]
    NoTerminal,

    #[error("The unlocked identity does not match the project public key {expected}")]
    IdentityMismatch { expected: String },

    #[error("This key is bound to project '{bound}' but is being used for '{current}'; refusing to unlock it for this project")]
    ProjectMismatch { bound: String, current: String },

    #[error("Unlock failed: {0}")]
    UnlockFailed(String),

    #[error("Secure Enclave unavailable: {0}")]
    SecureEnclaveUnavailable(String),

    #[error("Identity store error: {0}")]
    StoreError(String),

    #[error("Base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Reads `[encryption].public_key` from the project's `.stand.toml`, if present.
pub fn read_project_public_key(project_dir: &Path) -> Option<String> {
    let content = std::fs::read_to_string(project_dir.join(".stand.toml")).ok()?;
    let doc: toml::Table = content.parse().ok()?;
    doc.get("encryption")?
        .get("public_key")?
        .as_str()
        .map(str::to_string)
}

/// Unlocks the project identity (reading the public key from `.stand.toml`).
pub fn unlock_for_project(
    project_dir: &Path,
    purpose: &DecryptPurpose,
) -> Result<age::x25519::Identity, CryptoError> {
    let ctx = UnlockContext::production()?;
    unlock_for_project_with(project_dir, purpose, &ctx)
}

/// [`unlock_for_project`] with an injected [`UnlockContext`].
pub fn unlock_for_project_with(
    project_dir: &Path,
    purpose: &DecryptPurpose,
    ctx: &UnlockContext,
) -> Result<age::x25519::Identity, CryptoError> {
    unlock_for_project_verified_with(project_dir, purpose, ctx).map(|u| u.identity)
}

/// Like [`unlock_for_project_with`], also reporting whether the identity's
/// project binding was verified (see [`Binding`]).
pub fn unlock_for_project_verified_with(
    project_dir: &Path,
    purpose: &DecryptPurpose,
    ctx: &UnlockContext,
) -> Result<Unlocked, CryptoError> {
    unlock_project_identity_verified_with(
        project_dir,
        read_project_public_key(project_dir).as_deref(),
        purpose,
        ctx,
    )
}

/// [`unlock_for_project_verified_with`] for operations that write or delete key
/// material: a binding mismatch is refused without any confirmation prompt.
pub fn unlock_for_project_exact_with(
    project_dir: &Path,
    purpose: &DecryptPurpose,
    ctx: &UnlockContext,
) -> Result<Unlocked, CryptoError> {
    unlock_project_identity_exact_with(
        project_dir,
        read_project_public_key(project_dir).as_deref(),
        purpose,
        ctx,
    )
}

/// Decrypts all encrypted values in a HashMap.
///
/// If no value is encrypted the map is returned unchanged and nothing is
/// unlocked. Otherwise the project identity is unlocked once through the
/// unlock gate (see [`unlock`]), with `purpose.secret_count` set to the number
/// of encrypted values.
pub fn decrypt_variables(
    variables: HashMap<String, String>,
    project_dir: &Path,
    purpose: &DecryptPurpose,
) -> Result<HashMap<String, String>, CryptoError> {
    if !variables.values().any(|v| is_encrypted(v)) {
        return Ok(variables);
    }
    let ctx = UnlockContext::production()?;
    decrypt_variables_with(variables, project_dir, purpose, &ctx)
}

/// [`decrypt_variables`] with an injected [`UnlockContext`].
pub fn decrypt_variables_with(
    variables: HashMap<String, String>,
    project_dir: &Path,
    purpose: &DecryptPurpose,
    ctx: &UnlockContext,
) -> Result<HashMap<String, String>, CryptoError> {
    let count = variables.values().filter(|v| is_encrypted(v)).count();
    if count == 0 {
        return Ok(variables);
    }
    let mut purpose = purpose.clone();
    purpose.secret_count = count;
    let identity = unlock_for_project_with(project_dir, &purpose, ctx)?;

    let mut result = HashMap::new();
    for (key, value) in variables {
        if is_encrypted(&value) {
            let decrypted = decrypt_value(&value, &identity).map_err(|e| {
                CryptoError::DecryptionFailedForVariable {
                    variable: key.clone(),
                    reason: e.to_string(),
                }
            })?;
            result.insert(key, decrypted);
        } else {
            result.insert(key, value);
        }
    }
    Ok(result)
}

/// Decrypts a single value (e.g. an inherited value for `stand unset`).
///
/// Plain (non-encrypted) values are returned as is without unlocking.
/// `purpose.secret_count` is set to 1.
pub fn decrypt_single_value(
    value: &str,
    project_dir: &Path,
    purpose: &DecryptPurpose,
) -> Result<String, CryptoError> {
    if !is_encrypted(value) {
        return Ok(value.to_string());
    }
    let ctx = UnlockContext::production()?;
    decrypt_single_value_with(value, project_dir, purpose, &ctx)
}

/// [`decrypt_single_value`] with an injected [`UnlockContext`].
pub fn decrypt_single_value_with(
    value: &str,
    project_dir: &Path,
    purpose: &DecryptPurpose,
    ctx: &UnlockContext,
) -> Result<String, CryptoError> {
    if !is_encrypted(value) {
        return Ok(value.to_string());
    }
    let mut purpose = purpose.clone();
    purpose.secret_count = 1;
    let identity = unlock_for_project_with(project_dir, &purpose, ctx)?;
    decrypt_value(value, &identity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::testing::{MockSe, MockUi};
    use age::secrecy::{ExposeSecret, SecretString};
    use serial_test::serial;
    use std::ffi::OsString;
    use std::fs;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    /// Sets/clears process env vars and restores them on drop.
    struct EnvGuard(Vec<(&'static str, Option<OsString>)>);

    impl EnvGuard {
        fn set(vars: &[(&'static str, Option<&str>)]) -> Self {
            let saved = vars
                .iter()
                .map(|(k, _)| (*k, std::env::var_os(k)))
                .collect();
            for (k, v) in vars {
                match v {
                    Some(v) => std::env::set_var(k, v),
                    None => std::env::remove_var(k),
                }
            }
            EnvGuard(saved)
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (k, v) in self.0.drain(..) {
                match v {
                    Some(v) => std::env::set_var(k, v),
                    None => std::env::remove_var(k),
                }
            }
        }
    }

    fn purpose() -> DecryptPurpose {
        DecryptPurpose::new("stand exec dev -- true", Some("dev"))
    }

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

        // Should return the same values when nothing is encrypted (and never unlock)
        let result = decrypt_variables(variables.clone(), dir.path(), &purpose());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), variables);
    }

    #[test]
    #[serial]
    fn test_decrypt_variables_with_legacy_keys_file() {
        let _env = EnvGuard::set(&[
            ("STAND_PRIVATE_KEY", None),
            ("STAND_FORCE_NON_TTY", Some("1")),
        ]);
        let cfg = tempdir().unwrap();
        let _xdg = EnvGuard::set(&[("XDG_CONFIG_HOME", cfg.path().to_str())]);
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

        // Legacy .stand.keys needs no terminal
        let decrypted = decrypt_variables(variables, dir.path(), &purpose()).unwrap();
        assert_eq!(decrypted.get("PLAIN_KEY"), Some(&"plain-value".to_string()));
        assert_eq!(
            decrypted.get("SECRET_KEY"),
            Some(&"secret-value".to_string())
        );
    }

    #[test]
    #[serial]
    fn test_decrypt_variables_fails_without_private_key() {
        let _env = EnvGuard::set(&[
            ("STAND_PRIVATE_KEY", None),
            ("STAND_FORCE_NON_TTY", Some("1")),
        ]);
        let dir = tempdir().unwrap();

        // Create a config file without keys
        fs::write(dir.path().join(".stand.toml"), "version = \"1.0\"").unwrap();

        let mut variables = HashMap::new();
        variables.insert("SECRET".to_string(), "encrypted:somedata".to_string());

        // Should fail because no private key is available
        let result = decrypt_variables(variables, dir.path(), &purpose());
        assert!(matches!(result, Err(CryptoError::NoEncryptionKey)));
    }

    #[test]
    #[serial]
    fn test_force_non_tty_with_identity_file_reports_no_terminal() {
        let cfg = tempdir().unwrap();
        let dir = tempdir().unwrap();
        let _env = EnvGuard::set(&[
            ("XDG_CONFIG_HOME", cfg.path().to_str()),
            ("STAND_FORCE_NON_TTY", Some("1")),
            ("STAND_PRIVATE_KEY", None),
            ("CI", None),
            ("STAND_KEY_SOURCE", None),
        ]);

        let identity = age::x25519::Identity::generate();
        let public_key = identity.to_public().to_string();
        store::write_passphrase_identity(
            &public_key,
            &identity,
            &SecretString::from("pw".to_string()),
            Some(8),
            dir.path(),
        )
        .unwrap();
        fs::write(
            dir.path().join(".stand.toml"),
            format!("[encryption]\npublic_key = \"{}\"\n", public_key),
        )
        .unwrap();
        let encrypted = encrypt_value("v", &identity.to_public()).unwrap();

        let mut variables = HashMap::new();
        variables.insert("S".to_string(), encrypted);
        let err = decrypt_variables(variables, dir.path(), &purpose()).unwrap_err();
        assert!(matches!(err, CryptoError::NoTerminal));
        assert!(err.to_string().contains("interactive terminal"));
    }

    #[test]
    #[serial]
    fn test_ci_env_key_decrypts_without_tty() {
        let identity = age::x25519::Identity::generate();
        let key = identity.to_string().expose_secret().to_string();
        let cfg = tempdir().unwrap();
        let dir = tempdir().unwrap();
        let _env = EnvGuard::set(&[
            ("XDG_CONFIG_HOME", cfg.path().to_str()),
            ("STAND_FORCE_NON_TTY", Some("1")),
            ("CI", Some("true")),
            ("STAND_KEY_SOURCE", None),
            ("STAND_PRIVATE_KEY", Some(&key)),
        ]);
        let encrypted = encrypt_value("ci-secret", &identity.to_public()).unwrap();
        let mut variables = HashMap::new();
        variables.insert("S".to_string(), encrypted);

        let out = decrypt_variables(variables, dir.path(), &purpose()).unwrap();
        assert_eq!(out.get("S"), Some(&"ci-secret".to_string()));
    }

    #[test]
    #[serial]
    fn test_env_key_without_opt_in_is_rejected() {
        let identity = age::x25519::Identity::generate();
        let key = identity.to_string().expose_secret().to_string();
        let cfg = tempdir().unwrap();
        let dir = tempdir().unwrap();
        let _env = EnvGuard::set(&[
            ("XDG_CONFIG_HOME", cfg.path().to_str()),
            ("CI", None),
            ("STAND_KEY_SOURCE", None),
            ("STAND_PRIVATE_KEY", Some(&key)),
        ]);
        let encrypted = encrypt_value("x", &identity.to_public()).unwrap();
        let mut variables = HashMap::new();
        variables.insert("S".to_string(), encrypted);

        let err = decrypt_variables(variables, dir.path(), &purpose()).unwrap_err();
        assert!(matches!(err, CryptoError::EnvKeyNotAllowed));
        assert!(err.to_string().contains("CI=true"));
    }

    #[test]
    fn test_decrypt_variables_with_sets_secret_count_in_banner() {
        let cfg = tempdir().unwrap();
        let project = tempdir().unwrap();
        let identity = age::x25519::Identity::generate();
        let public_key = identity.to_public().to_string();
        store::write_passphrase_identity_in(
            cfg.path(),
            &public_key,
            &identity,
            &SecretString::from("pw".to_string()),
            Some(8),
            project.path(),
        )
        .unwrap();
        fs::write(
            project.path().join(".stand.toml"),
            format!("[encryption]\npublic_key = \"{}\"\n", public_key),
        )
        .unwrap();

        let ui = MockUi::new(vec![Some("pw")]);
        let ctx = UnlockContext {
            env: UnlockEnv {
                ci: None,
                key_source: None,
                private_key: None,
                unlock_mode: None,
                force_non_tty: false,
                config_dir: cfg.path().to_path_buf(),
                max_work_factor: Some(8),
            },
            ui: Some(ui.clone() as Arc<dyn UnlockUi>),
            secure_enclave: Box::new(MockSe::new(false, false)),
            warn: Arc::new(|_| {}),
        };
        let mut variables = HashMap::new();
        variables.insert(
            "A".to_string(),
            encrypt_value("1", &identity.to_public()).unwrap(),
        );
        variables.insert(
            "B".to_string(),
            encrypt_value("2", &identity.to_public()).unwrap(),
        );
        variables.insert("C".to_string(), "plain".to_string());

        let out = decrypt_variables_with(variables, project.path(), &purpose(), &ctx).unwrap();
        assert_eq!(out["A"], "1");
        assert_eq!(out["B"], "2");
        assert_eq!(out["C"], "plain");
        assert_eq!(ui.prompt_count(), 1, "one unlock for all secrets");
        assert!(ui.shown_messages()[0].contains("2 secret(s)"));
    }

    #[test]
    fn test_decrypt_single_value_with_unlocks_once_and_passes_plain_through() {
        let cfg = tempdir().unwrap();
        let project = tempdir().unwrap();
        let identity = age::x25519::Identity::generate();
        let public_key = identity.to_public().to_string();
        store::write_passphrase_identity_in(
            cfg.path(),
            &public_key,
            &identity,
            &SecretString::from("pw".to_string()),
            Some(8),
            project.path(),
        )
        .unwrap();
        fs::write(
            project.path().join(".stand.toml"),
            format!("[encryption]\npublic_key = \"{}\"\n", public_key),
        )
        .unwrap();
        let ui = MockUi::new(vec![Some("pw")]);
        let warnings = Arc::new(Mutex::new(vec![]));
        let w = warnings.clone();
        let ctx = UnlockContext {
            env: UnlockEnv {
                ci: None,
                key_source: None,
                private_key: None,
                unlock_mode: None,
                force_non_tty: false,
                config_dir: cfg.path().to_path_buf(),
                max_work_factor: Some(8),
            },
            ui: Some(ui.clone() as Arc<dyn UnlockUi>),
            secure_enclave: Box::new(MockSe::new(false, false)),
            warn: Arc::new(move |m| w.lock().unwrap().push(m.to_string())),
        };
        let enc = encrypt_value("hello", &identity.to_public()).unwrap();
        assert_eq!(
            decrypt_single_value_with("plain", project.path(), &purpose(), &ctx).unwrap(),
            "plain"
        );
        assert_eq!(ui.prompt_count(), 0);
        assert_eq!(
            decrypt_single_value_with(&enc, project.path(), &purpose(), &ctx).unwrap(),
            "hello"
        );
        assert_eq!(ui.prompt_count(), 1);
        assert!(ui.shown_messages()[0].contains("1 secret(s)"));
        assert!(warnings.lock().unwrap().is_empty());
    }
}
