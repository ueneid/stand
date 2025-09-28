use stand::commands::list;
use stand::config::loader;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_list_displays_multiple_environments() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[environments.dev]
description = "Development environment"
color = "green"
DATABASE_URL = "postgres://localhost:5432/dev"

[environments.staging]
description = "Staging environment"
color = "yellow"
DATABASE_URL = "postgres://staging.example.com/app"

[environments.prod]
description = "Production environment"
color = "red"
requires_confirmation = true
DATABASE_URL = "postgres://prod.example.com/app"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = list::list_environments(dir.path());
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(output.contains("dev"));
    assert!(output.contains("Development environment"));
    assert!(output.contains("staging"));
    assert!(output.contains("Staging environment"));
    assert!(output.contains("prod"));
    assert!(output.contains("Production environment"));
}

#[test]
fn test_list_marks_default_environment() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "prod"

[environments.dev]
description = "Development environment"

[environments.prod]
description = "Production environment"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = list::list_environments(dir.path());
    assert!(result.is_ok());

    let output = result.unwrap();
    // デフォルト環境には印が付く
    assert!(output.contains("→ prod") || output.contains("* prod"));
    // 非デフォルト環境には印が付かない
    assert!(!output.contains("→ dev") && !output.contains("* dev"));
}

#[test]
fn test_list_shows_colors() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[environments.dev]
description = "Development environment"
color = "green"

[environments.prod]
description = "Production environment"
color = "red"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = list::list_environments(dir.path());
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(output.contains("[green]") || output.contains("green"));
    assert!(output.contains("[red]") || output.contains("red"));
}

#[test]
fn test_list_handles_no_environments() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[environments]
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = list::list_environments(dir.path());
    assert!(result.is_err());

    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains("環境が定義されていません") || error_msg.contains("No environments"));
}

#[test]
fn test_list_handles_missing_config() {
    let dir = tempdir().unwrap();
    // 設定ファイルを作成しない

    let result = list::list_environments(dir.path());
    assert!(result.is_err());

    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains(".stand.toml") || error_msg.contains("設定ファイル"));
}

#[test]
fn test_list_shows_requires_confirmation() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[environments.dev]
description = "Development environment"

[environments.prod]
description = "Production environment"
requires_confirmation = true
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = list::list_environments(dir.path());
    assert!(result.is_ok());

    let output = result.unwrap();
    // 確認が必要な環境は特別な表示
    assert!(output.contains("確認要") || output.contains("confirmation"));
}