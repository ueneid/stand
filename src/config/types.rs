use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Configuration {
    pub version: String,
    pub environments: HashMap<String, Environment>,
    pub common: Option<HashMap<String, String>>,
    #[serde(default)]
    pub settings: Settings,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Environment {
    pub description: String,
    pub extends: Option<String>,
    #[serde(flatten)]
    pub variables: HashMap<String, String>,
    pub color: Option<String>,
    pub requires_confirmation: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Settings {
    pub nested_shell_behavior: Option<NestedBehavior>,
    pub show_env_in_prompt: Option<bool>,
    /// If true, automatically exit the Stand subshell when navigating outside the project directory
    pub auto_exit_on_dir_change: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum NestedBehavior {
    Prevent,
    Allow,
    Warn,
}
