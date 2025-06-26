//! Shex CLI - Phase 0
//!
//! Command-line interface for the Shex shell interpreter.

use clap::{Arg, Command};
use shex_interpreter::Interpreter;
use shex_parser::Parser;
use std::process;

fn main() {
    let matches = Command::new("shex")
        .version("0.1.0")
        .about("Shex shell interpreter")
        .arg(
            Arg::new("command")
                .short('c')
                .long("command")
                .value_name("STRING")
                .help("Execute command string")
                .num_args(1),
        )
        .arg(
            Arg::new("file")
                .value_name("FILE")
                .help("Script file to execute")
                .index(1),
        )
        .get_matches();

    let result = matches.get_one::<String>("command").map_or_else(
        || {
            matches.get_one::<String>("file").map_or_else(
                || {
                    // TODO: Interactive mode for Phase 1
                    eprintln!("Interactive mode not implemented yet");
                    process::exit(1);
                },
                // Execute script file
                |file_path| execute_file(file_path),
            )
        },
        // Execute command string
        |command_str| execute_string(command_str),
    );

    match result {
        Ok(exit_code) => process::exit(exit_code),
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    }
}

fn execute_string(command_str: &str) -> Result<i32, anyhow::Error> {
    let parser = Parser::new(command_str)?;
    let program = parser.parse()?;

    let mut interpreter = Interpreter::new();
    let status = interpreter.execute(program)?;

    // Print output
    if !status.stdout.is_empty() {
        print!("{}", status.stdout);
    }
    if !status.stderr.is_empty() {
        eprint!("{}", status.stderr);
    }

    Ok(status.code)
}

fn execute_file(file_path: &str) -> Result<i32, anyhow::Error> {
    let content = std::fs::read_to_string(file_path)?;
    execute_string(&content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_execute_string_success() {
        let result = execute_string("echo hello");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_execute_string_command_failure() {
        let result = execute_string("false");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_execute_string_syntax_error() {
        let result = execute_string("$invalid_expansion");
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_string_complex_command() {
        let result = execute_string("echo hello && echo world");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_execute_file_success() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, "echo test").unwrap();

        let result = execute_file(temp_file.path().to_str().unwrap());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_execute_file_not_found() {
        let result = execute_file("nonexistent_file.sh");
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_file_with_syntax_error() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, "$undefined_var").unwrap();

        let result = execute_file(temp_file.path().to_str().unwrap());
        assert!(result.is_err());
    }
}
