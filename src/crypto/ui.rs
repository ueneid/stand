//! User interaction abstraction for the unlock gate.
//!
//! Decryption must be approved by a human, so every prompt goes through
//! [`UnlockUi`]. The production implementation ([`super::tty::TtyUi`]) talks to
//! `/dev/tty` directly; tests inject a scripted implementation.

use std::sync::Arc;

use age::secrecy::SecretString;

/// Maximum number of characters of one interpolated field.
pub(crate) const MAX_FIELD_CHARS: usize = 200;

/// Makes `input` safe to print on a terminal.
///
/// Control characters (C0, DEL, C1, ESC) are replaced by visible escapes and the
/// result is truncated to [`MAX_FIELD_CHARS`] characters plus an ellipsis.
pub fn sanitize_for_terminal(input: &str) -> String {
    sanitize_with_limit(input, MAX_FIELD_CHARS)
}

/// [`sanitize_for_terminal`] with an explicit character limit.
pub(crate) fn sanitize_with_limit(input: &str, limit: usize) -> String {
    let mut out = String::new();
    for (count, c) in input.chars().enumerate() {
        if count >= limit {
            out.push('\u{2026}');
            break;
        }
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

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
            Some(e) => format!(" for environment '{}'", sanitize_for_terminal(e)),
            None => String::new(),
        };
        format!(
            "\u{1F510} stand: `{}` wants to decrypt {} secret(s){}.",
            sanitize_for_terminal(&self.command),
            self.secret_count,
            env
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

/// Plugins are untrusted: their messages are sanitized before being shown.
const PLUGIN_MESSAGE_CHARS: usize = 1000;

/// Bridges [`UnlockUi`] to age plugin callbacks.
#[derive(Clone)]
pub(crate) struct UiCallbacks(pub Arc<dyn UnlockUi>);

impl age::Callbacks for UiCallbacks {
    fn display_message(&self, message: &str) {
        self.0
            .show(&sanitize_with_limit(message, PLUGIN_MESSAGE_CHARS));
    }

    fn confirm(&self, message: &str, _yes: &str, _no: Option<&str>) -> Option<bool> {
        self.0
            .confirm(&sanitize_with_limit(message, PLUGIN_MESSAGE_CHARS))
    }

    fn request_public_string(&self, _description: &str) -> Option<String> {
        None
    }

    fn request_passphrase(&self, description: &str) -> Option<SecretString> {
        self.0
            .request_passphrase(&sanitize_with_limit(description, PLUGIN_MESSAGE_CHARS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_escapes_terminal_control_sequences() {
        let s = sanitize_for_terminal("a\x1b[2K\rb\nc\td\x7fe\u{85}f");
        assert_eq!(s, "a\\x1b[2K\\rb\\nc\\td\\x7fe\\x85f");
        assert!(!s.chars().any(|c| c.is_control()));
    }

    #[test]
    fn test_sanitize_truncates_long_strings_with_ellipsis() {
        let s = sanitize_for_terminal(&"x".repeat(500));
        assert_eq!(s.chars().count(), MAX_FIELD_CHARS + 1);
        assert!(s.ends_with('\u{2026}'));
        assert_eq!(sanitize_for_terminal("short"), "short");
    }

    #[test]
    fn test_sanitize_keeps_normal_unicode() {
        assert_eq!(
            sanitize_for_terminal("caf\u{e9} \u{1F510}"),
            "caf\u{e9} \u{1F510}"
        );
    }

    #[test]
    fn test_banner_sanitizes_command_and_environment() {
        let mut p = DecryptPurpose::new("evil\x1b[2K\rcmd\n", Some("e\x1bnv"));
        p.secret_count = 1;
        let b = p.banner();
        assert!(!b.contains('\x1b'));
        assert!(!b.contains('\r'));
        assert!(!b.contains('\n'));
        assert!(b.contains("evil\\x1b[2K\\rcmd\\n"));
        assert!(b.contains("e\\x1bnv"));
    }

    #[test]
    fn test_banner_truncates_long_command() {
        let p = DecryptPurpose::new("y".repeat(1000), None);
        assert!(p.banner().chars().count() < 400);
    }

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
