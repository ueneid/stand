use stand::config::loader;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_load_valid_yaml_configuration() {
    let dir = tempdir().unwrap();
    let config_dir = dir.path().join(".stand");
    fs::create_dir_all(&config_dir).unwrap();

    let config_content = r#"
version: "1.0"
environments:
  dev:
    description: "Development environment"
    files: [".stand.dev.env"]
  prod:
    description: "Production environment"
    files: [".stand.prod.env"]
settings:
  default_environment: "dev"
  show_env_in_prompt: true
"#;

    let config_path = config_dir.join("config.yaml");
    fs::write(&config_path, config_content).unwrap();

    // This should fail initially since loader::load_config is not implemented
    let result = loader::load_config(dir.path());
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.version, "1.0");
    assert_eq!(config.environments.len(), 2);
    assert!(config.environments.contains_key("dev"));
    assert!(config.environments.contains_key("prod"));
}

#[test]
fn test_load_missing_configuration() {
    let dir = tempdir().unwrap();

    // This should fail with appropriate error
    let result = loader::load_config(dir.path());
    assert!(result.is_err());
}

#[test]
fn test_load_invalid_yaml_configuration() {
    let dir = tempdir().unwrap();
    let config_dir = dir.path().join(".stand");
    fs::create_dir_all(&config_dir).unwrap();

    let invalid_yaml = r#"
version: "1.0"
environments:
  dev:
    description: "Development environment"
    files: [".stand.dev.env"
# Missing closing bracket - invalid YAML
"#;

    let config_path = config_dir.join("config.yaml");
    fs::write(&config_path, invalid_yaml).unwrap();

    let result = loader::load_config(dir.path());
    assert!(result.is_err());
}

// Comprehensive validation tests (these will fail initially - TDD RED phase)

#[test]
fn test_config_validation_missing_required_fields() {
    let dir = tempdir().unwrap();
    let config_dir = dir.path().join(".stand");
    fs::create_dir_all(&config_dir).unwrap();

    // Missing version field
    let config_content = r#"
environments:
  dev:
    description: "Development environment"
    files: [".stand.dev.env"]
settings:
  default_environment: "dev"
"#;

    let config_path = config_dir.join("config.yaml");
    fs::write(&config_path, config_content).unwrap();

    let result = loader::load_config_with_validation(dir.path());
    assert!(result.is_err());

    // Check error message contains information about missing version
    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains("version"));
}

#[test]
fn test_config_validation_invalid_default_environment() {
    let dir = tempdir().unwrap();
    setup_config_file(
        &dir,
        r#"
version: "1.0"
environments:
  dev:
    description: "Development environment"
    files: [".stand.dev.env"]
settings:
  default_environment: "nonexistent"
"#,
    );

    let result = loader::load_config_with_validation(dir.path());
    assert!(result.is_err());

    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains("nonexistent"));
}

#[test]
fn test_config_environment_extends_validation() {
    let dir = tempdir().unwrap();
    setup_config_file(
        &dir,
        r#"
version: "1.0"
environments:
  base:
    description: "Base environment"
    files: [".stand.base.env"]
  dev:
    description: "Development environment"
    extends: "base"
    files: [".stand.dev.env"]
  circular:
    description: "Circular reference"
    extends: "circular"
    files: [".stand.circular.env"]
settings:
  default_environment: "dev"
"#,
    );

    let result = loader::load_config_with_validation(dir.path());
    assert!(result.is_err());

    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains("circular"));
}

#[test]
fn test_config_environment_extends_nonexistent() {
    let dir = tempdir().unwrap();
    setup_config_file(
        &dir,
        r#"
version: "1.0"
environments:
  dev:
    description: "Development environment"
    extends: "nonexistent"
    files: [".stand.dev.env"]
settings:
  default_environment: "dev"
"#,
    );

    let result = loader::load_config_with_validation(dir.path());
    assert!(result.is_err());

    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains("nonexistent"));
}

#[test]
fn test_config_with_defaults_applied() {
    let dir = tempdir().unwrap();
    setup_config_file(
        &dir,
        r#"
version: "1.0"
environments:
  dev:
    description: "Development environment"
    files: [".stand.dev.env"]
settings:
  default_environment: "dev"
"#,
    );

    let result = loader::load_config_with_defaults(dir.path());
    assert!(result.is_ok());

    let config = result.unwrap();
    // Test that defaults are applied
    assert!(config.settings.show_env_in_prompt.unwrap_or(false)); // Should have default
}

#[test]
fn test_config_environment_variable_interpolation() {
    let dir = tempdir().unwrap();

    // Set test environment variable
    std::env::set_var("TEST_ENV_VAR", "test_value");

    setup_config_file(
        &dir,
        r#"
version: "1.0"
environments:
  dev:
    description: "Development with ${TEST_ENV_VAR}"
    files: [".stand.${TEST_ENV_VAR}.env"]
settings:
  default_environment: "dev"
"#,
    );

    let result = loader::load_config_with_interpolation(dir.path());
    assert!(result.is_ok());

    let config = result.unwrap();
    let dev_env = &config.environments["dev"];
    assert_eq!(dev_env.description, "Development with test_value");
    assert_eq!(dev_env.files[0].to_string_lossy(), ".stand.test_value.env");

    // Clean up
    std::env::remove_var("TEST_ENV_VAR");
}

#[test]
fn test_config_file_path_validation() {
    let dir = tempdir().unwrap();
    let config_dir = dir.path().join(".stand");
    fs::create_dir_all(&config_dir).unwrap();

    // Create actual env file
    fs::write(config_dir.join(".stand.dev.env"), "TEST_VAR=value").unwrap();

    setup_config_file(
        &dir,
        r#"
version: "1.0"
environments:
  dev:
    description: "Development environment"
    files: [".stand.dev.env", ".stand.nonexistent.env"]
settings:
  default_environment: "dev"
"#,
    );

    let result = loader::load_config_with_file_validation(dir.path());
    assert!(result.is_err());

    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains("nonexistent"));
}

#[test]
fn test_config_hierarchical_merge() {
    let dir = tempdir().unwrap();
    setup_config_file(
        &dir,
        r#"
version: "1.0"
common:
  files: [".stand.common.env"]
environments:
  base:
    description: "Base environment"
    files: [".stand.base.env"]
  dev:
    description: "Development environment"
    extends: "base"
    files: [".stand.dev.env"]
settings:
  default_environment: "dev"
"#,
    );

    let result = loader::load_config_with_hierarchy(dir.path());

    if let Err(e) = &result {
        panic!("Config loading failed: {}", e);
    }

    let config = result.unwrap();
    let dev_env = &config.environments["dev"];

    // Dev environment should inherit from base
    assert_eq!(dev_env.description, "Development environment");
    // Should have both base and dev files
    assert_eq!(dev_env.files.len(), 2);
}

// Helper function to set up config files
fn setup_config_file(temp_dir: &tempfile::TempDir, content: &str) {
    let config_dir = temp_dir.path().join(".stand");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.yaml");
    fs::write(&config_path, content).unwrap();
}
