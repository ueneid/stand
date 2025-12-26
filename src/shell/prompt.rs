// Prompt customization module
//
// Generates shell-specific prompt modifications to display
// the active Stand environment.

use crate::shell::detector::ShellType;
use std::collections::HashMap;

/// Environment variable for Stand prompt prefix
pub const STAND_PROMPT: &str = "STAND_PROMPT";

/// Generate the prompt prefix for displaying the active environment
///
/// Returns a string like "(stand:dev) " that can be prepended to PS1
pub fn generate_prompt_prefix(env_name: &str) -> String {
    format!("(stand:{}) ", env_name)
}

/// Get environment variables needed for prompt customization
///
/// Returns a HashMap of environment variables to set based on shell type
pub fn get_prompt_env_vars(shell_type: &ShellType, env_name: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    let prefix = generate_prompt_prefix(env_name);

    // Set STAND_PROMPT for all shells (can be used in custom prompts)
    vars.insert(STAND_PROMPT.to_string(), prefix.clone());

    match shell_type {
        ShellType::Bash => {
            // For bash, we set PROMPT_COMMAND to prepend to PS1
            // This approach works without modifying the user's existing PS1
            vars.insert(
                "PROMPT_COMMAND".to_string(),
                format!(
                    r#"PS1="{}${{STAND_ORIGINAL_PS1:-$PS1}}"; unset PROMPT_COMMAND"#,
                    prefix
                ),
            );
            // Save original PS1 so we can restore it
            vars.insert("STAND_ORIGINAL_PS1".to_string(), "${PS1}".to_string());
        }
        ShellType::Zsh => {
            // For zsh, we can use precmd hook or just modify PS1
            // Using a simpler approach that works with most zsh configs
            vars.insert(
                "PROMPT_COMMAND".to_string(),
                format!(r#"PS1="{}${{STAND_ORIGINAL_PS1:-$PS1}}""#, prefix),
            );
            vars.insert("STAND_ORIGINAL_PS1".to_string(), "${PS1}".to_string());
        }
        ShellType::Fish => {
            // Fish handles prompts differently via functions
            // We just set the STAND_PROMPT variable for users to incorporate
            // into their fish_prompt function if desired
        }
        ShellType::Other(_) => {
            // For other shells, just set STAND_PROMPT and hope for the best
        }
    }

    vars
}

/// Generate a colored prompt prefix with ANSI escape codes
///
/// Uses green color for the environment name
#[allow(dead_code)]
pub fn generate_colored_prompt_prefix(env_name: &str, color: Option<&str>) -> String {
    let color_code = match color {
        Some("red") => "\x1b[31m",
        Some("green") => "\x1b[32m",
        Some("yellow") => "\x1b[33m",
        Some("blue") => "\x1b[34m",
        Some("magenta") | Some("purple") => "\x1b[35m",
        Some("cyan") => "\x1b[36m",
        _ => "\x1b[32m", // Default to green
    };
    let reset = "\x1b[0m";

    format!("({}stand:{}{}){} ", color_code, env_name, reset, reset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_prompt_prefix() {
        assert_eq!(generate_prompt_prefix("dev"), "(stand:dev) ");
        assert_eq!(generate_prompt_prefix("production"), "(stand:production) ");
        assert_eq!(generate_prompt_prefix("staging"), "(stand:staging) ");
    }

    #[test]
    fn test_get_prompt_env_vars_includes_stand_prompt() {
        let vars = get_prompt_env_vars(&ShellType::Bash, "dev");
        assert_eq!(vars.get(STAND_PROMPT), Some(&"(stand:dev) ".to_string()));
    }

    #[test]
    fn test_get_prompt_env_vars_bash_sets_prompt_command() {
        let vars = get_prompt_env_vars(&ShellType::Bash, "dev");
        assert!(vars.contains_key("PROMPT_COMMAND"));
        assert!(vars.contains_key("STAND_ORIGINAL_PS1"));
    }

    #[test]
    fn test_get_prompt_env_vars_zsh_sets_prompt_command() {
        let vars = get_prompt_env_vars(&ShellType::Zsh, "staging");
        assert!(vars.contains_key("PROMPT_COMMAND"));
    }

    #[test]
    fn test_get_prompt_env_vars_fish_only_sets_stand_prompt() {
        let vars = get_prompt_env_vars(&ShellType::Fish, "prod");
        assert_eq!(vars.get(STAND_PROMPT), Some(&"(stand:prod) ".to_string()));
        // Fish doesn't use PROMPT_COMMAND
        assert!(!vars.contains_key("PROMPT_COMMAND"));
    }

    #[test]
    fn test_generate_colored_prompt_prefix_default_green() {
        let prefix = generate_colored_prompt_prefix("dev", None);
        assert!(prefix.contains("\x1b[32m")); // Green
        assert!(prefix.contains("stand:dev"));
        assert!(prefix.contains("\x1b[0m")); // Reset
    }

    #[test]
    fn test_generate_colored_prompt_prefix_red() {
        let prefix = generate_colored_prompt_prefix("prod", Some("red"));
        assert!(prefix.contains("\x1b[31m")); // Red
    }

    #[test]
    fn test_generate_colored_prompt_prefix_magenta() {
        let prefix = generate_colored_prompt_prefix("staging", Some("magenta"));
        assert!(prefix.contains("\x1b[35m")); // Magenta
    }
}
