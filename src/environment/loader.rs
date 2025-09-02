use std::fs;
use std::path::Path;
use std::io;
use indexmap::IndexMap;
use anyhow::Result;
use crate::environment::parser::{parse_env_content, ParseError};

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("File not found: {path:?}")]
    FileNotFound { path: std::path::PathBuf },
    
    #[error("Permission denied accessing file: {path:?}")]
    PermissionDenied { path: std::path::PathBuf },
    
    #[error("Path is not a file: {path:?}")]
    NotAFile { path: std::path::PathBuf },
    
    #[error("Parse error in file {path:?}: {source}")]
    ParseError { path: std::path::PathBuf, source: ParseError },
    
    #[error("I/O error reading file {path:?}: {source}")]
    IoError { path: std::path::PathBuf, source: io::Error },
}

pub fn load_env_file<P: AsRef<Path>>(path: P) -> Result<IndexMap<String, String>, LoadError> {
    let path = path.as_ref();
    let path_buf = path.to_path_buf();

    // Check if path exists
    if !path.exists() {
        return Err(LoadError::FileNotFound { path: path_buf });
    }

    // Check if it's a file (not a directory)
    let metadata = fs::metadata(path).map_err(|err| {
        match err.kind() {
            io::ErrorKind::PermissionDenied => LoadError::PermissionDenied { path: path_buf.clone() },
            _ => LoadError::IoError { path: path_buf.clone(), source: err },
        }
    })?;

    if !metadata.is_file() {
        return Err(LoadError::NotAFile { path: path_buf });
    }

    // Read file content
    let content = fs::read_to_string(path).map_err(|err| {
        match err.kind() {
            io::ErrorKind::PermissionDenied => LoadError::PermissionDenied { path: path_buf.clone() },
            _ => LoadError::IoError { path: path_buf.clone(), source: err },
        }
    })?;

    // Parse the content using our parser
    parse_env_content(&content).map_err(|parse_error| {
        LoadError::ParseError { 
            path: path_buf, 
            source: parse_error 
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_env_file_basic() {
        let temp_dir = TempDir::new().unwrap();
        let env_file = temp_dir.path().join(".env");
        
        fs::write(&env_file, "KEY=value").unwrap();
        
        let result = load_env_file(&env_file).unwrap();
        assert_eq!(result.get("KEY"), Some(&"value".to_string()));
    }

    #[test]
    fn test_load_env_file_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let env_file = temp_dir.path().join("nonexistent");
        
        let result = load_env_file(&env_file);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LoadError::FileNotFound { .. }));
    }

    #[test]
    fn test_load_env_file_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("directory");
        
        fs::create_dir(&dir_path).unwrap();
        
        let result = load_env_file(&dir_path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LoadError::NotAFile { .. }));
    }
}