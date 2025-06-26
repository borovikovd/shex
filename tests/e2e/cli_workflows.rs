//! E2E tests for complete CLI workflows
//! Tests the entire application through the command-line interface

use std::process::Command;

const CLI_BINARY: &str = "target/debug/shex-cli";

fn run_command(args: &[&str]) -> std::process::Output {
    Command::new(CLI_BINARY)
        .args(args)
        .output()
        .unwrap_or_else(|_| panic!("Failed to execute {}", CLI_BINARY))
}

fn run_command_string(command: &str) -> std::process::Output {
    run_command(&["-c", command])
}

#[test]
fn test_basic_echo() {
    let output = run_command_string("echo hello");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello");
}

#[test]
fn test_command_not_found() {
    let output = run_command_string("nonexistent_command_12345");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("nonexistent_command_12345"));
    assert!(stderr.contains("ERR_COMMAND_NOT_FOUND"));
}

#[test]
fn test_variable_assignment_and_expansion() {
    let output = run_command_string("name=world && echo hello $name");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello world"
    );
}

#[test]
fn test_logical_and_operator() {
    let output = run_command_string("true && echo success");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "success");
}

#[test]
fn test_logical_or_operator() {
    let output = run_command_string("false || echo fallback");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "fallback");
}

#[test]
fn test_sequence_operator() {
    let output = run_command_string("echo first; echo second");

    assert!(output.status.success());
    // Only last command output is returned in our current implementation
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "second");
}

#[test]
fn test_parameter_expansion_with_default() {
    let output = run_command_string("echo ${unset_var:-default_value}");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "default_value"
    );
}

#[test]
fn test_complex_command_combination() {
    let output = run_command_string("true && echo success || echo failure");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "success");
}

#[test]
fn test_undefined_variable_error() {
    let output = run_command_string("echo $undefined_variable");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("undefined_variable"));
    assert!(stderr.contains("ERR_UNDEF_VAR"));
}

#[test]
fn test_exit_code_success() {
    let output = run_command_string("true");
    assert!(output.status.success());
}

#[test]
fn test_exit_code_failure() {
    let output = run_command_string("false");
    assert!(!output.status.success());
}
