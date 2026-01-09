use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_cli_shows_help() {
    let mut cmd = Command::cargo_bin("stand").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "A CLI tool for explicit environment variable management",
        ));
}

#[test]
fn test_cli_shows_version() {
    let mut cmd = Command::cargo_bin("stand").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("stand"));
}

#[test]
fn test_cli_init_creates_config() {
    let dir = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("stand").unwrap();
    cmd.current_dir(dir.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created .stand.toml"));

    // Verify the file was created
    let config_path = dir.path().join(".stand.toml");
    assert!(config_path.exists());

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains(r#"version = "2.0""#));
    assert!(content.contains("[environments.dev]"));
    assert!(content.contains("[environments.prod]"));
}

#[test]
fn test_cli_init_fails_when_exists() {
    let dir = tempdir().unwrap();

    // Create existing config
    fs::write(dir.path().join(".stand.toml"), "existing").unwrap();

    let mut cmd = Command::cargo_bin("stand").unwrap();
    cmd.current_dir(dir.path())
        .arg("init")
        .assert()
        .failure()
        .stderr(predicate::str::contains("already initialized"));
}

#[test]
fn test_cli_init_force_overwrites() {
    let dir = tempdir().unwrap();

    // Create existing config
    fs::write(dir.path().join(".stand.toml"), "old content").unwrap();

    let mut cmd = Command::cargo_bin("stand").unwrap();
    cmd.current_dir(dir.path())
        .args(&["init", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Overwritten existing .stand.toml"));

    let content = fs::read_to_string(dir.path().join(".stand.toml")).unwrap();
    assert!(content.contains(r#"version = "2.0""#));
    assert!(!content.contains("old content"));
}

#[test]
fn test_cli_shell_command_no_config() {
    let dir = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("stand").unwrap();
    cmd.current_dir(dir.path())
        .args(&["shell", "dev"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error:")); // Should fail without .stand.toml file
}

#[test]
fn test_cli_list_command_no_config_basic() {
    let dir = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("stand").unwrap();
    cmd.current_dir(dir.path())
        .arg("list")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error:")); // Should fail without .stand.toml file
}

#[test]
fn test_cli_list_command_with_config() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[environments.dev]
description = "Development environment"
color = "green"

[environments.prod]
description = "Production environment"
color = "red"
requires_confirmation = true
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("stand").unwrap();
    cmd.current_dir(dir.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Available environments:"))
        .stdout(predicate::str::contains("dev"))
        .stdout(predicate::str::contains("Development environment"))
        .stdout(predicate::str::contains("prod"))
        .stdout(predicate::str::contains("Production environment"))
        .stdout(predicate::str::contains("[green]"))
        .stdout(predicate::str::contains("[red]"))
        .stdout(predicate::str::contains("(requires confirmation)"));
}

#[test]
fn test_cli_list_command_no_config() {
    let dir = tempdir().unwrap();
    // No .stand.toml file created

    let mut cmd = Command::cargo_bin("stand").unwrap();
    cmd.current_dir(dir.path())
        .arg("list")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error:"));
}

#[test]
fn test_cli_show_command_with_config() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"


[common]
APP_NAME = "MyApp"

[environments.dev]
description = "Development environment"
color = "green"
DATABASE_URL = "postgres://localhost:5432/dev"
DEBUG = "true"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("stand").unwrap();
    cmd.current_dir(dir.path())
        .args(&["show", "dev"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Environment: dev"))
        .stdout(predicate::str::contains("Variables:"))
        .stdout(predicate::str::contains("APP_NAME (from common)"))
        .stdout(predicate::str::contains("DATABASE_URL"))
        .stdout(predicate::str::contains("DEBUG"));
}

#[test]
fn test_cli_show_command_with_values() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"


[common]
APP_NAME = "MyApp"

[environments.dev]
description = "Development environment"
DATABASE_URL = "postgres://localhost:5432/dev"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("stand").unwrap();
    cmd.current_dir(dir.path())
        .args(&["show", "dev", "--values"])
        .assert()
        .success()
        .stdout(predicate::str::contains("APP_NAME=MyApp (from common)"))
        .stdout(predicate::str::contains(
            "DATABASE_URL=postgres://localhost:5432/dev",
        ));
}

#[test]
fn test_cli_show_command_nonexistent_env() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"


[environments.dev]
description = "Development environment"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("stand").unwrap();
    cmd.current_dir(dir.path())
        .args(&["show", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Environment 'nonexistent' not found",
        ));
}

#[test]
fn test_cli_env_command_not_in_subshell() {
    // When run outside of a Stand subshell (no STAND_ACTIVE), should fail
    let mut cmd = Command::cargo_bin("stand").unwrap();
    cmd.env_remove("STAND_ACTIVE")
        .env_remove("STAND_ENVIRONMENT")
        .arg("env")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not inside a Stand subshell"));
}

#[test]
fn test_cli_env_command_options_conflict() {
    // --stand-only and --user-only should conflict
    let mut cmd = Command::cargo_bin("stand").unwrap();
    cmd.args(&["env", "--stand-only", "--user-only"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}
