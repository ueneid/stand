use crate::state::types::State;
use crate::utils::paths::find_project_root;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Get the path to the state file
pub fn get_state_file_path() -> Result<PathBuf> {
    let project_root = find_project_root()?;
    get_state_file_path_from(&project_root)
}

/// Get the path to the state file from a specific project root
pub fn get_state_file_path_from(project_root: &Path) -> Result<PathBuf> {
    let stand_dir = project_root.join(".stand");

    // Create .stand directory if it doesn't exist
    if !stand_dir.exists() {
        fs::create_dir_all(&stand_dir).with_context(|| {
            format!("Failed to create .stand directory: {}", stand_dir.display())
        })?;
    }

    Ok(stand_dir.join("state.json"))
}

/// Load state from file, or return default state if file doesn't exist
pub fn load_state() -> Result<State> {
    let state_path = get_state_file_path()?;

    if !state_path.exists() {
        return Ok(State::default());
    }

    let content = fs::read_to_string(&state_path)
        .with_context(|| format!("Failed to read state file: {}", state_path.display()))?;

    let state: State = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse state file: {}", state_path.display()))?;

    Ok(state)
}

/// Save state to file
pub fn save_state(state: &State) -> Result<()> {
    let state_path = get_state_file_path()?;

    let content =
        serde_json::to_string_pretty(state).with_context(|| "Failed to serialize state")?;

    fs::write(&state_path, content)
        .with_context(|| format!("Failed to write state file: {}", state_path.display()))?;

    // Set secure permissions (0600) on Unix systems
    set_secure_permissions(&state_path)?;

    Ok(())
}

/// Set secure file permissions for the state file
fn set_secure_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o600); // Owner read/write only
        fs::set_permissions(path, perms)
            .with_context(|| format!("Failed to set permissions for {}", path.display()))?;
    }

    #[cfg(windows)]
    {
        // On Windows, files are typically secure by default within user directories
        // Could potentially use Windows ACL APIs here for additional security
        // For now, we rely on the default NTFS permissions
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_state_serialization() {
        let mut state = State::new();
        state.set_current_environment("test_env".to_string());
        state.set_project_root("/tmp/test".to_string());

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: State = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.get_current_environment(), Some("test_env"));
        assert_eq!(deserialized.get_project_root(), Some("/tmp/test"));
    }

    #[test]
    fn test_state_file_path_generation() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        let state_path = get_state_file_path_from(project_root).unwrap();
        let expected = project_root.join(".stand").join("state.json");

        assert_eq!(state_path, expected);
        assert!(project_root.join(".stand").exists());
    }
}
