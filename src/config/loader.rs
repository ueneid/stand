use crate::config::types::Configuration;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

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
