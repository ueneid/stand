//! Tests for TOML configuration with validation

use stand::config::loader;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_load_config_toml_with_validation_success() {
    let dir = TempDir::new().unwrap();
    let config_content = r#"
version = "2.0"

[common]
APP_NAME = "TestApp"
LOG_LEVEL = "info"

[environments.dev]
description = "Development environment"
DATABASE_URL = "postgres://localhost/dev"
DEBUG = "true"

[environments.prod]
description = "Production environment"
extends = "dev"
DATABASE_URL = "postgres://prod.example.com/app"
DEBUG = "false"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = loader::load_config_toml_with_validation(dir.path());
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.version, "2.0");
    assert!(config.common.is_some());

    let dev_env = config.environments.get("dev").unwrap();
    assert_eq!(dev_env.description, "Development environment");

    let prod_env = config.environments.get("prod").unwrap();
    assert_eq!(prod_env.description, "Production environment");
}

#[test]
fn test_load_config_toml_with_validation_missing_required_fields() {
    let dir = TempDir::new().unwrap();
    let config_content = r#"
# Missing version field
[environments.dev]
# Missing description field
DATABASE_URL = "postgres://localhost/dev"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = loader::load_config_toml_with_validation(dir.path());
    assert!(result.is_err());
}

#[test]
fn test_load_config_toml_with_validation_circular_reference() {
    let dir = TempDir::new().unwrap();
    let config_content = r#"
version = "2.0"

[environments.dev]
description = "Development environment"
extends = "prod"

[environments.prod]
description = "Production environment"
extends = "dev"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = loader::load_config_toml_with_validation(dir.path());
    assert!(result.is_err());
}

#[test]
fn test_load_config_toml_with_validation_empty_common_values() {
    let dir = TempDir::new().unwrap();
    let config_content = r#"
version = "2.0"

[common]
APP_NAME = ""
VALID_KEY = "valid_value"

[environments.dev]
description = "Development environment"
DATABASE_URL = "postgres://localhost/dev"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = loader::load_config_toml_with_validation(dir.path());
    assert!(result.is_err());
}

#[test]
fn test_load_config_toml_with_validation_invalid_extends_reference() {
    let dir = TempDir::new().unwrap();
    let config_content = r#"
version = "2.0"

[environments.dev]
description = "Development environment"
extends = "nonexistent"
DATABASE_URL = "postgres://localhost/dev"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = loader::load_config_toml_with_validation(dir.path());
    assert!(result.is_err());
}
