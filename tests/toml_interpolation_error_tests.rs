use serial_test::serial;
use stand::config::loader;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_interpolation_unterminated_placeholder() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"


[environments.dev]
description = "Development with ${UNTERMINATED"
DATABASE_URL = "postgres://localhost:5432/dev"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = loader::load_config_toml_with_validation(dir.path());
    assert!(result.is_err());

    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains("Unterminated") || error_msg.contains("missing closing '}'"));
}

#[test]
fn test_interpolation_empty_variable_name() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"


[environments.dev]
description = "Development with ${}"
DATABASE_URL = "postgres://localhost:5432/dev"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = loader::load_config_toml_with_validation(dir.path());
    assert!(result.is_err());

    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains("Empty variable name") || error_msg.contains("'${}' is not valid"));
}

#[test]
fn test_interpolation_nonexistent_variable() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"


[environments.dev]
description = "Development with ${NONEXISTENT_VAR}"
DATABASE_URL = "postgres://localhost:5432/dev"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = loader::load_config_toml_with_validation(dir.path());
    assert!(result.is_err());

    let error_msg = format!("{}", result.unwrap_err());
    assert!(
        error_msg.contains("NONEXISTENT_VAR")
            || error_msg.contains("not found")
            || error_msg.contains("undefined")
    );
}

#[test]
#[serial]
fn test_interpolation_multiple_variables_success() {
    let dir = tempdir().unwrap();

    // 環境変数を設定
    std::env::set_var("PREFIX", "api");
    std::env::set_var("VERSION", "v1");
    std::env::set_var("ENDPOINT", "users");

    let config_content = r#"
version = "2.0"


[environments.dev]
description = "API ${PREFIX} ${VERSION} for ${ENDPOINT}"
DATABASE_URL = "postgres://${PREFIX}_${VERSION}.example.com/app"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = loader::load_config_toml_with_validation(dir.path());
    assert!(result.is_ok());

    let config = result.unwrap();
    let dev_env = &config.environments["dev"];
    assert_eq!(dev_env.description, "API api v1 for users");
    assert_eq!(
        dev_env.variables["DATABASE_URL"],
        "postgres://api_v1.example.com/app"
    );

    // 環境変数をクリーンアップ
    std::env::remove_var("PREFIX");
    std::env::remove_var("VERSION");
    std::env::remove_var("ENDPOINT");
}

#[test]
#[serial]
fn test_interpolation_in_common_variables() {
    let dir = tempdir().unwrap();

    // 環境変数を設定
    std::env::set_var("APP_PREFIX", "myapp");

    let config_content = r#"
version = "2.0"


[common]
APP_NAME = "${APP_PREFIX}_service"
LOG_LEVEL = "info"

[environments.dev]
description = "Development environment"
DEBUG = "true"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = loader::load_config_toml_with_validation(dir.path());
    assert!(result.is_ok());

    let config = result.unwrap();
    let dev_env = &config.environments["dev"];

    // common変数が正しく展開されて継承されている
    assert_eq!(dev_env.variables["APP_NAME"], "myapp_service");
    assert_eq!(dev_env.variables["LOG_LEVEL"], "info");
    assert_eq!(dev_env.variables["DEBUG"], "true");

    // 環境変数をクリーンアップ
    std::env::remove_var("APP_PREFIX");
}
