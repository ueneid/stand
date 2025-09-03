use crate::config::types::Configuration;
use crate::config::ConfigError;
use std::collections::HashSet;

/// Validate that all required fields are present
pub fn validate_required_fields(config: &Configuration) -> Result<(), ConfigError> {
    if config.version.is_empty() {
        return Err(ConfigError::MissingField {
            field: "version".to_string(),
        });
    }

    // Check that environments map is not empty
    if config.environments.is_empty() {
        return Err(ConfigError::MissingField {
            field: "environments".to_string(),
        });
    }

    // Check that default_environment is not empty
    if config.settings.default_environment.is_empty() {
        return Err(ConfigError::MissingField {
            field: "settings.default_environment".to_string(),
        });
    }

    // Check that each environment has a non-empty description
    for (env_name, env) in &config.environments {
        if env.description.is_empty() {
            return Err(ConfigError::ValidationError {
                message: format!("Environment '{}' must have a non-empty description", env_name),
            });
        }
    }

    Ok(())
}

/// Validate that all environment references are valid
pub fn validate_environment_references(config: &Configuration) -> Result<(), ConfigError> {
    let env_names: HashSet<&String> = config.environments.keys().collect();

    // Check that default_environment exists
    if !env_names.contains(&config.settings.default_environment) {
        return Err(ConfigError::InvalidEnvironment {
            name: config.settings.default_environment.clone(),
        });
    }

    // Check that all 'extends' references are valid
    for env in config.environments.values() {
        if let Some(extends) = &env.extends {
            if !env_names.contains(&extends) {
                return Err(ConfigError::InvalidEnvironment {
                    name: extends.clone(),
                });
            }
        }
    }

    Ok(())
}

/// Validate that there are no circular references in environment hierarchy
pub fn validate_no_circular_references(config: &Configuration) -> Result<(), ConfigError> {
    for env_name in config.environments.keys() {
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        if detect_circular_reference(config, env_name, &mut visited, &mut path)? {
            return Err(ConfigError::CircularReference { cycle: path });
        }
    }

    Ok(())
}

/// Detect circular references using DFS
fn detect_circular_reference(
    config: &Configuration,
    current: &str,
    visited: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> Result<bool, ConfigError> {
    if path.contains(&current.to_string()) {
        path.push(current.to_string());
        return Ok(true);
    }

    if visited.contains(current) {
        return Ok(false);
    }

    visited.insert(current.to_string());
    path.push(current.to_string());

    if let Some(env) = config.environments.get(current) {
        if let Some(extends) = &env.extends {
            if detect_circular_reference(config, extends, visited, path)? {
                return Ok(true);
            }
        }
    }

    path.pop();
    Ok(false)
}

/// Validate common configuration if present
pub fn validate_common_config(config: &Configuration) -> Result<(), ConfigError> {
    if let Some(common) = &config.common {
        for (key, value) in common {
            if key.is_empty() {
                return Err(ConfigError::ValidationError {
                    message: "Common variable keys cannot be empty".to_string(),
                });
            }
            if value.is_empty() {
                return Err(ConfigError::ValidationError {
                    message: format!("Common variable '{}' cannot have empty value", key),
                });
            }
        }
    }

    Ok(())
}
