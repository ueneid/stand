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
            // We capture the original PS1 on first run, then prepend our prefix with color.
            // Uses $STAND_ENVIRONMENT and $STAND_ENV_COLOR for dynamic values.
            // Color codes: bold=1, reverse=7, green=32, reset=0
            // Note: Using tr for uppercase conversion for compatibility with Bash 3.x (macOS default)
            vars.insert(
                "PROMPT_COMMAND".to_string(),
                r#"if [ -z "$STAND_ORIGINAL_PS1" ]; then export STAND_ORIGINAL_PS1="$PS1"; fi; _c="${STAND_ENV_COLOR:-green}"; case "$_c" in red) _cc=31;; green) _cc=32;; yellow) _cc=33;; blue) _cc=34;; magenta|purple) _cc=35;; cyan) _cc=36;; *) _cc=32;; esac; _env_upper=$(echo "$STAND_ENVIRONMENT" | tr '[:lower:]' '[:upper:]'); PS1=$'\n\e[1;7;'"$_cc"'m stand:'"$_env_upper"$' \e[0m'"$STAND_ORIGINAL_PS1""#.to_string(),
            );
        }
        ShellType::Zsh => {
            // Zsh: Set STAND_ZSH_PRECMD which will be evaled by the spawner's init command.
            // This ensures our precmd runs after .zshrc has loaded.
        }
        ShellType::Fish => {
            // Fish handles prompts via fish_prompt function, not environment variables.
            // We set STAND_PROMPT here, and the spawner injects an init command
            // that wraps the existing fish_prompt to prepend STAND_PROMPT.
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
        // Uses $STAND_ENVIRONMENT variable instead of embedded name for safety
        assert!(prompt_cmd.contains("STAND_ENVIRONMENT"));
        // Uses $STAND_ENV_COLOR for color customization
        assert!(prompt_cmd.contains("STAND_ENV_COLOR"));
    }

    #[test]
    fn test_get_prompt_env_vars_zsh_only_sets_stand_prompt() {
        let vars = get_prompt_env_vars(&ShellType::Zsh, "staging");
        // STAND_PROMPT is set for all shells
        assert_eq!(
            vars.get(STAND_PROMPT),
            Some(&"(stand:staging) ".to_string())
        );
        // Zsh prompt customization is handled via ZDOTDIR in spawner,
        // so only STAND_PROMPT is set here
        assert!(!vars.contains_key("RPS1"));
        assert!(!vars.contains_key("PROMPT"));
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
