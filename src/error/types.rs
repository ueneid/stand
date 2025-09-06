use thiserror::Error;

/// CLI-specific errors for user-facing messages
#[derive(Error, Debug)]
pub enum CliError {
    #[error("Stand project not initialized. Run 'stand init' to get started.")]
    ProjectNotInitialized,

    #[error("Environment '{name}' not found in configuration.")]
    EnvironmentNotFound { name: String },

    #[error("Configuration file not found. Run 'stand init' to create one.")]
    ConfigurationNotFound,

    #[error("Configuration validation failed: {reason}")]
    ConfigurationInvalid { reason: String },

    #[error("Stand is already initialized in this directory.")]
    AlreadyInitialized,

    #[error("Cannot write to file '{path}': {reason}")]
    FileWriteError { path: String, reason: String },

    #[error("Cannot read file '{path}': {reason}")]
    FileReadError { path: String, reason: String },

    #[error("Invalid environment name '{name}'. Names must be alphanumeric and may contain hyphens or underscores.")]
    InvalidEnvironmentName { name: String },
}

impl CliError {
    /// Convert a configuration error to a CLI error with user-friendly message
    pub fn from_config_error(err: crate::config::ConfigError) -> Self {
        match err {
            crate::config::ConfigError::ValidationError { message } => {
                Self::ConfigurationInvalid { reason: message }
            }
            crate::config::ConfigError::InvalidEnvironment { name } => {
                Self::EnvironmentNotFound { name }
            }
            _ => Self::ConfigurationInvalid {
                reason: err.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_not_initialized_error() {
        let error = CliError::ProjectNotInitialized;
        let message = error.to_string();
        assert!(message.contains("Stand project not initialized"));
        assert!(message.contains("'stand init'"));
    }

    #[test]
    fn test_environment_not_found_error() {
        let error = CliError::EnvironmentNotFound {
            name: "nonexistent".to_string(),
        };
        let message = error.to_string();
        assert!(message.contains("Environment 'nonexistent' not found"));
    }

    #[test]
    fn test_configuration_not_found_error() {
        let error = CliError::ConfigurationNotFound;
        let message = error.to_string();
        assert!(message.contains("Configuration file not found"));
        assert!(message.contains("'stand init'"));
    }

    #[test]
    fn test_configuration_invalid_error() {
        let error = CliError::ConfigurationInvalid {
            reason: "missing version field".to_string(),
        };
        let message = error.to_string();
        assert!(message.contains("Configuration validation failed"));
        assert!(message.contains("missing version field"));
    }

    #[test]
    fn test_already_initialized_error() {
        let error = CliError::AlreadyInitialized;
        let message = error.to_string();
        assert!(message.contains("already initialized"));
    }

    #[test]
    fn test_file_write_error() {
        let error = CliError::FileWriteError {
            path: "/some/path/.stand.toml".to_string(),
            reason: "permission denied".to_string(),
        };
        let message = error.to_string();
        assert!(message.contains("Cannot write to file '/some/path/.stand.toml'"));
        assert!(message.contains("permission denied"));
    }

    #[test]
    fn test_file_read_error() {
        let error = CliError::FileReadError {
            path: "/some/path/.stand.toml".to_string(),
            reason: "file not found".to_string(),
        };
        let message = error.to_string();
        assert!(message.contains("Cannot read file '/some/path/.stand.toml'"));
        assert!(message.contains("file not found"));
    }

    #[test]
    fn test_invalid_environment_name_error() {
        let error = CliError::InvalidEnvironmentName {
            name: "invalid@name".to_string(),
        };
        let message = error.to_string();
        assert!(message.contains("Invalid environment name 'invalid@name'"));
        assert!(message.contains("alphanumeric"));
    }

    #[test]
    fn test_from_config_error_validation() {
        let config_err = crate::config::ConfigError::ValidationError {
            message: "test validation error".to_string(),
        };
        let cli_err = CliError::from_config_error(config_err);

        match cli_err {
            CliError::ConfigurationInvalid { reason } => {
                assert_eq!(reason, "test validation error");
            }
            _ => panic!("Expected ConfigurationInvalid error"),
        }
    }

    #[test]
    fn test_from_config_error_invalid_environment() {
        let config_err = crate::config::ConfigError::InvalidEnvironment {
            name: "test-env".to_string(),
        };
        let cli_err = CliError::from_config_error(config_err);

        match cli_err {
            CliError::EnvironmentNotFound { name } => {
                assert_eq!(name, "test-env");
            }
            _ => panic!("Expected EnvironmentNotFound error"),
        }
    }
}
