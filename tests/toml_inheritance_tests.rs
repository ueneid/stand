use stand::config::loader;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_variable_inheritance_basic() {
    let dir = tempdir().unwrap();
    
    let toml_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[common]
APP_NAME = "MyApp"
LOG_LEVEL = "info"

[environments.dev]
description = "Development environment"
DATABASE_URL = "postgres://localhost:5432/dev"
DEBUG = "true"

[environments.prod]
description = "Production environment"
extends = "dev"
DATABASE_URL = "postgres://prod.example.com/myapp"
DEBUG = "false"
"#;

    fs::write(dir.path().join(".stand"), toml_content).unwrap();
    
    let result = loader::load_config_toml_with_inheritance(dir.path());
    
    match result {
        Ok(config) => {
            // Check dev environment has common + its own variables
            let dev_env = config.environments.get("dev").unwrap();
            assert_eq!(dev_env.variables.get("APP_NAME").unwrap(), "MyApp");
            assert_eq!(dev_env.variables.get("LOG_LEVEL").unwrap(), "info");
            assert_eq!(dev_env.variables.get("DATABASE_URL").unwrap(), "postgres://localhost:5432/dev");
            assert_eq!(dev_env.variables.get("DEBUG").unwrap(), "true");
            
            // Check prod environment has common + dev (inherited) + its own variables
            let prod_env = config.environments.get("prod").unwrap();
            assert_eq!(prod_env.variables.get("APP_NAME").unwrap(), "MyApp"); // from common
            assert_eq!(prod_env.variables.get("LOG_LEVEL").unwrap(), "info"); // from common
            assert_eq!(prod_env.variables.get("DATABASE_URL").unwrap(), "postgres://prod.example.com/myapp"); // overridden
            assert_eq!(prod_env.variables.get("DEBUG").unwrap(), "false"); // overridden
        }
        Err(e) => panic!("Failed to load TOML config with inheritance: {}", e),
    }
}

#[test]
fn test_circular_inheritance_detection() {
    let dir = tempdir().unwrap();
    
    let toml_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[environments.dev]
description = "Development environment"
extends = "prod"
DATABASE_URL = "postgres://localhost:5432/dev"

[environments.prod]
description = "Production environment"
extends = "dev"
DATABASE_URL = "postgres://prod.example.com/myapp"
"#;

    fs::write(dir.path().join(".stand"), toml_content).unwrap();
    
    let result = loader::load_config_toml_with_inheritance(dir.path());
    assert!(result.is_err());
    
    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains("Circular reference"));
}

#[test]
fn test_inheritance_chain() {
    let dir = tempdir().unwrap();
    
    let toml_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[common]
APP_NAME = "MyApp"

[environments.base]
description = "Base environment"
LOG_LEVEL = "info"
DEBUG = "false"

[environments.dev]
description = "Development environment"
extends = "base"
DEBUG = "true"
DATABASE_URL = "postgres://localhost:5432/dev"

[environments.prod]
description = "Production environment"
extends = "dev"
DATABASE_URL = "postgres://prod.example.com/myapp"
DEBUG = "false"
"#;

    fs::write(dir.path().join(".stand"), toml_content).unwrap();
    
    let result = loader::load_config_toml_with_inheritance(dir.path());
    
    match result {
        Ok(config) => {
            // Check prod has variables from common -> base -> dev -> prod
            let prod_env = config.environments.get("prod").unwrap();
            assert_eq!(prod_env.variables.get("APP_NAME").unwrap(), "MyApp"); // from common
            assert_eq!(prod_env.variables.get("LOG_LEVEL").unwrap(), "info"); // from base
            assert_eq!(prod_env.variables.get("DATABASE_URL").unwrap(), "postgres://prod.example.com/myapp"); // overridden in prod
            assert_eq!(prod_env.variables.get("DEBUG").unwrap(), "false"); // overridden in prod
        }
        Err(e) => panic!("Failed to load TOML config with inheritance chain: {}", e),
    }
}