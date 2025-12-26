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
/// Returns a HashMap of environment variables to set based on shell type.
/// Each shell type has a different mechanism for modifying the prompt.
pub fn get_prompt_env_vars(shell_type: &ShellType, env_name: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    let prefix = generate_prompt_prefix(env_name);

    // Set STAND_PROMPT for all shells (can be used in custom prompts)
    vars.insert(STAND_PROMPT.to_string(), prefix.clone());

    match shell_type {
        ShellType::Bash => {
            // For bash, PROMPT_COMMAND runs before each prompt display.
            // We capture the original PS1 on first run, then prepend our prefix.
            // The command uses shell parameter expansion to avoid recursion.
            vars.insert(
                "PROMPT_COMMAND".to_string(),
                format!(
                    r#"if [ -z "$STAND_ORIGINAL_PS1" ]; then export STAND_ORIGINAL_PS1="$PS1"; fi; PS1="{prefix}$STAND_ORIGINAL_PS1""#,
                    prefix = prefix
                ),
            );
        }
        ShellType::Zsh => {
            // For zsh, we use the precmd hook via precmd_functions array.
            // This is sourced when the shell starts via ZDOTDIR or evaluated.
            // We set up a function that modifies PROMPT (zsh's PS1 equivalent).
            //
            // Note: zsh doesn't use PROMPT_COMMAND. Instead, we use a precmd function.
            // The simplest reliable approach is to directly set PROMPT with the prefix.
            vars.insert("PROMPT".to_string(), format!("{}%# ", prefix));
            // Also set PS1 for compatibility
            vars.insert("PS1".to_string(), format!("{}%# ", prefix));
        }
        ShellType::Fish => {
            // Fish handles prompts differently via fish_prompt function.
            // We just set the STAND_PROMPT variable for users to incorporate
            // into their fish_prompt function if desired.
            // Fish users can add: echo -n $STAND_PROMPT to their fish_prompt.
        }
        ShellType::Other(_) => {
            // For other shells (sh, dash, etc.), try basic PS1 modification
            vars.insert("PS1".to_string(), format!("{}$ ", prefix));
        }
    }

    vars
}

/// Generate a colored prompt prefix with ANSI escape codes
///
/// Uses green color for the environment name
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
        // Should capture original PS1 before modifying
        let prompt_cmd = vars.get("PROMPT_COMMAND").unwrap();
        assert!(prompt_cmd.contains("STAND_ORIGINAL_PS1"));
        assert!(prompt_cmd.contains("(stand:dev)"));
    }

    #[test]
    fn test_get_prompt_env_vars_zsh_sets_prompt() {
        let vars = get_prompt_env_vars(&ShellType::Zsh, "staging");
        // zsh uses PROMPT, not PROMPT_COMMAND
        assert!(vars.contains_key("PROMPT"));
        assert!(vars.contains_key("PS1"));
        assert!(!vars.contains_key("PROMPT_COMMAND"));
        let prompt = vars.get("PROMPT").unwrap();
        assert!(prompt.contains("(stand:staging)"));
    }

    #[test]
    fn test_get_prompt_env_vars_fish_only_sets_stand_prompt() {
        let vars = get_prompt_env_vars(&ShellType::Fish, "prod");
        assert_eq!(vars.get(STAND_PROMPT), Some(&"(stand:prod) ".to_string()));
        // Fish doesn't use PROMPT_COMMAND or PS1
        assert!(!vars.contains_key("PROMPT_COMMAND"));
        assert!(!vars.contains_key("PS1"));
    }

    #[test]
    fn test_get_prompt_env_vars_other_sets_ps1() {
        let vars = get_prompt_env_vars(&ShellType::Other("sh".to_string()), "dev");
        assert!(vars.contains_key("PS1"));
        let ps1 = vars.get("PS1").unwrap();
        assert!(ps1.contains("(stand:dev)"));
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
