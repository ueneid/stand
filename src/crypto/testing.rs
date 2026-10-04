//! Test doubles for the unlock gate (compiled only for tests).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use age::secrecy::SecretString;

use super::touch_id::{SeKeyMaterial, SecureEnclaveBackend};
use super::ui::UnlockUi;
use super::CryptoError;

/// Scripted UI: hands out queued passphrases and records everything shown.
pub struct MockUi {
    passphrases: Mutex<VecDeque<Option<String>>>,
    pub shown: Mutex<Vec<String>>,
    pub prompts: Mutex<Vec<String>>,
}

impl MockUi {
    /// `passphrases` are returned in order; `None` simulates a cancelled prompt.
    pub fn new(passphrases: Vec<Option<&str>>) -> Arc<Self> {
        Arc::new(Self {
            passphrases: Mutex::new(
                passphrases
                    .into_iter()
                    .map(|p| p.map(str::to_string))
                    .collect(),
            ),
            shown: Mutex::new(vec![]),
            prompts: Mutex::new(vec![]),
        })
    }

    pub fn shown_messages(&self) -> Vec<String> {
        self.shown.lock().unwrap().clone()
    }

    pub fn prompt_count(&self) -> usize {
        self.prompts.lock().unwrap().len()
    }
}

impl UnlockUi for MockUi {
    fn show(&self, message: &str) {
        self.shown.lock().unwrap().push(message.to_string());
    }

    fn request_passphrase(&self, prompt: &str) -> Option<SecretString> {
        self.prompts.lock().unwrap().push(prompt.to_string());
        self.passphrases
            .lock()
            .unwrap()
            .pop_front()
            .flatten()
            .map(SecretString::from)
    }

    fn confirm(&self, _message: &str) -> Option<bool> {
        Some(true)
    }
}

/// Fake Secure Enclave: "encrypts" by prefixing, optionally fails on unwrap.
pub struct MockSe {
    pub available: bool,
    pub fail_unwrap: bool,
    /// Error text used when `fail_unwrap` is set.
    pub fail_reason: String,
    /// Path reported by `plugin_path`.
    pub plugin_path: Option<std::path::PathBuf>,
    pub unwrap_calls: AtomicUsize,
    pub keygen_calls: AtomicUsize,
}

impl MockSe {
    pub fn new(available: bool, fail_unwrap: bool) -> Self {
        Self {
            available,
            fail_unwrap,
            fail_reason: "mock biometric failure".to_string(),
            plugin_path: Some(std::path::PathBuf::from("/mock/bin/age-plugin-se")),
            unwrap_calls: AtomicUsize::new(0),
            keygen_calls: AtomicUsize::new(0),
        }
    }

    pub fn with_fail_reason(mut self, reason: &str) -> Self {
        self.fail_reason = reason.to_string();
        self
    }

    pub fn unwrap_count(&self) -> usize {
        self.unwrap_calls.load(Ordering::SeqCst)
    }
}

const MOCK_PREFIX: &[u8] = b"MOCK-SE:";

impl SecureEnclaveBackend for MockSe {
    fn is_available(&self) -> bool {
        self.available
    }

    fn plugin_path(&self) -> Option<std::path::PathBuf> {
        self.plugin_path.clone()
    }

    fn keygen(&self) -> Result<SeKeyMaterial, CryptoError> {
        self.keygen_calls.fetch_add(1, Ordering::SeqCst);
        Ok(SeKeyMaterial {
            recipient: "age1se1mockrecipient".to_string(),
            identity_file: "# public key: age1se1mockrecipient\nAGE-PLUGIN-SE-1MOCK\n".to_string(),
        })
    }

    fn wrap(&self, _recipient: &str, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut out = MOCK_PREFIX.to_vec();
        out.extend_from_slice(plaintext);
        Ok(out)
    }

    fn unwrap(
        &self,
        _identity_file: &str,
        ciphertext: &[u8],
        ui: Arc<dyn UnlockUi>,
    ) -> Result<Vec<u8>, CryptoError> {
        self.unwrap_calls.fetch_add(1, Ordering::SeqCst);
        ui.show("(mock) Touch ID requested");
        if self.fail_unwrap {
            return Err(CryptoError::DecryptionFailed(self.fail_reason.clone()));
        }
        ciphertext
            .strip_prefix(MOCK_PREFIX)
            .map(<[u8]>::to_vec)
            .ok_or_else(|| CryptoError::DecryptionFailed("bad mock ciphertext".to_string()))
    }
}
