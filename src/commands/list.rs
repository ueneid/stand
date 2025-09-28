use crate::config::loader;
use anyhow::{anyhow, Result};
use std::path::Path;

/// 利用可能な環境を一覧表示する
pub fn list_environments(project_path: &Path) -> Result<String> {
    let config = loader::load_config_toml(project_path)?;

    if config.environments.is_empty() {
        return Err(anyhow!("環境が定義されていません"));
    }

    let default_env = &config.settings.default_environment;

    // 環境一覧をアルファベット順にソート
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

/// 環境の1行を整形する
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
