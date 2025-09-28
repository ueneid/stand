use crate::config::loader;
use anyhow::{anyhow, Result};
use std::path::Path;

/// 利用可能な環境を一覧表示する
pub fn list_environments(project_path: &Path) -> Result<String> {
    // 設定ファイルを読み込む
    let config = loader::load_config_toml(project_path)?;

    // 環境が存在しない場合はエラー
    if config.environments.is_empty() {
        return Err(anyhow!("環境が定義されていません"));
    }

    // 出力を構築
    let mut output = String::new();
    output.push_str("Available environments:\n");

    // デフォルト環境の確認
    let default_env = &config.settings.default_environment;

    // 環境一覧を表示（アルファベット順にソート）
    let mut env_names: Vec<_> = config.environments.keys().collect();
    env_names.sort();

    for env_name in env_names {
        let env = &config.environments[env_name];

        // デフォルト環境の印
        let marker = if env_name == default_env { "→" } else { " " };

        // 色の表示
        let color_display = match &env.color {
            Some(color) => format!(" [{}]", color),
            None => String::new(),
        };

        // 確認要求の表示
        let confirmation_display = if env.requires_confirmation.unwrap_or(false) {
            " 確認要"
        } else {
            ""
        };

        output.push_str(&format!(
            "  {} {}     {}{}{}\n",
            marker,
            env_name,
            env.description,
            color_display,
            confirmation_display
        ));
    }

    output.push_str("\n→ indicates default environment");

    Ok(output)
}