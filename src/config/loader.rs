use crate::config::types::Configuration;
use crate::config::ConfigError;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Load configuration from the given directory
pub fn load_config(project_path: &Path) -> Result<Configuration> {
    let config_path = project_path.join(".stand").join("config.yaml");

    if !config_path.exists() {
        anyhow::bail!("Stand configuration not found. Run 'stand init' to initialize.")
    }

    let content = fs::read_to_string(&config_path).with_context(|| {
        format!(
            "Failed to read configuration file: {}",
            config_path.display()
        )
    })?;

    let config: Configuration = serde_yaml::from_str(&content).with_context(|| {
        format!(
            "Failed to parse configuration file: {}",
            config_path.display()
        )
    })?;

    Ok(config)
}

/// Load configuration with comprehensive validation
pub fn load_config_with_validation(project_path: &Path) -> Result<Configuration, ConfigError> {
    let config = load_config_basic(project_path)?;

    // Validate required fields
    validate_required_fields(&config)?;

    // Validate environment references
    validate_environment_references(&config)?;

    // Validate circular references
    validate_no_circular_references(&config)?;

    Ok(config)
}

/// Load configuration with default values applied
pub fn load_config_with_defaults(project_path: &Path) -> Result<Configuration> {
    let mut config = load_config_basic(project_path).map_err(anyhow::Error::from)?;

    // Apply defaults
    apply_default_values(&mut config);

    Ok(config)
}

/// Load configuration with environment variable interpolation
pub fn load_config_with_interpolation(project_path: &Path) -> Result<Configuration, ConfigError> {
    let mut config = load_config_basic(project_path)?;

    // Interpolate environment variables
    interpolate_environment_variables(&mut config)?;

    Ok(config)
}

/// Load configuration with file path validation
pub fn load_config_with_file_validation(project_path: &Path) -> Result<Configuration, ConfigError> {
    let config = load_config_basic(project_path)?;

    // Validate that all referenced files exist
    validate_file_paths(&config, project_path)?;

    Ok(config)
}

/// Load configuration with hierarchical merge support
pub fn load_config_with_hierarchy(project_path: &Path) -> Result<Configuration> {
    let mut config = load_config_basic(project_path).map_err(anyhow::Error::from)?;

    // Apply hierarchical merging
    apply_hierarchical_merge(&mut config).map_err(anyhow::Error::from)?;

    Ok(config)
}

/// Basic configuration loading without validation
fn load_config_basic(project_path: &Path) -> Result<Configuration, ConfigError> {
    let config_path = project_path.join(".stand").join("config.yaml");

    if !config_path.exists() {
        return Err(ConfigError::ValidationError {
            message: "Stand configuration not found. Run 'stand init' to initialize.".to_string(),
        });
    }

    let content = fs::read_to_string(&config_path)?;
    let config: Configuration = serde_yaml::from_str(&content)?;

    Ok(config)
}

/// Validate that all required fields are present
fn validate_required_fields(config: &Configuration) -> Result<(), ConfigError> {
    if config.version.is_empty() {
        return Err(ConfigError::MissingField {
            field: "version".to_string(),
        });
    }

    Ok(())
}

/// Validate that all environment references are valid
fn validate_environment_references(config: &Configuration) -> Result<(), ConfigError> {
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
fn validate_no_circular_references(config: &Configuration) -> Result<(), ConfigError> {
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

/// Apply default values to configuration
fn apply_default_values(config: &mut Configuration) {
    // Apply settings defaults
    if config.settings.show_env_in_prompt.is_none() {
        config.settings.show_env_in_prompt = Some(true);
    }

    // Apply environment defaults
    for env in config.environments.values_mut() {
        if env.requires_confirmation.is_none() {
            env.requires_confirmation = Some(false);
        }
    }
}

/// Interpolate environment variables in configuration
fn interpolate_environment_variables(config: &mut Configuration) -> Result<(), ConfigError> {
    for env in config.environments.values_mut() {
        // Interpolate description
        env.description = interpolate_string(&env.description)?;

        // Interpolate file paths
        let mut interpolated_files = Vec::new();
        for file in &env.files {
            let path_str = file.to_string_lossy();
            let interpolated = interpolate_string(&path_str)?;
            interpolated_files.push(PathBuf::from(interpolated));
        }
        env.files = interpolated_files;
    }

    Ok(())
}

/// Interpolate environment variables in a single string
fn interpolate_string(input: &str) -> Result<String, ConfigError> {
    let mut result = input.to_string();

    // Find and replace ${VAR} patterns
    while let Some(start) = result.find("${") {
        if let Some(end) = result[start..].find('}') {
            let var_name = &result[start + 2..start + end];

            let replacement = env::var(var_name).map_err(|_| ConfigError::InterpolationError {
                variable: var_name.to_string(),
            })?;

            result.replace_range(start..start + end + 1, &replacement);
        } else {
            break;
        }
    }

    Ok(result)
}

/// Validate that all referenced files exist
fn validate_file_paths(config: &Configuration, project_path: &Path) -> Result<(), ConfigError> {
    let stand_dir = project_path.join(".stand");

    for env in config.environments.values() {
        for file in &env.files {
            let file_path = stand_dir.join(file);
            if !file_path.exists() {
                return Err(ConfigError::FileNotFound {
                    path: file.to_string_lossy().to_string(),
                });
            }
        }
    }

    Ok(())
}

/// Apply hierarchical merging (extends functionality)
fn apply_hierarchical_merge(config: &mut Configuration) -> Result<(), ConfigError> {
    let mut processed = HashSet::new();
    let env_names: Vec<String> = config.environments.keys().cloned().collect();

    for env_name in env_names {
        if !processed.contains(&env_name) {
            merge_environment_hierarchy(config, &env_name, &mut processed)?;
        }
    }

    Ok(())
}

/// Merge a single environment with its parent hierarchy
fn merge_environment_hierarchy(
    config: &mut Configuration,
    env_name: &str,
    processed: &mut HashSet<String>,
) -> Result<(), ConfigError> {
    if processed.contains(env_name) {
        return Ok(());
    }

    let env = config.environments.get(env_name).cloned();
    if let Some(mut current_env) = env {
        if let Some(extends) = &current_env.extends {
            // Process parent first
            merge_environment_hierarchy(config, extends, processed)?;

            // Get parent environment
            if let Some(parent_env) = config.environments.get(extends).cloned() {
                // Merge files: parent files first, then current files
                let mut merged_files = parent_env.files;
                merged_files.extend(current_env.files);
                current_env.files = merged_files;

                // Update the environment in config
                config
                    .environments
                    .insert(env_name.to_string(), current_env);
            }
        }
    }

    processed.insert(env_name.to_string());
    Ok(())
}
