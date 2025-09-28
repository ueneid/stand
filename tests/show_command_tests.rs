use stand::commands::show;
use std::fs;
use tempfile::tempdir;
use serial_test::serial;

#[test]
fn test_show_names_only_simple() {
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
DATABASE_URL = "postgres://localhost:5432/dev"
DEBUG = "true"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = show::show_environment(dir.path(), "dev", false).unwrap();

    assert!(result.contains("Environment: dev"));
    assert!(result.contains("Variables:"));
    assert!(result.contains("APP_NAME (from common)"));
    assert!(result.contains("DATABASE_URL"));
    assert!(result.contains("DEBUG"));
    assert!(result.contains("LOG_FORMAT (from common)"));
    // Values should not be shown in names-only mode
    assert!(!result.contains("="));
    assert!(!result.contains("MyApp"));
    assert!(!result.contains("postgres://"));
}

#[test]
fn test_show_with_values() {
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
DEBUG = "true"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = show::show_environment(dir.path(), "dev", true).unwrap();

    assert!(result.contains("Environment: dev"));
    assert!(result.contains("Variables:"));
    assert!(result.contains("APP_NAME=MyApp (from common)"));
    assert!(result.contains("DATABASE_URL=postgres://localhost:5432/dev"));
    assert!(result.contains("DEBUG=true"));
}

#[test]
#[serial]
fn test_show_with_interpolation() {
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

    let result = show::show_environment(dir.path(), "dev", true).unwrap();

    assert!(result.contains("DATABASE_URL=postgres://localhost:5432/dev"));

    // Clean up environment variables
    std::env::remove_var("DB_HOST");
    std::env::remove_var("DB_PORT");
}

#[test]
fn test_show_inheritance_chain() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[common]
APP_NAME = "MyApp"

[environments.base]
description = "Base environment"
LOG_LEVEL = "info"
PORT = "3000"

[environments.dev]
description = "Development environment"
extends = "base"
LOG_LEVEL = "debug"
DEBUG = "true"

[environments.prod]
description = "Production environment"
extends = "dev"
DEBUG = "false"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = show::show_environment(dir.path(), "prod", false).unwrap();

    assert!(result.contains("Environment: prod"));
    assert!(result.contains("APP_NAME (from common)"));
    assert!(result.contains("PORT (inherited from base)"));
    assert!(result.contains("LOG_LEVEL (inherited from dev)"));
    assert!(result.contains("DEBUG"));  // Local to prod, no suffix
}

#[test]
fn test_show_nonexistent_environment() {
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

    let result = show::show_environment(dir.path(), "nonexistent", false);

    assert!(result.is_err());
    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains("Environment 'nonexistent' not found"));
    assert!(error_msg.contains("Available: dev"));
}