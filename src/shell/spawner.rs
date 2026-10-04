// Shell spawner module
//
// Handles spawning interactive shell sessions with environment variables.

use crate::shell::detector::ShellType;
use crate::shell::prompt::get_prompt_env_vars;
use crate::shell::reflect::{wrapper_function, ShellKind};
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
    let mut args = get_shell_args(&shell_type);

    // The `stand` wrapper function lets `stand set/unset` update this shell.
    // It is only installed for bash, zsh and fish; other shells get no wrapper.
    let wrapper = std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(str::to_string))
        .and_then(|bin| wrapper_for_shell(&shell_type, &bin));

    let mut cmd = Command::new(shell_path);

    // Add environment variables
    for (key, value) in &env_vars {
        cmd.env(key, value);
    }

    // Temp dirs must stay alive while the shell runs
    let mut temp_dir: Option<tempfile::TempDir> = None;
    match shell_type {
        // For zsh, set up ZDOTDIR with custom .zshrc
        ShellType::Zsh => {
            temp_dir = Some(setup_zsh_zdotdir(&mut cmd, &env_vars, wrapper.as_deref())?);
        }
        // For bash, use a generated rcfile so the wrapper is defined after ~/.bashrc
        ShellType::Bash => {
            if let Some(wrapper) = wrapper.as_deref() {
                temp_dir = Some(setup_bash_rcfile(&mut args, wrapper)?);
            }
        }
        // For fish, the wrapper is appended to the -C init command
        ShellType::Fish => {
            if let Some(wrapper) = wrapper.as_deref() {
                append_fish_wrapper(&mut args, wrapper);
            }
        }
        ShellType::Other(_) => {}
    }

    cmd.args(&args);
    let status = cmd.status();

    // Clean up the temp dir (also removed on drop if an error occurs)
    if let Some(dir) = temp_dir {
        let _ = dir.close();
    }
    let status = status?;

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
/// Returns the temp directory; it is removed when dropped
fn setup_zsh_zdotdir(
    cmd: &mut Command,
    env_vars: &HashMap<String, String>,
    wrapper: Option<&str>,
) -> Result<tempfile::TempDir> {
    use std::io::Write;

    // Create a private (0700), randomly named temp directory
    let temp = create_secure_tempdir("stand-zsh-")?;
    let temp_dir = temp.path().to_path_buf();

    // Get color from env vars and validate against allowlist to prevent command injection
    let color = env_vars
        .get("STAND_ENV_COLOR")
        .map(|s| s.as_str())
        .unwrap_or("green");
    let safe_color = match color {
        "red" | "green" | "yellow" | "blue" | "magenta" | "purple" | "cyan" | "white" | "black" => {
            color
        }
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
    // This sources the user's .zshrc first, then adds our precmd and chpwd hooks
    let zshrc_content = format!(
        r#"# Stand temporary zshrc
# Restore original ZDOTDIR for child shells
export ZDOTDIR="$HOME"

# Source user's original .zshrc if it exists
[[ -f "$HOME/.zshrc" ]] && source "$HOME/.zshrc"

# Track previous directory for reverting
typeset -g _stand_prev_dir="$PWD"

# Stand chpwd function for directory guard when leaving project directory
# Only active when STAND_AUTO_EXIT=1
# Uses logical paths ($PWD) instead of physical paths to allow symlinks
_stand_chpwd() {{
    if [[ "$STAND_AUTO_EXIT" = "1" ]] && [[ -n "$STAND_PROJECT_ROOT" ]]; then
        case "$PWD" in
            "$STAND_PROJECT_ROOT"|"$STAND_PROJECT_ROOT"/*)
                _stand_prev_dir="$PWD"
                ;;
            *)
                # Revert to previous directory with fallback
                if ! builtin cd "$_stand_prev_dir" 2>/dev/null; then
                    if ! builtin cd "$STAND_PROJECT_ROOT" 2>/dev/null; then
                        echo "⚠️  Cannot return to project directory. Exiting Stand shell."
                        exit 1
                    fi
                fi
                echo "⚠️  Cannot leave project directory while in Stand shell."
                echo "    Type 'exit' to leave the Stand shell first."
                ;;
        esac
    fi
}}

# Add to chpwd_functions array (runs on directory change)
chpwd_functions+=(_stand_chpwd)

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

# Stand wrapper so `stand set/unset` update this shell (defined after user's rc)
{wrapper}"#,
        wrapper = wrapper.unwrap_or("")
    );

    let zshrc_path = temp_dir.join(".zshrc");
    let mut file = std::fs::File::create(&zshrc_path)?;
    file.write_all(zshrc_content.as_bytes())?;

    // Set ZDOTDIR to our temp directory
    cmd.env("ZDOTDIR", &temp_dir);

    Ok(temp)
}

/// Create a private temp directory (mode 0700, random name) with the given prefix.
fn create_secure_tempdir(prefix: &str) -> Result<tempfile::TempDir> {
    let dir = tempfile::Builder::new().prefix(prefix).tempdir()?;
    // The default mode follows the umask (often 0755), so restrict it explicitly
    // before any file is written into the directory.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(dir)
}

/// Wrapper function text for the given shell, or None for unsupported shells.
fn wrapper_for_shell(shell_type: &ShellType, stand_bin: &str) -> Option<String> {
    match shell_type {
        ShellType::Bash | ShellType::Zsh => Some(wrapper_function(ShellKind::Posix, stand_bin)),
        ShellType::Fish => Some(wrapper_function(ShellKind::Fish, stand_bin)),
        ShellType::Other(_) => None,
    }
}

/// Set up a generated bash rcfile that sources ~/.bashrc, then defines the wrapper.
///
/// Prepends `--rcfile <dir>/bashrc` to the shell arguments and returns the temp
/// directory, which is removed when dropped.
fn setup_bash_rcfile(args: &mut Vec<String>, wrapper: &str) -> Result<tempfile::TempDir> {
    let temp = create_secure_tempdir("stand-bash-")?;
    let rcfile = temp.path().join("bashrc");
    let content = format!(
        r#"# Stand temporary bashrc
# Source user's original .bashrc if it exists
[ -f "$HOME/.bashrc" ] && . "$HOME/.bashrc"

# Stand wrapper so `stand set/unset` update this shell (defined after user's rc)
{wrapper}"#
    );
    std::fs::write(&rcfile, content)?;
    args.insert(0, "--rcfile".to_string());
    args.insert(1, rcfile.to_string_lossy().into_owned());
    Ok(temp)
}

/// Append the wrapper to fish's `-C` init command (args[1]).
fn append_fish_wrapper(args: &mut [String], wrapper: &str) {
    if let Some(init) = args.get_mut(1) {
        init.push('\n');
        init.push_str(wrapper);
    }
}

/// Get appropriate shell arguments for interactive mode
fn get_shell_args(shell_type: &ShellType) -> Vec<String> {
    match shell_type {
        ShellType::Fish => {
            // Fish uses functions for prompts, not environment variables.
            // We use -C to inject an init command that wraps the existing fish_prompt
            // function to prepend our Stand indicator with color from config.
            // Also adds a PWD variable watcher for directory guard when leaving project directory.
            // Uses logical paths ($PWD) instead of physical paths to allow symlinks.
            // The _stand_reverting flag prevents recursion when we revert the directory.
            let init_cmd = concat!(
                // Initialize state variables
                "set -g _stand_prev_dir \"$PWD\"; ",
                "set -g _stand_reverting 0; ",
                // Directory guard function when leaving project directory
                "function _stand_check_dir --on-variable PWD; ",
                "if test \"$_stand_reverting\" = \"1\"; set -g _stand_reverting 0; return; end; ",
                "if test \"$STAND_AUTO_EXIT\" = \"1\" -a -n \"$STAND_PROJECT_ROOT\"; ",
                "if not string match -q \"$STAND_PROJECT_ROOT\" \"$PWD\"; ",
                "and not string match -q \"$STAND_PROJECT_ROOT/*\" \"$PWD\"; ",
                "set -g _stand_reverting 1; ",
                "if not builtin cd \"$_stand_prev_dir\" 2>/dev/null; ",
                "if not builtin cd \"$STAND_PROJECT_ROOT\" 2>/dev/null; ",
                "echo '⚠️  Cannot return to project directory. Exiting Stand shell.'; exit 1; end; end; ",
                "echo '⚠️  Cannot leave project directory while in Stand shell.'; ",
                "echo '    Type \\'exit\\' to leave the Stand shell first.'; ",
                "return; end; end; ",
                "set -g _stand_prev_dir \"$PWD\"; end; ",
                // Prompt customization
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

    #[cfg(unix)]
    fn mode_of(path: &std::path::Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[cfg(unix)]
    #[test]
    fn test_create_secure_tempdir_is_private_and_removed() {
        let dir = create_secure_tempdir("stand-test-").unwrap();
        let path = dir.path().to_path_buf();
        assert_eq!(mode_of(&path), 0o700);
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with("stand-test-"));
        assert_ne!(name, format!("stand-test-{}", std::process::id()));
        dir.close().unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_wrapper_for_shell_supported_and_unsupported() {
        let b = wrapper_for_shell(&ShellType::Bash, "/x/stand").unwrap();
        assert!(b.contains("stand()"));
        let z = wrapper_for_shell(&ShellType::Zsh, "/x/stand").unwrap();
        assert!(z.contains("stand()"));
        let f = wrapper_for_shell(&ShellType::Fish, "/x/stand").unwrap();
        assert!(f.contains("function stand"));
        assert!(wrapper_for_shell(&ShellType::Other("sh".into()), "/x/stand").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn test_setup_bash_rcfile_creates_private_rcfile_with_wrapper() {
        let mut args = vec!["-i".to_string()];
        let dir = setup_bash_rcfile(&mut args, "stand() { :; }\n").unwrap();
        assert_eq!(mode_of(dir.path()), 0o700);
        let dir_name = dir.path().file_name().unwrap().to_str().unwrap();
        assert!(dir_name.starts_with("stand-bash-"));
        let rc = dir.path().join("bashrc");
        assert_eq!(args[0], "--rcfile");
        assert_eq!(args[1], rc.to_string_lossy());
        assert_eq!(args[2], "-i");
        let content = std::fs::read_to_string(&rc).unwrap();
        let bashrc_pos = content.find("$HOME/.bashrc").unwrap();
        let wrapper_pos = content.find("stand() { :; }").unwrap();
        assert!(
            bashrc_pos < wrapper_pos,
            "wrapper must follow user's bashrc"
        );
        let path = dir.path().to_path_buf();
        dir.close().unwrap();
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_setup_zsh_zdotdir_private_and_wrapper_after_user_zshrc() {
        let mut cmd = Command::new("zsh");
        let dir = setup_zsh_zdotdir(&mut cmd, &HashMap::new(), Some("stand() { :; }\n")).unwrap();
        assert_eq!(mode_of(dir.path()), 0o700);
        let dir_name = dir.path().file_name().unwrap().to_str().unwrap();
        assert!(dir_name.starts_with("stand-zsh-"));
        let content = std::fs::read_to_string(dir.path().join(".zshrc")).unwrap();
        let user_pos = content.find("source \"$HOME/.zshrc\"").unwrap();
        let wrapper_pos = content.find("stand() { :; }").unwrap();
        assert!(user_pos < wrapper_pos);
        let path = dir.path().to_path_buf();
        dir.close().unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_setup_zsh_zdotdir_without_wrapper_has_no_wrapper() {
        let mut cmd = Command::new("zsh");
        let dir = setup_zsh_zdotdir(&mut cmd, &HashMap::new(), None).unwrap();
        let content = std::fs::read_to_string(dir.path().join(".zshrc")).unwrap();
        assert!(!content.contains("stand() {"));
    }

    #[test]
    fn test_append_fish_wrapper_extends_init_command() {
        let mut args = get_shell_args(&ShellType::Fish);
        append_fish_wrapper(&mut args, "function stand\nend\n");
        assert_eq!(args[0], "-C");
        assert!(args[1].contains("function fish_prompt"));
        assert!(args[1].ends_with("function stand\nend\n"));
    }

    /// Run `shell` with `args`/`envs`, feed `script` on stdin, return stdout.
    #[cfg(unix)]
    fn run_with_stdin(
        shell: &std::path::Path,
        args: &[String],
        envs: &[(&str, &std::path::Path)],
        script: &str,
    ) -> String {
        use std::io::Write;
        use std::process::Stdio;
        let mut cmd = Command::new(shell);
        cmd.args(args);
        for (k, v) in envs {
            cmd.env(k, v);
        }
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(script.as_bytes())
            .unwrap();
        String::from_utf8_lossy(&child.wait_with_output().unwrap().stdout).into_owned()
    }

    #[cfg(unix)]
    fn fake_stand(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let fake = dir.join("fake stand");
        std::fs::write(&fake, body).unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        fake
    }

    /// Fish: wrapper appended to the -C init command works in a real fish.
    #[cfg(unix)]
    #[test]
    fn test_fish_init_command_defines_working_wrapper() {
        let Some(fish) = [
            "/opt/homebrew/bin/fish",
            "/usr/bin/fish",
            "/usr/local/bin/fish",
        ]
        .iter()
        .map(std::path::Path::new)
        .find(|p| p.is_file()) else {
            return;
        };
        let home = tempfile::tempdir().unwrap();
        let fake = fake_stand(
            home.path(),
            "#!/bin/sh\n[ \"$STAND_SHELL_FD\" = 3 ] && echo \"set -gx KEY 'v2'\" >&3\n",
        );
        let wrapper = wrapper_function(ShellKind::Fish, fake.to_str().unwrap());
        let mut args = vec!["--no-config".to_string()];
        args.extend(get_shell_args(&ShellType::Fish));
        append_fish_wrapper(&mut args[1..], &wrapper);
        let out = run_with_stdin(
            fish,
            &args,
            &[("HOME", home.path())],
            "stand set dev KEY\necho R:$KEY\nexit\n",
        );
        assert!(out.contains("R:v2"), "got: {out}");
    }

    /// Zsh: generated ZDOTDIR defines the wrapper after the user's .zshrc.
    #[cfg(unix)]
    #[test]
    fn test_zsh_zdotdir_defines_working_wrapper_after_user_zshrc() {
        let Some(zsh) = ["/bin/zsh", "/usr/bin/zsh", "/opt/homebrew/bin/zsh"]
            .iter()
            .map(std::path::Path::new)
            .find(|p| p.is_file())
        else {
            return;
        };
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join(".zshrc"), "USER_RC_LOADED=yes\n").unwrap();
        let fake = fake_stand(
            home.path(),
            "#!/bin/sh\n[ \"$STAND_SHELL_FD\" = 3 ] && echo \"export KEY='v3'\" >&3\n",
        );
        let wrapper = wrapper_function(ShellKind::Posix, fake.to_str().unwrap());
        let mut cmd = Command::new(zsh);
        let dir = setup_zsh_zdotdir(&mut cmd, &HashMap::new(), Some(&wrapper)).unwrap();
        let out = run_with_stdin(
            zsh,
            &["-i".to_string()],
            &[("HOME", home.path()), ("ZDOTDIR", dir.path())],
            "stand set dev KEY\necho \"R:$USER_RC_LOADED:$KEY\"\nexit\n",
        );
        assert!(out.contains("R:yes:v3"), "got: {out}");
        dir.close().unwrap();
    }

    /// Run a generated bash rcfile in a real interactive bash and call the wrapper.
    #[cfg(unix)]
    #[test]
    fn test_bash_rcfile_defines_working_wrapper_after_user_bashrc() {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        use std::process::Stdio;

        let Some(bash) = ["/bin/bash", "/usr/bin/bash"]
            .iter()
            .find(|p| std::path::Path::new(p).is_file())
        else {
            return;
        };
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join(".bashrc"), "USER_RC_LOADED=yes\n").unwrap();
        let fake = home.path().join("fake stand");
        std::fs::write(
            &fake,
            "#!/bin/sh\n[ \"$STAND_SHELL_FD\" = 3 ] && echo \"export KEY='v1'\" >&3\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

        let wrapper = wrapper_function(ShellKind::Posix, fake.to_str().unwrap());
        let mut args = vec!["-i".to_string()];
        let dir = setup_bash_rcfile(&mut args, &wrapper).unwrap();

        let mut child = Command::new(bash)
            .args(&args)
            .env("HOME", home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"stand set dev KEY\necho \"R:$USER_RC_LOADED:$KEY\"\nexit\n")
            .unwrap();
        let out = child.wait_with_output().unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("R:yes:v1"), "got: {stdout}");
        dir.close().unwrap();
    }
}
