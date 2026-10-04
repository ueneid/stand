//! User interaction abstraction for the unlock gate.
//!
//! Decryption must be approved by a human, so every prompt goes through
//! [`UnlockUi`]. The production implementation ([`super::tty::TtyUi`]) talks to
//! `/dev/tty` directly; tests inject a scripted implementation.

use std::sync::Arc;

use age::secrecy::SecretString;

/// Why a decryption is requested. Shown to the user before anything is unlocked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecryptPurpose {
    /// The command line that wants the secrets, e.g. `stand exec dev -- npm start`.
    pub command: String,
    /// The environment the secrets belong to, if there is a single one.
    pub environment: Option<String>,
    /// Number of encrypted values that will be decrypted.
    pub secret_count: usize,
}

impl DecryptPurpose {
    /// Creates a purpose with an unknown (zero) secret count.
    pub fn new(command: impl Into<String>, environment: Option<&str>) -> Self {
        Self {
            command: command.into(),
            environment: environment.map(str::to_string),
            secret_count: 0,
        }
    }

    /// Human-readable banner shown before unlocking.
    pub fn banner(&self) -> String {
        let env = match &self.environment {
            Some(e) => format!(" for environment '{}'", e),
            None => String::new(),
        };
        format!(
            "\u{1F510} stand: `{}` wants to decrypt {} secret(s){}.",
            self.command, self.secret_count, env
        )
    }
}

/// Interaction surface used while unlocking. Implementations must not reuse
/// stdin/stdout of the process (they may be attached to an agent).
pub trait UnlockUi: Send + Sync {
    /// Shows an informational message.
    fn show(&self, message: &str);
    /// Asks for a secret without echo. `None` means cancelled or unavailable.
    fn request_passphrase(&self, prompt: &str) -> Option<SecretString>;
    /// Asks a yes/no question. `None` means the question could not be asked.
    fn confirm(&self, message: &str) -> Option<bool>;
}

/// Bridges [`UnlockUi`] to age plugin callbacks.
#[derive(Clone)]
pub(crate) struct UiCallbacks(pub Arc<dyn UnlockUi>);

impl age::Callbacks for UiCallbacks {
    fn display_message(&self, message: &str) {
        self.0.show(message);
    }

    fn confirm(&self, message: &str, _yes: &str, _no: Option<&str>) -> Option<bool> {
        self.0.confirm(message)
    }

    fn request_public_string(&self, _description: &str) -> Option<String> {
        None
    }

    fn request_passphrase(&self, description: &str) -> Option<SecretString> {
        self.0.request_passphrase(description)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_banner_with_environment() {
        let mut p = DecryptPurpose::new("stand exec dev -- npm start", Some("dev"));
        p.secret_count = 3;
        let b = p.banner();
        assert!(b.contains("`stand exec dev -- npm start`"));
        assert!(b.contains("3 secret(s)"));
        assert!(b.contains("environment 'dev'"));
    }

    #[test]
    fn test_banner_without_environment() {
        let mut p = DecryptPurpose::new("stand encrypt disable", None);
        p.secret_count = 1;
        assert!(!p.banner().contains("environment"));
    }
}
