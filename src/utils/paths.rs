use anyhow::Result;
use std::path::{Path, PathBuf};

/// Find the project root directory by searching for .stand.toml or .stand/ directory
pub fn find_project_root() -> Result<PathBuf> {
    let current_dir = std::env::current_dir()?;
    let mut dir = current_dir.as_path();

    loop {
        // Check for .stand.toml file
        if dir.join(".stand.toml").exists() {
            return Ok(dir.to_path_buf());
        }

        // Check for .stand directory (legacy)
        if dir.join(".stand").exists() && dir.join(".stand").is_dir() {
            return Ok(dir.to_path_buf());
        }

        // Move to parent directory
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }

    anyhow::bail!("Stand project not found. Run 'stand init' to initialize.")
}

/// Get the path to the configuration file (.stand.toml)
pub fn get_config_path(project_root: &Path) -> PathBuf {
    project_root.join(".stand.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_project_root_with_stand_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join(".stand.toml");
        fs::write(&config_file, "version = \"2.0\"").unwrap();

        // Change to temp directory to simulate being in project
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = find_project_root();
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            temp_dir.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn test_find_project_root_with_stand_dir() {
        let temp_dir = TempDir::new().unwrap();
        let stand_dir = temp_dir.path().join(".stand");
        fs::create_dir(&stand_dir).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = find_project_root();
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            temp_dir.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn test_find_project_root_not_found() {
        let temp_dir = TempDir::new().unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = find_project_root();
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_err());
    }

    #[test]
    fn test_find_project_root_in_subdirectory() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join(".stand.toml");
        fs::write(&config_file, "version = \"2.0\"").unwrap();

        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&sub_dir).unwrap();

        let result = find_project_root();
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            temp_dir.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn test_get_config_path() {
        let project_root = Path::new("/some/project");
        let config_path = get_config_path(project_root);

        assert_eq!(config_path, project_root.join(".stand.toml"));
    }
}
