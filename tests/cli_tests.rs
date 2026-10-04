use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_cli_shows_help() {
    let mut cmd = cargo_bin_cmd!("stand");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "A CLI tool for explicit environment variable management",
        ));
}

#[test]
fn test_cli_shows_version() {
    let mut cmd = cargo_bin_cmd!("stand");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("stand"));
}

#[test]
fn test_cli_init_creates_config() {
    let dir = tempdir().unwrap();

    let mut cmd = cargo_bin_cmd!("stand");
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

    let mut cmd = cargo_bin_cmd!("stand");
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

    let mut cmd = cargo_bin_cmd!("stand");
    cmd.current_dir(dir.path())
        .args(["init", "--force"])
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

    let mut cmd = cargo_bin_cmd!("stand");
    cmd.current_dir(dir.path())
        .args(["shell", "dev"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error:")); // Should fail without .stand.toml file
}

#[test]
fn test_cli_list_command_no_config_basic() {
    let dir = tempdir().unwrap();

    let mut cmd = cargo_bin_cmd!("stand");
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

    let mut cmd = cargo_bin_cmd!("stand");
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

    let mut cmd = cargo_bin_cmd!("stand");
    cmd.current_dir(dir.path())
        .arg("list")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error:"));
}

#[test]
fn test_cli_inspect_command_with_config() {
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

    let mut cmd = cargo_bin_cmd!("stand");
    cmd.current_dir(dir.path())
        .args(["inspect", "dev"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Environment: dev"))
        .stdout(predicate::str::contains("Variables:"))
        .stdout(predicate::str::contains("APP_NAME (from common)"))
        .stdout(predicate::str::contains("DATABASE_URL"))
        .stdout(predicate::str::contains("DEBUG"));
}

#[test]
fn test_cli_inspect_command_with_values() {
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

    let mut cmd = cargo_bin_cmd!("stand");
    cmd.current_dir(dir.path())
        .args(["inspect", "dev", "--values"])
        .assert()
        .success()
        .stdout(predicate::str::contains("APP_NAME=MyApp (from common)"))
        .stdout(predicate::str::contains(
            "DATABASE_URL=postgres://localhost:5432/dev",
        ));
}

#[test]
fn test_cli_inspect_command_nonexistent_env() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"


[environments.dev]
description = "Development environment"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = cargo_bin_cmd!("stand");
    cmd.current_dir(dir.path())
        .args(["inspect", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Environment 'nonexistent' not found",
        ));
}

#[test]
fn test_cli_env_command_not_in_subshell() {
    // When run outside of a Stand subshell (no STAND_ACTIVE), should fail
    let mut cmd = cargo_bin_cmd!("stand");
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
    let mut cmd = cargo_bin_cmd!("stand");
    cmd.args(["env", "--stand-only", "--user-only"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

// === Encryption Integration Tests ===

/// Runs the binary hermetically: config dir and HOME in a temp dir, no terminal,
/// no inherited stand/CI variables.
fn hermetic_stand(project: &std::path::Path) -> assert_cmd::Command {
    let cfg = project.join(".test-config-home");
    fs::create_dir_all(&cfg).unwrap();
    let mut cmd = cargo_bin_cmd!("stand");
    cmd.current_dir(project)
        .env("XDG_CONFIG_HOME", &cfg)
        .env("HOME", &cfg)
        .env("STAND_FORCE_NON_TTY", "1")
        .env_remove("CI")
        .env_remove("STAND_KEY_SOURCE")
        .env_remove("STAND_PRIVATE_KEY")
        .env_remove("STAND_UNLOCK");
    cmd
}

/// Writes a project config with encryption enabled and a legacy `.stand.keys`
/// (the only way to set up encryption without a terminal).
fn write_encrypted_project(dir: &std::path::Path) {
    let key_pair = stand::crypto::generate_key_pair();
    stand::crypto::keys::save_private_key(&dir.join(".stand.keys"), &key_pair.private_key).unwrap();
    fs::write(
        dir.join(".stand.toml"),
        format!(
            "version = \"1.0\"\n\n[encryption]\npublic_key = \"{}\"\n\n[environments.dev]\ndescription = \"Dev\"\n",
            key_pair.public_key
        ),
    )
    .unwrap();
}

#[test]
fn test_cli_encrypt_enable_requires_terminal() {
    let dir = tempdir().unwrap();
    hermetic_stand(dir.path()).arg("init").assert().success();
    let before = fs::read_to_string(dir.path().join(".stand.toml")).unwrap();

    // Enabling encryption asks for a passphrase, which needs a terminal.
    hermetic_stand(dir.path())
        .args(["encrypt", "enable"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("interactive terminal"));

    assert!(!dir.path().join(".stand.keys").exists());
    let after = fs::read_to_string(dir.path().join(".stand.toml")).unwrap();
    assert_eq!(before, after);
}

#[test]
fn test_cli_encrypt_enable_already_enabled() {
    let dir = tempdir().unwrap();
    write_encrypted_project(dir.path());

    hermetic_stand(dir.path())
        .args(["encrypt", "enable"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already enabled"));
}

#[test]
fn test_cli_init_with_encrypt_flag_requires_terminal() {
    let dir = tempdir().unwrap();

    // The config is created, then enabling encryption fails without a terminal.
    hermetic_stand(dir.path())
        .args(["init", "--encrypt"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Created .stand.toml"))
        .stderr(predicate::str::contains("interactive terminal"));

    assert!(dir.path().join(".stand.toml").exists());
    assert!(!dir.path().join(".stand.keys").exists());
}

#[test]
fn test_cli_encrypt_subcommands_exist() {
    let mut cmd = cargo_bin_cmd!("stand");
    cmd.args(["encrypt", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("enable"))
        .stdout(predicate::str::contains("disable"))
        .stdout(predicate::str::contains("migrate"))
        .stdout(predicate::str::contains("add-touch-id"));
}

#[test]
fn test_cli_set_and_get_encrypted_value() {
    let dir = tempdir().unwrap();
    write_encrypted_project(dir.path());

    hermetic_stand(dir.path())
        .args(["set", "dev", "API_KEY", "secret-value-123", "--encrypt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("(encrypted)"));

    let config_content = fs::read_to_string(dir.path().join(".stand.toml")).unwrap();
    assert!(config_content.contains("API_KEY = \"encrypted:"));
    assert!(!config_content.contains("secret-value-123")); // Plain value should NOT appear

    // Legacy .stand.keys needs no terminal, so get can decrypt.
    hermetic_stand(dir.path())
        .args(["get", "dev", "API_KEY"])
        .assert()
        .success()
        .stdout(predicate::str::contains("secret-value-123"));
}

#[test]
fn test_cli_inspect_shows_encrypted_marker() {
    let dir = tempdir().unwrap();
    write_encrypted_project(dir.path());

    hermetic_stand(dir.path())
        .args(["set", "dev", "SECRET", "my-secret", "--encrypt"])
        .assert()
        .success();

    hermetic_stand(dir.path())
        .args(["inspect", "dev", "--values"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[ENCRYPTED]"))
        .stdout(predicate::str::contains("SECRET"));
}

#[test]
fn test_cli_set_encrypted_without_encryption_enabled() {
    let dir = tempdir().unwrap();

    // Initialize WITHOUT encryption
    let mut cmd = cargo_bin_cmd!("stand");
    cmd.current_dir(dir.path()).arg("init").assert().success();

    // Try to set encrypted value - should fail
    let mut cmd = cargo_bin_cmd!("stand");
    cmd.current_dir(dir.path())
        .args(["set", "dev", "API_KEY", "secret", "--encrypt"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Encryption is not enabled"));
}

#[test]
fn test_cli_env_reads_values_from_process_env_and_never_decrypts() {
    let dir = tempdir().unwrap();

    let config_content = r#"
version = "2.0"

[encryption]
public_key = "age1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"

[environments.dev]
description = "Development"
SECRET = "encrypted:invaliddata"
MISSING = "plain"
"#;
    fs::write(dir.path().join(".stand.toml"), config_content).unwrap();

    // `stand env` runs inside a Stand subshell: values were already decrypted
    // and exported, so it reads them from the process environment.
    hermetic_stand(dir.path())
        .env("STAND_ACTIVE", "1")
        .env("STAND_ENVIRONMENT", "dev")
        .env("STAND_PROJECT_ROOT", dir.path().to_str().unwrap())
        .env("SECRET", "already-decrypted")
        .env_remove("MISSING")
        .arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("SECRET=already-decrypted"))
        .stdout(predicate::str::contains("MISSING").not())
        .stdout(predicate::str::contains("encrypted:").not());
}

#[test]
fn test_cli_exec_fails_on_decryption_error() {
    let dir = tempdir().unwrap();

    // Create a config with an encrypted value but NO .stand.keys file
    let config_content = r#"
version = "2.0"

[encryption]
public_key = "age1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"

[environments.dev]
description = "Development"
SECRET = "encrypted:invaliddata"
"#;
    fs::write(dir.path().join(".stand.toml"), config_content).unwrap();

    // Run `stand exec dev -- echo test` — should fail before executing the command
    hermetic_stand(dir.path())
        .args(["exec", "dev", "--", "echo", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("decrypt").or(predicate::str::contains("private key")));
}
