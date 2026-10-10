// Integration tests for the `stand` shell wrapper function.
//
// Each test sources wrapper_function() into a real shell (bash, zsh, fish) and
// runs it against a fake `stand` binary that writes render() output to fd 3.
// Shells that are not installed are skipped.
#![cfg(unix)]

use stand::shell::{render, wrapper_function, ShellKind, ShellUpdate};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const NASTY_VALUES: &[&str] = &[
    "plain",
    "it's",
    "say \"hi\"",
    "back\\slash",
    "\\'",
    "$(echo pwned)",
    "`echo pwned`",
    "a;b && c | d",
    "line1\nline2",
    "trailing newline\n",
    "\n\nleading newlines",
    "héllo 日本語 🎉",
    "$HOME ${HOME} $?",
    "",
];

fn find_shell(name: &str) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).map(|d| d.join(name)).collect())
        .unwrap_or_default();
    candidates.push(PathBuf::from("/opt/homebrew/bin").join(name));
    candidates.into_iter().find(|p| p.is_file())
}

fn shells() -> Vec<(&'static str, ShellKind, PathBuf)> {
    let mut v = Vec::new();
    for (name, kind) in [
        ("bash", ShellKind::Posix),
        ("zsh", ShellKind::Posix),
        ("fish", ShellKind::Fish),
    ] {
        match find_shell(name) {
            Some(p) => v.push((name, kind, p)),
            None => eprintln!("skipping {name}: not installed"),
        }
    }
    v
}

struct Fixture {
    _dir: tempfile::TempDir,
    bin: PathBuf,
    payload: PathBuf,
    wrapper: PathBuf,
}

/// Create a fake stand binary inside `subdir` and the wrapper file for `kind`.
fn fixture(kind: ShellKind, subdir: &str, updates: &[ShellUpdate]) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let bindir = dir.path().join(subdir);
    std::fs::create_dir_all(&bindir).unwrap();
    let bin = bindir.join("stand");
    std::fs::write(
        &bin,
        r#"#!/bin/sh
case "$1" in
  set|unset)
    echo "normal message"
    if [ "$STAND_SHELL_FD" = 3 ] && [ "$STAND_SHELL_KIND" != "" ]; then
      cat "$PAYLOAD" >&3
    fi ;;
  *) echo "plain: $*" ;;
esac
exit "${FAKE_RC:-0}"
"#,
    )
    .unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();

    let payload = dir.path().join("payload");
    let text: String = updates.iter().map(|u| render(u, kind).unwrap()).collect();
    std::fs::write(&payload, text).unwrap();

    let wrapper = dir.path().join("wrapper");
    std::fs::write(&wrapper, wrapper_function(kind, bin.to_str().unwrap())).unwrap();
    Fixture {
        _dir: dir,
        bin,
        payload,
        wrapper,
    }
}

fn run(shell: &Path, kind: ShellKind, fx: &Fixture, body: &str, rc: Option<&str>) -> Output {
    let mut cmd = Command::new(shell);
    match (shell.file_name().unwrap().to_str().unwrap(), kind) {
        ("fish", _) => {
            cmd.args(["--no-config", "-c"]);
            cmd.arg(format!("source '{}'; {}", fx.wrapper.display(), body));
        }
        ("zsh", _) => {
            cmd.args(["-f", "-c"]);
            cmd.arg(format!(". '{}'; {}", fx.wrapper.display(), body));
        }
        _ => {
            cmd.args(["--norc", "--noprofile", "-c"]);
            cmd.arg(format!(". '{}'; {}", fx.wrapper.display(), body));
        }
    }
    cmd.env("PAYLOAD", &fx.payload);
    cmd.env_remove("KEY");
    if let Some(rc) = rc {
        cmd.env("FAKE_RC", rc);
    }
    cmd.output().unwrap()
}

/// Shell snippet printing KEY as `[value]`, or `UNSET`.
fn print_key(kind: ShellKind) -> &'static str {
    match kind {
        ShellKind::Posix => r#"printf '[%s]' "${KEY-UNSET}""#,
        ShellKind::Fish => "if set -q KEY; printf '[%s]' \"$KEY\"; else; printf '[UNSET]'; end",
    }
}

fn set_update(v: &str) -> ShellUpdate {
    ShellUpdate::Set {
        key: "KEY".into(),
        value: v.into(),
    }
}

#[test]
fn test_wrapper_set_round_trips_nasty_values() {
    for (name, kind, path) in shells() {
        for v in NASTY_VALUES {
            let fx = fixture(kind, "bin", &[set_update(v)]);
            let body = format!("stand set dev KEY; {}", print_key(kind));
            let out = run(&path, kind, &fx, &body, None);
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert_eq!(
                stdout,
                format!("[{v}]"),
                "{name}: value {v:?} did not round-trip (stderr: {stderr})"
            );
            assert!(
                stderr.contains("normal message"),
                "{name}: normal message should go to stderr, got {stderr:?}"
            );
            assert!(!stdout.contains("normal message"));
        }
    }
}

#[test]
fn test_wrapper_unset_removes_variable() {
    for (name, kind, path) in shells() {
        let fx = fixture(kind, "bin", &[ShellUpdate::Unset { key: "KEY".into() }]);
        let export = match kind {
            ShellKind::Posix => "export KEY=old",
            ShellKind::Fish => "set -gx KEY old",
        };
        let body = format!("{export}; stand unset dev KEY; {}", print_key(kind));
        let out = run(&path, kind, &fx, &body, None);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "[UNSET]",
            "{name}: unset failed"
        );
    }
}

#[test]
fn test_wrapper_set_exports_to_child_processes() {
    for (name, kind, path) in shells() {
        let fx = fixture(kind, "bin", &[set_update("exported")]);
        let body = "stand set dev KEY; env | grep '^KEY='";
        let out = run(&path, kind, &fx, body, None);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            "KEY=exported",
            "{name}: variable was not exported"
        );
    }
}

#[test]
fn test_wrapper_passes_other_subcommands_through_unchanged() {
    for (name, kind, path) in shells() {
        let fx = fixture(kind, "bin", &[set_update("x")]);
        let body = format!("stand list --all; {}", print_key(kind));
        let out = run(&path, kind, &fx, &body, None);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "plain: list --all\n[UNSET]",
            "{name}: pass-through stdout changed"
        );
    }
}

#[test]
fn test_wrapper_propagates_exit_code() {
    for (name, kind, path) in shells() {
        let fx = fixture(kind, "bin", &[set_update("x")]);
        let status_var = match kind {
            ShellKind::Posix => "$?",
            ShellKind::Fish => "$status",
        };
        for sub in ["set dev KEY", "list"] {
            let body = format!("stand {sub}; echo rc={status_var}");
            let out = run(&path, kind, &fx, &body, Some("3"));
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(
                stdout.contains("rc=3"),
                "{name}: `{sub}` exit code lost: {stdout:?}"
            );
        }
    }
}

#[test]
fn test_wrapper_quotes_bin_path_with_space_and_single_quote() {
    for (name, kind, path) in shells() {
        let fx = fixture(kind, "my dir's bin", &[set_update("ok")]);
        assert!(fx.bin.to_str().unwrap().contains("dir's"));
        let body = format!("stand set dev KEY; {}", print_key(kind));
        let out = run(&path, kind, &fx, &body, None);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "[ok]",
            "{name}: odd bin path failed (stderr: {})",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
