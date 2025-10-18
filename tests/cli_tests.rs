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
fn test_cli_parses_init_command() {
    let mut cmd = Command::cargo_bin("stand").unwrap();
    // This test should fail initially since we haven't implemented the command handling
    cmd.arg("init").assert().failure(); // Expecting failure for now
}

#[test]
fn test_cli_parses_shell_command() {
    let mut cmd = Command::cargo_bin("stand").unwrap();
    cmd.args(&["shell", "dev"]).assert().failure(); // Expecting failure for now
}

#[test]
fn test_cli_parses_list_command() {
    let mut cmd = Command::cargo_bin("stand").unwrap();
    cmd.arg("list").assert().failure(); // Should fail without .stand.toml file
}

#[test]
fn test_cli_list_command_with_config() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

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
        .stdout(predicate::str::contains("→ dev"))
        .stdout(predicate::str::contains("Development environment"))
        .stdout(predicate::str::contains("prod"))
        .stdout(predicate::str::contains("Production environment"))
        .stdout(predicate::str::contains("[green]"))
        .stdout(predicate::str::contains("[red]"))
        .stdout(predicate::str::contains("確認要"))
        .stdout(predicate::str::contains("→ indicates default environment"));
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

[settings]
default_environment = "dev"

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

[settings]
default_environment = "dev"

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

[settings]
default_environment = "dev"

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
