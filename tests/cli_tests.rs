use assert_cmd::Command;
use predicates::prelude::*;

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
    cmd.arg("list").assert().failure(); // Expecting failure for now
}
