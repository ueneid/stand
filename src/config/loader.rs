use crate::config::types::Configuration;
use crate::config::validator::{validate_required_fields, validate_environment_references, validate_no_circular_references, validate_common_config};
use crate::config::ConfigError;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;

/// Load configuration from TOML file (.stand)
pub fn load_config_toml(project_path: &Path) -> Result<Configuration, ConfigError> {
    let config_path = project_path.join(".stand");

    if !config_path.exists() {
        return Err(ConfigError::ValidationError {
            message: "Stand configuration not found. Run 'stand init' to initialize.".to_string(),
        });
    }

    let content = fs::read_to_string(&config_path)?;
    let mut config: Configuration = toml::from_str(&content)
        .map_err(|e| ConfigError::ValidationError {
            message: format!("Failed to parse TOML configuration: {}", e),
        })?;

    // Apply environment variable interpolation
    interpolate_configuration(&mut config)?;

    Ok(config)
}

/// Load configuration from TOML file with variable inheritance
pub fn load_config_toml_with_inheritance(project_path: &Path) -> Result<Configuration, ConfigError> {
    let mut config = load_config_toml(project_path)?;
    
    // Apply variable inheritance
    apply_variable_inheritance(&mut config)?;
    
    Ok(config)
}

/// Load configuration from the given directory (legacy YAML format)
pub fn load_config(project_path: &Path) -> Result<Configuration, ConfigError> {
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

/// Load configuration with comprehensive validation
pub fn load_config_with_validation(project_path: &Path) -> Result<Configuration, ConfigError> {
    let config = load_config_basic(project_path)?;

    // Validate required fields
    validate_required_fields(&config)?;

    // Validate environment references
    validate_environment_references(&config)?;

    // Validate circular references
    validate_no_circular_references(&config)?;

    // Validate common configuration if present
    validate_common_config(&config)?;

    Ok(config)
}

/// Load configuration with default values applied
pub fn load_config_with_defaults(project_path: &Path) -> Result<Configuration, ConfigError> {
    let mut config = load_config_basic(project_path)?;

    // Apply defaults
    apply_default_values(&mut config);

    Ok(config)
}

// TODO: Update for new TOML format 
// /// Load configuration with environment variable interpolation
// pub fn load_config_with_interpolation(project_path: &Path) -> Result<Configuration, ConfigError> {
//     let mut config = load_config_basic(project_path)?;

//     // Interpolate environment variables
//     interpolate_environment_variables(&mut config)?;

//     Ok(config)
// }

// TODO: Update for new TOML format without files field
// /// Load configuration with file path validation
// pub fn load_config_with_file_validation(project_path: &Path) -> Result<Configuration, ConfigError> {
//     let config = load_config_basic(project_path)?;

//     // Validate that all referenced files exist
//     validate_file_paths(&config, project_path)?;

//     Ok(config)
// }

// TODO: Update for TOML format with variable inheritance
// /// Load configuration with hierarchical merge support
// pub fn load_config_with_hierarchy(project_path: &Path) -> Result<Configuration, ConfigError> {
//     let mut config = load_config_basic(project_path)?;

//     // Apply hierarchical merging
//     apply_hierarchical_merge(&mut config)?;

//     Ok(config)
// }

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

// TODO: Replaced by interpolate_configuration for new TOML format
// /// Interpolate environment variables in configuration
// fn interpolate_environment_variables(config: &mut Configuration) -> Result<(), ConfigError> {
//     for env in config.environments.values_mut() {
//         // Interpolate description
//         env.description = interpolate_string(&env.description)?;

//         // Interpolate file paths
//         let mut interpolated_files = Vec::new();
//         for file in &env.files {
//             let path_str = file.to_string_lossy();
//             let interpolated = interpolate_string(&path_str)?;
//             interpolated_files.push(PathBuf::from(interpolated));
//         }
//         env.files = interpolated_files;
//     }

//     Ok(())
// }

/// Interpolate environment variables in a single string
/// Uses single-pass expansion to avoid reprocessing inserted content
/// Supports ${VAR} format only - nested expansions are not supported
fn interpolate_string(input: &str) -> Result<String, ConfigError> {
    let mut result = String::new();
    let mut chars = input.char_indices();
    let input_bytes = input.as_bytes();
    
    while let Some((i, ch)) = chars.next() {
        if ch == '$' && i + 1 < input.len() && input_bytes[i + 1] == b'{' {
            // Skip the '{' character
            chars.next();
            
            // Find the end of the variable name
            let var_start = i + 2;
            let mut var_end = None;
            
            for (pos, ch) in chars.by_ref() {
                if ch == '}' {
                    var_end = Some(pos);
                    break;
                }
            }
            
            let var_end = var_end.ok_or_else(|| ConfigError::ValidationError {
                message: format!(
                    "Unterminated variable placeholder starting at position {}: missing closing '}}' for '${{...'", 
                    i
                ),
            })?;
            
            let var_name = &input[var_start..var_end];
            
            // Empty variable names are not allowed
            if var_name.is_empty() {
                return Err(ConfigError::ValidationError {
                    message: format!(
                        "Empty variable name in placeholder at position {}: '${{}}' is not valid",
                        i
                    ),
                });
            }
            
            let replacement = env::var(var_name).map_err(|_| ConfigError::InterpolationError {
                variable: var_name.to_string(),
            })?;
            
            result.push_str(&replacement);
        } else {
            result.push(ch);
        }
    }
    
    Ok(result)
}

/// Apply environment variable interpolation to the entire configuration
fn interpolate_configuration(config: &mut Configuration) -> Result<(), ConfigError> {
    // Interpolate common variables
    if let Some(ref mut common) = config.common {
        for (_, value) in common.iter_mut() {
            *value = interpolate_string(value)?;
        }
    }

    // Interpolate environment variables and descriptions
    for (_, env) in config.environments.iter_mut() {
        // Interpolate description
        env.description = interpolate_string(&env.description)?;
        
        // Interpolate all environment variables
        for (_, value) in env.variables.iter_mut() {
            *value = interpolate_string(value)?;
        }
    }

    Ok(())
}

/// Apply variable inheritance (common variables and extends relationships)
fn apply_variable_inheritance(config: &mut Configuration) -> Result<(), ConfigError> {
    // First, merge common variables into all environments
    if let Some(common) = &config.common {
        let common_vars = common.clone();
        for env in config.environments.values_mut() {
            // Common variables go first (lowest priority)
            let mut merged_vars = common_vars.clone();
            // Environment variables override common variables
            merged_vars.extend(env.variables.clone());
            env.variables = merged_vars;
        }
    }
    
    // Then apply inheritance through extends relationships
    let mut processed = HashSet::new();
    let env_names: Vec<String> = config.environments.keys().cloned().collect();

    for env_name in env_names {
        if !processed.contains(&env_name) {
            apply_environment_inheritance(config, &env_name, &mut processed, &mut Vec::new())?;
        }
    }

    Ok(())
}

/// Apply inheritance for a single environment and its hierarchy
fn apply_environment_inheritance(
    config: &mut Configuration,
    env_name: &str,
    processed: &mut HashSet<String>,
    inheritance_chain: &mut Vec<String>,
) -> Result<(), ConfigError> {
    // Check for circular reference
    if inheritance_chain.contains(&env_name.to_string()) {
        return Err(ConfigError::CircularReference {
            cycle: inheritance_chain.clone(),
        });
    }
    
    if processed.contains(env_name) {
        return Ok(());
    }
    
    inheritance_chain.push(env_name.to_string());
    
    // Get a copy of the environment to avoid borrow checker issues
    let env = config.environments.get(env_name).cloned()
        .ok_or_else(|| ConfigError::InvalidEnvironment { name: env_name.to_string() })?;
    
    if let Some(parent_name) = &env.extends {
        // Process parent first
        apply_environment_inheritance(config, parent_name, processed, inheritance_chain)?;
        
        // Get parent environment data before mutable borrow
        let parent_data = config.environments.get(parent_name)
            .map(|p| (p.variables.clone(), p.color.clone(), p.requires_confirmation))
            .unwrap_or_default();
        
        // Merge: parent variables first, then current environment variables (child overrides parent)
        if let Some(current_env) = config.environments.get_mut(env_name) {
            let mut merged_vars = parent_data.0;
            merged_vars.extend(current_env.variables.clone());
            current_env.variables = merged_vars;
            
            // Also inherit other properties if not set
            if current_env.color.is_none() {
                current_env.color = parent_data.1;
            }
            if current_env.requires_confirmation.is_none() {
                current_env.requires_confirmation = parent_data.2;
            }
        }
    }
    
    inheritance_chain.pop();
    processed.insert(env_name.to_string());
    Ok(())
}

// TODO: Update for new TOML format - no longer needed since variables are in config file
// /// Validate that all referenced files exist and are files (not directories)
// fn validate_file_paths(config: &Configuration, project_path: &Path) -> Result<(), ConfigError> {
//     let stand_dir = project_path.join(".stand");

//     for env in config.environments.values() {
//         for file in &env.files {
//             let configured_path = file.to_string_lossy().to_string();
//             let resolved_path = stand_dir.join(file);
//             let resolved_path_str = resolved_path.to_string_lossy().to_string();
            
//             if !resolved_path.exists() {
//                 return Err(ConfigError::FileNotFound {
//                     configured_path,
//                     resolved_path: resolved_path_str,
//                 });
//             }
            
//             if !resolved_path.is_file() {
//                 return Err(ConfigError::NotAFile {
//                     configured_path,
//                     resolved_path: resolved_path_str,
//                 });
//             }
//         }
//     }

//     Ok(())
// }

// TODO: Update for new TOML format with variable inheritance instead of file merging
// /// Apply hierarchical merging (extends functionality)
// fn apply_hierarchical_merge(config: &mut Configuration) -> Result<(), ConfigError> {
//     // First, merge common.files into all environments if common exists
//     if let Some(common) = &config.common {
//         let common_files = common.files.clone();
//         for env in config.environments.values_mut() {
//             let mut merged_files = common_files.clone();
//             merged_files.extend(env.files.clone());
//             env.files = deduplicate_files(merged_files);
//         }
//     }
    
//     // Then apply hierarchical merging for extends relationships
//     let mut processed = HashSet::new();
//     let env_names: Vec<String> = config.environments.keys().cloned().collect();

//     for env_name in env_names {
//         if !processed.contains(&env_name) {
//             merge_environment_hierarchy(config, &env_name, &mut processed)?;
//         }
//     }

//     Ok(())
// }

// TODO: Update for TOML format with variable inheritance  
// /// Merge a single environment with its parent hierarchy
// fn merge_environment_hierarchy(
//     config: &mut Configuration,
//     env_name: &str,
//     processed: &mut HashSet<String>,
// ) -> Result<(), ConfigError> {
//     if processed.contains(env_name) {
//         return Ok(());
//     }
// 
//     let env = config.environments.get(env_name).cloned();
//     if let Some(mut current_env) = env {
//         if let Some(extends) = &current_env.extends {
//             // Process parent first
//             merge_environment_hierarchy(config, extends, processed)?;
// 
//             // Get parent environment
//             if let Some(parent_env) = config.environments.get(extends).cloned() {
//                 // Merge files: parent files first, then current files, deduplicated
//                 let mut merged_files = parent_env.files;
//                 merged_files.extend(current_env.files);
//                 current_env.files = deduplicate_files(merged_files);
//                 
//                 // Merge color: child overrides parent
//                 if current_env.color.is_none() && parent_env.color.is_some() {
//                     current_env.color = parent_env.color;
//                 }
//                 
//                 // Merge requires_confirmation: child overrides parent
//                 if current_env.requires_confirmation.is_none() && parent_env.requires_confirmation.is_some() {
//                     current_env.requires_confirmation = parent_env.requires_confirmation;
//                 }
// 
//                 // Update the environment in config
//                 config
//                     .environments
//                     .insert(env_name.to_string(), current_env);
//             }
//         }
//     }
// 
//     processed.insert(env_name.to_string());
//     Ok(())
// }

// /// Deduplicate files while preserving order (first occurrence wins)
// fn deduplicate_files(files: Vec<PathBuf>) -> Vec<PathBuf> {
//     let mut seen = HashSet::new();
//     let mut result = Vec::new();
//     
//     for file in files {
//         if seen.insert(file.clone()) {
//             result.push(file);
//         }
//     }
//     
//     result
// }
