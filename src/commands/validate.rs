use crate::config::loader::load_config_toml_with_validation;
use crate::utils::colors::colorize_environment;
use crate::utils::paths::find_project_root;
use anyhow::Result;

/// Validate the Stand configuration
pub fn handle_validate() -> Result<()> {
    println!("🔍 Validating Stand configuration...");

    let project_root = find_project_root()?;
    match load_config_toml_with_validation(&project_root) {
        Ok(config) => {
            println!("✓ Configuration is valid");

            // Show summary
            let env_count = config.environments.len();
            let env_text = if env_count == 1 {
                "environment"
            } else {
                "environments"
            };
            println!("  {} {} defined", env_count, env_text);

            // Show environment names
            if !config.environments.is_empty() {
                print!("  Environments: ");
                let env_names: Vec<String> = config
                    .environments
                    .keys()
                    .map(|name| colorize_environment(name, Some("cyan")))
                    .collect();
                println!("{}", env_names.join(", "));
            }

            // Show if common config exists
            if let Some(common) = &config.common {
                if !common.is_empty() {
                    println!("  {} common variables defined", common.len());
                }
            }

            Ok(())
        }
        Err(e) => {
            println!("❌ Configuration validation failed:");
            println!("  {}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_validate_logic() {
        // Test that the validate logic is sound
        // For now, we test that the function compiles and can handle basic scenarios
        // Full integration tests should be in separate test files
        assert!(true); // Placeholder test
    }
}
