// Shell reflection module
//
// Lets `stand set` / `stand unset` update the environment of the enclosing
// `stand shell`. The subshell defines a `stand` function that runs the real
// binary with fd 3 captured and evals what the binary wrote to it.
// No temporary files are involved on the data path.
//
// Limitation: the wrapper is only installed for bash, zsh and fish. Other
// shells get no wrapper, so `stand set` does not affect their environment.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Environment variable naming the fd the binary must write shell code to.
pub const STAND_SHELL_FD: &str = "STAND_SHELL_FD";
/// Environment variable naming the shell dialect the wrapper expects.
pub const STAND_SHELL_KIND: &str = "STAND_SHELL_KIND";

/// Shell dialect used to render updates. `Posix` covers bash and zsh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Posix,
    Fish,
}

/// A single environment change to reflect into the parent shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellUpdate {
    Set { key: String, value: String },
    Unset { key: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ReflectError {
    #[error("invalid variable name for shell update: {0}")]
    InvalidName(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Context describing the enclosing `stand shell`, read from the environment.
pub struct ReflectContext {
    pub environment: String,
    pub project_root: PathBuf,
    pub kind: ShellKind,
    fd: i32,
}

fn is_valid_name(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_') && !key.starts_with("STAND_")
}

/// Render one update as a single shell line ending with a newline.
///
/// Rejects keys not matching `^[A-Za-z_][A-Za-z0-9_]*$` or starting with `STAND_`.
pub fn render(update: &ShellUpdate, kind: ShellKind) -> Result<String, ReflectError> {
    let key = match update {
        ShellUpdate::Set { key, .. } | ShellUpdate::Unset { key } => key,
    };
    if !is_valid_name(key) {
        return Err(ReflectError::InvalidName(key.clone()));
    }
    Ok(match (update, kind) {
        (ShellUpdate::Set { key, value }, ShellKind::Posix) => {
            format!("export {}={}\n", key, quote_posix(value))
        }
        (ShellUpdate::Set { key, value }, ShellKind::Fish) => {
            format!("set -gx {} {}\n", key, quote_fish(value))
        }
        (ShellUpdate::Unset { key }, ShellKind::Posix) => format!("unset {}\n", key),
        (ShellUpdate::Unset { key }, ShellKind::Fish) => format!("set -e {}\n", key),
    })
}

/// Single-quote for POSIX shells: `'` becomes `'\''`.
fn quote_posix(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Single-quote for fish: `\` becomes `\\` and `'` becomes `\'`.
fn quote_fish(s: &str) -> String {
    format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
}

impl ReflectContext {
    /// Some only if STAND_ACTIVE=1, STAND_ENVIRONMENT, STAND_PROJECT_ROOT,
    /// STAND_SHELL_FD="3" and a valid STAND_SHELL_KIND are all present.
    pub fn from_env() -> Option<Self> {
        if std::env::var("STAND_ACTIVE").ok()? != "1" {
            return None;
        }
        let environment = std::env::var("STAND_ENVIRONMENT").ok()?;
        let project_root = PathBuf::from(std::env::var("STAND_PROJECT_ROOT").ok()?);
        if std::env::var(STAND_SHELL_FD).ok()? != "3" {
            return None;
        }
        let kind = match std::env::var(STAND_SHELL_KIND).ok()?.as_str() {
            "posix" | "bash" | "zsh" => ShellKind::Posix,
            "fish" => ShellKind::Fish,
            _ => return None,
        };
        Some(Self {
            environment,
            project_root,
            kind,
            fd: 3,
        })
    }

    /// Whether `project_dir` is the same project as STAND_PROJECT_ROOT.
    pub fn applies_to(&self, project_dir: &Path) -> bool {
        match (self.project_root.canonicalize(), project_dir.canonicalize()) {
            (Ok(a), Ok(b)) => a == b,
            _ => self.project_root == project_dir,
        }
    }

    /// Render all updates and write them to the fd. Writes nothing if any
    /// update is invalid or the slice is empty.
    pub fn emit(&self, updates: &[ShellUpdate]) -> Result<(), ReflectError> {
        if updates.is_empty() {
            return Ok(());
        }
        let mut out = String::new();
        for u in updates {
            out.push_str(&render(u, self.kind)?);
        }
        self.write_fd(out.as_bytes())
    }

    #[cfg(unix)]
    fn write_fd(&self, data: &[u8]) -> Result<(), ReflectError> {
        use std::mem::ManuallyDrop;
        use std::os::fd::FromRawFd;
        // ManuallyDrop: we must not close an fd we do not own. A closed fd
        // surfaces as an EBADF io::Error from write_all.
        let mut file = ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(self.fd) });
        file.write_all(data)?;
        file.flush()?;
        Ok(())
    }

    #[cfg(not(unix))]
    fn write_fd(&self, _data: &[u8]) -> Result<(), ReflectError> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "shell reflection is only supported on unix",
        )
        .into())
    }
}

/// Shell function text that wraps `stand` so set/unset update the current shell.
///
/// `stand_bin` is quoted safely for the target shell.
pub fn wrapper_function(kind: ShellKind, stand_bin: &str) -> String {
    match kind {
        ShellKind::Posix => {
            let bin = quote_posix(stand_bin);
            format!(
                r#"stand() {{
  case "$1" in
    set|unset)
      local __stand_out __stand_rc
      __stand_out="$(STAND_SHELL_FD=3 STAND_SHELL_KIND=posix {bin} "$@" 3>&1 1>&2)"
      __stand_rc=$?
      if [ -n "$__stand_out" ]; then eval "$__stand_out"; fi
      return $__stand_rc ;;
    *) {bin} "$@" ;;
  esac
}}
"#
            )
        }
        ShellKind::Fish => {
            let bin = quote_fish(stand_bin);
            format!(
                r#"function stand
    if contains -- "$argv[1]" set unset
        env STAND_SHELL_FD=3 STAND_SHELL_KIND=fish {bin} $argv 3>&1 1>&2 | source
        return $pipestatus[1]
    else
        {bin} $argv
    end
end
"#
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn set(k: &str, v: &str) -> ShellUpdate {
        ShellUpdate::Set {
            key: k.into(),
            value: v.into(),
        }
    }
    fn unset(k: &str) -> ShellUpdate {
        ShellUpdate::Unset { key: k.into() }
    }

    #[test]
    fn test_render_posix_set_quotes_single_quote() {
        assert_eq!(
            render(&set("A", "it's"), ShellKind::Posix).unwrap(),
            "export A='it'\\''s'\n"
        );
    }

    #[test]
    fn test_render_posix_unset() {
        assert_eq!(render(&unset("A"), ShellKind::Posix).unwrap(), "unset A\n");
    }

    #[test]
    fn test_render_fish_set_escapes_backslash_and_quote() {
        assert_eq!(
            render(&set("A", "a\\b'c"), ShellKind::Fish).unwrap(),
            "set -gx A 'a\\\\b\\'c'\n"
        );
    }

    #[test]
    fn test_render_fish_unset() {
        assert_eq!(render(&unset("A"), ShellKind::Fish).unwrap(), "set -e A\n");
    }

    #[test]
    fn test_render_rejects_invalid_names() {
        for bad in ["A-B", "1X", "STAND_FOO", "", "A B", "A=B", "A;rm"] {
            for kind in [ShellKind::Posix, ShellKind::Fish] {
                assert!(matches!(
                    render(&set(bad, "v"), kind),
                    Err(ReflectError::InvalidName(_))
                ));
                assert!(matches!(
                    render(&unset(bad), kind),
                    Err(ReflectError::InvalidName(_))
                ));
            }
        }
    }

    #[test]
    fn test_render_accepts_underscore_digits_and_stand_lookalike() {
        assert!(render(&set("_a1", "v"), ShellKind::Posix).is_ok());
        assert!(render(&set("STANDARD", "v"), ShellKind::Posix).is_ok());
    }

    #[test]
    fn test_wrapper_function_quotes_bin() {
        let w = wrapper_function(ShellKind::Posix, "/a b/it's");
        assert!(w.contains("'/a b/it'\\''s'"));
        let f = wrapper_function(ShellKind::Fish, "/a b/it's\\x");
        assert!(f.contains("'/a b/it\\'s\\\\x'"));
    }

    fn ctx(fd: i32) -> ReflectContext {
        ReflectContext {
            environment: "dev".into(),
            project_root: PathBuf::from("/nonexistent/x"),
            kind: ShellKind::Posix,
            fd,
        }
    }

    #[test]
    fn test_emit_empty_writes_nothing_and_ok() {
        assert!(ctx(987654).emit(&[]).is_ok());
    }

    #[test]
    fn test_emit_invalid_name_errors_before_write() {
        let r = ctx(987654).emit(&[set("A", "1"), set("1X", "2")]);
        assert!(matches!(r, Err(ReflectError::InvalidName(_))));
    }

    #[cfg(unix)]
    #[test]
    fn test_emit_closed_fd_returns_io_error() {
        let r = ctx(987654).emit(&[set("A", "1")]);
        assert!(matches!(r, Err(ReflectError::Io(_))));
    }

    #[test]
    fn test_applies_to_plain_compare_when_missing() {
        let c = ctx(3);
        assert!(c.applies_to(Path::new("/nonexistent/x")));
        assert!(!c.applies_to(Path::new("/nonexistent/y")));
    }

    #[test]
    fn test_applies_to_canonicalizes() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("p");
        std::fs::create_dir(&sub).unwrap();
        let mut c = ctx(3);
        c.project_root = sub.join("..").join("p");
        assert!(c.applies_to(&sub));
    }

    fn clear_env() {
        for k in [
            "STAND_ACTIVE",
            "STAND_ENVIRONMENT",
            "STAND_PROJECT_ROOT",
            STAND_SHELL_FD,
            STAND_SHELL_KIND,
        ] {
            std::env::remove_var(k);
        }
    }

    #[test]
    #[serial]
    fn test_from_env_requires_all_vars() {
        clear_env();
        assert!(ReflectContext::from_env().is_none());
        std::env::set_var("STAND_ACTIVE", "1");
        std::env::set_var("STAND_ENVIRONMENT", "dev");
        std::env::set_var("STAND_PROJECT_ROOT", "/p");
        std::env::set_var(STAND_SHELL_FD, "3");
        assert!(ReflectContext::from_env().is_none()); // no kind
        std::env::set_var(STAND_SHELL_KIND, "fish");
        let c = ReflectContext::from_env().unwrap();
        assert_eq!(c.kind, ShellKind::Fish);
        assert_eq!(c.environment, "dev");
        std::env::set_var(STAND_SHELL_KIND, "zsh");
        assert_eq!(ReflectContext::from_env().unwrap().kind, ShellKind::Posix);
        std::env::set_var(STAND_SHELL_KIND, "tcsh");
        assert!(ReflectContext::from_env().is_none());
        std::env::set_var(STAND_SHELL_KIND, "bash");
        std::env::set_var(STAND_SHELL_FD, "4");
        assert!(ReflectContext::from_env().is_none());
        std::env::set_var(STAND_SHELL_FD, "3");
        std::env::set_var("STAND_ACTIVE", "0");
        assert!(ReflectContext::from_env().is_none());
        clear_env();
    }
}
