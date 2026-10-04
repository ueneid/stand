//! Secure Enclave (Touch ID) support through the `age-plugin-se` age plugin.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

use super::ui::{UiCallbacks, UnlockUi};
use super::CryptoError;

/// Name of the age plugin binary (`age-plugin-<name>`).
const PLUGIN_NAME: &str = "se";
const PLUGIN_BINARY: &str = "age-plugin-se";

/// Result of generating a Secure Enclave key.
#[derive(Debug, Clone)]
pub struct SeKeyMaterial {
    /// The `age1se1...` recipient string.
    pub recipient: String,
    /// Contents of the identity file (contains the `AGE-PLUGIN-SE-1...` handle).
    pub identity_file: String,
}

/// Operations needed from a Secure Enclave backend. The real implementation
/// shells out to `age-plugin-se`; tests inject a mock.
pub trait SecureEnclaveBackend: Send + Sync {
    /// Whether the backend can be used on this machine (plugin binary present).
    fn is_available(&self) -> bool;
    /// Creates a new Secure Enclave key protected by biometry or passcode.
    fn keygen(&self) -> Result<SeKeyMaterial, CryptoError>;
    /// Encrypts `plaintext` to the Secure Enclave `recipient` (needs no human).
    fn wrap(&self, recipient: &str, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError>;
    /// Decrypts `ciphertext` with the identity file. This is where Touch ID is asked.
    fn unwrap(
        &self,
        identity_file: &str,
        ciphertext: &[u8],
        ui: Arc<dyn UnlockUi>,
    ) -> Result<Vec<u8>, CryptoError>;
}

/// Finds `name` in `PATH`.
pub fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// Extracts the `age1se1...` recipient from an identity file's `# public key:` line.
pub fn recipient_from_identity_file(identity_file: &str) -> Result<String, CryptoError> {
    for line in identity_file.lines() {
        if let Some(rest) = line.trim().strip_prefix("# public key:") {
            let recipient = rest.trim().to_string();
            check_se_recipient(&recipient)?;
            return Ok(recipient);
        }
    }
    Err(CryptoError::SecureEnclaveUnavailable(
        "identity file has no `# public key:` line".to_string(),
    ))
}

/// Parses `recipient` as an age plugin recipient and asserts it belongs to `se`.
///
/// A plain `age1...` X25519 string also parses as a plugin recipient, hence the check.
fn check_se_recipient(recipient: &str) -> Result<age::plugin::Recipient, CryptoError> {
    let parsed = recipient
        .parse::<age::plugin::Recipient>()
        .map_err(|e| CryptoError::InvalidPublicKey(format!("not a plugin recipient: {}", e)))?;
    if parsed.plugin() != PLUGIN_NAME {
        return Err(CryptoError::InvalidPublicKey(format!(
            "expected a Secure Enclave recipient (age1se1...), got plugin '{}'",
            parsed.plugin()
        )));
    }
    Ok(parsed)
}

/// The real backend using the `age-plugin-se` binary found in `PATH`.
pub struct RealSecureEnclave;

impl SecureEnclaveBackend for RealSecureEnclave {
    fn is_available(&self) -> bool {
        find_in_path(PLUGIN_BINARY).is_some()
    }

    fn keygen(&self) -> Result<SeKeyMaterial, CryptoError> {
        let binary = find_in_path(PLUGIN_BINARY).ok_or_else(|| {
            CryptoError::SecureEnclaveUnavailable(format!("{} not found in PATH", PLUGIN_BINARY))
        })?;

        // Private scratch directory so the identity file is never world-readable.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("stand-se-keygen-{}-{}", std::process::id(), nanos));
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            std::fs::DirBuilder::new().mode(0o700).create(&dir)?;
        }
        #[cfg(not(unix))]
        std::fs::create_dir(&dir)?;
        let out_file = dir.join("identity");

        let result = (|| -> Result<SeKeyMaterial, CryptoError> {
            let output = Command::new(&binary)
                .args([
                    "keygen",
                    "--access-control",
                    "any-biometry-or-passcode",
                    "-o",
                ])
                .arg(&out_file)
                .stdin(Stdio::null())
                .output()?;
            if !output.status.success() {
                return Err(CryptoError::SecureEnclaveUnavailable(format!(
                    "age-plugin-se keygen failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                )));
            }
            let identity_file = std::fs::read_to_string(&out_file)?;
            let recipient = recipient_from_identity_file(&identity_file)?;
            Ok(SeKeyMaterial {
                recipient,
                identity_file,
            })
        })();
        let _ = std::fs::remove_dir_all(&dir);
        result
    }

    fn wrap(&self, recipient: &str, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let parsed = check_se_recipient(recipient)?;
        let plugin =
            age::plugin::RecipientPluginV1::new(PLUGIN_NAME, &[parsed], &[], age::NoCallbacks)
                .map_err(|e| CryptoError::SecureEnclaveUnavailable(e.to_string()))?;
        let encryptor =
            age::Encryptor::with_recipients(std::iter::once(&plugin as &dyn age::Recipient))
                .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
        let mut out = vec![];
        let mut writer = encryptor
            .wrap_output(&mut out)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
        writer
            .write_all(plaintext)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
        writer
            .finish()
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
        Ok(out)
    }

    fn unwrap(
        &self,
        identity_file: &str,
        ciphertext: &[u8],
        ui: Arc<dyn UnlockUi>,
    ) -> Result<Vec<u8>, CryptoError> {
        let identities =
            age::IdentityFile::from_buffer(std::io::BufReader::new(identity_file.as_bytes()))
                .map_err(|e| CryptoError::SecureEnclaveUnavailable(e.to_string()))?
                .with_callbacks(UiCallbacks(ui))
                .into_identities()
                .map_err(|e| CryptoError::SecureEnclaveUnavailable(e.to_string()))?;

        let decryptor = age::Decryptor::new(ciphertext)
            .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;
        let mut reader = decryptor
            .decrypt(identities.iter().map(|i| i.as_ref() as &dyn age::Identity))
            .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;
        let mut out = vec![];
        reader
            .read_to_end(&mut out)
            .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipient_from_identity_file_rejects_x25519_recipient() {
        let id = age::x25519::Identity::generate();
        let file = format!("# public key: {}\nAGE-PLUGIN-SE-1XXXX\n", id.to_public());
        let err = recipient_from_identity_file(&file).unwrap_err();
        assert!(matches!(err, CryptoError::InvalidPublicKey(_)));
    }

    #[test]
    fn test_recipient_from_identity_file_requires_public_key_line() {
        assert!(recipient_from_identity_file("AGE-PLUGIN-SE-1XXXX\n").is_err());
    }

    #[test]
    fn test_find_in_path_missing_binary() {
        assert!(find_in_path("definitely-not-a-real-binary-stand").is_none());
    }

    /// Real hardware test: creates a Secure Enclave key and wraps to it.
    /// Never decrypts (that needs a human). Run with `--ignored` on macOS.
    #[test]
    #[ignore]
    #[cfg(target_os = "macos")]
    #[serial_test::serial]
    fn test_real_secure_enclave_keygen_and_wrap() {
        // Make sure Homebrew's bin is visible in this test process only.
        let old_path = std::env::var_os("PATH").unwrap_or_default();
        let mut paths: Vec<PathBuf> = std::env::split_paths(&old_path).collect();
        paths.push(PathBuf::from("/opt/homebrew/bin"));
        std::env::set_var("PATH", std::env::join_paths(paths).unwrap());

        let backend = RealSecureEnclave;
        if !backend.is_available() {
            eprintln!("age-plugin-se not on PATH; skipping");
            std::env::set_var("PATH", old_path);
            return;
        }
        let result = (|| -> Result<(), CryptoError> {
            let material = backend.keygen()?;
            let parsed = check_se_recipient(&material.recipient)?;
            assert_eq!(parsed.plugin(), "se");
            assert!(material.identity_file.contains("AGE-PLUGIN-SE-1"));
            let ct = backend.wrap(&material.recipient, b"test plaintext")?;
            assert!(!ct.is_empty());
            Ok(())
        })();
        std::env::set_var("PATH", old_path);
        result.unwrap();
    }
}
