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
