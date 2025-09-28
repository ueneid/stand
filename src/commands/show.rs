use crate::config::loader;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::Path;

/// Shows environment variables for the specified environment
pub fn show_environment(project_path: &Path, env_name: &str, show_values: bool) -> Result<String> {
    // Load configuration with inheritance applied
    let config_with_inheritance = loader::load_config_toml_with_inheritance(project_path)?;

    // Load raw configuration for source detection
    let raw_config = loader::load_config_toml(project_path)?;

    // Check if environment exists
    let env = config_with_inheritance.environments.get(env_name)
        .ok_or_else(|| {
            let mut available: Vec<_> = config_with_inheritance.environments.keys().cloned().collect();
            available.sort();
            anyhow!("Environment '{}' not found. Available: {}", env_name, available.join(", "))
        })?;

    // Detect variable sources
    let sources = detect_variable_sources(&raw_config, env_name)?;

    // Format output
    let output = format_variables(env_name, &env.variables, &sources, show_values);

    Ok(output)
}

/// Enum to represent the source of a variable
#[derive(Debug, Clone, PartialEq)]
enum VarSource {
    Local,
    Inherited(String),
    Common,
}

/// Detect the source of each variable (local, inherited, or common)
fn detect_variable_sources(
    raw_config: &crate::config::types::Configuration,
    env_name: &str,
) -> Result<HashMap<String, VarSource>> {
    let mut sources = HashMap::new();

    // Get environment
    let env = raw_config.environments.get(env_name)
        .ok_or_else(|| anyhow!("Environment '{}' not found", env_name))?;

    // Get inheritance chain
    let inheritance_chain = get_inheritance_chain(raw_config, env_name)?;

    // Variables in common section
    let common_vars: HashMap<String, String> = raw_config.common.clone().unwrap_or_default();

    // Process all variables that would be available after inheritance
    let mut all_vars = HashMap::new();

    // Start with common variables
    all_vars.extend(common_vars.clone());

    // Apply inheritance chain
    for ancestor_name in inheritance_chain.iter().rev() {
        if let Some(ancestor) = raw_config.environments.get(ancestor_name) {
            all_vars.extend(ancestor.variables.clone());
        }
    }

    // Now determine sources
    for var_name in all_vars.keys() {
        // Check if variable is defined locally in the target environment
        if env.variables.contains_key(var_name) {
            sources.insert(var_name.clone(), VarSource::Local);
        } else {
            // Check inheritance chain (excluding the target environment itself)
            let mut found_in_ancestor = false;
            for ancestor_name in &inheritance_chain[1..] {
                if let Some(ancestor) = raw_config.environments.get(ancestor_name) {
                    if ancestor.variables.contains_key(var_name) {
                        sources.insert(var_name.clone(), VarSource::Inherited(ancestor_name.clone()));
                        found_in_ancestor = true;
                        break;
                    }
                }
            }

            // If not found in ancestors, check if it's from common
            if !found_in_ancestor && common_vars.contains_key(var_name) {
                sources.insert(var_name.clone(), VarSource::Common);
            }
        }
    }

    Ok(sources)
}

/// Get inheritance chain from environment to root (including the environment itself)
fn get_inheritance_chain(
    config: &crate::config::types::Configuration,
    env_name: &str,
) -> Result<Vec<String>> {
    let mut chain = Vec::new();
    let mut current = env_name;

    loop {
        chain.push(current.to_string());

        if let Some(env) = config.environments.get(current) {
            if let Some(parent) = &env.extends {
                current = parent;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    Ok(chain)
}

/// Format variables for display
fn format_variables(
    env_name: &str,
    variables: &HashMap<String, String>,
    sources: &HashMap<String, VarSource>,
    show_values: bool,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("Environment: {}\n", env_name));
    output.push_str("Variables:\n");

    // Sort variables alphabetically
    let mut var_names: Vec<_> = variables.keys().collect();
    var_names.sort();

    for var_name in var_names {
        let value = &variables[var_name];
        let source = sources.get(var_name).unwrap_or(&VarSource::Local);

        let line = if show_values {
            format!("  {}={}", var_name, value)
        } else {
            format!("  {}", var_name)
        };

        let suffix = match source {
            VarSource::Local => "".to_string(),
            VarSource::Inherited(ancestor) => format!(" (inherited from {})", ancestor),
            VarSource::Common => " (from common)".to_string(),
        };

        output.push_str(&format!("{}{}\n", line, suffix));
    }

    output
}
