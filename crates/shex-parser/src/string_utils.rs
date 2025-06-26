//! String processing utilities for Shex parser
//!
//! Centralized handling of quote removal, assignment parsing, and other
//! string manipulations needed by the parser and future parameter expansion.

use crate::variable_resolver::{ExpansionMode, ExpansionRequest};
use shex_lexer::{SpannedToken, Token};

/// Remove quotes from a string token while preserving the content
///
/// Handles both single and double quotes according to POSIX rules
pub fn remove_quotes(text: &str) -> String {
    if text.len() < 2 {
        return text.to_string();
    }

    let first_char = text.chars().next().unwrap();
    let last_char = text.chars().last().unwrap();

    if (first_char == '"' && last_char == '"') || (first_char == '\'' && last_char == '\'') {
        // Remove surrounding quotes
        text[1..text.len() - 1].to_string()
    } else {
        text.to_string()
    }
}

/// Convert a token to its string representation
///
/// Handles quote removal for string tokens and preserves other token text
/// Parameter expansion tokens are returned as-is for later processing
pub fn token_to_string(token: &SpannedToken) -> String {
    match token.token {
        Token::String => remove_quotes(&token.text),
        Token::SimpleParameterExpansion | Token::ParameterExpansion => {
            // Return parameter expansion as-is for later resolution
            token.text.clone()
        }
        _ => token.text.clone(),
    }
}

/// Parse an assignment word into name and value components
///
/// Returns None if the text doesn't contain a valid assignment pattern
pub fn parse_assignment(text: &str) -> Option<(String, String)> {
    if let Some(eq_pos) = text.find('=') {
        let name = text[..eq_pos].to_string();
        let value = text[eq_pos + 1..].to_string();

        // Validate variable name follows POSIX rules
        if is_valid_variable_name(&name) {
            Some((name, value))
        } else {
            None
        }
    } else {
        None
    }
}

/// Check if a string is a valid POSIX variable name
///
/// Variable names must start with letter or underscore, followed by
/// letters, digits, or underscores
fn is_valid_variable_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap();

    // First character must be letter or underscore
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }

    // Remaining characters must be alphanumeric or underscore
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Extract assignment tokens from a list and convert to (name, value) pairs
pub fn extract_assignments(tokens: &[SpannedToken]) -> Vec<(String, String)> {
    let mut assignments = Vec::new();

    for token in tokens {
        if token.token == Token::AssignmentWord {
            if let Some((name, value)) = parse_assignment(&token.text) {
                assignments.push((name, value));
            }
        }
    }

    assignments
}

/// Extract non-assignment tokens from a list and convert to strings
pub fn extract_arguments(tokens: &[SpannedToken]) -> Vec<String> {
    tokens
        .iter()
        .filter(|token| token.token != Token::AssignmentWord)
        .map(token_to_string)
        .collect()
}

/// Combine prefix and suffix tokens into a single argument list
///
/// Filters out assignment words from prefix, includes all suffix tokens
pub fn combine_args(prefix: &[SpannedToken], suffix: &[SpannedToken]) -> Vec<String> {
    let mut args = extract_arguments(prefix);
    args.extend(extract_arguments(suffix));
    args
}

/// Parse a simple parameter expansion ($var) into an expansion request
///
/// Returns None if the text doesn't match the expected format
pub fn parse_simple_parameter_expansion(text: &str) -> Option<ExpansionRequest> {
    if text.starts_with('$') && text.len() > 1 {
        let var_name = &text[1..];
        if is_valid_variable_name(var_name) {
            Some(ExpansionRequest::simple(var_name.to_string()))
        } else {
            None
        }
    } else {
        None
    }
}

/// Parse a braced parameter expansion (${var}, ${var:-default}, etc.) into an expansion request
///
/// Supports all POSIX parameter expansion modes
pub fn parse_parameter_expansion(text: &str) -> Option<ExpansionRequest> {
    if !text.starts_with("${") || !text.ends_with('}') {
        return None;
    }

    let inner = &text[2..text.len() - 1];

    // Check for different expansion modes
    if let Some(colon_pos) = inner.find(':') {
        let var_name = &inner[..colon_pos];
        let rest = &inner[colon_pos + 1..];

        if !is_valid_variable_name(var_name) {
            return None;
        }

        match rest.chars().next() {
            Some('-') => {
                // ${var:-default} - use default if unset or null
                let default_value = if rest.len() > 1 { &rest[1..] } else { "" };
                Some(ExpansionRequest {
                    variable_name: var_name.to_string(),
                    mode: ExpansionMode::DefaultValue,
                    parameter: Some(default_value.to_string()),
                    check_unset: true,
                })
            }
            Some('=') => {
                // ${var:=default} - assign default if unset or null
                let default_value = if rest.len() > 1 { &rest[1..] } else { "" };
                Some(ExpansionRequest {
                    variable_name: var_name.to_string(),
                    mode: ExpansionMode::AssignDefault,
                    parameter: Some(default_value.to_string()),
                    check_unset: true,
                })
            }
            Some('?') => {
                // ${var:?message} - error if unset or null
                let message = if rest.len() > 1 {
                    Some(rest[1..].to_string())
                } else {
                    None
                };
                Some(ExpansionRequest {
                    variable_name: var_name.to_string(),
                    mode: ExpansionMode::ErrorIfUnset,
                    parameter: message,
                    check_unset: true,
                })
            }
            Some('+') => {
                // ${var:+alternative} - use alternative if set and not null
                let alternative = if rest.len() > 1 { &rest[1..] } else { "" };
                Some(ExpansionRequest {
                    variable_name: var_name.to_string(),
                    mode: ExpansionMode::AlternativeValue,
                    parameter: Some(alternative.to_string()),
                    check_unset: true,
                })
            }
            _ => None,
        }
    } else if let Some(operator_pos) = inner.find_any(&['-', '=', '?', '+']) {
        // Non-colon versions (test only for unset, not null)
        let var_name = &inner[..operator_pos];
        let operator = inner.chars().nth(operator_pos).unwrap();
        let rest = &inner[operator_pos + 1..];

        if !is_valid_variable_name(var_name) {
            return None;
        }

        match operator {
            '-' => {
                // ${var-default} - use default if unset
                Some(ExpansionRequest {
                    variable_name: var_name.to_string(),
                    mode: ExpansionMode::DefaultValue,
                    parameter: Some(rest.to_string()),
                    check_unset: false,
                })
            }
            '=' => {
                // ${var=default} - assign default if unset
                Some(ExpansionRequest {
                    variable_name: var_name.to_string(),
                    mode: ExpansionMode::AssignDefault,
                    parameter: Some(rest.to_string()),
                    check_unset: false,
                })
            }
            '?' => {
                // ${var?message} - error if unset
                let message = if rest.is_empty() {
                    None
                } else {
                    Some(rest.to_string())
                };
                Some(ExpansionRequest {
                    variable_name: var_name.to_string(),
                    mode: ExpansionMode::ErrorIfUnset,
                    parameter: message,
                    check_unset: false,
                })
            }
            '+' => {
                // ${var+alternative} - use alternative if set
                Some(ExpansionRequest {
                    variable_name: var_name.to_string(),
                    mode: ExpansionMode::AlternativeValue,
                    parameter: Some(rest.to_string()),
                    check_unset: false,
                })
            }
            _ => None,
        }
    } else {
        // Simple ${var} expansion
        if is_valid_variable_name(inner) {
            Some(ExpansionRequest::simple(inner.to_string()))
        } else {
            None
        }
    }
}

/// Helper trait to find any of multiple characters
trait FindAny {
    fn find_any(&self, chars: &[char]) -> Option<usize>;
}

impl FindAny for str {
    fn find_any(&self, chars: &[char]) -> Option<usize> {
        self.char_indices()
            .find(|(_, c)| chars.contains(c))
            .map(|(i, _)| i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shex_ast::Span;

    fn make_token(token: Token, text: &str) -> SpannedToken {
        SpannedToken {
            token,
            span: Span::dummy(),
            text: text.to_string(),
        }
    }

    #[test]
    fn test_remove_quotes() {
        assert_eq!(remove_quotes("\"hello world\""), "hello world");
        assert_eq!(remove_quotes("'hello world'"), "hello world");
        assert_eq!(remove_quotes("hello"), "hello");
        assert_eq!(remove_quotes("\"hello"), "\"hello");
        assert_eq!(remove_quotes(""), "");
    }

    #[test]
    fn test_token_to_string() {
        let string_token = make_token(Token::String, "\"hello world\"");
        assert_eq!(token_to_string(&string_token), "hello world");

        let word_token = make_token(Token::Word, "hello");
        assert_eq!(token_to_string(&word_token), "hello");
    }

    #[test]
    fn test_parse_assignment() {
        assert_eq!(
            parse_assignment("var=value"),
            Some(("var".to_string(), "value".to_string()))
        );
        assert_eq!(
            parse_assignment("_var=value"),
            Some(("_var".to_string(), "value".to_string()))
        );
        assert_eq!(
            parse_assignment("var123=value"),
            Some(("var123".to_string(), "value".to_string()))
        );
        assert_eq!(
            parse_assignment("PATH=/usr/bin"),
            Some(("PATH".to_string(), "/usr/bin".to_string()))
        );
        assert_eq!(
            parse_assignment("empty="),
            Some(("empty".to_string(), String::new()))
        );

        // Invalid cases
        assert_eq!(parse_assignment("123var=value"), None);
        assert_eq!(parse_assignment("-var=value"), None);
        assert_eq!(parse_assignment("var"), None);
        assert_eq!(parse_assignment("=value"), None);
    }

    #[test]
    fn test_is_valid_variable_name() {
        assert!(is_valid_variable_name("var"));
        assert!(is_valid_variable_name("_var"));
        assert!(is_valid_variable_name("var123"));
        assert!(is_valid_variable_name("PATH"));
        assert!(is_valid_variable_name("_"));

        assert!(!is_valid_variable_name("123var"));
        assert!(!is_valid_variable_name("-var"));
        assert!(!is_valid_variable_name("var-name"));
        assert!(!is_valid_variable_name(""));
        assert!(!is_valid_variable_name("var.name"));
    }

    #[test]
    fn test_extract_assignments() {
        let tokens = vec![
            make_token(Token::AssignmentWord, "var1=value1"),
            make_token(Token::Word, "echo"),
            make_token(Token::AssignmentWord, "var2=value2"),
            make_token(Token::String, "\"hello\""),
        ];

        let assignments = extract_assignments(&tokens);
        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments[0], ("var1".to_string(), "value1".to_string()));
        assert_eq!(assignments[1], ("var2".to_string(), "value2".to_string()));
    }

    #[test]
    fn test_extract_arguments() {
        let tokens = vec![
            make_token(Token::AssignmentWord, "var=value"),
            make_token(Token::Word, "echo"),
            make_token(Token::String, "\"hello world\""),
            make_token(Token::Word, "test"),
        ];

        let args = extract_arguments(&tokens);
        assert_eq!(args.len(), 3);
        assert_eq!(args[0], "echo");
        assert_eq!(args[1], "hello world");
        assert_eq!(args[2], "test");
    }

    #[test]
    fn test_combine_args() {
        let prefix = vec![
            make_token(Token::AssignmentWord, "var=value"),
            make_token(Token::Word, "arg1"),
        ];
        let suffix = vec![
            make_token(Token::Word, "arg2"),
            make_token(Token::String, "\"arg 3\""),
        ];

        let combined = combine_args(&prefix, &suffix);
        assert_eq!(combined.len(), 3);
        assert_eq!(combined[0], "arg1");
        assert_eq!(combined[1], "arg2");
        assert_eq!(combined[2], "arg 3");
    }

    #[test]
    fn test_parse_simple_parameter_expansion() {
        // Valid simple expansions
        let request = parse_simple_parameter_expansion("$var").unwrap();
        assert_eq!(request.variable_name, "var");
        assert_eq!(request.mode, ExpansionMode::Normal);

        // Invalid cases
        assert!(parse_simple_parameter_expansion("$123").is_none());
        assert!(parse_simple_parameter_expansion("$").is_none());
    }

    #[test]
    fn test_parse_parameter_expansion_default_value() {
        // With colon (check unset and null)
        let request = parse_parameter_expansion("${var:-default}").unwrap();
        assert_eq!(request.variable_name, "var");
        assert_eq!(request.mode, ExpansionMode::DefaultValue);
        assert_eq!(request.parameter, Some("default".to_string()));
        assert!(request.check_unset);

        // Without colon (check only unset)
        let request = parse_parameter_expansion("${var-default}").unwrap();
        assert!(!request.check_unset);
    }

    #[test]
    fn test_parse_parameter_expansion_assign_default() {
        let request = parse_parameter_expansion("${var:=default}").unwrap();
        assert_eq!(request.variable_name, "var");
        assert_eq!(request.mode, ExpansionMode::AssignDefault);
        assert_eq!(request.parameter, Some("default".to_string()));
        assert!(request.check_unset);
    }

    #[test]
    fn test_parse_parameter_expansion_error_if_unset() {
        let request = parse_parameter_expansion("${var:?message}").unwrap();
        assert_eq!(request.variable_name, "var");
        assert_eq!(request.mode, ExpansionMode::ErrorIfUnset);
        assert_eq!(request.parameter, Some("message".to_string()));
        assert!(request.check_unset);
    }

    #[test]
    fn test_parse_parameter_expansion_alternative_value() {
        let request = parse_parameter_expansion("${var:+alternative}").unwrap();
        assert_eq!(request.variable_name, "var");
        assert_eq!(request.mode, ExpansionMode::AlternativeValue);
        assert_eq!(request.parameter, Some("alternative".to_string()));
        assert!(request.check_unset);
    }

    #[test]
    fn test_find_any() {
        assert_eq!("hello-world".find_any(&['-', '+']), Some(5));
        assert_eq!("hello+world".find_any(&['-', '+']), Some(5));
        assert_eq!("hello=world".find_any(&['-', '+', '=']), Some(5));
        assert_eq!("helloworld".find_any(&['-', '+']), None);
    }
}
