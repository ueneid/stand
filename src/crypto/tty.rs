//! Controlling-terminal UI.
//!
//! Every interaction opens `/dev/tty` explicitly. Processes without a
//! controlling terminal (such as an AI agent's shell tool) cannot open it, so
//! they cannot approve a decryption.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};

use age::secrecy::SecretString;

use super::ui::UnlockUi;
use super::CryptoError;

const TTY_PATH: &str = "/dev/tty";

/// [`UnlockUi`] backed by `/dev/tty`.
pub struct TtyUi;

fn open_tty() -> std::io::Result<File> {
    OpenOptions::new().read(true).write(true).open(TTY_PATH)
}

impl TtyUi {
    /// Fails with [`CryptoError::NoTerminal`] when `/dev/tty` cannot be opened.
    pub fn new() -> Result<Self, CryptoError> {
        open_tty()
            .map(|_| TtyUi)
            .map_err(|_| CryptoError::NoTerminal)
    }
}

impl UnlockUi for TtyUi {
    fn show(&self, message: &str) {
        if let Ok(mut tty) = open_tty() {
            let _ = writeln!(tty, "{}", message);
        }
    }

    fn request_passphrase(&self, prompt: &str) -> Option<SecretString> {
        // rpassword opens /dev/tty for both the prompt and the (non-echoing) input.
        rpassword::prompt_password(format!("{} ", prompt))
            .ok()
            .map(SecretString::from)
    }

    fn confirm(&self, message: &str) -> Option<bool> {
        let mut tty = open_tty().ok()?;
        write!(tty, "{} [y/N] ", message).ok()?;
        tty.flush().ok()?;
        let mut line = String::new();
        BufReader::new(tty.try_clone().ok()?)
            .read_line(&mut line)
            .ok()?;
        Some(matches!(line.trim().to_lowercase().as_str(), "y" | "yes"))
    }
}
