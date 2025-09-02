use stand::environment::parser::{parse_env_content, ParseError};

#[test]
fn test_parse_basic_key_value() {
    let content = "KEY=value";
    let result = parse_env_content(content).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result.get("KEY"), Some(&"value".to_string()));
}

#[test]
fn test_parse_quoted_values() {
    let content = r#"
SINGLE_QUOTED='value with spaces'
DOUBLE_QUOTED="another value with spaces"
MIXED_QUOTES='value with "inner" quotes'
    "#;

    let result = parse_env_content(content).unwrap();

    assert_eq!(
        result.get("SINGLE_QUOTED"),
        Some(&"value with spaces".to_string())
    );
    assert_eq!(
        result.get("DOUBLE_QUOTED"),
        Some(&"another value with spaces".to_string())
    );
    assert_eq!(
        result.get("MIXED_QUOTES"),
        Some(&r#"value with "inner" quotes"#.to_string())
    );
}

#[test]
fn test_parse_comments_and_empty_lines() {
    let content = r#"
# This is a comment
KEY1=value1

# Another comment
KEY2=value2
    
KEY3=value3 # Inline comment
    "#;

    let result = parse_env_content(content).unwrap();

    assert_eq!(result.len(), 3);
    assert_eq!(result.get("KEY1"), Some(&"value1".to_string()));
    assert_eq!(result.get("KEY2"), Some(&"value2".to_string()));
    assert_eq!(result.get("KEY3"), Some(&"value3".to_string()));
}

#[test]
fn test_parse_escape_sequences() {
    let content = r#"
ESCAPED_NEWLINE="line1\nline2"
ESCAPED_TAB="value\tvalue"
ESCAPED_QUOTE="value with \"quote\""
    "#;

    let result = parse_env_content(content).unwrap();

    assert_eq!(
        result.get("ESCAPED_NEWLINE"),
        Some(&"line1\nline2".to_string())
    );
    assert_eq!(result.get("ESCAPED_TAB"), Some(&"value\tvalue".to_string()));
    assert_eq!(
        result.get("ESCAPED_QUOTE"),
        Some(&r#"value with "quote""#.to_string())
    );
}

#[test]
fn test_parse_multiline_values() {
    let content = r#"
MULTILINE="line1
line2
line3"
    "#;

    let result = parse_env_content(content).unwrap();

    assert_eq!(
        result.get("MULTILINE"),
        Some(&"line1\nline2\nline3".to_string())
    );
}

#[test]
fn test_parse_variable_expansion() {
    let content = r#"
BASE_URL=https://api.example.com
API_ENDPOINT=${BASE_URL}/v1
NESTED_VAR=${API_ENDPOINT}/users
    "#;

    let result = parse_env_content(content).unwrap();

    assert_eq!(
        result.get("BASE_URL"),
        Some(&"https://api.example.com".to_string())
    );
    assert_eq!(
        result.get("API_ENDPOINT"),
        Some(&"https://api.example.com/v1".to_string())
    );
    assert_eq!(
        result.get("NESTED_VAR"),
        Some(&"https://api.example.com/v1/users".to_string())
    );
}

#[test]
fn test_parse_empty_values() {
    let content = r#"
EMPTY_VALUE=
QUOTED_EMPTY=""
SPACED_EMPTY= 
    "#;

    let result = parse_env_content(content).unwrap();

    assert_eq!(result.get("EMPTY_VALUE"), Some(&"".to_string()));
    assert_eq!(result.get("QUOTED_EMPTY"), Some(&"".to_string()));
    assert_eq!(result.get("SPACED_EMPTY"), Some(&" ".to_string()));
}

#[test]
fn test_parse_special_characters() {
    let content = r#"
SPECIAL_CHARS="!@#$%^&*()_+-={}[]|;:,.<>?"
UNICODE_VALUE="こんにちは世界🌍"
    "#;

    let result = parse_env_content(content).unwrap();

    assert_eq!(
        result.get("SPECIAL_CHARS"),
        Some(&"!@#$%^&*()_+-={}[]|;:,.<>?".to_string())
    );
    assert_eq!(
        result.get("UNICODE_VALUE"),
        Some(&"こんにちは世界🌍".to_string())
    );
}

#[test]
fn test_parse_error_invalid_format() {
    let content = "INVALID LINE WITHOUT EQUALS";
    let result = parse_env_content(content);

    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::InvalidFormat { line, .. } => assert_eq!(line, 1),
        _ => panic!("Expected InvalidFormat error"),
    }
}

#[test]
fn test_parse_error_unterminated_quote() {
    let content = r#"KEY="unterminated quote"#;
    let result = parse_env_content(content);

    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::UnterminatedQuote { line } => assert_eq!(line, 1),
        _ => panic!("Expected UnterminatedQuote error"),
    }
}

#[test]
fn test_parse_preserves_order() {
    let content = r#"
THIRD=3
FIRST=1
SECOND=2
    "#;

    let result = parse_env_content(content).unwrap();

    let keys: Vec<_> = result.keys().collect();
    assert_eq!(keys, vec!["THIRD", "FIRST", "SECOND"]);
}

#[test]
fn test_parse_later_value_overrides_earlier() {
    let content = r#"
KEY=first_value
KEY=second_value
KEY=final_value
    "#;

    let result = parse_env_content(content).unwrap();

    assert_eq!(result.get("KEY"), Some(&"final_value".to_string()));
}
