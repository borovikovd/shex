//! Integration tests for parser + interpreter pipeline
//! Tests AST execution and variable resolution

use shex_ast::ShexError;
use shex_interpreter::Interpreter;
use shex_parser::Parser;

#[test]
fn test_parser_interpreter_simple_execution() {
    let parser = Parser::new("echo hello").unwrap();
    let program = parser.parse().unwrap();

    let mut interpreter = Interpreter::new();
    let result = interpreter.execute(program).unwrap();

    assert_eq!(result.code, 0);
    assert_eq!(result.stdout, "hello\n");
}

#[test]
fn test_parser_interpreter_logical_operators() {
    let parser = Parser::new("true && echo success").unwrap();
    let program = parser.parse().unwrap();

    let mut interpreter = Interpreter::new();
    let result = interpreter.execute(program).unwrap();

    assert_eq!(result.code, 0);
    assert_eq!(result.stdout, "success\n");
}

#[test]
fn test_parser_interpreter_variable_assignment() {
    let parser = Parser::new("name=world echo hello $name").unwrap();
    let program = parser.parse().unwrap();

    let mut interpreter = Interpreter::new();
    let result = interpreter.execute(program).unwrap();

    assert_eq!(result.code, 0);
    assert_eq!(result.stdout, "hello world\n");
}

#[test]
fn test_parser_interpreter_parameter_expansion() {
    let parser = Parser::new("echo ${undefined:-fallback}").unwrap();
    let program = parser.parse().unwrap();

    let mut interpreter = Interpreter::new();
    let result = interpreter.execute(program).unwrap();

    assert_eq!(result.code, 0);
    assert_eq!(result.stdout, "fallback\n");
}

#[test]
fn test_error_propagation_undefined_variable() {
    let parser = Parser::new("echo $undefined_variable").unwrap();
    let program = parser.parse().unwrap();

    let mut interpreter = Interpreter::new();
    let result = interpreter.execute(program);

    assert!(result.is_err());
    match result.unwrap_err() {
        ShexError::UndefinedVariable { var, .. } => {
            assert_eq!(var, "undefined_variable");
        }
        _ => panic!("Expected UndefinedVariable error"),
    }
}

#[test]
fn test_command_failure_propagation() {
    let parser = Parser::new("false").unwrap();
    let program = parser.parse().unwrap();

    let mut interpreter = Interpreter::new();
    let result = interpreter.execute(program).unwrap();

    assert_eq!(result.code, 1);
}

#[test]
fn test_complex_command_chain() {
    let parser = Parser::new("echo first; echo second && echo third").unwrap();
    let program = parser.parse().unwrap();

    let mut interpreter = Interpreter::new();
    let result = interpreter.execute(program).unwrap();

    assert_eq!(result.code, 0);
    // Should return the last successful command's output
    assert_eq!(result.stdout, "third\n");
}
