use stand::environment::loader::{load_env_file, LoadError};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_load_existing_env_file() {
    let temp_dir = TempDir::new().unwrap();
    let env_file = temp_dir.path().join(".env");

    fs::write(&env_file, "KEY1=value1\nKEY2=value2").unwrap();

    let result = load_env_file(&env_file).unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result.get("KEY1"), Some(&"value1".to_string()));
    assert_eq!(result.get("KEY2"), Some(&"value2".to_string()));
}

#[test]
fn test_load_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let env_file = temp_dir.path().join("nonexistent.env");

    let result = load_env_file(&env_file);

    assert!(result.is_err());
    match result.unwrap_err() {
        LoadError::FileNotFound { path } => {
            assert_eq!(path, env_file);
        }
        _ => panic!("Expected FileNotFound error"),
    }
}

#[test]
fn test_load_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let env_file = temp_dir.path().join(".env");

    fs::write(&env_file, "").unwrap();

    let result = load_env_file(&env_file).unwrap();

    assert!(result.is_empty());
}

#[test]
fn test_load_file_with_parse_error() {
    let temp_dir = TempDir::new().unwrap();
    let env_file = temp_dir.path().join(".env");

    fs::write(&env_file, "INVALID_LINE_WITHOUT_EQUALS").unwrap();

    let result = load_env_file(&env_file);

    assert!(result.is_err());
    match result.unwrap_err() {
        LoadError::ParseError { source, .. } => {
            // Should contain the parse error from the parser
            assert!(source.to_string().contains("Invalid format"));
        }
        _ => panic!("Expected ParseError"),
    }
}

#[test]
fn test_load_file_with_complex_content() {
    let temp_dir = TempDir::new().unwrap();
    let env_file = temp_dir.path().join(".env");

    let content = r#"
# Comment
DATABASE_URL="postgresql://user:pass@localhost/db"
API_KEY=abc123
DEBUG=true

# Multi-line value
DESCRIPTION="A long
description that spans
multiple lines"

# Variable expansion
BASE_URL=https://api.example.com
FULL_URL=${BASE_URL}/v1/users
"#;

    fs::write(&env_file, content).unwrap();

    let result = load_env_file(&env_file).unwrap();

    assert_eq!(
        result.get("DATABASE_URL"),
        Some(&"postgresql://user:pass@localhost/db".to_string())
    );
    assert_eq!(result.get("API_KEY"), Some(&"abc123".to_string()));
    assert_eq!(result.get("DEBUG"), Some(&"true".to_string()));
    assert_eq!(
        result.get("DESCRIPTION"),
        Some(&"A long\ndescription that spans\nmultiple lines".to_string())
    );
    assert_eq!(
        result.get("BASE_URL"),
        Some(&"https://api.example.com".to_string())
    );
    assert_eq!(
        result.get("FULL_URL"),
        Some(&"https://api.example.com/v1/users".to_string())
    );
}

#[test]
fn test_load_file_with_permission_denied() {
    let temp_dir = TempDir::new().unwrap();
    let env_file = temp_dir.path().join(".env");

    // Create file and write content
    fs::write(&env_file, "KEY=value").unwrap();

    // Remove read permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&env_file).unwrap().permissions();
        perms.set_mode(0o000); // No permissions
        fs::set_permissions(&env_file, perms).unwrap();

        let result = load_env_file(&env_file);

        assert!(result.is_err());
        match result.unwrap_err() {
            LoadError::PermissionDenied { path } => {
                assert_eq!(path, env_file);
            }
            _ => panic!("Expected PermissionDenied error"),
        }

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&env_file).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&env_file, perms).unwrap();
    }

    #[cfg(not(unix))]
    {
        // On non-Unix systems, just test that the function works normally
        let result = load_env_file(&env_file).unwrap();
        assert_eq!(result.get("KEY"), Some(&"value".to_string()));
    }
}

#[test]
fn test_load_directory_instead_of_file() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("not_a_file");

    fs::create_dir(&dir_path).unwrap();

    let result = load_env_file(&dir_path);

    assert!(result.is_err());
    match result.unwrap_err() {
        LoadError::NotAFile { path } => {
            assert_eq!(path, dir_path);
        }
        _ => panic!("Expected NotAFile error"),
    }
}

#[test]
fn test_load_file_preserves_insertion_order() {
    let temp_dir = TempDir::new().unwrap();
    let env_file = temp_dir.path().join(".env");

    let content = "THIRD=3\nFIRST=1\nSECOND=2\n";
    fs::write(&env_file, content).unwrap();

    let result = load_env_file(&env_file).unwrap();

    let keys: Vec<_> = result.keys().collect();
    assert_eq!(keys, vec!["THIRD", "FIRST", "SECOND"]);
}
