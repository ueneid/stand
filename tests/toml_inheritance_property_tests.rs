use stand::config::loader;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_color_inheritance_from_parent() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[environments.base]
description = "Base environment"
color = "blue"
DATABASE_URL = "postgres://base.example.com/app"

[environments.dev]
description = "Development environment"
extends = "base"
DEBUG = "true"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = loader::load_config_toml_with_inheritance(dir.path());
    assert!(result.is_ok());

    let config = result.unwrap();
    let dev_env = &config.environments["dev"];

    // 子環境が親から色を継承している
    assert_eq!(dev_env.color, Some("blue".to_string()));
    // 説明は上書きされている
    assert_eq!(dev_env.description, "Development environment");
    // 変数も継承されている
    assert_eq!(
        dev_env.variables["DATABASE_URL"],
        "postgres://base.example.com/app"
    );
    assert_eq!(dev_env.variables["DEBUG"], "true");
}

#[test]
fn test_color_override_in_child() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "prod"

[environments.base]
description = "Base environment"
color = "blue"
requires_confirmation = true

[environments.staging]
description = "Staging environment"
extends = "base"
color = "yellow"

[environments.prod]
description = "Production environment"
extends = "base"
color = "red"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = loader::load_config_toml_with_inheritance(dir.path());
    assert!(result.is_ok());

    let config = result.unwrap();
    let staging_env = &config.environments["staging"];
    let prod_env = &config.environments["prod"];

    // staging環境は色を上書き、確認要求は継承
    assert_eq!(staging_env.color, Some("yellow".to_string()));
    assert_eq!(staging_env.requires_confirmation, Some(true));

    // prod環境も色を上書き、確認要求は継承
    assert_eq!(prod_env.color, Some("red".to_string()));
    assert_eq!(prod_env.requires_confirmation, Some(true));
}

#[test]
fn test_requires_confirmation_inheritance() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[environments.base]
description = "Base environment"
requires_confirmation = true
DATABASE_URL = "postgres://base.example.com/app"

[environments.dev]
description = "Development environment"
extends = "base"
# requires_confirmationは未設定（継承される）

[environments.unsafe]
description = "Unsafe environment"
extends = "base"
requires_confirmation = false
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = loader::load_config_toml_with_inheritance(dir.path());
    assert!(result.is_ok());

    let config = result.unwrap();
    let dev_env = &config.environments["dev"];
    let unsafe_env = &config.environments["unsafe"];

    // dev環境は確認要求を継承
    assert_eq!(dev_env.requires_confirmation, Some(true));

    // unsafe環境は確認要求を明示的にfalseに上書き
    assert_eq!(unsafe_env.requires_confirmation, Some(false));
}

#[test]
fn test_extends_nonexistent_parent_validation() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[environments.dev]
description = "Development environment"
extends = "nonexistent_parent"
DATABASE_URL = "postgres://localhost:5432/dev"
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = loader::load_config_toml_with_validation(dir.path());
    assert!(result.is_err());

    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains("nonexistent_parent") || error_msg.contains("Parent environment"));
}

#[test]
fn test_multilevel_property_inheritance() {
    let dir = tempdir().unwrap();
    let config_content = r#"
version = "2.0"

[settings]
default_environment = "dev"

[environments.global]
description = "Global base"
color = "white"
requires_confirmation = true

[environments.base]
description = "Base environment"
extends = "global"
color = "blue"
# requires_confirmationは未設定（globalから継承）

[environments.dev]
description = "Development environment"
extends = "base"
# colorとrequires_confirmationは両方継承
"#;

    let config_path = dir.path().join(".stand.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = loader::load_config_toml_with_inheritance(dir.path());
    assert!(result.is_ok());

    let config = result.unwrap();
    let dev_env = &config.environments["dev"];

    // 多段階継承：colorはbaseから、requires_confirmationはglobalから
    assert_eq!(dev_env.color, Some("blue".to_string()));
    assert_eq!(dev_env.requires_confirmation, Some(true));
    assert_eq!(dev_env.description, "Development environment");
}
