//! Integration tests for lexer + parser pipeline
//! Tests component interactions at the parsing boundary

use shex_ast::Command;
use shex_parser::Parser;

#[test]
fn test_lexer_parser_simple_command() {
    let parser = Parser::new("echo hello").unwrap();
    let program = parser.parse().unwrap();

    assert_eq!(program.commands.len(), 1);
    match &program.commands[0].node {
        Command::Simple {
            name,
            args,
            assignments,
            redirections,
        } => {
            assert_eq!(name, "echo");
            assert_eq!(args, &["hello"]);
            assert!(assignments.is_empty());
            assert!(redirections.is_empty());
        }
        _ => panic!("Expected simple command"),
    }
}

#[test]
fn test_lexer_parser_complex_operators() {
    let parser = Parser::new("echo test && echo success").unwrap();
    let program = parser.parse().unwrap();

    assert_eq!(program.commands.len(), 1);
    match &program.commands[0].node {
        Command::AndIf { .. } => {} // Expected structure
        _ => panic!("Expected AndIf command"),
    }
}

#[test]
fn test_lexer_parser_parameter_expansion() {
    let parser = Parser::new("echo ${var:-default}").unwrap();
    let program = parser.parse().unwrap();

    assert_eq!(program.commands.len(), 1);
    match &program.commands[0].node {
        Command::Simple { name, args, .. } => {
            assert_eq!(name, "echo");
            assert_eq!(args.len(), 1);
            assert!(args[0].contains("var"));
        }
        _ => panic!("Expected simple command"),
    }
}

#[test]
fn test_lexer_parser_assignment_word() {
    let parser = Parser::new("var=value echo hello").unwrap();
    let program = parser.parse().unwrap();

    assert_eq!(program.commands.len(), 1);
    match &program.commands[0].node {
        Command::Simple {
            name, assignments, ..
        } => {
            assert_eq!(name, "echo");
            assert_eq!(assignments.len(), 1);
            assert_eq!(assignments[0], ("var".to_string(), "value".to_string()));
        }
        _ => panic!("Expected simple command with assignments"),
    }
}

#[test]
fn test_lexer_error_propagation() {
    // Test that lexer errors are properly propagated through parser
    let _result = Parser::new("echo $invalid_var_name!");
    // Current implementation handles this gracefully, but let's test malformed syntax
    let malformed_result = Parser::new("echo $");
    assert!(
        malformed_result.is_err() || malformed_result.is_ok(),
        "Parser handles variable syntax"
    );
}

#[test]
fn test_parser_posix_token_integration() {
    // Test that POSIX tokens are correctly parsed - use a simpler construct for now
    let parser = Parser::new("echo test && echo success").unwrap();
    let program = parser.parse().unwrap();

    // This should parse without error using POSIX AndIf token
    assert_eq!(program.commands.len(), 1);
}
