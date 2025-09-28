use crate::config::loader;
use anyhow::{anyhow, Result};
use std::path::Path;

/// Lists all available environments from the configuration file
pub fn list_environments(project_path: &Path) -> Result<String> {
    let config = loader::load_config_toml(project_path)?;

    if config.environments.is_empty() {
        return Err(anyhow!("環境が定義されていません"));
    }

    let default_env = &config.settings.default_environment;

    // Sort environments alphabetically
    let mut env_names: Vec<_> = config.environments.keys().collect();
    env_names.sort();

    let mut output = String::from("Available environments:\n");

    for env_name in env_names {
        let env = &config.environments[env_name];
        let env_line = format_environment_line(env_name, env, env_name == default_env);
        output.push_str(&env_line);
    }

    output.push_str("\n→ indicates default environment");
    Ok(output)
}

/// Formats a single environment line for display
fn format_environment_line(
    name: &str,
    env: &crate::config::types::Environment,
    is_default: bool,
) -> String {
    let marker = if is_default { "→" } else { " " };

    let color_part = env
        .color
        .as_ref()
        .map(|c| format!(" [{}]", c))
        .unwrap_or_default();

    let confirmation_part = if env.requires_confirmation.unwrap_or(false) {
        " 確認要"
    } else {
        ""
    };

    format!(
        "  {} {}     {}{}{}\n",
        marker, name, env.description, color_part, confirmation_part
    )
}
