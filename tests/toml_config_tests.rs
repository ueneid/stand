use stand::config::types::{Configuration, Environment, NestedBehavior, Settings};
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toml_configuration_parsing() {
        let toml_content = r#"
version = "2.0"

[settings]
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
        assert_eq!(config.settings.show_env_in_prompt, Some(true));

        // Verify common variables
        let common = config
            .common
            .as_ref()
            .expect("Common variables should exist");
        assert_eq!(common.get("APP_NAME"), Some(&"MyApp".to_string()));
        assert_eq!(common.get("LOG_FORMAT"), Some(&"json".to_string()));

        // Verify dev environment
        let dev_env = config
            .environments
            .get("dev")
            .expect("Dev environment should exist");
        assert_eq!(dev_env.description, "Development environment");
        assert_eq!(dev_env.color, Some("green".to_string()));
        assert_eq!(dev_env.requires_confirmation, Some(false));
        assert_eq!(dev_env.extends, None);
        assert_eq!(
            dev_env.variables.get("DATABASE_URL"),
            Some(&"postgres://localhost:5432/dev".to_string())
        );
        assert_eq!(dev_env.variables.get("DEBUG"), Some(&"true".to_string()));

        // Verify prod environment
        let prod_env = config
            .environments
            .get("prod")
            .expect("Prod environment should exist");
        assert_eq!(prod_env.description, "Production environment");
        assert_eq!(prod_env.color, Some("red".to_string()));
        assert_eq!(prod_env.requires_confirmation, Some(true));
        assert_eq!(prod_env.extends, Some("dev".to_string()));
        assert_eq!(
            prod_env.variables.get("DATABASE_URL"),
            Some(&"postgres://prod.example.com/myapp".to_string())
        );
        assert_eq!(prod_env.variables.get("DEBUG"), Some(&"false".to_string()));
    }

    #[test]
    fn test_toml_serialization() {
        let mut config = Configuration {
            version: "2.0".to_string(),
            settings: Settings {
                nested_shell_behavior: Some(NestedBehavior::Prevent),
                show_env_in_prompt: Some(true),
                auto_exit_on_dir_change: None,
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
        dev_env
            .variables
            .insert("DEBUG".to_string(), "true".to_string());

        config.environments.insert("dev".to_string(), dev_env);

        let toml_output = toml::to_string(&config).expect("Failed to serialize to TOML");

        // Verify we can parse it back
        let parsed: Configuration =
            toml::from_str(&toml_output).expect("Failed to parse serialized TOML");
        assert_eq!(parsed.version, config.version);
    }

    #[test]
    fn test_minimal_configuration() {
        let toml_content = r#"
version = "2.0"

[environments.dev]
description = "Development environment"
"#;

        let config: Configuration =
            toml::from_str(toml_content).expect("Failed to parse minimal TOML");

        assert_eq!(config.version, "2.0");
        assert!(config.common.is_none());

        let dev_env = config
            .environments
            .get("dev")
            .expect("Dev environment should exist");
        assert_eq!(dev_env.description, "Development environment");
        assert!(dev_env.extends.is_none());
        assert!(dev_env.variables.is_empty());
    }

    #[test]
    fn test_backward_compatibility_with_default_environment() {
        // Old config files with default_environment should still be parseable
        let toml_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[environments.dev]
description = "Development environment"
"#;

        let config: Configuration = toml::from_str(toml_content)
            .expect("Failed to parse TOML with legacy default_environment");

        assert_eq!(config.version, "2.0");
        // default_environment is ignored but file should parse without error
    }

    #[test]
    fn test_auto_exit_on_dir_change_setting() {
        let toml_content = r#"
version = "2.0"

[settings]
auto_exit_on_dir_change = true

[environments.dev]
description = "Development environment"
"#;

        let config: Configuration = toml::from_str(toml_content)
            .expect("Failed to parse TOML with auto_exit_on_dir_change");

        assert_eq!(config.settings.auto_exit_on_dir_change, Some(true));
    }

    #[test]
    fn test_auto_exit_on_dir_change_default_none() {
        let toml_content = r#"
version = "2.0"

[environments.dev]
description = "Development environment"
"#;

        let config: Configuration =
            toml::from_str(toml_content).expect("Failed to parse minimal TOML");

        // Default should be None (not set)
        assert_eq!(config.settings.auto_exit_on_dir_change, None);
    }

    #[test]
    fn test_auto_exit_on_dir_change_false() {
        let toml_content = r#"
version = "2.0"

[settings]
auto_exit_on_dir_change = false

[environments.dev]
description = "Development environment"
"#;

        let config: Configuration = toml::from_str(toml_content)
            .expect("Failed to parse TOML with auto_exit_on_dir_change = false");

        assert_eq!(config.settings.auto_exit_on_dir_change, Some(false));
    }
}
