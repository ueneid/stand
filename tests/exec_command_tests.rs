use serial_test::serial;
use stand::commands::exec;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_exec_nonexistent_environment() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[environments.dev]
description = "Development environment"
DATABASE_URL = "postgres://localhost:5432/dev"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = exec::execute_with_environment(
        dir.path(),
        "nonexistent",
        vec!["echo".to_string(), "hello".to_string()],
        false,
    );

    assert!(result.is_err());
    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains("Environment 'nonexistent' not found"));
    assert!(error_msg.contains("Available: dev"));
}

#[test]
fn test_exec_simple_environment() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[environments.dev]
description = "Development environment"
TEST_VAR = "test_value"
ANOTHER_VAR = "another_value"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let exit_code = exec::execute_with_environment(
        dir.path(),
        "dev",
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "test \"$TEST_VAR\" = \"test_value\" && test \"$ANOTHER_VAR\" = \"another_value\""
                .to_string(),
        ],
        false,
    )
    .unwrap();

    assert_eq!(exit_code, 0);
}

#[test]
fn test_exec_with_inheritance() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "prod"

[environments.base]
description = "Base environment"
PORT = "3000"
LOG_LEVEL = "info"

[environments.prod]
description = "Production environment"
extends = "base"
LOG_LEVEL = "error"
DEBUG = "false"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let exit_code = exec::execute_with_environment(
        dir.path(),
        "prod",
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "test \"$PORT\" = \"3000\" && test \"$LOG_LEVEL\" = \"error\" && test \"$DEBUG\" = \"false\"".to_string(),
        ],
        false,
    )
    .unwrap();

    assert_eq!(exit_code, 0);
}

#[test]
fn test_exec_with_common_variables() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[common]
APP_NAME = "MyApp"
LOG_FORMAT = "json"

[environments.dev]
description = "Development environment"
DEBUG = "true"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let exit_code = exec::execute_with_environment(
        dir.path(),
        "dev",
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "test \"$APP_NAME\" = \"MyApp\" && test \"$LOG_FORMAT\" = \"json\" && test \"$DEBUG\" = \"true\"".to_string(),
        ],
        false,
    )
    .unwrap();

    assert_eq!(exit_code, 0);
}

#[test]
#[serial]
fn test_exec_with_interpolation() {
    let dir = tempdir().unwrap();

    // Set environment variable for interpolation
    std::env::set_var("DB_HOST", "localhost");
    std::env::set_var("DB_PORT", "5432");

    let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[environments.dev]
description = "Development environment"
DATABASE_URL = "postgres://${DB_HOST}:${DB_PORT}/dev"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let exit_code = exec::execute_with_environment(
        dir.path(),
        "dev",
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "test \"$DATABASE_URL\" = \"postgres://localhost:5432/dev\"".to_string(),
        ],
        false,
    )
    .unwrap();

    assert_eq!(exit_code, 0);

    // Clean up environment variables
    std::env::remove_var("DB_HOST");
    std::env::remove_var("DB_PORT");
}

#[test]
fn test_exec_nonexistent_command() {
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

    let result = exec::execute_with_environment(
        dir.path(),
        "dev",
        vec!["nonexistent_command_12345".to_string()],
        false,
    );

    assert!(result.is_err());
}

#[test]
fn test_exec_empty_command() {
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

    let result = exec::execute_with_environment(dir.path(), "dev", vec![], false);

    assert!(result.is_err());
    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains("Command cannot be empty"));
}

#[test]
fn test_exec_exit_code_propagation() {
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

    // Test successful command
    let exit_code =
        exec::execute_with_environment(dir.path(), "dev", vec!["true".to_string()], false).unwrap();
    assert_eq!(exit_code, 0);

    // Test failed command
    let exit_code =
        exec::execute_with_environment(dir.path(), "dev", vec!["false".to_string()], false)
            .unwrap();
    assert_eq!(exit_code, 1);

    // Test custom exit code
    let exit_code = exec::execute_with_environment(
        dir.path(),
        "dev",
        vec!["sh".to_string(), "-c".to_string(), "exit 42".to_string()],
        false,
    )
    .unwrap();
    assert_eq!(exit_code, 42);
}

#[test]
fn test_exec_requires_confirmation_without_yes_flag() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "prod"

[environments.prod]
description = "Production environment"
requires_confirmation = true
DATABASE_URL = "postgres://prod:5432/prod"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    // Without skip_confirmation, should return error
    let result = exec::execute_with_environment(
        dir.path(),
        "prod",
        vec!["echo".to_string(), "hello".to_string()],
        false,
    );

    assert!(result.is_err());
    let error_msg = format!("{}", result.unwrap_err());
    // When stdin is empty (test environment), the prompt returns false
    // and the function returns "Execution cancelled" error
    assert!(error_msg.contains("Execution cancelled"));
    assert!(error_msg.contains("-y") || error_msg.contains("--yes"));
}

#[test]
fn test_exec_requires_confirmation_with_yes_flag() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "prod"

[environments.prod]
description = "Production environment"
requires_confirmation = true
TEST_VAR = "prod_value"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    // With skip_confirmation = true, should succeed
    let exit_code = exec::execute_with_environment(
        dir.path(),
        "prod",
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "test \"$TEST_VAR\" = \"prod_value\"".to_string(),
        ],
        true,
    )
    .unwrap();

    assert_eq!(exit_code, 0);
}

#[test]
fn test_exec_no_confirmation_required_works_without_flag() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[environments.dev]
description = "Development environment"
requires_confirmation = false
TEST_VAR = "dev_value"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    // With requires_confirmation = false, should work without skip_confirmation
    let exit_code = exec::execute_with_environment(
        dir.path(),
        "dev",
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "test \"$TEST_VAR\" = \"dev_value\"".to_string(),
        ],
        false,
    )
    .unwrap();

    assert_eq!(exit_code, 0);
}
