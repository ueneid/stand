use anyhow::Result;
use indexmap::IndexMap;
use std::env;
use std::path::PathBuf;

use crate::environment::loader::{load_env_file_with_options, LoadError};
use crate::environment::parser::ParseOptions;

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("Circular reference detected in variable expansion: {cycle:?}")]
    CircularReference { cycle: Vec<String> },

    #[error("Undefined variable referenced: {variable}")]
    UndefinedVariable { variable: String },

    #[error("Error loading from source: {source}")]
    SourceError { source: LoadError },
}

#[derive(Debug, Clone)]
pub enum VariableSource {
    Default(IndexMap<String, String>),
    EnvFile(PathBuf),
    SystemEnv,
    CliArgs(IndexMap<String, String>),
}

#[derive(Debug, Clone)]
pub enum UndefinedVariableBehavior {
    Error,
    EmptyString,
    LeaveUnexpanded,
}

#[derive(Debug, Clone)]
pub struct ResolutionOptions {
    pub undefined_variable_behavior: UndefinedVariableBehavior,
}

impl Default for ResolutionOptions {
    fn default() -> Self {
        Self {
            undefined_variable_behavior: UndefinedVariableBehavior::EmptyString,
        }
    }
}

#[derive(Debug)]
pub struct EnvironmentResolver {
    sources: Vec<VariableSource>,
}

impl EnvironmentResolver {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    pub fn add_source(&mut self, source: VariableSource) {
        self.sources.push(source);
    }

    pub fn resolve(&self) -> Result<IndexMap<String, String>, ResolveError> {
        self.resolve_with_options(&ResolutionOptions::default())
    }

    pub fn resolve_with_options(
        &self,
        options: &ResolutionOptions,
    ) -> Result<IndexMap<String, String>, ResolveError> {
        // Step 1: Collect variables from all sources in order (later sources override earlier ones)
        let mut variables = IndexMap::new();

        for source in &self.sources {
            let source_vars = self.load_source_variables(source)?;
            for (key, value) in source_vars {
                variables.insert(key, value);
            }
        }

        // Step 2: Expand variables with circular reference detection
        self.expand_variables(variables, options)
    }

    fn load_source_variables(
        &self,
        source: &VariableSource,
    ) -> Result<IndexMap<String, String>, ResolveError> {
        match source {
            VariableSource::Default(vars) => Ok(vars.clone()),

            VariableSource::EnvFile(path) => {
                let parse_options = ParseOptions {
                    expand_variables: false,
                };
                load_env_file_with_options(path, &parse_options)
                    .map_err(|e| ResolveError::SourceError { source: e })
            }

            VariableSource::SystemEnv => {
                let mut vars = IndexMap::new();
                for (key, value) in env::vars() {
                    vars.insert(key, value);
                }
                Ok(vars)
            }

            VariableSource::CliArgs(vars) => Ok(vars.clone()),
        }
    }

    fn expand_variables(
        &self,
        variables: IndexMap<String, String>,
        options: &ResolutionOptions,
    ) -> Result<IndexMap<String, String>, ResolveError> {
        let mut resolved = IndexMap::new();

        for (key, value) in &variables {
            let mut expansion_stack = Vec::new(); // Fresh stack for each variable
            let expanded_value =
                Self::expand_value(value, &variables, options, &mut expansion_stack)?;
            resolved.insert(key.clone(), expanded_value);
        }

        Ok(resolved)
    }

    fn expand_value(
        value: &str,
        all_variables: &IndexMap<String, String>,
        options: &ResolutionOptions,
        expansion_stack: &mut Vec<String>,
    ) -> Result<String, ResolveError> {
        let mut result = value.to_string();

        // Find and expand all ${VAR} patterns
        while let Some(start) = result.find("${") {
            if let Some(end) = result[start..].find('}') {
                let var_name = &result[start + 2..start + end];

                // Check for circular reference
                if expansion_stack.contains(&var_name.to_string()) {
                    // Find the cycle starting from where the variable was first encountered
                    let start_pos = expansion_stack.iter().position(|v| v == var_name).unwrap();
                    let mut cycle: Vec<String> = expansion_stack[start_pos..].to_vec();
                    cycle.push(var_name.to_string());
                    return Err(ResolveError::CircularReference { cycle });
                }

                // Get the variable value
                let replacement = if let Some(var_value) = all_variables.get(var_name) {
                    // Recursively expand the variable value
                    expansion_stack.push(var_name.to_string());
                    let expanded =
                        Self::expand_value(var_value, all_variables, options, expansion_stack)?;
                    expansion_stack.pop();
                    expanded
                } else {
                    // Handle undefined variable based on options
                    match options.undefined_variable_behavior {
                        UndefinedVariableBehavior::Error => {
                            return Err(ResolveError::UndefinedVariable {
                                variable: var_name.to_string(),
                            });
                        }
                        UndefinedVariableBehavior::EmptyString => String::new(),
                        UndefinedVariableBehavior::LeaveUnexpanded => {
                            format!("${{{}}}", var_name)
                        }
                    }
                };

                // Replace the variable reference
                result.replace_range(start..start + end + 1, &replacement);
            } else {
                // No closing brace found, stop expansion
                break;
            }
        }

        Ok(result)
    }
}

impl Default for EnvironmentResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_resolver_basic() {
        let mut resolver = EnvironmentResolver::new();

        let mut defaults = IndexMap::new();
        defaults.insert("KEY".to_string(), "value".to_string());
        resolver.add_source(VariableSource::Default(defaults));

        let resolved = resolver.resolve().unwrap();
        assert_eq!(resolved.get("KEY"), Some(&"value".to_string()));
    }

    #[test]
    fn test_variable_source_priority() {
        let mut resolver = EnvironmentResolver::new();

        // Lower priority
        let mut defaults = IndexMap::new();
        defaults.insert("KEY".to_string(), "default".to_string());
        resolver.add_source(VariableSource::Default(defaults));

        // Higher priority
        let mut cli_args = IndexMap::new();
        cli_args.insert("KEY".to_string(), "cli".to_string());
        resolver.add_source(VariableSource::CliArgs(cli_args));

        let resolved = resolver.resolve().unwrap();
        assert_eq!(resolved.get("KEY"), Some(&"cli".to_string()));
    }

    #[test]
    fn test_variable_expansion_basic() {
        let mut resolver = EnvironmentResolver::new();

        let mut variables = IndexMap::new();
        variables.insert("BASE".to_string(), "https://api.example.com".to_string());
        variables.insert("ENDPOINT".to_string(), "${BASE}/v1".to_string());
        resolver.add_source(VariableSource::Default(variables));

        let resolved = resolver.resolve().unwrap();
        assert_eq!(
            resolved.get("ENDPOINT"),
            Some(&"https://api.example.com/v1".to_string())
        );
    }

    #[test]
    fn test_circular_reference_detection() {
        let mut resolver = EnvironmentResolver::new();

        let mut variables = IndexMap::new();
        variables.insert("A".to_string(), "${B}".to_string());
        variables.insert("B".to_string(), "${A}".to_string());
        resolver.add_source(VariableSource::Default(variables));

        let result = resolver.resolve();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ResolveError::CircularReference { .. }
        ));
    }
}
