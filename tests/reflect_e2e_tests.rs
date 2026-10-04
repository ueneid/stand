// End-to-end tests for `stand set` / `stand unset` reflection into a shell.
//
// A real shell (bash, zsh, fish; skipped when absent) runs non-interactively
// with the wrapper function sourced and the real `stand` binary behind it.
#![cfg(unix)]

use stand::shell::{wrapper_function, ShellKind};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

struct Case<'a> {
    config: &'a str,
    active_env: &'a str,
    /// Value exported before the commands run, if any: (key, value).
    preset: Option<(&'a str, &'a str)>,
    /// Commands run after the wrapper is defined, in the shell's own syntax
    /// for `stand ...` (identical in all shells).
    commands: &'a [&'a str],
    /// Variable printed at the end.
    probe: &'a str,
    extra_env: &'a [(&'a str, &'a str)],
}

/// Runs the case in one shell and returns the probe as `Some(value)` or None if unset.
fn run(kind: ShellKind, shell: &Path, case: &Case) -> Option<String> {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fs::write(dir.join(".stand.toml"), case.config).unwrap();
    let cfg = dir.join("cfg");
    fs::create_dir_all(&cfg).unwrap();

    let mut script = wrapper_function(kind, env!("CARGO_BIN_EXE_stand"));
    if let Some((k, v)) = case.preset {
        match kind {
            ShellKind::Posix => script.push_str(&format!("export {k}='{v}'\n")),
            ShellKind::Fish => script.push_str(&format!("set -gx {k} '{v}'\n")),
        }
    }
    for c in case.commands {
        script.push_str(c);
        script.push('\n');
    }
    let p = case.probe;
    match kind {
        ShellKind::Posix => script.push_str(&format!(
            "if [ -n \"${{{p}+x}}\" ]; then printf 'SET[%s]' \"${p}\"; else printf 'UNSET'; fi\n"
        )),
        ShellKind::Fish => script.push_str(&format!(
            "if set -q {p}; printf 'SET[%s]' \"${p}\"; else; printf 'UNSET'; end\n"
        )),
    }

    let mut cmd = Command::new(shell);
    cmd.arg("-c")
        .arg(&script)
        .current_dir(dir)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", &cfg)
        .env("STAND_ACTIVE", "1")
        .env("STAND_ENVIRONMENT", case.active_env)
        .env("STAND_PROJECT_ROOT", dir);
    for (k, v) in case.extra_env {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        stdout.contains("SET[") || stdout.contains("UNSET"),
        "probe missing; stdout={stdout:?} stderr={stderr:?}"
    );
    // "UNSET" contains "SET" but never "SET[".
    stdout.rfind("SET[").map(|i| {
        let rest = &stdout[i + 4..];
        let end = rest.rfind(']').expect("closing bracket");
        rest[..end].to_string()
    })
}

fn check(case: &Case, expected: Option<&str>) {
    for (name, kind, path) in shells() {
        let got = run(kind, &path, case);
        assert_eq!(got.as_deref(), expected, "shell {name}");
    }
}

const BASIC: &str = r#"version = "1.0"

[environments.dev]
description = "Dev"
"#;

#[test]
fn test_set_reflects_plain_value() {
    check(
        &Case {
            config: BASIC,
            active_env: "dev",
            preset: None,
            commands: &["stand set dev FOO bar"],
            probe: "FOO",
            extra_env: &[],
        },
        Some("bar"),
    );
}

#[test]
fn test_set_on_parent_does_not_change_overridden_value() {
    let config = r#"version = "1.0"

[environments.dev]
description = "Dev"
FOO = "parent"

[environments.child]
description = "Child"
extends = "dev"
FOO = "child-value"
"#;
    check(
        &Case {
            config,
            active_env: "child",
            preset: Some(("FOO", "child-value")),
            commands: &["stand set dev FOO x"],
            probe: "FOO",
            extra_env: &[],
        },
        Some("child-value"),
    );
}

#[test]
fn test_unset_falls_back_to_common_value() {
    let config = r#"version = "1.0"

[common]
FOO = "base"

[environments.dev]
description = "Dev"
FOO = "dev-value"
"#;
    check(
        &Case {
            config,
            active_env: "dev",
            preset: Some(("FOO", "dev-value")),
            commands: &["stand unset dev FOO"],
            probe: "FOO",
            extra_env: &[],
        },
        Some("base"),
    );
}

#[test]
fn test_unset_without_fallback_unsets_variable() {
    let config = r#"version = "1.0"

[environments.dev]
description = "Dev"
FOO = "dev-value"
"#;
    check(
        &Case {
            config,
            active_env: "dev",
            preset: Some(("FOO", "dev-value")),
            commands: &["stand unset dev FOO"],
            probe: "FOO",
            extra_env: &[],
        },
        None,
    );
}

#[test]
fn test_set_encrypted_reflects_plaintext_without_unlock() {
    let key_pair = stand::crypto::generate_key_pair();
    let config = format!(
        "version = \"1.0\"\n\n[encryption]\npublic_key = \"{}\"\n\n[environments.dev]\ndescription = \"Dev\"\n",
        key_pair.public_key
    );
    // No CI / private key is provided: any attempt to unlock would fail.
    check(
        &Case {
            config: &config,
            active_env: "dev",
            preset: None,
            commands: &["stand set -e dev SECRET 'v a$l'"],
            probe: "SECRET",
            extra_env: &[],
        },
        Some("v a$l"),
    );
}

#[test]
fn test_unset_with_encrypted_fallback_reflects_decrypted_parent_value() {
    let key_pair = stand::crypto::generate_key_pair();
    let recipient = key_pair.to_recipient().unwrap();
    let encrypted = stand::crypto::encrypt_value("parent-secret", &recipient).unwrap();
    let config = format!(
        "version = \"1.0\"\n\n[encryption]\npublic_key = \"{}\"\n\n[environments.base]\ndescription = \"Base\"\nKEY = \"{}\"\n\n[environments.dev]\ndescription = \"Dev\"\nextends = \"base\"\nKEY = \"override\"\n",
        key_pair.public_key, encrypted
    );
    check(
        &Case {
            config: &config,
            active_env: "dev",
            preset: Some(("KEY", "override")),
            commands: &["stand unset dev KEY"],
            probe: "KEY",
            extra_env: &[
                ("CI", "true"),
                ("STAND_PRIVATE_KEY", key_pair.private_key.as_str()),
            ],
        },
        Some("parent-secret"),
    );
}

#[test]
fn test_unset_with_undecryptable_fallback_unsets_variable() {
    let key_pair = stand::crypto::generate_key_pair();
    let recipient = key_pair.to_recipient().unwrap();
    let encrypted = stand::crypto::encrypt_value("parent-secret", &recipient).unwrap();
    let config = format!(
        "version = \"1.0\"\n\n[encryption]\npublic_key = \"{}\"\n\n[environments.base]\ndescription = \"Base\"\nKEY = \"{}\"\n\n[environments.dev]\ndescription = \"Dev\"\nextends = \"base\"\nKEY = \"override\"\n",
        key_pair.public_key, encrypted
    );
    // No key available: the fallback cannot be decrypted, so the stale value is removed.
    check(
        &Case {
            config: &config,
            active_env: "dev",
            preset: Some(("KEY", "override")),
            commands: &["stand unset dev KEY"],
            probe: "KEY",
            extra_env: &[("STAND_FORCE_NON_TTY", "1")],
        },
        None,
    );
}
