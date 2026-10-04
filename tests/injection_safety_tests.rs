//! Blocked variable names are refused when injected by `exec` and `shell`.

use assert_cmd::Command;
use stand::commands::shell::validate_shell_environment;
use std::fs;

fn project(vars: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".stand.toml"),
        format!("version = \"1.0\"\n\n[environments.dev]\ndescription = \"Dev\"\n{vars}"),
    )
    .unwrap();
    dir
}

fn exec(dir: &tempfile::TempDir) -> assert_cmd::assert::Assert {
    Command::cargo_bin("stand")
        .unwrap()
        .current_dir(dir.path())
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join("cfg"))
        .args(["exec", "dev", "--", "true"])
        .assert()
}

#[test]
fn test_exec_refuses_blocked_variables_case_insensitively() {
    let dir = project("LD_PRELOAD = \"/tmp/x.so\"\nbash_env = \"/tmp/x\"\n");
    let out = exec(&dir).failure();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(stderr.contains("LD_PRELOAD"), "{stderr}");
    assert!(stderr.contains("bash_env"), "{stderr}");
    assert!(stderr.contains("inject code"), "{stderr}");
}

#[test]
fn test_exec_warns_on_path_but_runs() {
    let dir = project("PATH = \"/usr/bin:/bin\"\n");
    let out = exec(&dir).success();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(stderr.contains("PATH"), "{stderr}");
    assert!(stderr.contains("warning"), "{stderr}");
}

#[test]
fn test_shell_validation_refuses_blocked_variables() {
    let dir = project("LD_PRELOAD = \"/tmp/x.so\"\nbash_env = \"/tmp/x\"\n");
    let err = validate_shell_environment(dir.path(), "dev", true, Some("/bin/sh".to_string()))
        .unwrap_err()
        .to_string();
    assert!(err.contains("LD_PRELOAD"), "{err}");
    assert!(err.contains("bash_env"), "{err}");
    assert!(err.contains("inject code"), "{err}");
}

#[test]
fn test_shell_validation_accepts_warned_variable() {
    let dir = project("PATH = \"/usr/bin\"\n");
    let env = validate_shell_environment(dir.path(), "dev", true, Some("/bin/sh".to_string()));
    assert!(env.is_ok(), "{env:?}");
}
