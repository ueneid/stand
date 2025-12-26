// Shell spawner module
//
// Handles spawning interactive shell sessions with environment variables.

use crate::shell::detector::ShellType;
use crate::shell::prompt::get_prompt_env_vars;
use anyhow::Result;
use std::collections::HashMap;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

/// Environment variable names used by Stand
pub const STAND_ACTIVE: &str = "STAND_ACTIVE";
pub const STAND_ENVIRONMENT: &str = "STAND_ENVIRONMENT";
pub const STAND_PROJECT_ROOT: &str = "STAND_PROJECT_ROOT";

/// Build the complete environment for a Stand shell session
///
/// Combines:
/// 1. User-defined environment variables from config
/// 2. Stand marker variables (STAND_ACTIVE, STAND_ENVIRONMENT, STAND_PROJECT_ROOT)
/// 3. Prompt customization variables
pub fn build_shell_environment(
    user_env: HashMap<String, String>,
    env_name: &str,
    project_root: &str,
    shell_path: &str,
) -> HashMap<String, String> {
    let mut env = user_env;

    // Add Stand marker variables
    env.insert(STAND_ACTIVE.to_string(), "1".to_string());
    env.insert(STAND_ENVIRONMENT.to_string(), env_name.to_string());
    env.insert(STAND_PROJECT_ROOT.to_string(), project_root.to_string());

    // Add prompt customization variables based on the actual shell being spawned
    let shell_type = ShellType::from_path(shell_path);
    let prompt_vars = get_prompt_env_vars(&shell_type, env_name);
    for (key, value) in prompt_vars {
        env.insert(key, value);
    }

    env
}

/// Spawn an interactive shell with the given environment variables
///
/// # Arguments
/// * `shell_path` - Path to the shell executable (e.g., "/bin/bash")
/// * `env_vars` - Environment variables to inject into the shell
///
/// # Returns
/// The exit code of the shell process
pub fn spawn_shell(shell_path: &str, env_vars: HashMap<String, String>) -> Result<i32> {
    let shell_type = ShellType::from_path(shell_path);

    // Build shell arguments based on shell type
    let args = get_shell_args(&shell_type);

    let mut cmd = Command::new(shell_path);
    cmd.args(&args);

    // Add environment variables
    for (key, value) in &env_vars {
        cmd.env(key, value);
    }

    // For zsh, set up ZDOTDIR with custom .zshrc
    let zdotdir_cleanup = if matches!(shell_type, ShellType::Zsh) {
        setup_zsh_zdotdir(&mut cmd, &env_vars)?
    } else {
        None
    };

    let status = cmd.status()?;

    // Clean up ZDOTDIR if we created one
    if let Some(path) = zdotdir_cleanup {
        let _ = std::fs::remove_dir_all(path);
    }

    // Return exit code, handling signal termination on Unix
    match status.code() {
        Some(code) => Ok(code),
        None => {
            #[cfg(unix)]
            {
                if let Some(signal) = status.signal() {
                    return Ok(128 + signal);
                }
            }
            Ok(1)
        }
    }
}

/// Set up ZDOTDIR for zsh with a custom .zshrc
///
/// Creates a temporary directory with a .zshrc that:
/// 1. Sources the user's original .zshrc
/// 2. Adds our precmd function for prompt customization
///
/// Returns the path to the temp directory for cleanup
fn setup_zsh_zdotdir(
    cmd: &mut Command,
    env_vars: &HashMap<String, String>,
) -> Result<Option<std::path::PathBuf>> {
    use std::io::Write;

    // Create temp directory
    let temp_dir = std::env::temp_dir().join(format!("stand-zsh-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir)?;

    // Get color from env vars and validate against allowlist to prevent command injection
    let color = env_vars
        .get("STAND_ENV_COLOR")
        .map(|s| s.as_str())
        .unwrap_or("green");
    let safe_color = match color {
        "red" | "green" | "yellow" | "blue" | "magenta" | "purple" | "cyan" | "white" | "black" => color,
        _ => "green", // Default to green for invalid/unknown colors
    };

    // Write .zshenv to source user's original .zshenv
    // This ensures environment setup from .zshenv is not skipped
    let zshenv_content = r#"# Stand temporary zshenv
# Source user's original .zshenv if it exists
[[ -f "$HOME/.zshenv" ]] && source "$HOME/.zshenv"
"#;
    let zshenv_path = temp_dir.join(".zshenv");
    let mut zshenv_file = std::fs::File::create(&zshenv_path)?;
    zshenv_file.write_all(zshenv_content.as_bytes())?;

    // Write custom .zshrc
    // This sources the user's .zshrc first, then adds our precmd
    let zshrc_content = format!(
        r#"# Stand temporary zshrc
# Restore original ZDOTDIR for child shells
export ZDOTDIR="$HOME"

# Source user's original .zshrc if it exists
[[ -f "$HOME/.zshrc" ]] && source "$HOME/.zshrc"

# Stand precmd function for prompt customization
_stand_precmd() {{
    # Save original prompt on first run
    if [[ -z "$STAND_ORIGINAL_PROMPT" ]]; then
        export STAND_ORIGINAL_PROMPT="$PROMPT"
    fi
    # Set prompt with Stand indicator (newline, bold, reverse, colored)
    local color="{safe_color}"
    local env_upper="${{(U)STAND_ENVIRONMENT}}"
    PROMPT=$'\n%B%S%F{{'"$color"'}} stand:'"$env_upper"$' %f%s%b'"$STAND_ORIGINAL_PROMPT"
}}

# Add to precmd_functions array (runs after any existing precmd)
precmd_functions+=(_stand_precmd)
"#
    );

    let zshrc_path = temp_dir.join(".zshrc");
    let mut file = std::fs::File::create(&zshrc_path)?;
    file.write_all(zshrc_content.as_bytes())?;

    // Set ZDOTDIR to our temp directory
    cmd.env("ZDOTDIR", &temp_dir);

    Ok(Some(temp_dir))
}

/// Get appropriate shell arguments for interactive mode
fn get_shell_args(shell_type: &ShellType) -> Vec<String> {
    match shell_type {
        ShellType::Fish => {
            // Fish uses functions for prompts, not environment variables.
            // We use -C to inject an init command that wraps the existing fish_prompt
            // function to prepend our Stand indicator with color from config.
            let init_cmd = concat!(
                "functions -c fish_prompt _stand_original_fish_prompt 2>/dev/null; ",
                "or function _stand_original_fish_prompt; echo '> '; end; ",
                "function fish_prompt; ",
                "echo; ",
                "set -q STAND_ENV_COLOR; and set_color --bold --reverse $STAND_ENV_COLOR; or set_color --bold --reverse green; ",
                "echo -n ' stand:'(string upper $STAND_ENVIRONMENT)' '; ",
                "set_color normal; ",
                "_stand_original_fish_prompt; end"
            );
            vec!["-C".to_string(), init_cmd.to_string()]
        }
        ShellType::Zsh => {
            // Zsh: Use -i for interactive mode.
            // Prompt customization is done via RPS1 (right prompt) environment variable
            // which is set in get_prompt_env_vars and is rarely overridden by users.
            vec!["-i".to_string()]
        }
        _ => {
            // bash and others use -i for interactive mode
            vec!["-i".to_string()]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_shell_environment_includes_user_vars() {
        let mut user_env = HashMap::new();
        user_env.insert(
            "DATABASE_URL".to_string(),
            "postgres://localhost".to_string(),
        );
        user_env.insert("API_KEY".to_string(), "secret123".to_string());

        let result = build_shell_environment(user_env, "dev", "/home/user/project", "/bin/bash");

        assert_eq!(
            result.get("DATABASE_URL"),
            Some(&"postgres://localhost".to_string())
        );
        assert_eq!(result.get("API_KEY"), Some(&"secret123".to_string()));
    }

    #[test]
    fn test_build_shell_environment_includes_stand_markers() {
        let user_env = HashMap::new();
        let result = build_shell_environment(user_env, "production", "/var/www/app", "/bin/bash");

        assert_eq!(result.get(STAND_ACTIVE), Some(&"1".to_string()));
        assert_eq!(
            result.get(STAND_ENVIRONMENT),
            Some(&"production".to_string())
        );
        assert_eq!(
            result.get(STAND_PROJECT_ROOT),
            Some(&"/var/www/app".to_string())
        );
    }

    #[test]
    fn test_build_shell_environment_stand_markers_override_user_vars() {
        let mut user_env = HashMap::new();
        // User tries to set STAND_ACTIVE (should be overridden)
        user_env.insert(STAND_ACTIVE.to_string(), "0".to_string());

        let result = build_shell_environment(user_env, "dev", "/home/user/project", "/bin/bash");

        // Stand markers should override user-provided values
        assert_eq!(result.get(STAND_ACTIVE), Some(&"1".to_string()));
    }

    #[test]
    fn test_get_shell_args_bash() {
        let args = get_shell_args(&ShellType::Bash);
        assert_eq!(args, vec!["-i".to_string()]);
    }

    #[test]
    fn test_get_shell_args_zsh() {
        let args = get_shell_args(&ShellType::Zsh);
        // Zsh uses -i for interactive mode, prompt customization via RPS1 env var
        assert_eq!(args, vec!["-i".to_string()]);
    }

    #[test]
    fn test_get_shell_args_fish() {
        let args = get_shell_args(&ShellType::Fish);
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], "-C");
        // The init command should wrap fish_prompt and use STAND_ENVIRONMENT
        assert!(args[1].contains("fish_prompt"));
        assert!(args[1].contains("STAND_ENVIRONMENT"));
    }

    #[test]
    fn test_get_shell_args_other() {
        let args = get_shell_args(&ShellType::Other("sh".to_string()));
        assert_eq!(args, vec!["-i".to_string()]);
    }
}
