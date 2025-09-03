use stand::config::loader;
use std::fs;
use tempfile::tempdir;

#[test] 
fn test_load_toml_configuration() {
    let dir = tempdir().unwrap();
    
    let toml_content = r#"
version = "2.0"

[settings]
default_environment = "dev"
show_env_in_prompt = true

[common]
APP_NAME = "MyApp"
LOG_FORMAT = "json"

[environments.dev]
description = "Development environment"
color = "green"
DATABASE_URL = "postgres://localhost:5432/dev"
DEBUG = "true"

[environments.prod]
description = "Production environment"
color = "red"
extends = "dev"
requires_confirmation = true
DATABASE_URL = "postgres://prod.example.com/myapp"
DEBUG = "false"
"#;

    fs::write(dir.path().join(".stand.toml"), toml_content).unwrap();
    
    // This should work once we implement the TOML loader
    let result = loader::load_config_toml(dir.path());
    
    match result {
        Ok(config) => {
            assert_eq!(config.version, "2.0");
            assert_eq!(config.settings.default_environment, "dev");
            
            // Check common variables exist
            let common = config.common.as_ref().unwrap();
            assert_eq!(common.get("APP_NAME").unwrap(), "MyApp");
            
            // Check dev environment
            let dev_env = config.environments.get("dev").unwrap();
            assert_eq!(dev_env.description, "Development environment");
            assert_eq!(dev_env.variables.get("DEBUG").unwrap(), "true");
            
            // Check prod environment
            let prod_env = config.environments.get("prod").unwrap();
            assert_eq!(prod_env.extends, Some("dev".to_string()));
            assert_eq!(prod_env.variables.get("DEBUG").unwrap(), "false");
        }
        Err(e) => panic!("Failed to load TOML config: {}", e),
    }
}

#[test]
fn test_load_toml_missing_file() {
    let dir = tempdir().unwrap();
    
    let result = loader::load_config_toml(dir.path());
    assert!(result.is_err());
}