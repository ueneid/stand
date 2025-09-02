use std::env;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use indexmap::IndexMap;

use stand::environment::resolver::{
    EnvironmentResolver, VariableSource, ResolveError, ResolutionOptions, UndefinedVariableBehavior
};

#[test]
fn test_resolve_basic_priority() {
    let mut resolver = EnvironmentResolver::new();
    
    // Add sources in reverse priority order (lowest to highest)
    let mut defaults = IndexMap::new();
    defaults.insert("KEY".to_string(), "default_value".to_string());
    resolver.add_source(VariableSource::Default(defaults));
    
    let mut cli_args = IndexMap::new();
    cli_args.insert("KEY".to_string(), "cli_value".to_string());
    resolver.add_source(VariableSource::CliArgs(cli_args));
    
    let resolved = resolver.resolve().unwrap();
    
    // CLI args should override defaults
    assert_eq!(resolved.get("KEY"), Some(&"cli_value".to_string()));
}

#[test]
fn test_resolve_multiple_sources_priority_order() {
    let mut resolver = EnvironmentResolver::new();
    
    // Default values (lowest priority)
    let mut defaults = IndexMap::new();
    defaults.insert("DATABASE_URL".to_string(), "default_db".to_string());
    defaults.insert("API_KEY".to_string(), "default_key".to_string());
    defaults.insert("DEBUG".to_string(), "false".to_string());
    resolver.add_source(VariableSource::Default(defaults));
    
    // System environment (medium-high priority)
    env::set_var("API_KEY", "system_key");
    env::set_var("DEBUG", "true");
    resolver.add_source(VariableSource::SystemEnv);
    
    // CLI arguments (highest priority)
    let mut cli_args = IndexMap::new();
    cli_args.insert("DEBUG".to_string(), "false".to_string());
    resolver.add_source(VariableSource::CliArgs(cli_args));
    
    let resolved = resolver.resolve().unwrap();
    
    // Check priority order: CLI > System Env > Defaults
    assert_eq!(resolved.get("DATABASE_URL"), Some(&"default_db".to_string()));  // From defaults
    assert_eq!(resolved.get("API_KEY"), Some(&"system_key".to_string()));       // From system env
    assert_eq!(resolved.get("DEBUG"), Some(&"false".to_string()));              // From CLI (highest)
    
    // Clean up environment variables
    env::remove_var("API_KEY");
    env::remove_var("DEBUG");
}

#[test]
fn test_resolve_with_env_file() {
    let temp_dir = TempDir::new().unwrap();
    let env_file = temp_dir.path().join(".env");
    
    fs::write(&env_file, "DATABASE_URL=file_database\nAPI_KEY=file_key").unwrap();
    
    let mut resolver = EnvironmentResolver::new();
    
    // Add default values
    let mut defaults = IndexMap::new();
    defaults.insert("DATABASE_URL".to_string(), "default_database".to_string());
    defaults.insert("PORT".to_string(), "3000".to_string());
    resolver.add_source(VariableSource::Default(defaults));
    
    // Add env file
    resolver.add_source(VariableSource::EnvFile(env_file.clone()));
    
    let resolved = resolver.resolve().unwrap();
    
    // Env file should override defaults
    assert_eq!(resolved.get("DATABASE_URL"), Some(&"file_database".to_string()));
    assert_eq!(resolved.get("API_KEY"), Some(&"file_key".to_string()));
    assert_eq!(resolved.get("PORT"), Some(&"3000".to_string())); // From defaults
}

#[test]
fn test_resolve_multiple_env_files() {
    let temp_dir = TempDir::new().unwrap();
    let base_env = temp_dir.path().join(".env");
    let override_env = temp_dir.path().join(".env.local");
    
    fs::write(&base_env, "DATABASE_URL=base_db\nAPI_KEY=base_key\nDEBUG=false").unwrap();
    fs::write(&override_env, "DATABASE_URL=local_db\nPORT=4000").unwrap();
    
    let mut resolver = EnvironmentResolver::new();
    resolver.add_source(VariableSource::EnvFile(base_env));
    resolver.add_source(VariableSource::EnvFile(override_env)); // Later files override earlier ones
    
    let resolved = resolver.resolve().unwrap();
    
    assert_eq!(resolved.get("DATABASE_URL"), Some(&"local_db".to_string()));   // Overridden
    assert_eq!(resolved.get("API_KEY"), Some(&"base_key".to_string()));        // From base
    assert_eq!(resolved.get("DEBUG"), Some(&"false".to_string()));             // From base
    assert_eq!(resolved.get("PORT"), Some(&"4000".to_string()));               // From override
}

#[test]
fn test_resolve_with_variable_expansion() {
    let mut resolver = EnvironmentResolver::new();
    
    let mut variables = IndexMap::new();
    variables.insert("BASE_URL".to_string(), "https://api.example.com".to_string());
    variables.insert("API_ENDPOINT".to_string(), "${BASE_URL}/v1".to_string());
    variables.insert("USERS_ENDPOINT".to_string(), "${API_ENDPOINT}/users".to_string());
    resolver.add_source(VariableSource::Default(variables));
    
    let resolved = resolver.resolve().unwrap();
    
    assert_eq!(resolved.get("BASE_URL"), Some(&"https://api.example.com".to_string()));
    assert_eq!(resolved.get("API_ENDPOINT"), Some(&"https://api.example.com/v1".to_string()));
    assert_eq!(resolved.get("USERS_ENDPOINT"), Some(&"https://api.example.com/v1/users".to_string()));
}

#[test]
fn test_resolve_cross_source_variable_expansion() {
    let temp_dir = TempDir::new().unwrap();
    let env_file = temp_dir.path().join(".env");
    
    fs::write(&env_file, "API_ENDPOINT=${BASE_URL}/v1").unwrap();
    
    let mut resolver = EnvironmentResolver::new();
    
    // Base URL from defaults
    let mut defaults = IndexMap::new();
    defaults.insert("BASE_URL".to_string(), "https://api.example.com".to_string());
    resolver.add_source(VariableSource::Default(defaults));
    
    // API endpoint from file, referencing BASE_URL
    resolver.add_source(VariableSource::EnvFile(env_file));
    
    let resolved = resolver.resolve().unwrap();
    
    assert_eq!(resolved.get("BASE_URL"), Some(&"https://api.example.com".to_string()));
    assert_eq!(resolved.get("API_ENDPOINT"), Some(&"https://api.example.com/v1".to_string()));
}

#[test]
fn test_resolve_undefined_variable_handling() {
    let mut resolver = EnvironmentResolver::new();
    
    let mut variables = IndexMap::new();
    variables.insert("MISSING_REF".to_string(), "${UNDEFINED_VAR}".to_string());
    variables.insert("PARTIAL_REF".to_string(), "prefix-${UNDEFINED_VAR}-suffix".to_string());
    resolver.add_source(VariableSource::Default(variables));
    
    let options = ResolutionOptions {
        undefined_variable_behavior: UndefinedVariableBehavior::EmptyString,
    };
    let resolved = resolver.resolve_with_options(&options).unwrap();
    
    assert_eq!(resolved.get("MISSING_REF"), Some(&"".to_string()));
    assert_eq!(resolved.get("PARTIAL_REF"), Some(&"prefix--suffix".to_string()));
}

#[test]
fn test_resolve_circular_reference_detection() {
    let mut resolver = EnvironmentResolver::new();
    
    let mut variables = IndexMap::new();
    variables.insert("VAR_A".to_string(), "${VAR_B}".to_string());
    variables.insert("VAR_B".to_string(), "${VAR_C}".to_string());
    variables.insert("VAR_C".to_string(), "${VAR_A}".to_string());
    resolver.add_source(VariableSource::Default(variables));
    
    let result = resolver.resolve();
    
    assert!(result.is_err());
    match result.unwrap_err() {
        ResolveError::CircularReference { cycle } => {
            assert!(cycle.contains(&"VAR_A".to_string()));
            assert!(cycle.contains(&"VAR_B".to_string()));
            assert!(cycle.contains(&"VAR_C".to_string()));
        }
        _ => panic!("Expected CircularReference error"),
    }
}

#[test]
fn test_resolve_self_reference_detection() {
    let mut resolver = EnvironmentResolver::new();
    
    let mut variables = IndexMap::new();
    variables.insert("SELF_REF".to_string(), "prefix-${SELF_REF}-suffix".to_string());
    resolver.add_source(VariableSource::Default(variables));
    
    let result = resolver.resolve();
    
    assert!(result.is_err());
    match result.unwrap_err() {
        ResolveError::CircularReference { cycle } => {
            // Self-reference creates a cycle with 2 elements: [SELF_REF, SELF_REF]
            assert_eq!(cycle.len(), 2);
            assert_eq!(cycle[0], "SELF_REF");
            assert_eq!(cycle[1], "SELF_REF");
        }
        _ => panic!("Expected CircularReference error"),
    }
}

#[test]
fn test_resolve_env_file_not_found() {
    let mut resolver = EnvironmentResolver::new();
    resolver.add_source(VariableSource::EnvFile(PathBuf::from("/nonexistent/path/.env")));
    
    let result = resolver.resolve();
    
    assert!(result.is_err());
    match result.unwrap_err() {
        ResolveError::SourceError { source, .. } => {
            assert!(source.to_string().contains("File not found"));
        }
        _ => panic!("Expected SourceError with file not found"),
    }
}

#[test]
fn test_resolve_preserve_insertion_order() {
    let mut resolver = EnvironmentResolver::new();
    
    let mut variables = IndexMap::new();
    variables.insert("THIRD".to_string(), "3".to_string());
    variables.insert("FIRST".to_string(), "1".to_string());
    variables.insert("SECOND".to_string(), "2".to_string());
    resolver.add_source(VariableSource::Default(variables));
    
    let resolved = resolver.resolve().unwrap();
    
    let keys: Vec<_> = resolved.keys().collect();
    assert_eq!(keys, vec!["THIRD", "FIRST", "SECOND"]);
}

#[test]
fn test_resolve_empty_sources() {
    let resolver = EnvironmentResolver::new();
    let resolved = resolver.resolve().unwrap();
    
    assert!(resolved.is_empty());
}

#[test]
fn test_resolve_complex_scenario() {
    let temp_dir = TempDir::new().unwrap();
    let base_env = temp_dir.path().join(".env");
    let local_env = temp_dir.path().join(".env.local");
    
    // Base configuration
    fs::write(&base_env, r#"
# Base configuration
APP_NAME=MyApp
APP_VERSION=1.0.0
DATABASE_HOST=localhost
DATABASE_NAME=myapp_${NODE_ENV}
API_URL=http://${DATABASE_HOST}:3000
"#).unwrap();
    
    // Local overrides
    fs::write(&local_env, r#"
DATABASE_HOST=prod-server
DATABASE_PASSWORD=secret123
"#).unwrap();
    
    let mut resolver = EnvironmentResolver::new();
    
    // Add sources in priority order
    let mut defaults = IndexMap::new();
    defaults.insert("NODE_ENV".to_string(), "development".to_string());
    defaults.insert("PORT".to_string(), "3000".to_string());
    resolver.add_source(VariableSource::Default(defaults));
    
    resolver.add_source(VariableSource::EnvFile(base_env));
    resolver.add_source(VariableSource::EnvFile(local_env));
    
    // System environment
    env::set_var("NODE_ENV", "production");
    resolver.add_source(VariableSource::SystemEnv);
    
    // CLI overrides
    let mut cli_args = IndexMap::new();
    cli_args.insert("PORT".to_string(), "8080".to_string());
    resolver.add_source(VariableSource::CliArgs(cli_args));
    
    let resolved = resolver.resolve().unwrap();
    
    // Verify resolution with expansion
    assert_eq!(resolved.get("NODE_ENV"), Some(&"production".to_string()));      // From system env
    assert_eq!(resolved.get("PORT"), Some(&"8080".to_string()));                // From CLI
    assert_eq!(resolved.get("DATABASE_HOST"), Some(&"prod-server".to_string())); // From local env
    assert_eq!(resolved.get("DATABASE_NAME"), Some(&"myapp_production".to_string())); // Expanded
    assert_eq!(resolved.get("API_URL"), Some(&"http://prod-server:3000".to_string()));  // Expanded
    assert_eq!(resolved.get("DATABASE_PASSWORD"), Some(&"secret123".to_string())); // From local env
    
    // Clean up
    env::remove_var("NODE_ENV");
}

#[test]
fn test_resolve_with_options_strict_undefined() {
    let mut resolver = EnvironmentResolver::new();
    
    let mut variables = IndexMap::new();
    variables.insert("VALID_VAR".to_string(), "valid_value".to_string());
    variables.insert("INVALID_VAR".to_string(), "${UNDEFINED_VAR}".to_string());
    resolver.add_source(VariableSource::Default(variables));
    
    let options = ResolutionOptions {
        undefined_variable_behavior: UndefinedVariableBehavior::Error,
    };
    
    let result = resolver.resolve_with_options(&options);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        ResolveError::UndefinedVariable { variable } => {
            assert_eq!(variable, "UNDEFINED_VAR");
        }
        _ => panic!("Expected UndefinedVariable error"),
    }
}