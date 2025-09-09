use colored::Colorize;

/// Colorize an environment name with the specified color
pub fn colorize_environment(env_name: &str, color: Option<&str>) -> String {
    match color {
        Some(c) => {
            let color_lower = c.to_lowercase();
            match color_lower.as_str() {
                "red" => env_name.red().to_string(),
                "green" => env_name.green().to_string(),
                "blue" => env_name.blue().to_string(),
                "yellow" => env_name.yellow().to_string(),
                "purple" | "magenta" => env_name.magenta().to_string(),
                "cyan" => env_name.cyan().to_string(),
                _ => env_name.to_string(), // Invalid colors fallback to plain text
            }
        }
        None => env_name.to_string(),
    }
}

/// Format the default marker for environment listing
pub fn format_default_marker(is_default: bool) -> &'static str {
    if is_default {
        "*"
    } else {
        " "
    }
}

/// Mask sensitive values for display
pub fn mask_value(value: &str, show_values: bool) -> String {
    if show_values || value.is_empty() {
        value.to_string()
    } else {
        "********".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colorize_environment_with_green() {
        let result = colorize_environment("dev", Some("green"));
        // The function should return colored text (may not contain ANSI codes in test env)
        // but should at least contain the original text
        assert!(result.contains("dev"));
        // In test environment, colored crate may or may not add ANSI codes
        // The important thing is that the function handles green color without panicking
    }

    #[test]
    fn test_colorize_environment_with_red() {
        let result = colorize_environment("prod", Some("red"));
        // Same logic as green test
        assert!(result.contains("prod"));
    }

    #[test]
    fn test_colorize_environment_with_invalid_color() {
        let result = colorize_environment("test", Some("invalid"));
        // Should fallback to no color
        assert_eq!(result, "test");
    }

    #[test]
    fn test_colorize_environment_with_no_color() {
        let result = colorize_environment("staging", None);
        assert_eq!(result, "staging");
    }

    #[test]
    fn test_colorize_environment_with_purple() {
        let result = colorize_environment("staging", Some("purple"));
        assert!(result.contains("staging"));
        // Should use magenta() method internally
    }

    #[test]
    fn test_colorize_environment_with_magenta() {
        let result = colorize_environment("staging", Some("magenta"));
        assert!(result.contains("staging"));
    }

    #[test]
    fn test_colorize_environment_case_insensitive() {
        let result_upper = colorize_environment("prod", Some("RED"));
        let result_lower = colorize_environment("prod", Some("red"));
        assert!(result_upper.contains("prod"));
        assert!(result_lower.contains("prod"));
        // Both should work the same way
    }

    #[test]
    fn test_format_default_marker_true() {
        let result = format_default_marker(true);
        assert_eq!(result, "*");
    }

    #[test]
    fn test_format_default_marker_false() {
        let result = format_default_marker(false);
        assert_eq!(result, " ");
    }

    #[test]
    fn test_mask_value_hidden() {
        let result = mask_value("sensitive_password", false);
        assert_eq!(result, "********");
    }

    #[test]
    fn test_mask_value_shown() {
        let result = mask_value("some_value", true);
        assert_eq!(result, "some_value");
    }

    #[test]
    fn test_mask_empty_value() {
        let result = mask_value("", false);
        assert_eq!(result, "");
    }
}
