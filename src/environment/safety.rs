//! Safety checks for variable names stored in `.stand.toml`.
//!
//! Values in `.stand.toml` become environment variables of the human's
//! interactive shell after secrets are unlocked. Anyone who can edit the file
//! (for example an AI agent) could plant shell or loader hooks that exfiltrate
//! secrets. The lists here are a speed bump, not a complete defense.

/// Keys that are environment settings, not variables.
pub const RESERVED_KEYS: &[&str] = &["description", "extends", "color", "requires_confirmation"];

const BLOCKED_EXACT: &[&str] = &[
    "BASH_ENV",
    "ENV",
    "PROMPT_COMMAND",
    "PS0",
    "PS1",
    "PS2",
    "PS3",
    "PS4",
    "SHELLOPTS",
    "BASHOPTS",
    "ZDOTDIR",
    "HOME",
    "XDG_CONFIG_HOME",
    "IFS",
];

const BLOCKED_PREFIXES: &[&str] = &["LD_", "DYLD_", "BASH_FUNC_", "STAND_"];

const WARN_EXACT: &[&str] = &[
    "PATH",
    "NODE_OPTIONS",
    "PYTHONSTARTUP",
    "PYTHONPATH",
    "PERL5OPT",
    "PERL5LIB",
    "RUBYOPT",
    "RUBYLIB",
    "GIT_SSH_COMMAND",
    "GIT_EXEC_PATH",
];

/// Returns true if `name` is a reserved environment setting key.
pub fn is_reserved_key(name: &str) -> bool {
    RESERVED_KEYS.contains(&name)
}

/// Returns true if `name` matches `^[A-Za-z_][A-Za-z0-9_]*$`.
pub fn is_valid_variable_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// How dangerous a variable name is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableRisk {
    Safe,
    Warn,
    Blocked,
}

/// Classify a variable name (case-sensitive).
pub fn classify_variable(name: &str) -> VariableRisk {
    if BLOCKED_EXACT.contains(&name) || BLOCKED_PREFIXES.iter().any(|p| name.starts_with(p)) {
        VariableRisk::Blocked
    } else if WARN_EXACT.contains(&name) {
        VariableRisk::Warn
    } else {
        VariableRisk::Safe
    }
}

/// Risky variable names found in a set of names.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RiskReport {
    /// Blocked names, sorted and deduplicated.
    pub blocked: Vec<String>,
    /// Warned names, sorted and deduplicated.
    pub warned: Vec<String>,
}

/// Collect risky names from `names`.
pub fn find_risky_variables<'a, I: IntoIterator<Item = &'a str>>(names: I) -> RiskReport {
    let mut report = RiskReport::default();
    for name in names {
        match classify_variable(name) {
            VariableRisk::Blocked => report.blocked.push(name.to_string()),
            VariableRisk::Warn => report.warned.push(name.to_string()),
            VariableRisk::Safe => {}
        }
    }
    report.blocked.sort();
    report.blocked.dedup();
    report.warned.sort();
    report.warned.dedup();
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reserved_keys() {
        for k in ["description", "extends", "color", "requires_confirmation"] {
            assert!(is_reserved_key(k));
        }
        assert!(!is_reserved_key("DESCRIPTION"));
        assert!(!is_reserved_key("API_KEY"));
    }

    #[test]
    fn test_valid_variable_names() {
        for n in ["A", "_a", "API_KEY", "a1", "__"] {
            assert!(is_valid_variable_name(n), "{n}");
        }
        for n in ["", "1BAD", "A-B", "A B", "A=B", "É", "A.B"] {
            assert!(!is_valid_variable_name(n), "{n}");
        }
    }

    #[test]
    fn test_blocked_exact() {
        for n in [
            "BASH_ENV",
            "ENV",
            "PROMPT_COMMAND",
            "PS0",
            "PS1",
            "PS2",
            "PS3",
            "PS4",
            "SHELLOPTS",
            "BASHOPTS",
            "ZDOTDIR",
            "HOME",
            "XDG_CONFIG_HOME",
            "IFS",
        ] {
            assert_eq!(classify_variable(n), VariableRisk::Blocked, "{n}");
        }
    }

    #[test]
    fn test_blocked_prefixes() {
        for n in [
            "LD_PRELOAD",
            "LD_LIBRARY_PATH",
            "DYLD_INSERT_LIBRARIES",
            "BASH_FUNC_foo%%",
            "STAND_PRIVATE_KEY",
        ] {
            assert_eq!(classify_variable(n), VariableRisk::Blocked, "{n}");
        }
    }

    #[test]
    fn test_warn_exact() {
        for n in [
            "PATH",
            "NODE_OPTIONS",
            "PYTHONSTARTUP",
            "PYTHONPATH",
            "PERL5OPT",
            "PERL5LIB",
            "RUBYOPT",
            "RUBYLIB",
            "GIT_SSH_COMMAND",
            "GIT_EXEC_PATH",
        ] {
            assert_eq!(classify_variable(n), VariableRisk::Warn, "{n}");
        }
    }

    #[test]
    fn test_safe_and_case_sensitive() {
        for n in [
            "API_KEY",
            "path",
            "home",
            "ld_preload",
            "LDX",
            "MY_LD_X",
            "ENVIRONMENT",
            "PS5",
        ] {
            assert_eq!(classify_variable(n), VariableRisk::Safe, "{n}");
        }
    }

    #[test]
    fn test_find_risky_variables_sorted_deduped() {
        let r = find_risky_variables([
            "ZDOTDIR",
            "API",
            "PATH",
            "BASH_ENV",
            "ZDOTDIR",
            "PATH",
            "NODE_OPTIONS",
        ]);
        assert_eq!(r.blocked, vec!["BASH_ENV", "ZDOTDIR"]);
        assert_eq!(r.warned, vec!["NODE_OPTIONS", "PATH"]);
        assert_eq!(find_risky_variables(["A", "B"]), RiskReport::default());
    }
}
