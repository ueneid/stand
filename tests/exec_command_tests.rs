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

    let result = exec::execute_with_environment(dir.path(), "dev", vec![]);

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
        exec::execute_with_environment(dir.path(), "dev", vec!["true".to_string()]).unwrap();
    assert_eq!(exit_code, 0);

    // Test failed command
    let exit_code =
        exec::execute_with_environment(dir.path(), "dev", vec!["false".to_string()]).unwrap();
    assert_eq!(exit_code, 1);

    // Test custom exit code
    let exit_code = exec::execute_with_environment(
        dir.path(),
        "dev",
        vec!["sh".to_string(), "-c".to_string(), "exit 42".to_string()],
    )
    .unwrap();
    assert_eq!(exit_code, 42);
}
