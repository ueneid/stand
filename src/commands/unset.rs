//! Unset command implementation.
//!
//! Removes a variable from an environment in the configuration file.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;

use colored::Colorize;
use toml_edit::DocumentMut;

use crate::config::{loader, ConfigError};
use crate::environment::safety;

/// Remove a variable from `[environments.<environment>]`.
///
/// Risky names are not blocked here, because removing them is safe.
pub fn unset_variable(
    project_dir: &Path,
    environment: &str,
    key: &str,
) -> Result<(), UnsetCommandError> {
    let config_path = project_dir.join(".stand.toml");
    let config = loader::load_config_toml(project_dir)?;

    if !config.environments.contains_key(environment) {
        return Err(UnsetCommandError::EnvironmentNotFound(
            environment.to_string(),
        ));
    }

    if safety::is_reserved_key(key) {
        return Err(UnsetCommandError::ReservedKey(key.to_string()));
    }

    let content = fs::read_to_string(&config_path)?;
    let mut doc: DocumentMut = content
        .parse()
        .map_err(|e: toml_edit::TomlError| UnsetCommandError::TomlParse(e.to_string()))?;

    let removed = doc
        .get_mut("environments")
        .and_then(|e| e.get_mut(environment))
        .and_then(|e| e.as_table_like_mut())
        .ok_or_else(|| UnsetCommandError::EnvironmentNotFound(environment.to_string()))?
        .remove(key);

    if removed.is_none() {
        return Err(UnsetCommandError::VariableNotFound {
            key: key.to_string(),
            environment: environment.to_string(),
            hint: find_hint(&doc, environment, key),
        });
    }

    let shell = super::set::ShellTarget::for_project(project_dir);
    let before = shell
        .as_ref()
        .and_then(|shell| super::set::effective_value(project_dir, shell.environment(), key));

    // Write to the existing path so file permissions are preserved.
    fs::write(&config_path, doc.to_string())?;

    println!(
        "{} Unset {} from [environments.{}]",
        "✓".green(),
        key,
        environment
    );

    if let Some(shell) = shell {
        shell.apply(project_dir, "unset", key, before, None);
    }

    Ok(())
}

/// Explain where an absent key is actually defined, if anywhere.
fn find_hint(doc: &DocumentMut, environment: &str, key: &str) -> Option<String> {
    if doc
        .get("common")
        .and_then(|c| c.as_table_like())
        .is_some_and(|c| c.contains_key(key))
    {
        return Some("defined in [common]".to_string());
    }

    let envs = doc.get("environments")?;
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(environment.to_string());
    let mut current = environment.to_string();
    loop {
        let parent = envs
            .get(&current)
            .and_then(|e| e.get("extends"))
            .and_then(|p| p.as_str())?
            .to_string();
        if !visited.insert(parent.clone()) {
            return None;
        }
        let parent_table = envs.get(&parent).and_then(|e| e.as_table_like())?;
        if parent_table.contains_key(key) {
            return Some(format!("inherited from environment '{}'", parent));
        }
        current = parent;
    }
}

/// Error type for unset command.
#[derive(Debug, thiserror::Error)]
pub enum UnsetCommandError {
    #[error("Environment not found: {0}")]
    EnvironmentNotFound(String),

    #[error(
        "'{0}' is an environment setting, not a variable. Edit .stand.toml directly to change it."
    )]
    ReservedKey(String),

    #[error("Variable '{key}' not found in [environments.{environment}]{}", format_hint(.hint))]
    VariableNotFound {
        key: String,
        environment: String,
        hint: Option<String>,
    },

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("TOML parsing error: {0}")]
    TomlParse(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

fn format_hint(hint: &Option<String>) -> String {
    match hint {
        Some(h) => format!(" ({})", h),
        None => String::new(),
    }
}
