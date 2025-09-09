use serde::{Deserialize, Serialize};

/// Represents the runtime state of Stand
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct State {
    /// Currently active environment name
    pub current_environment: Option<String>,
    /// Last used project root path
    pub project_root: Option<String>,
}

impl State {
    /// Create a new empty state
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the current environment
    pub fn set_current_environment(&mut self, env_name: String) {
        self.current_environment = Some(env_name);
    }

    /// Clear the current environment
    pub fn clear_current_environment(&mut self) {
        self.current_environment = None;
    }

    /// Get the current environment name
    pub fn get_current_environment(&self) -> Option<&str> {
        self.current_environment.as_deref()
    }

    /// Set the project root path
    pub fn set_project_root(&mut self, path: String) {
        self.project_root = Some(path);
    }

    /// Get the project root path
    pub fn get_project_root(&self) -> Option<&str> {
        self.project_root.as_deref()
    }
}
