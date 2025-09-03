use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Test the new TOML configuration structure
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Configuration {
    pub version: String,
    pub environments: HashMap<String, Environment>,
    pub common: Option<HashMap<String, String>>,
    pub settings: Settings,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Environment {
    pub description: String,
    pub extends: Option<String>,
    #[serde(flatten)]
    pub variables: HashMap<String, String>,
    pub color: Option<String>,
    pub requires_confirmation: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Settings {
    pub default_environment: String,
    pub nested_shell_behavior: Option<String>,
    pub show_env_in_prompt: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toml_configuration_parsing() {
        let toml_content = r#"
version = "2.0"

[settings]
default_environment = "dev"
show_env_in_prompt = true

[common]
APP_NAME = "MyApp"
LOG_FORMAT = "json"
TIMEZONE = "UTC"

[environments.dev]
description = "Development environment"
color = "green"
requires_confirmation = false
DATABASE_URL = "postgres://localhost:5432/dev"
API_KEY = "dev-key-123"
DEBUG = "true"

[environments.prod]
description = "Production environment"
color = "red"
extends = "dev"
requires_confirmation = true
DATABASE_URL = "postgres://prod.example.com/myapp"
API_KEY = "${PROD_API_KEY}"
DEBUG = "false"
"#;

        let config: Configuration = toml::from_str(toml_content).expect("Failed to parse TOML");
        
        // Verify basic structure
        assert_eq!(config.version, "2.0");
        assert_eq!(config.settings.default_environment, "dev");
        assert_eq!(config.settings.show_env_in_prompt, Some(true));
        
        // Verify common variables
        let common = config.common.as_ref().expect("Common variables should exist");
        assert_eq!(common.get("APP_NAME"), Some(&"MyApp".to_string()));
        assert_eq!(common.get("LOG_FORMAT"), Some(&"json".to_string()));
        
        // Verify dev environment
        let dev_env = config.environments.get("dev").expect("Dev environment should exist");
        assert_eq!(dev_env.description, "Development environment");
        assert_eq!(dev_env.color, Some("green".to_string()));
        assert_eq!(dev_env.requires_confirmation, Some(false));
        assert_eq!(dev_env.extends, None);
        assert_eq!(dev_env.variables.get("DATABASE_URL"), Some(&"postgres://localhost:5432/dev".to_string()));
        assert_eq!(dev_env.variables.get("DEBUG"), Some(&"true".to_string()));
        
        // Verify prod environment
        let prod_env = config.environments.get("prod").expect("Prod environment should exist");
        assert_eq!(prod_env.description, "Production environment");
        assert_eq!(prod_env.color, Some("red".to_string()));
        assert_eq!(prod_env.requires_confirmation, Some(true));
        assert_eq!(prod_env.extends, Some("dev".to_string()));
        assert_eq!(prod_env.variables.get("DATABASE_URL"), Some(&"postgres://prod.example.com/myapp".to_string()));
        assert_eq!(prod_env.variables.get("DEBUG"), Some(&"false".to_string()));
    }

    #[test]
    fn test_toml_serialization() {
        let mut config = Configuration {
            version: "2.0".to_string(),
            settings: Settings {
                default_environment: "dev".to_string(),
                nested_shell_behavior: Some("prevent".to_string()),
                show_env_in_prompt: Some(true),
            },
            common: Some({
                let mut map = HashMap::new();
                map.insert("APP_NAME".to_string(), "TestApp".to_string());
                map
            }),
            environments: HashMap::new(),
        };

        let mut dev_env = Environment {
            description: "Development".to_string(),
            extends: None,
            variables: HashMap::new(),
            color: Some("green".to_string()),
            requires_confirmation: Some(false),
        };
        dev_env.variables.insert("DEBUG".to_string(), "true".to_string());
        
        config.environments.insert("dev".to_string(), dev_env);

        let toml_output = toml::to_string(&config).expect("Failed to serialize to TOML");
        
        // Verify we can parse it back
        let parsed: Configuration = toml::from_str(&toml_output).expect("Failed to parse serialized TOML");
        assert_eq!(parsed.version, config.version);
        assert_eq!(parsed.settings.default_environment, config.settings.default_environment);
    }

    #[test]
    fn test_minimal_configuration() {
        let toml_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[environments.dev]
description = "Development environment"
"#;

        let config: Configuration = toml::from_str(toml_content).expect("Failed to parse minimal TOML");
        
        assert_eq!(config.version, "2.0");
        assert_eq!(config.settings.default_environment, "dev");
        assert!(config.common.is_none());
        
        let dev_env = config.environments.get("dev").expect("Dev environment should exist");
        assert_eq!(dev_env.description, "Development environment");
        assert!(dev_env.extends.is_none());
        assert!(dev_env.variables.is_empty());
    }
}