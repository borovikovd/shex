//! E2E tests for script file execution
//! Tests running Shex scripts from files

use std::fs;
use std::process::Command;
use tempfile::NamedTempFile;

const CLI_BINARY: &str = "target/debug/shex-cli";

#[test]
fn test_script_file_execution() {
    let temp_file = NamedTempFile::new().unwrap();
    fs::write(&temp_file, "echo hello from script").unwrap();

    let output = Command::new(CLI_BINARY)
        .arg(temp_file.path().to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello from script"
    );
}

#[test]
fn test_script_with_shebang() {
    let temp_file = NamedTempFile::new().unwrap();
    // Phase 0: Single command only (no multi-line support yet)
    fs::write(&temp_file, "echo shebang works").unwrap();

    let output = Command::new(CLI_BINARY)
        .arg(temp_file.path().to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "shebang works"
    );
}

#[test]
fn test_script_with_variables() {
    let temp_file = NamedTempFile::new().unwrap();
    // Phase 0: Single command with variable assignment
    fs::write(&temp_file, "name=script echo hello from $name").unwrap();

    let output = Command::new(CLI_BINARY)
        .arg(temp_file.path().to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello from script"));
}

#[test]
fn test_script_with_logic() {
    let temp_file = NamedTempFile::new().unwrap();
    // Phase 0: Single logical command
    fs::write(
        &temp_file,
        r#"true && echo "logic works" || echo "logic failed""#,
    )
    .unwrap();

    let output = Command::new(CLI_BINARY)
        .arg(temp_file.path().to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "logic works"
    );
}

#[test]
fn test_script_file_not_found() {
    let output = Command::new(CLI_BINARY)
        .arg("nonexistent_script.sh")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No such file") || stderr.contains("not found"));
}

#[test]
fn test_script_syntax_error() {
    let temp_file = NamedTempFile::new().unwrap();
    fs::write(&temp_file, "echo $invalid_variable_syntax!").unwrap();

    let output = Command::new(CLI_BINARY)
        .arg(temp_file.path().to_str().unwrap())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Shex:"));
}

#[test]
fn test_empty_script() {
    let temp_file = NamedTempFile::new().unwrap();
    fs::write(&temp_file, "").unwrap();

    let output = Command::new(CLI_BINARY)
        .arg(temp_file.path().to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "");
}
