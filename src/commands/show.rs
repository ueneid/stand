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
    let env = config_with_inheritance
        .environments
        .get(env_name)
        .ok_or_else(|| {
            let mut available: Vec<_> = config_with_inheritance
                .environments
                .keys()
                .cloned()
                .collect();
            available.sort();
            anyhow!(
                "Environment '{}' not found. Available: {}",
                env_name,
                available.join(", ")
            )
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
    let env = raw_config
        .environments
        .get(env_name)
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
                        sources.insert(
                            var_name.clone(),
                            VarSource::Inherited(ancestor_name.clone()),
                        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::{Configuration, Environment, Settings};
    use std::collections::HashMap;

    fn create_test_config() -> Configuration {
        let mut environments = HashMap::new();
        let mut common = HashMap::new();

        common.insert("APP_NAME".to_string(), "MyApp".to_string());
        common.insert("LOG_FORMAT".to_string(), "json".to_string());

        let mut base_vars = HashMap::new();
        base_vars.insert("LOG_LEVEL".to_string(), "info".to_string());
        base_vars.insert("PORT".to_string(), "3000".to_string());

        let mut dev_vars = HashMap::new();
        dev_vars.insert("LOG_LEVEL".to_string(), "debug".to_string());
        dev_vars.insert("DEBUG".to_string(), "true".to_string());

        environments.insert(
            "base".to_string(),
            Environment {
                description: "Base environment".to_string(),
                extends: None,
                variables: base_vars,
                color: None,
                requires_confirmation: None,
            },
        );

        environments.insert(
            "dev".to_string(),
            Environment {
                description: "Development environment".to_string(),
                extends: Some("base".to_string()),
                variables: dev_vars,
                color: Some("green".to_string()),
                requires_confirmation: None,
            },
        );

        Configuration {
            version: "2.0".to_string(),
            environments,
            common: Some(common),
            settings: Settings::default(),
        }
    }

    #[test]
    fn test_get_inheritance_chain() {
        let config = create_test_config();

        let chain = get_inheritance_chain(&config, "dev").unwrap();
        assert_eq!(chain, vec!["dev", "base"]);

        let chain = get_inheritance_chain(&config, "base").unwrap();
        assert_eq!(chain, vec!["base"]);
    }

    #[test]
    fn test_detect_variable_sources() {
        let config = create_test_config();

        let sources = detect_variable_sources(&config, "dev").unwrap();

        // APP_NAME and LOG_FORMAT should be from common
        assert_eq!(sources.get("APP_NAME"), Some(&VarSource::Common));
        assert_eq!(sources.get("LOG_FORMAT"), Some(&VarSource::Common));

        // DEBUG should be local to dev
        assert_eq!(sources.get("DEBUG"), Some(&VarSource::Local));

        // LOG_LEVEL should be local to dev (overrides base)
        assert_eq!(sources.get("LOG_LEVEL"), Some(&VarSource::Local));

        // PORT should be inherited from base
        assert_eq!(
            sources.get("PORT"),
            Some(&VarSource::Inherited("base".to_string()))
        );
    }

    #[test]
    fn test_format_variables_names_only() {
        let mut variables = HashMap::new();
        variables.insert("APP_NAME".to_string(), "MyApp".to_string());
        variables.insert("DEBUG".to_string(), "true".to_string());

        let mut sources = HashMap::new();
        sources.insert("APP_NAME".to_string(), VarSource::Common);
        sources.insert("DEBUG".to_string(), VarSource::Local);

        let output = format_variables("dev", &variables, &sources, false);

        assert!(output.contains("Environment: dev"));
        assert!(output.contains("Variables:"));
        assert!(output.contains("APP_NAME (from common)"));
        assert!(output.contains("DEBUG"));
        assert!(!output.contains("DEBUG ("));
        assert!(!output.contains("="));
        assert!(!output.contains("MyApp"));
        assert!(!output.contains("true"));
    }

    #[test]
    fn test_format_variables_with_values() {
        let mut variables = HashMap::new();
        variables.insert("APP_NAME".to_string(), "MyApp".to_string());
        variables.insert("DEBUG".to_string(), "true".to_string());

        let mut sources = HashMap::new();
        sources.insert("APP_NAME".to_string(), VarSource::Common);
        sources.insert("DEBUG".to_string(), VarSource::Local);

        let output = format_variables("dev", &variables, &sources, true);

        assert!(output.contains("Environment: dev"));
        assert!(output.contains("APP_NAME=MyApp (from common)"));
        assert!(output.contains("DEBUG=true"));
        assert!(!output.contains("DEBUG=true ("));
    }
}
