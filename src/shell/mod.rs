pub mod detector;
pub mod prompt;
pub mod spawner;

// Re-export commonly used items
pub use detector::{
    detect_user_shell, get_active_environment, get_active_project_root, get_shell_type,
    is_stand_shell_active, ShellType,
};
pub use prompt::{generate_prompt_prefix, get_prompt_env_vars, STAND_PROMPT};
pub use spawner::{
    build_shell_environment, spawn_shell, STAND_ACTIVE, STAND_ENVIRONMENT, STAND_PROJECT_ROOT,
};
