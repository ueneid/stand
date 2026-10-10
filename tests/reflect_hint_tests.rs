// Tests for the hint printed when `stand set` / `stand unset` run inside a
// `stand shell` but cannot update it (called by path, or the shell was started
// by an older stand without the `stand` shell function).

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

const HINT: &str = "this shell was not updated";

const CONFIG: &str = r#"version = "2.0"

[common]
SHARED = "base"

[environments.dev]
description = "Development"
FOO = "old"

[environments.child]
description = "Child"
extends = "dev"
FOO = "child-value"
"#;

/// A `stand` command that looks like it runs inside a `stand shell` for
/// `active_env`, but without the reflect fd the shell function provides.
fn stand_in_shell_without_wrapper(dir: &Path, active_env: &str) -> assert_cmd::Command {
    let mut cmd = cargo_bin_cmd!("stand");
    cmd.current_dir(dir)
        .env("STAND_ACTIVE", "1")
        .env("STAND_ENVIRONMENT", active_env)
        .env("STAND_PROJECT_ROOT", dir)
        .env("XDG_CONFIG_HOME", dir.join("cfg"))
        .env_remove("STAND_SHELL_FD")
        .env_remove("STAND_SHELL_KIND");
    cmd
}

#[test]
fn test_set_by_path_inside_stand_shell_prints_hint() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join(".stand.toml"), CONFIG).unwrap();

    stand_in_shell_without_wrapper(dir.path(), "dev")
        .args(["set", "dev", "FOO", "new"])
        .assert()
        .success()
        .stderr(predicate::str::contains(HINT))
        .stderr(predicate::str::contains("FOO"));
}

#[test]
fn test_unset_by_path_inside_stand_shell_prints_hint() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join(".stand.toml"), CONFIG).unwrap();

    stand_in_shell_without_wrapper(dir.path(), "dev")
        .args(["unset", "dev", "FOO"])
        .assert()
        .success()
        .stderr(predicate::str::contains(HINT));
}

#[test]
fn test_no_hint_when_active_environment_is_unaffected() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join(".stand.toml"), CONFIG).unwrap();

    // `child` overrides FOO, so changing dev's FOO does not change the shell.
    stand_in_shell_without_wrapper(dir.path(), "child")
        .args(["set", "dev", "FOO", "new"])
        .assert()
        .success()
        .stderr(predicate::str::contains(HINT).not());
}

#[test]
fn test_no_hint_outside_stand_shell() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join(".stand.toml"), CONFIG).unwrap();

    cargo_bin_cmd!("stand")
        .current_dir(dir.path())
        .env_remove("STAND_ACTIVE")
        .env_remove("STAND_ENVIRONMENT")
        .env_remove("STAND_PROJECT_ROOT")
        .env_remove("STAND_SHELL_FD")
        .env("XDG_CONFIG_HOME", dir.path().join("cfg"))
        .args(["set", "dev", "FOO", "new"])
        .assert()
        .success()
        .stderr(predicate::str::contains(HINT).not());
}

#[test]
fn test_no_hint_for_stand_shell_of_another_project() {
    let dir = tempdir().unwrap();
    let other = tempdir().unwrap();
    fs::write(dir.path().join(".stand.toml"), CONFIG).unwrap();

    stand_in_shell_without_wrapper(dir.path(), "dev")
        .env("STAND_PROJECT_ROOT", other.path())
        .args(["set", "dev", "FOO", "new"])
        .assert()
        .success()
        .stderr(predicate::str::contains(HINT).not());
}
