use anyhow::Result;
use indexmap::IndexMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    InvalidFormat { line: usize, content: String },
    UnterminatedQuote { line: usize },
    InvalidEscape { line: usize, sequence: String },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidFormat { line, content } => {
                write!(f, "Invalid format at line {}: '{}'", line, content)
            }
            ParseError::UnterminatedQuote { line } => {
                write!(f, "Unterminated quote at line {}", line)
            }
            ParseError::InvalidEscape { line, sequence } => {
                write!(f, "Invalid escape sequence '{}' at line {}", sequence, line)
            }
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse_env_content(content: &str) -> Result<IndexMap<String, String>, ParseError> {
    parse_env_content_with_options(content, &ParseOptions::default())
}

#[derive(Debug, Clone)]
pub struct ParseOptions {
    pub expand_variables: bool,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            expand_variables: true,
        }
    }
}

pub fn parse_env_content_with_options(
    content: &str,
    options: &ParseOptions,
) -> Result<IndexMap<String, String>, ParseError> {
    let mut variables = IndexMap::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut line_idx = 0;

    while line_idx < lines.len() {
        let line_num = line_idx + 1;
        let line = lines[line_idx];
        let trimmed_for_check = line.trim();

        // Skip empty lines and comments
        if trimmed_for_check.is_empty() || trimmed_for_check.starts_with('#') {
            line_idx += 1;
            continue;
        }

        // Find the first '=' that's not inside quotes (use original line to preserve spaces)
        let eq_pos = find_equals_position(line).ok_or_else(|| ParseError::InvalidFormat {
            line: line_num,
            content: line.to_string(),
        })?;

        let key = line[..eq_pos].trim();
        let value_part = &line[eq_pos + 1..];

        // Validate key name
        if key.is_empty() || !is_valid_key(key) {
            return Err(ParseError::InvalidFormat {
                line: line_num,
                content: line.to_string(),
            });
        }

        // Parse and process the value (may consume multiple lines)
        let (parsed_value, lines_consumed) =
            parse_value_multiline(value_part, &lines[line_idx..], line_num)?;

        let final_value = if options.expand_variables {
            expand_variables(&parsed_value, &variables)
        } else {
            parsed_value
        };

        variables.insert(key.to_string(), final_value);
        line_idx += lines_consumed;
    }

    Ok(variables)
}

fn find_equals_position(line: &str) -> Option<usize> {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for (i, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '=' if !in_single_quote && !in_double_quote => return Some(i),
            _ => {}
        }
    }

    None
}

fn is_valid_key(key: &str) -> bool {
    !key.is_empty() && key.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn parse_value_multiline(
    value_part: &str,
    remaining_lines: &[&str],
    line_num: usize,
) -> Result<(String, usize), ParseError> {
    // Check if this is a quoted value by looking at the trimmed start
    let trimmed_start = value_part.trim_start();

    // Handle quoted values first
    if trimmed_start.starts_with('"') {
        return parse_multiline_double_quoted(remaining_lines, line_num);
    } else if trimmed_start.starts_with('\'') {
        return parse_multiline_single_quoted(remaining_lines, line_num);
    }

    // Handle unquoted values
    let value = if let Some(comment_pos) = value_part.find('#') {
        // Remove inline comment and trim trailing whitespace
        value_part[..comment_pos].trim_end()
    } else {
        // No comment - preserve exact value (including spaces)
        value_part
    };

    Ok((value.to_string(), 1))
}

fn parse_multiline_double_quoted(
    lines: &[&str],
    start_line: usize,
) -> Result<(String, usize), ParseError> {
    let first_line = lines[0];
    let value_part = &first_line[first_line.find('=').unwrap() + 1..];

    if !value_part.trim().starts_with('"') {
        return Err(ParseError::InvalidFormat {
            line: start_line,
            content: first_line.to_string(),
        });
    }

    let mut content = String::new();
    let mut lines_consumed = 1;
    let mut found_closing_quote = false;

    // Start from the opening quote
    let start_quote_pos = value_part.find('"').unwrap();
    let current_content = &value_part[start_quote_pos + 1..];

    // Check if the quote is closed on the same line
    if let Some(end_pos) = find_closing_quote(current_content, '"') {
        found_closing_quote = true;
        content = current_content[..end_pos].to_string();
    } else {
        // Multi-line case
        content.push_str(current_content);

        for (i, line) in lines[1..].iter().enumerate() {
            if let Some(end_pos) = find_closing_quote(line, '"') {
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(&line[..end_pos]);
                found_closing_quote = true;
                lines_consumed += i + 1;
                break;
            } else {
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(line);
            }
        }
    }

    if !found_closing_quote {
        return Err(ParseError::UnterminatedQuote { line: start_line });
    }

    Ok((process_escape_sequences(&content)?, lines_consumed))
}

fn parse_multiline_single_quoted(
    lines: &[&str],
    start_line: usize,
) -> Result<(String, usize), ParseError> {
    let first_line = lines[0];
    let value_part = &first_line[first_line.find('=').unwrap() + 1..];

    if !value_part.trim().starts_with('\'') {
        return Err(ParseError::InvalidFormat {
            line: start_line,
            content: first_line.to_string(),
        });
    }

    let mut content = String::new();
    let mut lines_consumed = 1;
    let mut found_closing_quote = false;

    // Start from the opening quote
    let start_quote_pos = value_part.find('\'').unwrap();
    let current_content = &value_part[start_quote_pos + 1..];

    // Check if the quote is closed on the same line
    if let Some(end_pos) = current_content.find('\'') {
        found_closing_quote = true;
        content = current_content[..end_pos].to_string();
    } else {
        // Multi-line case - single quotes preserve everything literally
        content.push_str(current_content);

        for (i, line) in lines[1..].iter().enumerate() {
            if let Some(end_pos) = line.find('\'') {
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(&line[..end_pos]);
                found_closing_quote = true;
                lines_consumed += i + 1;
                break;
            } else {
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(line);
            }
        }
    }

    if !found_closing_quote {
        return Err(ParseError::UnterminatedQuote { line: start_line });
    }

    // Single quotes preserve literal values (no escape processing)
    Ok((content, lines_consumed))
}

fn find_closing_quote(content: &str, quote_char: char) -> Option<usize> {
    let mut escaped = false;

    for (i, ch) in content.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if ch == '\\' && quote_char == '"' {
            escaped = true;
        } else if ch == quote_char {
            return Some(i);
        }
    }

    None
}

fn process_escape_sequences(value: &str) -> Result<String, ParseError> {
    let mut result = String::new();
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some('\'') => result.push('\''),
                Some(other) => {
                    // For unknown escape sequences, preserve the backslash and character
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'), // Trailing backslash
            }
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

fn expand_variables(value: &str, variables: &IndexMap<String, String>) -> String {
    let mut result = value.to_string();

    // Simple variable expansion for ${VAR} pattern
    while let Some(start) = result.find("${") {
        if let Some(end) = result[start..].find('}') {
            let var_name = &result[start + 2..start + end];
            let replacement = variables.get(var_name).map(|v| v.as_str()).unwrap_or("");

            result.replace_range(start..start + end + 1, replacement);
        } else {
            break; // No closing brace found
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_equals_position() {
        assert_eq!(find_equals_position("KEY=value"), Some(3));
        assert_eq!(find_equals_position("KEY=\"value=with=equals\""), Some(3));
        assert_eq!(find_equals_position("KEY='value=with=equals'"), Some(3));
        assert_eq!(find_equals_position("NO_EQUALS"), None);
    }

    #[test]
    fn test_is_valid_key() {
        assert!(is_valid_key("VALID_KEY"));
        assert!(is_valid_key("KEY123"));
        assert!(is_valid_key("_PRIVATE"));
        assert!(!is_valid_key("INVALID-KEY"));
        assert!(!is_valid_key("INVALID.KEY"));
        assert!(!is_valid_key(""));
    }

    #[test]
    fn test_process_escape_sequences() {
        assert_eq!(
            process_escape_sequences("line1\\nline2").unwrap(),
            "line1\nline2"
        );
        assert_eq!(process_escape_sequences("tab\\there").unwrap(), "tab\there");
        assert_eq!(
            process_escape_sequences("quote\\\"here").unwrap(),
            "quote\"here"
        );
        assert_eq!(
            process_escape_sequences("backslash\\\\here").unwrap(),
            "backslash\\here"
        );
    }

    #[test]
    fn test_expand_variables() {
        let mut vars = IndexMap::new();
        vars.insert("BASE".to_string(), "https://api.example.com".to_string());

        assert_eq!(
            expand_variables("${BASE}/v1", &vars),
            "https://api.example.com/v1"
        );
        assert_eq!(expand_variables("${UNDEFINED}/v1", &vars), "/v1");
    }
}
