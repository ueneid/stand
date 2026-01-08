// init.rs - Initialize Stand configuration

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

/// Handle the init command to create .stand.toml
///
/// # Arguments
/// * `current_dir` - The directory where .stand.toml will be created
/// * `force` - If true, overwrites existing .stand.toml; if false, returns error when file exists
///
/// # Errors
/// Returns error if:
/// - .stand.toml already exists and force is false
/// - Failed to write the configuration file
pub fn handle_init(current_dir: &Path, force: bool) -> Result<()> {
    let config_path = current_dir.join(".stand.toml");

    // Check if config already exists
    let existed = config_path.exists();
    if existed && !force {
        bail!(
            "Stand is already initialized in this directory.\n\
             Use 'stand init --force' to overwrite the existing configuration."
        );
    }

    // Generate and write template
    let template = generate_default_template();
    fs::write(&config_path, &template)
        .with_context(|| format!("Failed to write .stand.toml to {}", config_path.display()))?;

    // Set secure permissions (0600) on Unix systems
    set_secure_permissions(&config_path)?;

    if existed {
        println!("✓ Overwritten existing .stand.toml");
    } else {
        println!("✓ Created .stand.toml");
    }

    println!("\nNext steps:");
    println!("  1. Edit .stand.toml to add your environment variables");
    println!("  2. Run 'stand list' to see available environments");
    println!("  3. Run 'stand shell <env>' to start a shell with that environment");

    Ok(())
}

/// Set secure file permissions (0600) for configuration files
fn set_secure_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o600); // Owner read/write only
        fs::set_permissions(path, perms)
            .with_context(|| format!("Failed to set permissions for {}", path.display()))?;
    }

    #[cfg(windows)]
    {
        // On Windows, files are typically secure by default within user directories
        let _ = path; // Silence unused variable warning
    }

    Ok(())
}

/// Generate the default .stand.toml template
///
/// Creates a template with:
/// - Version 2.0 configuration format
/// - Default environment set to "dev"
/// - Commented `[common]` section for shared variables
/// - Pre-configured "dev" environment (green color)
/// - Pre-configured "prod" environment (red color, requires confirmation)
fn generate_default_template() -> String {
    r#"version = "2.0"

[settings]
default_environment = "dev"

# Common variables shared across all environments
# Uncomment and add your shared variables here:
# [common]
# APP_NAME = "MyApp"

# Development environment
[environments.dev]
description = "Development environment"
# Display color in terminal (red, green, blue, yellow, purple, cyan)
color = "green"
# Add your environment variables here:
# DATABASE_URL = "postgres://localhost/myapp_dev"

# Production environment
[environments.prod]
description = "Production environment"
color = "red"
# Require confirmation before activating this environment
requires_confirmation = true
# Add your environment variables here:
# DATABASE_URL = "postgres://prod.example.com/myapp"
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_generate_default_template_is_valid_toml() {
        let template = generate_default_template();
        let parsed: Result<toml::Value, _> = toml::from_str(&template);
        assert!(
            parsed.is_ok(),
            "Generated template should be valid TOML: {:?}",
            parsed.err()
        );
    }

    #[test]
    fn test_generate_default_template_contains_version() {
        let template = generate_default_template();
        assert!(template.contains(r#"version = "2.0""#));
    }

    #[test]
    fn test_generate_default_template_contains_settings() {
        let template = generate_default_template();
        assert!(template.contains("[settings]"));
        assert!(template.contains(r#"default_environment = "dev""#));
    }

    #[test]
    fn test_generate_default_template_contains_dev_environment() {
        let template = generate_default_template();
        assert!(template.contains("[environments.dev]"));
        assert!(template.contains(r#"description = "Development environment""#));
        assert!(template.contains(r#"color = "green""#));
    }

    #[test]
    fn test_generate_default_template_contains_prod_environment() {
        let template = generate_default_template();
        assert!(template.contains("[environments.prod]"));
        assert!(template.contains(r#"description = "Production environment""#));
        assert!(template.contains(r#"color = "red""#));
        assert!(template.contains("requires_confirmation = true"));
    }

    #[test]
    fn test_generate_default_template_contains_common_section_comment() {
        let template = generate_default_template();
        assert!(template.contains("# [common]"));
    }

    #[test]
    fn test_generate_default_template_contains_color_options_comment() {
        let template = generate_default_template();
        assert!(template.contains("red, green, blue, yellow, purple, cyan"));
    }

    #[test]
    fn test_generate_default_template_contains_requires_confirmation_comment() {
        let template = generate_default_template();
        assert!(template.contains("# Require confirmation before activating"));
    }

    #[test]
    fn test_init_creates_stand_toml() {
        let dir = tempdir().unwrap();

        let result = handle_init(dir.path(), false);

        assert!(result.is_ok());
        let config_path = dir.path().join(".stand.toml");
        assert!(config_path.exists());

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains(r#"version = "2.0""#));
        assert!(content.contains("[environments.dev]"));
        assert!(content.contains("[environments.prod]"));
    }

    #[test]
    fn test_init_fails_when_config_exists_without_force() {
        let dir = tempdir().unwrap();

        // Create existing config
        fs::write(dir.path().join(".stand.toml"), "existing").unwrap();

        let result = handle_init(dir.path(), false);

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("already initialized"));
        assert!(error_msg.contains("--force"));
    }

    #[test]
    fn test_init_overwrites_with_force_flag() {
        let dir = tempdir().unwrap();

        // Create existing config
        fs::write(dir.path().join(".stand.toml"), "old content").unwrap();

        let result = handle_init(dir.path(), true);

        assert!(result.is_ok());
        let content = fs::read_to_string(dir.path().join(".stand.toml")).unwrap();
        assert!(content.contains(r#"version = "2.0""#));
        assert!(!content.contains("old content"));
    }

    #[test]
    #[cfg(unix)]
    fn test_init_sets_secure_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();

        handle_init(dir.path(), false).unwrap();

        let config_path = dir.path().join(".stand.toml");
        let metadata = fs::metadata(&config_path).unwrap();
        let permissions = metadata.permissions();

        // Verify file has 0600 permissions (owner read/write only)
        assert_eq!(
            permissions.mode() & 0o777,
            0o600,
            "Config file should have 0600 permissions"
        );
    }
}
