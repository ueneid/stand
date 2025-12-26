// Shell detection module
//
// Provides functionality to detect the user's shell and check if
// we're already inside a Stand shell session.

use std::env;
use std::path::PathBuf;

/// Represents the type of shell
#[derive(Debug, Clone, PartialEq)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    Other(String),
}

impl ShellType {
    /// Get the shell type from a path
    pub fn from_path(path: &str) -> Self {
        let path_buf = PathBuf::from(path);
        let shell_name = path_buf.file_name().and_then(|s| s.to_str()).unwrap_or("");

        match shell_name {
            "bash" => ShellType::Bash,
            "zsh" => ShellType::Zsh,
            "fish" => ShellType::Fish,
            other => ShellType::Other(other.to_string()),
        }
    }
}

/// Detect the user's shell from environment
///
/// Returns the path to the user's shell, detected from:
/// 1. $SHELL environment variable
/// 2. Fallback to /bin/sh if not found
pub fn detect_user_shell() -> String {
    env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

/// Get the shell type for the current user
pub fn get_shell_type() -> ShellType {
    ShellType::from_path(&detect_user_shell())
}

/// Check if we're already inside a Stand shell session
///
/// Returns true if the STAND_ACTIVE environment variable is set
pub fn is_stand_shell_active() -> bool {
    env::var("STAND_ACTIVE").is_ok()
}

/// Get the current Stand environment name if active
///
/// Returns Some(env_name) if inside a Stand shell, None otherwise
pub fn get_active_environment() -> Option<String> {
    env::var("STAND_ENVIRONMENT").ok()
}

/// Get the project root of the active Stand session
///
/// Returns Some(path) if inside a Stand shell, None otherwise
pub fn get_active_project_root() -> Option<String> {
    env::var("STAND_PROJECT_ROOT").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_shell_type_from_path_bash() {
        assert_eq!(ShellType::from_path("/bin/bash"), ShellType::Bash);
        assert_eq!(ShellType::from_path("/usr/bin/bash"), ShellType::Bash);
        assert_eq!(ShellType::from_path("/usr/local/bin/bash"), ShellType::Bash);
    }

    #[test]
    fn test_shell_type_from_path_zsh() {
        assert_eq!(ShellType::from_path("/bin/zsh"), ShellType::Zsh);
        assert_eq!(ShellType::from_path("/usr/bin/zsh"), ShellType::Zsh);
    }

    #[test]
    fn test_shell_type_from_path_fish() {
        assert_eq!(ShellType::from_path("/usr/bin/fish"), ShellType::Fish);
        assert_eq!(ShellType::from_path("/usr/local/bin/fish"), ShellType::Fish);
    }

    #[test]
    fn test_shell_type_from_path_other() {
        assert_eq!(
            ShellType::from_path("/bin/sh"),
            ShellType::Other("sh".to_string())
        );
        assert_eq!(
            ShellType::from_path("/bin/dash"),
            ShellType::Other("dash".to_string())
        );
    }

    #[test]
    fn test_detect_user_shell_returns_shell_env() {
        // This test relies on the actual $SHELL environment variable
        // In a real environment, this should return a valid shell path
        let shell = detect_user_shell();
        assert!(!shell.is_empty());
    }

    #[test]
    #[serial]
    fn test_is_stand_shell_active_false_when_not_set() {
        // Ensure STAND_ACTIVE is not set
        env::remove_var("STAND_ACTIVE");
        assert!(!is_stand_shell_active());
    }

    #[test]
    #[serial]
    fn test_is_stand_shell_active_true_when_set() {
        env::set_var("STAND_ACTIVE", "1");
        assert!(is_stand_shell_active());
        env::remove_var("STAND_ACTIVE");
    }

    #[test]
    #[serial]
    fn test_get_active_environment_none_when_not_set() {
        env::remove_var("STAND_ENVIRONMENT");
        assert_eq!(get_active_environment(), None);
    }

    #[test]
    #[serial]
    fn test_get_active_environment_returns_value() {
        env::set_var("STAND_ENVIRONMENT", "production");
        assert_eq!(get_active_environment(), Some("production".to_string()));
        env::remove_var("STAND_ENVIRONMENT");
    }

    #[test]
    #[serial]
    fn test_get_active_project_root_none_when_not_set() {
        env::remove_var("STAND_PROJECT_ROOT");
        assert_eq!(get_active_project_root(), None);
    }

    #[test]
    #[serial]
    fn test_get_active_project_root_returns_value() {
        env::set_var("STAND_PROJECT_ROOT", "/home/user/project");
        assert_eq!(
            get_active_project_root(),
            Some("/home/user/project".to_string())
        );
        env::remove_var("STAND_PROJECT_ROOT");
    }
}
