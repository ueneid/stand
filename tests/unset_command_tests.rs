use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use stand::commands::unset::{unset_variable, UnsetCommandError};
use std::fs;
use tempfile::tempdir;

const CONFIG: &str = r#"version = "1.0"

[common]
SHARED = "common-value"

[environments.base]
description = "Base"
BASE_VAR = "b"

[environments.dev]
description = "Development"
extends = "base"
# keep this comment
API_URL = "http://localhost"
KEY = "value"
OTHER = "other"
"#;

fn setup() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join(".stand.toml"), CONFIG).unwrap();
    dir
}

#[test]
fn test_unset_removes_key_and_keeps_rest() {
    let dir = setup();
    unset_variable(dir.path(), "dev", "KEY").unwrap();
    let content = fs::read_to_string(dir.path().join(".stand.toml")).unwrap();
    assert!(!content.contains("KEY = "));
    assert!(content.contains("# keep this comment"));
    assert!(content.contains("API_URL = \"http://localhost\""));
    assert!(content.contains("OTHER = \"other\""));
    assert!(content.contains("SHARED = \"common-value\""));
    content.parse::<toml_edit::DocumentMut>().unwrap();
}

#[test]
fn test_unset_unknown_environment() {
    let dir = setup();
    let r = unset_variable(dir.path(), "nope", "KEY");
    assert!(matches!(r, Err(UnsetCommandError::EnvironmentNotFound(_))));
}

#[test]
fn test_unset_missing_key_in_common_hints_common() {
    let dir = setup();
    let r = unset_variable(dir.path(), "dev", "SHARED").unwrap_err();
    assert!(matches!(r, UnsetCommandError::VariableNotFound { .. }));
    assert!(r.to_string().contains("[common]"), "{r}");
}

#[test]
fn test_unset_missing_key_in_parent_hints_parent() {
    let dir = setup();
    let r = unset_variable(dir.path(), "dev", "BASE_VAR").unwrap_err();
    assert!(r.to_string().contains("base"), "{r}");
}

#[test]
fn test_unset_missing_key_without_hint() {
    let dir = setup();
    match unset_variable(dir.path(), "dev", "NOPE").unwrap_err() {
        UnsetCommandError::VariableNotFound { hint, .. } => assert!(hint.is_none()),
        e => panic!("unexpected {e}"),
    }
}

#[test]
fn test_unset_extends_cycle_does_not_hang() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join(".stand.toml"),
        "version = \"1.0\"\n\n[environments.a]\ndescription = \"a\"\nextends = \"b\"\n\n[environments.b]\ndescription = \"b\"\nextends = \"a\"\n",
    )
    .unwrap();
    let r = unset_variable(dir.path(), "a", "X");
    assert!(matches!(r, Err(UnsetCommandError::VariableNotFound { .. })));
}

#[test]
fn test_unset_reserved_key() {
    let dir = setup();
    let r = unset_variable(dir.path(), "dev", "description");
    assert!(matches!(r, Err(UnsetCommandError::ReservedKey(_))));
}

#[test]
fn test_unset_allows_risky_names() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join(".stand.toml"),
        "version = \"1.0\"\n\n[environments.dev]\ndescription = \"d\"\nBASH_ENV = \"x\"\n",
    )
    .unwrap();
    unset_variable(dir.path(), "dev", "BASH_ENV").unwrap();
}

#[cfg(unix)]
#[test]
fn test_unset_preserves_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = setup();
    let path = dir.path().join(".stand.toml");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    unset_variable(dir.path(), "dev", "KEY").unwrap();
    let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn test_cli_unset_removes_key() {
    let dir = setup();
    cargo_bin_cmd!("stand")
        .current_dir(dir.path())
        .args(["unset", "dev", "KEY"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Unset KEY"));
    let content = fs::read_to_string(dir.path().join(".stand.toml")).unwrap();
    assert!(!content.contains("KEY = "));
}

#[test]
fn test_cli_unset_help() {
    cargo_bin_cmd!("stand")
        .args(["unset", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Remove a variable from the configuration file",
        ));
}
