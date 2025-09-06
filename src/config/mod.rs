pub mod loader;
pub mod types;
pub mod validator;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration validation failed: {message}")]
    ValidationError { message: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid environment reference: {name}")]
    InvalidEnvironment { name: String },

    #[error("Circular reference detected in environment hierarchy: {cycle:?}")]
    CircularReference { cycle: Vec<String> },

    #[error("Environment file not found: '{configured_path}' (resolved to '{resolved_path}')")]
    FileNotFound {
        configured_path: String,
        resolved_path: String,
    },

    #[error("Path is not a file: '{configured_path}' (resolved to '{resolved_path}') - directories are not allowed")]
    NotAFile {
        configured_path: String,
        resolved_path: String,
    },

    #[error("Environment variable interpolation failed: {variable}")]
    InterpolationError { variable: String },

    #[error("IO error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    #[error("YAML parsing error: {source}")]
    YamlError {
        #[from]
        source: serde_yaml::Error,
    },

    #[error("TOML parsing error: {source}")]
    TomlError {
        #[from]
        source: toml::de::Error,
    },
}
