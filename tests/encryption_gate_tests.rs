//! CLI integration tests for the decryption gate.
//!
//! Every test runs the real binary against a temporary project with
//! `XDG_CONFIG_HOME` and `HOME` pointing into a temporary directory, so the
//! user's real configuration and keys are never touched.

use age::secrecy::SecretString;
use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;
use tempfile::{tempdir, TempDir};

const SECRET: &str = "super-secret-value";

struct Project {
    _tmp: TempDir,
    dir: PathBuf,
    xdg: PathBuf,
    home: PathBuf,
    private_key: String,
}

/// Creates a project with one encrypted variable (`TOKEN`) and a plain one.
/// With `wrapped` the private key is stored passphrase-wrapped in the (temporary)
/// config dir; with `legacy` it is written to `.stand.keys`.
fn project(wrapped: bool, legacy: bool) -> Project {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join("project");
    let xdg = tmp.path().join("xdg");
    let home = tmp.path().join("home");
    for d in [&dir, &xdg, &home] {
        fs::create_dir_all(d).unwrap();
    }

    let key_pair = stand::crypto::generate_key_pair();
    let recipient = key_pair.to_recipient().unwrap();
    let encrypted = stand::crypto::encrypt_value(SECRET, &recipient).unwrap();
    fs::write(
        dir.join(".stand.toml"),
        format!(
            "version = \"1.0\"\n\n[encryption]\npublic_key = \"{}\"\n\n[environments.dev]\ndescription = \"Dev\"\nPLAIN = \"visible\"\nTOKEN = \"{}\"\n",
            key_pair.public_key, encrypted
        ),
    )
    .unwrap();

    if wrapped {
        let identity = key_pair.to_identity().unwrap();
        stand::crypto::store::write_passphrase_identity_in(
            &xdg.join("stand"),
            &key_pair.public_key,
            &identity,
            &SecretString::from("test-passphrase".to_string()),
            Some(8),
        )
        .unwrap();
    }
    if legacy {
        stand::crypto::keys::save_private_key(&dir.join(".stand.keys"), &key_pair.private_key)
            .unwrap();
    }

    Project {
        _tmp: tmp,
        dir,
        xdg,
        home,
        private_key: key_pair.private_key,
    }
}

impl Project {
    fn stand(&self) -> Command {
        let mut cmd = Command::cargo_bin("stand").unwrap();
        cmd.current_dir(&self.dir)
            .env("XDG_CONFIG_HOME", &self.xdg)
            .env("HOME", &self.home)
            .env_remove("CI")
            .env_remove("STAND_KEY_SOURCE")
            .env_remove("STAND_PRIVATE_KEY")
            .env_remove("STAND_UNLOCK")
            .env_remove("STAND_FORCE_NON_TTY")
            .env_remove("STAND_ACTIVE")
            .env_remove("STAND_ENVIRONMENT");
        cmd
    }
}

fn stderr_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn test_get_fails_without_terminal_when_identity_is_wrapped() {
    let p = project(true, false);
    let out = p
        .stand()
        .env("STAND_FORCE_NON_TTY", "1")
        .args(["get", "dev", "TOKEN"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(stderr_of(&out).contains("interactive terminal"));
    assert!(!stdout_of(&out).contains(SECRET));
}

#[test]
fn test_exec_fails_without_terminal_when_identity_is_wrapped() {
    let p = project(true, false);
    let out = p
        .stand()
        .env("STAND_FORCE_NON_TTY", "1")
        .args(["exec", "dev", "--", "printenv", "TOKEN"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(stderr_of(&out).contains("interactive terminal"));
    assert!(!stdout_of(&out).contains(SECRET));
}

#[test]
fn test_shell_fails_without_terminal_when_identity_is_wrapped() {
    let p = project(true, false);
    let out = p
        .stand()
        .env("STAND_FORCE_NON_TTY", "1")
        .args(["shell", "dev", "--yes", "--shell", "/bin/sh"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(stderr_of(&out).contains("interactive terminal"));
}

#[test]
fn test_plain_values_work_without_terminal() {
    let p = project(true, false);
    let out = p
        .stand()
        .env("STAND_FORCE_NON_TTY", "1")
        .args(["get", "dev", "PLAIN"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(stdout_of(&out).trim(), "visible");
}

#[test]
fn test_get_decrypts_with_env_key_in_ci() {
    let p = project(true, false);
    let out = p
        .stand()
        .env("CI", "true")
        .env("STAND_PRIVATE_KEY", &p.private_key)
        .env("STAND_FORCE_NON_TTY", "1")
        .args(["get", "dev", "TOKEN"])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    assert_eq!(stdout_of(&out).trim(), SECRET);
}

#[test]
fn test_exec_decrypts_with_env_key_in_ci() {
    let p = project(false, false);
    let out = p
        .stand()
        .env("CI", "true")
        .env("STAND_PRIVATE_KEY", &p.private_key)
        .env("STAND_FORCE_NON_TTY", "1")
        .args(["exec", "dev", "--", "printenv", "TOKEN"])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    assert_eq!(stdout_of(&out).trim(), SECRET);
}

#[test]
fn test_env_key_without_opt_in_is_rejected() {
    let p = project(true, false);
    let out = p
        .stand()
        .env("STAND_PRIVATE_KEY", &p.private_key)
        .env("STAND_FORCE_NON_TTY", "1")
        .args(["get", "dev", "TOKEN"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(stderr_of(&out).contains("only used when CI=true or STAND_KEY_SOURCE=env"));
    assert!(!stdout_of(&out).contains(SECRET));
}

#[test]
fn test_legacy_keys_file_works_and_warns() {
    let p = project(false, true);
    let out = p
        .stand()
        .env("STAND_FORCE_NON_TTY", "1")
        .args(["get", "dev", "TOKEN"])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    assert_eq!(stdout_of(&out).trim(), SECRET);
    let err = stderr_of(&out);
    assert!(err.contains("legacy .stand.keys"));
    assert!(err.contains("stand encrypt migrate"));

    let out = p
        .stand()
        .env("STAND_FORCE_NON_TTY", "1")
        .args(["exec", "dev", "--", "printenv", "TOKEN"])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    assert_eq!(stdout_of(&out).trim(), SECRET);
    assert!(stderr_of(&out).contains("legacy .stand.keys"));
}

#[test]
fn test_encrypt_enable_requires_terminal() {
    let p = project(false, false);
    fs::write(
        p.dir.join(".stand.toml"),
        "version = \"1.0\"\n\n[environments.dev]\ndescription = \"Dev\"\n",
    )
    .unwrap();
    let out = p
        .stand()
        .env("STAND_FORCE_NON_TTY", "1")
        .args(["encrypt", "enable"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(stderr_of(&out).contains("interactive terminal"));
    assert!(!p.dir.join(".stand.keys").exists());
}
