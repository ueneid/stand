pub mod colors;
pub mod paths;

// Re-export commonly used functions for convenience
pub use colors::{colorize_environment, format_default_marker, mask_value};
pub use paths::{find_project_root, find_project_root_from, get_config_path};
