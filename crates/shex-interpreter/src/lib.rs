//! Shex interpreter - Phase 0
//!
//! Simple command execution for basic shell functionality.

use shex_ast::{Command, Program, ShexError, SourceMap, Spanned, Redirection, RedirectionKind, CaseArm};
use shex_parser::string_utils::{parse_parameter_expansion, parse_simple_parameter_expansion};
use shex_parser::variable_resolver::{ResolutionResult, VariableContext, resolve_expansion};
use std::fs::File;
use std::process::{Command as StdCommand, Stdio};

pub struct Interpreter {
    variable_context: VariableContext,
    exit_code: i32,
}

#[derive(Debug)]
pub struct ExitStatus {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Interpreter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            variable_context: VariableContext::new(),
            exit_code: 0,
        }
    }

    /// Execute a Shex program
    ///
    /// # Errors
    ///
    /// Returns `ShexError` if command execution fails, command not found, or syntax errors occur
    pub fn execute(&mut self, program: Program) -> Result<ExitStatus, ShexError> {
        let mut last_stdout = String::new();
        let mut last_stderr = String::new();
        let mut last_code = 0;

        for command in program.commands {
            let result = self.execute_command(&command)?;
            last_stdout = result.stdout;
            last_stderr = result.stderr;
            last_code = result.code;

            // For now, stop on first error (errexit behavior)
            if result.code != 0 {
                break;
            }
        }

        self.exit_code = last_code;
        Ok(ExitStatus {
            code: last_code,
            stdout: last_stdout,
            stderr: last_stderr,
        })
    }

    fn execute_command(&mut self, command: &Spanned<Command>) -> Result<ExitStatus, ShexError> {
        match &command.node {
            Command::Simple {
                name,
                args,
                assignments,
                redirections,
            } => self.execute_simple_command(name, args, assignments, redirections, command.span),
            Command::Pipeline { commands, redirections } => self.execute_pipeline(commands, redirections, command.span),
            Command::Assignment { assignments } => {
                self.execute_assignments(assignments);
                Ok(ExitStatus {
                    code: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                })
            }
            Command::AndIf { left, right } => self.execute_and_if(left, right, command.span),
            Command::OrIf { left, right } => self.execute_or_if(left, right, command.span),
            Command::Sequence { commands } => self.execute_sequence(commands, command.span),
            Command::Background { command } => self.execute_background(command, command.span),
            Command::If { condition, then_body, elif_clauses, else_body } => {
                self.execute_if(condition, then_body, elif_clauses, else_body, command.span)
            }
            Command::While { condition, body } => {
                self.execute_while(condition, body, command.span)
            }
            Command::Until { condition, body } => {
                self.execute_until(condition, body, command.span)
            }
            Command::For { variable, words, body } => {
                self.execute_for(variable, words, body, command.span)
            }
            Command::Case { word, arms } => {
                self.execute_case(word, arms, command.span)
            }
            Command::Function { name, body, redirections } => {
                self.execute_function_definition(name, body, redirections, command.span)
            }
            Command::Subshell { commands } => {
                self.execute_subshell(commands, command.span)
            }
            Command::BraceGroup { commands } => {
                self.execute_brace_group(commands, command.span)
            }
        }
    }

    fn execute_simple_command(
        &mut self,
        name: &str,
        args: &[String],
        assignments: &[(String, String)],
        redirections: &[Redirection],
        span: shex_ast::Span,
    ) -> Result<ExitStatus, ShexError> {
        // First, process prefix assignments
        self.execute_assignments(assignments);

        // Then expand parameter expansions in arguments
        let expanded_args = self.expand_arguments(args, span)?;
        // Handle built-in commands
        match name {
            "echo" => {
                let output = expanded_args.join(" ");
                Ok(ExitStatus {
                    code: 0,
                    stdout: output + "\n",
                    stderr: String::new(),
                })
            }
            "true" => Ok(ExitStatus {
                code: 0,
                stdout: String::new(),
                stderr: String::new(),
            }),
            "false" => Ok(ExitStatus {
                code: 1,
                stdout: String::new(),
                stderr: String::new(),
            }),
            _ => {
                // Try to execute external command
                let mut cmd = StdCommand::new(name);
                cmd.args(&expanded_args);
                
                // Apply redirections
                self.apply_redirections(&mut cmd, redirections)?;

                // Default to piped if no redirections specified
                if redirections.is_empty() || !redirections.iter().any(|r| matches!(r.kind, RedirectionKind::Output | RedirectionKind::Append | RedirectionKind::Clobber)) {
                    cmd.stdout(Stdio::piped());
                }
                if redirections.is_empty() || !redirections.iter().any(|r| matches!(r.kind, RedirectionKind::OutputDup) && r.fd == Some(2)) {
                    cmd.stderr(Stdio::piped());
                }

                if let Ok(output) = cmd.output() {
                    Ok(ExitStatus {
                        code: output.status.code().unwrap_or(-1),
                        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    })
                } else {
                    let source_map = SourceMap::new(""); // Dummy for now
                    Err(ShexError::command_not_found(
                        name.to_string(),
                        span,
                        &source_map,
                        "<interpreter>",
                    ))
                }
            }
        }
    }

    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        self.exit_code
    }

    fn execute_assignments(&mut self, assignments: &[(String, String)]) {
        for (name, value) in assignments {
            self.variable_context.set(name.clone(), value.clone());
        }
    }

    /// Expand parameter expansions in command arguments
    ///
    /// Processes arguments containing $var and ${var} expansions
    fn expand_arguments(
        &mut self,
        args: &[String],
        span: shex_ast::Span,
    ) -> Result<Vec<String>, ShexError> {
        let mut expanded_args = Vec::new();

        for arg in args {
            let expanded_arg = self.expand_single_argument(arg, span)?;
            expanded_args.push(expanded_arg);
        }

        Ok(expanded_args)
    }

    /// Expand parameter expansions in a single argument
    ///
    /// Handles both simple ($var) and braced (${var}) parameter expansions
    fn expand_single_argument(
        &mut self,
        arg: &str,
        span: shex_ast::Span,
    ) -> Result<String, ShexError> {
        // Check if this argument is a parameter expansion
        if let Some(request) = parse_simple_parameter_expansion(arg) {
            // Simple parameter expansion: $var
            match resolve_expansion(&mut self.variable_context, &request) {
                ResolutionResult::Resolved(value) => Ok(value),
                ResolutionResult::Unset => {
                    // POSIX behavior: unset variables expand to empty string by default
                    // But with nounset option (implied by Shex safety), this should error
                    let source_map = SourceMap::new(""); // Dummy for now
                    Err(ShexError::undefined_variable(
                        request.variable_name,
                        span,
                        &source_map,
                        "<interpreter>",
                    ))
                }
                ResolutionResult::Error(msg) => {
                    let source_map = SourceMap::new(""); // Dummy for now
                    Err(ShexError::syntax(msg, span, &source_map, "<interpreter>"))
                }
            }
        } else if let Some(request) = parse_parameter_expansion(arg) {
            // Braced parameter expansion: ${var}, ${var:-default}, etc.
            match resolve_expansion(&mut self.variable_context, &request) {
                ResolutionResult::Resolved(value) => Ok(value),
                ResolutionResult::Unset => {
                    // For braced expansions without default, this is an error with nounset
                    let source_map = SourceMap::new(""); // Dummy for now
                    Err(ShexError::undefined_variable(
                        request.variable_name,
                        span,
                        &source_map,
                        "<interpreter>",
                    ))
                }
                ResolutionResult::Error(msg) => {
                    let source_map = SourceMap::new(""); // Dummy for now
                    Err(ShexError::syntax(msg, span, &source_map, "<interpreter>"))
                }
            }
        } else {
            // Not a parameter expansion, return as-is
            Ok(arg.to_string())
        }
    }

    /// Execute a pipeline: cmd1 | cmd2 | cmd3
    fn execute_pipeline(
        &mut self,
        commands: &[Spanned<Command>],
        _redirections: &[Redirection],
        _span: shex_ast::Span,
    ) -> Result<ExitStatus, ShexError> {
        // For now, just execute commands sequentially without actual piping
        // TODO: Implement proper pipeline with stdio chaining
        let mut last_result = ExitStatus {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        };

        for command in commands {
            last_result = self.execute_command(command)?;
            // In a real pipeline, each command's stdout becomes the next command's stdin
            // For now, we'll just continue with the last command's result
        }

        Ok(last_result)
    }

    /// Execute logical AND: cmd1 && cmd2
    fn execute_and_if(
        &mut self,
        left: &Spanned<Command>,
        right: &Spanned<Command>,
        _span: shex_ast::Span,
    ) -> Result<ExitStatus, ShexError> {
        let left_result = self.execute_command(left)?;

        if left_result.code == 0 {
            // Left succeeded, execute right
            self.execute_command(right)
        } else {
            // Left failed, return its result without executing right
            Ok(left_result)
        }
    }

    /// Execute logical OR: cmd1 || cmd2
    fn execute_or_if(
        &mut self,
        left: &Spanned<Command>,
        right: &Spanned<Command>,
        _span: shex_ast::Span,
    ) -> Result<ExitStatus, ShexError> {
        let left_result = self.execute_command(left)?;

        if left_result.code == 0 {
            // Left succeeded, return its result without executing right
            Ok(left_result)
        } else {
            // Left failed, execute right
            self.execute_command(right)
        }
    }

    /// Execute sequence: cmd1; cmd2; cmd3
    fn execute_sequence(
        &mut self,
        commands: &[Spanned<Command>],
        _span: shex_ast::Span,
    ) -> Result<ExitStatus, ShexError> {
        let mut last_result = ExitStatus {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        };

        for command in commands {
            last_result = self.execute_command(command)?;
            // Continue executing regardless of exit status
        }

        Ok(last_result)
    }

    /// Execute background command: cmd &
    fn execute_background(
        &mut self,
        command: &Spanned<Command>,
        _span: shex_ast::Span,
    ) -> Result<ExitStatus, ShexError> {
        // For now, just execute the command normally
        // TODO: Implement proper background execution with job control
        let _result = self.execute_command(command)?;

        // Background commands return immediately with success
        Ok(ExitStatus {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        })
    }

    /// Apply I/O redirections to a command
    fn apply_redirections(&self, cmd: &mut StdCommand, redirections: &[Redirection]) -> Result<(), ShexError> {
        for redirection in redirections {
            match &redirection.kind {
                RedirectionKind::Input => {
                    // < file - redirect stdin from file
                    match File::open(&redirection.target) {
                        Ok(file) => {
                            cmd.stdin(Stdio::from(file));
                        }
                        Err(_) => {
                            let source_map = SourceMap::new("");
                            return Err(ShexError::syntax(
                                format!("Cannot open {} for input", redirection.target),
                                shex_ast::Span::dummy(),
                                &source_map,
                                "<interpreter>",
                            ));
                        }
                    }
                }
                RedirectionKind::Output => {
                    // > file - redirect stdout to file (truncate)
                    match File::create(&redirection.target) {
                        Ok(file) => {
                            cmd.stdout(Stdio::from(file));
                        }
                        Err(_) => {
                            let source_map = SourceMap::new("");
                            return Err(ShexError::syntax(
                                format!("Cannot create {}", redirection.target),
                                shex_ast::Span::dummy(),
                                &source_map,
                                "<interpreter>",
                            ));
                        }
                    }
                }
                RedirectionKind::Append => {
                    // >> file - redirect stdout to file (append)
                    match std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&redirection.target)
                    {
                        Ok(file) => {
                            cmd.stdout(Stdio::from(file));
                        }
                        Err(_) => {
                            let source_map = SourceMap::new("");
                            return Err(ShexError::syntax(
                                format!("Cannot open {} for append", redirection.target),
                                shex_ast::Span::dummy(),
                                &source_map,
                                "<interpreter>",
                            ));
                        }
                    }
                }
                // TODO: Implement other redirection types
                _ => {
                    // For now, ignore unsupported redirection types
                }
            }
        }
        Ok(())
    }

    /// Execute if/then/else/fi control structure
    fn execute_if(
        &mut self,
        condition: &Spanned<Command>,
        then_body: &[Spanned<Command>],
        elif_clauses: &[(Spanned<Command>, Vec<Spanned<Command>>)],
        else_body: &Option<Vec<Spanned<Command>>>,
        _span: shex_ast::Span,
    ) -> Result<ExitStatus, ShexError> {
        // Execute condition
        let condition_result = self.execute_command(condition)?;
        
        if condition_result.code == 0 {
            // Condition succeeded, execute then body
            self.execute_command_list(then_body)
        } else {
            // Check elif clauses
            for (elif_condition, elif_body) in elif_clauses {
                let elif_result = self.execute_command(elif_condition)?;
                if elif_result.code == 0 {
                    return self.execute_command_list(elif_body);
                }
            }
            
            // Execute else body if present
            if let Some(else_commands) = else_body {
                self.execute_command_list(else_commands)
            } else {
                // No else clause, return success
                Ok(ExitStatus {
                    code: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                })
            }
        }
    }

    /// Execute while/do/done loop
    fn execute_while(
        &mut self,
        condition: &Spanned<Command>,
        body: &[Spanned<Command>],
        _span: shex_ast::Span,
    ) -> Result<ExitStatus, ShexError> {
        let mut last_result = ExitStatus {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        };

        loop {
            // Check condition
            let condition_result = self.execute_command(condition)?;
            if condition_result.code != 0 {
                break; // Condition failed, exit loop
            }

            // Execute body
            last_result = self.execute_command_list(body)?;
        }

        Ok(last_result)
    }

    /// Execute until/do/done loop
    fn execute_until(
        &mut self,
        condition: &Spanned<Command>,
        body: &[Spanned<Command>],
        _span: shex_ast::Span,
    ) -> Result<ExitStatus, ShexError> {
        let mut last_result = ExitStatus {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        };

        loop {
            // Check condition (until loops when condition fails)
            let condition_result = self.execute_command(condition)?;
            if condition_result.code == 0 {
                break; // Condition succeeded, exit loop
            }

            // Execute body
            last_result = self.execute_command_list(body)?;
        }

        Ok(last_result)
    }

    /// Execute for/in/do/done loop
    fn execute_for(
        &mut self,
        variable: &str,
        words: &Option<Vec<String>>,
        body: &[Spanned<Command>],
        _span: shex_ast::Span,
    ) -> Result<ExitStatus, ShexError> {
        let mut last_result = ExitStatus {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        };

        // Get words to iterate over
        let word_list = if let Some(words) = words {
            words.clone()
        } else {
            // Default to $@ (positional parameters) - for now use empty list
            vec![]
        };

        // Execute body for each word
        for word in word_list {
            // Set loop variable
            self.variable_context.set(variable.to_string(), word);
            
            // Execute body
            last_result = self.execute_command_list(body)?;
        }

        Ok(last_result)
    }

    /// Execute case/esac pattern matching
    fn execute_case(
        &mut self,
        word: &str,
        arms: &[CaseArm],
        _span: shex_ast::Span,
    ) -> Result<ExitStatus, ShexError> {
        // Expand the word
        let expanded_word = self.expand_single_argument(word, shex_ast::Span::dummy())?;
        
        // Try each case arm
        for arm in arms {
            for pattern in &arm.patterns {
                if self.pattern_matches(pattern, &expanded_word) {
                    return self.execute_command_list(&arm.commands);
                }
            }
        }

        // No pattern matched
        Ok(ExitStatus {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        })
    }

    /// Execute function definition
    fn execute_function_definition(
        &mut self,
        _name: &str,
        _body: &Spanned<Command>,
        _redirections: &[Redirection],
        _span: shex_ast::Span,
    ) -> Result<ExitStatus, ShexError> {
        // TODO: Implement function storage and calling
        Ok(ExitStatus {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        })
    }

    /// Execute subshell
    fn execute_subshell(
        &mut self,
        commands: &[Spanned<Command>],
        _span: shex_ast::Span,
    ) -> Result<ExitStatus, ShexError> {
        // TODO: Implement proper subshell with separate environment
        // For now, just execute commands in current context
        self.execute_command_list(commands)
    }

    /// Execute brace group
    fn execute_brace_group(
        &mut self,
        commands: &[Spanned<Command>],
        _span: shex_ast::Span,
    ) -> Result<ExitStatus, ShexError> {
        // Brace groups execute in current shell context
        self.execute_command_list(commands)
    }

    /// Helper: Execute a list of commands
    fn execute_command_list(&mut self, commands: &[Spanned<Command>]) -> Result<ExitStatus, ShexError> {
        let mut last_result = ExitStatus {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        };

        for command in commands {
            last_result = self.execute_command(command)?;
        }

        Ok(last_result)
    }

    /// Helper: Simple pattern matching for case statements
    fn pattern_matches(&self, pattern: &str, word: &str) -> bool {
        // Very basic pattern matching - just exact match for now
        // TODO: Implement proper shell pattern matching with * and ?
        pattern == word
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shex_ast::{Span, Spanned};

    fn make_simple_command(name: &str, args: Vec<&str>) -> Spanned<Command> {
        Spanned::new(
            Command::Simple {
                name: name.to_string(),
                args: args
                    .into_iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
                assignments: vec![],
                redirections: vec![],
            },
            Span::dummy(),
        )
    }

    #[test]
    fn test_echo_command() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            commands: vec![make_simple_command("echo", vec!["hello", "world"])],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "hello world\n");
        assert_eq!(result.stderr, "");
    }

    #[test]
    fn test_true_command() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            commands: vec![make_simple_command("true", vec![])],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "");
    }

    #[test]
    fn test_false_command() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            commands: vec![make_simple_command("false", vec![])],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 1);
        assert_eq!(result.stdout, "");
    }

    #[test]
    fn test_command_not_found() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            commands: vec![make_simple_command("nonexistent_command_12345", vec![])],
        };

        let result = interpreter.execute(program);
        assert!(result.is_err());
        match result.unwrap_err() {
            ShexError::CommandNotFound { command, .. } => {
                assert_eq!(command, "nonexistent_command_12345");
            }
            _ => panic!("Expected CommandNotFound error"),
        }
    }

    #[test]
    fn test_multiple_commands() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            commands: vec![
                make_simple_command("true", vec![]),
                make_simple_command("echo", vec!["test"]),
            ],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "test\n");
    }

    #[test]
    fn test_variable_assignment() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            commands: vec![Spanned::new(
                Command::Assignment {
                    assignments: vec![("var".to_string(), "hello".to_string())],
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "");

        // Check that variable was stored
        assert_eq!(
            interpreter.variable_context.get("var"),
            Some(&"hello".to_string())
        );
    }

    #[test]
    fn test_simple_parameter_expansion() {
        let mut interpreter = Interpreter::new();

        // Set a variable first
        interpreter
            .variable_context
            .set("greeting".to_string(), "hello".to_string());

        let program = Program {
            commands: vec![make_simple_command("echo", vec!["$greeting"])],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "hello\n");
    }

    #[test]
    fn test_braced_parameter_expansion() {
        let mut interpreter = Interpreter::new();

        // Set a variable first
        interpreter
            .variable_context
            .set("name".to_string(), "world".to_string());

        let program = Program {
            commands: vec![make_simple_command("echo", vec!["${name}"])],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "world\n");
    }

    #[test]
    fn test_parameter_expansion_with_default() {
        let mut interpreter = Interpreter::new();

        // Test with unset variable - should use default
        let program = Program {
            commands: vec![make_simple_command(
                "echo",
                vec!["${unset_var:-default_value}"],
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "default_value\n");

        // Set the variable and test again - should use variable value
        interpreter
            .variable_context
            .set("unset_var".to_string(), "actual_value".to_string());

        let program2 = Program {
            commands: vec![make_simple_command(
                "echo",
                vec!["${unset_var:-default_value}"],
            )],
        };

        let result = interpreter.execute(program2).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "actual_value\n");
    }

    #[test]
    fn test_undefined_variable_error() {
        let mut interpreter = Interpreter::new();

        let program = Program {
            commands: vec![make_simple_command("echo", vec!["$undefined_var"])],
        };

        let result = interpreter.execute(program);
        assert!(result.is_err());
        match result.unwrap_err() {
            ShexError::UndefinedVariable { var, .. } => {
                assert_eq!(var, "undefined_var");
            }
            _ => panic!("Expected UndefinedVariable error"),
        }
    }

    #[test]
    fn test_multiple_parameter_expansions() {
        let mut interpreter = Interpreter::new();

        interpreter
            .variable_context
            .set("first".to_string(), "hello".to_string());
        interpreter
            .variable_context
            .set("second".to_string(), "world".to_string());

        let program = Program {
            commands: vec![make_simple_command("echo", vec!["$first", "${second}"])],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "hello world\n");
    }

    #[test]
    fn test_assign_default_expansion() {
        let mut interpreter = Interpreter::new();

        // Test ${var:=default} - should assign and return default value
        let program = Program {
            commands: vec![make_simple_command(
                "echo",
                vec!["${new_var:=assigned_value}"],
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "assigned_value\n");

        // Check that variable was assigned
        assert_eq!(
            interpreter.variable_context.get("new_var"),
            Some(&"assigned_value".to_string())
        );
    }

    #[test]
    fn test_prefix_assignment_with_expansion() {
        let mut interpreter = Interpreter::new();

        // Test cmd_prefix assignment with parameter expansion: name=world echo $name
        let program = Program {
            commands: vec![Spanned::new(
                Command::Simple {
                    name: "echo".to_string(),
                    args: vec!["hello".to_string(), "$name".to_string()],
                    assignments: vec![("name".to_string(), "world".to_string())],
                    redirections: vec![],
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "hello world\n");

        // Check that variable was assigned
        assert_eq!(
            interpreter.variable_context.get("name"),
            Some(&"world".to_string())
        );
    }

    #[test]
    fn test_posix_examples_basic() {
        let mut interpreter = Interpreter::new();

        // POSIX example demonstrates why braces are needed: a=1; echo ${a}b vs $ab
        interpreter
            .variable_context
            .set("a".to_string(), "1".to_string());

        // Test ${a}b - currently tokenized as separate tokens due to implementation limitation
        let program = Program {
            commands: vec![make_simple_command("echo", vec!["${a}", "b"])],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "1 b\n"); // Space because they're separate arguments

        // Test $ab should fail because 'ab' is not defined (demonstrates why braces are needed)
        let program = Program {
            commands: vec![make_simple_command("echo", vec!["$ab"])],
        };

        let result = interpreter.execute(program);
        assert!(result.is_err());
        match result.unwrap_err() {
            ShexError::UndefinedVariable { var, .. } => {
                assert_eq!(var, "ab");
            }
            _ => panic!("Expected UndefinedVariable error"),
        }
    }

    #[test]
    fn test_posix_examples_unset_vs_empty() {
        let mut interpreter = Interpreter::new();

        // POSIX example: foo=asdf; echo ${foo-bar}
        interpreter
            .variable_context
            .set("foo".to_string(), "asdf".to_string());

        let program = Program {
            commands: vec![make_simple_command("echo", vec!["${foo-bar}"])],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "asdf\n");

        // Test empty value: foo=""; echo ${foo-bar}
        interpreter
            .variable_context
            .set("foo".to_string(), "".to_string());

        let program = Program {
            commands: vec![make_simple_command("echo", vec!["${foo-bar}"])],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "\n"); // Empty string, not "bar"

        // Test unset: echo ${unset_foo-bar}
        let program = Program {
            commands: vec![make_simple_command("echo", vec!["${unset_foo-bar}"])],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "bar\n");
    }

    #[test]
    fn test_posix_examples_colon_versions() {
        let mut interpreter = Interpreter::new();

        // Test ${foo:-bar} with empty value
        interpreter
            .variable_context
            .set("foo".to_string(), "".to_string());

        let program = Program {
            commands: vec![make_simple_command("echo", vec!["${foo:-bar}"])],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "bar\n"); // Empty string treated as unset with colon

        // Test ${foo:-bar} with set value
        interpreter
            .variable_context
            .set("foo".to_string(), "value".to_string());

        let program = Program {
            commands: vec![make_simple_command("echo", vec!["${foo:-bar}"])],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "value\n");
    }

    #[test]
    fn test_posix_examples_assign_default() {
        let mut interpreter = Interpreter::new();

        // POSIX example: unset X; echo ${X:=abc}
        let program = Program {
            commands: vec![make_simple_command("echo", vec!["${X:=abc}"])],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "abc\n");

        // Check that X was assigned
        assert_eq!(
            interpreter.variable_context.get("X"),
            Some(&"abc".to_string())
        );

        // Run again - should use existing value
        let program2 = Program {
            commands: vec![make_simple_command("echo", vec!["${X:=abc}"])],
        };
        let result = interpreter.execute(program2).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "abc\n");
    }

    #[test]
    fn test_posix_examples_error_if_unset() {
        let mut interpreter = Interpreter::new();

        // POSIX example: echo ${posix:?} (unset variable)
        let program = Program {
            commands: vec![make_simple_command("echo", vec!["${posix:?}"])],
        };

        let result = interpreter.execute(program);
        assert!(result.is_err());
        match result.unwrap_err() {
            ShexError::Syntax { message, .. } => {
                assert!(message.contains("posix: parameter null or not set"));
            }
            _ => panic!("Expected Syntax error with parameter message"),
        }

        // Test with custom message
        let program = Program {
            commands: vec![make_simple_command("echo", vec!["${posix:?custom error}"])],
        };

        let result = interpreter.execute(program);
        assert!(result.is_err());
        match result.unwrap_err() {
            ShexError::Syntax { message, .. } => {
                assert!(message.contains("custom error"));
            }
            _ => panic!("Expected Syntax error with custom message"),
        }
    }

    #[test]
    fn test_posix_examples_alternative_value() {
        let mut interpreter = Interpreter::new();

        // POSIX example: ${3:+posix} - test with set variable
        interpreter
            .variable_context
            .set("var".to_string(), "value".to_string());

        let program = Program {
            commands: vec![make_simple_command("echo", vec!["${var:+alternative}"])],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "alternative\n");

        // Test with unset variable
        let program = Program {
            commands: vec![make_simple_command(
                "echo",
                vec!["${unset_var:+alternative}"],
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "\n"); // Empty string for unset variable

        // Test with empty variable
        interpreter
            .variable_context
            .set("empty_var".to_string(), "".to_string());

        let program = Program {
            commands: vec![make_simple_command(
                "echo",
                vec!["${empty_var:+alternative}"],
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "\n"); // Empty string for empty variable with colon
    }

    // Phase 1.5: Complete command structure tests

    #[test]
    fn test_pipeline_execution() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            commands: vec![Spanned::new(
                Command::Pipeline {
                    commands: vec![
                        make_simple_command("echo", vec!["hello"]),
                        make_simple_command("echo", vec!["world"]),
                    ],
                    redirections: vec![],
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        // In our simplified implementation, it executes sequentially
        assert_eq!(result.stdout, "world\n");
    }

    #[test]
    fn test_and_if_success() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            commands: vec![Spanned::new(
                Command::AndIf {
                    left: Box::new(make_simple_command("true", vec![])),
                    right: Box::new(make_simple_command("echo", vec!["success"])),
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "success\n");
    }

    #[test]
    fn test_and_if_failure() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            commands: vec![Spanned::new(
                Command::AndIf {
                    left: Box::new(make_simple_command("false", vec![])),
                    right: Box::new(make_simple_command("echo", vec!["should_not_run"])),
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 1); // false returns 1
        assert_eq!(result.stdout, ""); // right side should not execute
    }

    #[test]
    fn test_or_if_success() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            commands: vec![Spanned::new(
                Command::OrIf {
                    left: Box::new(make_simple_command("true", vec![])),
                    right: Box::new(make_simple_command("echo", vec!["should_not_run"])),
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, ""); // right side should not execute
    }

    #[test]
    fn test_or_if_failure() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            commands: vec![Spanned::new(
                Command::OrIf {
                    left: Box::new(make_simple_command("false", vec![])),
                    right: Box::new(make_simple_command("echo", vec!["fallback"])),
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "fallback\n");
    }

    #[test]
    fn test_sequence_execution() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            commands: vec![Spanned::new(
                Command::Sequence {
                    commands: vec![
                        make_simple_command("echo", vec!["first"]),
                        make_simple_command("echo", vec!["second"]),
                        make_simple_command("echo", vec!["third"]),
                    ],
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        // Returns result of last command
        assert_eq!(result.stdout, "third\n");
    }

    #[test]
    fn test_sequence_with_failure() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            commands: vec![Spanned::new(
                Command::Sequence {
                    commands: vec![
                        make_simple_command("echo", vec!["first"]),
                        make_simple_command("false", vec![]),
                        make_simple_command("echo", vec!["third"]),
                    ],
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0); // Last command (echo) succeeds
        assert_eq!(result.stdout, "third\n");
    }

    #[test]
    fn test_background_execution() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            commands: vec![Spanned::new(
                Command::Background {
                    command: Box::new(make_simple_command("echo", vec!["background"])),
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0); // Background commands return success immediately
        assert_eq!(result.stdout, ""); // No output returned from background
    }

    #[test]
    fn test_complex_command_combination() {
        let mut interpreter = Interpreter::new();

        // Test: true && echo "success" || echo "fallback"
        let program = Program {
            commands: vec![Spanned::new(
                Command::OrIf {
                    left: Box::new(Spanned::new(
                        Command::AndIf {
                            left: Box::new(make_simple_command("true", vec![])),
                            right: Box::new(make_simple_command("echo", vec!["success"])),
                        },
                        Span::dummy(),
                    )),
                    right: Box::new(make_simple_command("echo", vec!["fallback"])),
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "success\n");
    }

    #[test]
    fn test_nested_command_with_variables() {
        let mut interpreter = Interpreter::new();

        // Test: var=hello echo $var && echo "world"
        let program = Program {
            commands: vec![Spanned::new(
                Command::AndIf {
                    left: Box::new(Spanned::new(
                        Command::Simple {
                            name: "echo".to_string(),
                            args: vec!["$var".to_string()],
                            assignments: vec![("var".to_string(), "hello".to_string())],
                            redirections: vec![],
                        },
                        Span::dummy(),
                    )),
                    right: Box::new(make_simple_command("echo", vec!["world"])),
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "world\n");
    }

    #[test]
    fn test_if_statement_true_condition() {
        let mut interpreter = Interpreter::new();

        // Test: if true; then echo "success"; fi
        let program = Program {
            commands: vec![Spanned::new(
                Command::If {
                    condition: Box::new(make_simple_command("true", vec![])),
                    then_body: vec![make_simple_command("echo", vec!["success"])],
                    elif_clauses: vec![],
                    else_body: None,
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "success\n");
    }

    #[test]
    fn test_if_statement_false_condition() {
        let mut interpreter = Interpreter::new();

        // Test: if false; then echo "fail"; fi
        let program = Program {
            commands: vec![Spanned::new(
                Command::If {
                    condition: Box::new(make_simple_command("false", vec![])),
                    then_body: vec![make_simple_command("echo", vec!["fail"])],
                    elif_clauses: vec![],
                    else_body: None,
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0); // if statement itself succeeds
        assert_eq!(result.stdout, ""); // but then body is not executed
    }

    #[test]
    fn test_if_else_statement() {
        let mut interpreter = Interpreter::new();

        // Test: if false; then echo "fail"; else echo "success"; fi
        let program = Program {
            commands: vec![Spanned::new(
                Command::If {
                    condition: Box::new(make_simple_command("false", vec![])),
                    then_body: vec![make_simple_command("echo", vec!["fail"])],
                    elif_clauses: vec![],
                    else_body: Some(vec![make_simple_command("echo", vec!["success"])]),
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "success\n");
    }

    #[test]
    fn test_while_loop() {
        let mut interpreter = Interpreter::new();
        
        // Set up a counter variable
        interpreter.variable_context.set("count".to_string(), "0".to_string());

        // Test: while [ $count -lt 3 ]; do echo $count; count=$((count+1)); done
        // Simplified: while false; do echo "never"; done (should not execute body)
        let program = Program {
            commands: vec![Spanned::new(
                Command::While {
                    condition: Box::new(make_simple_command("false", vec![])),
                    body: vec![make_simple_command("echo", vec!["never"])],
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, ""); // Body never executed
    }

    #[test]
    fn test_for_loop_with_words() {
        let mut interpreter = Interpreter::new();

        // Test: for item in apple banana cherry; do echo $item; done
        let program = Program {
            commands: vec![Spanned::new(
                Command::For {
                    variable: "item".to_string(),
                    words: Some(vec!["apple".to_string(), "banana".to_string(), "cherry".to_string()]),
                    body: vec![make_simple_command("echo", vec!["$item"])],
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        // Should execute echo for each item: apple, banana, cherry
        assert_eq!(result.stdout, "cherry\n"); // Last iteration result
    }

    #[test]
    fn test_for_loop_empty_list() {
        let mut interpreter = Interpreter::new();

        // Test: for item in; do echo $item; done (empty word list)
        let program = Program {
            commands: vec![Spanned::new(
                Command::For {
                    variable: "item".to_string(),
                    words: Some(vec![]), // Empty list
                    body: vec![make_simple_command("echo", vec!["never"])],
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, ""); // Body never executed
    }

    #[test]
    fn test_case_statement_match() {
        let mut interpreter = Interpreter::new();

        // Test: case "apple" in apple) echo "fruit" ;; esac
        let program = Program {
            commands: vec![Spanned::new(
                Command::Case {
                    word: "apple".to_string(),
                    arms: vec![
                        CaseArm {
                            patterns: vec!["apple".to_string()],
                            commands: vec![make_simple_command("echo", vec!["fruit"])],
                        },
                        CaseArm {
                            patterns: vec!["*".to_string()],
                            commands: vec![make_simple_command("echo", vec!["other"])],
                        },
                    ],
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "fruit\n"); // First pattern matches
    }

    #[test]
    fn test_case_statement_no_match() {
        let mut interpreter = Interpreter::new();

        // Test: case "banana" in apple) echo "fruit" ;; esac
        let program = Program {
            commands: vec![Spanned::new(
                Command::Case {
                    word: "banana".to_string(),
                    arms: vec![
                        CaseArm {
                            patterns: vec!["apple".to_string()],
                            commands: vec![make_simple_command("echo", vec!["fruit"])],
                        },
                    ],
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, ""); // No pattern matches
    }

    #[test]
    fn test_case_statement_multiple_patterns() {
        let mut interpreter = Interpreter::new();

        // Test: case "banana" in apple|banana|cherry) echo "fruit" ;; esac
        let program = Program {
            commands: vec![Spanned::new(
                Command::Case {
                    word: "banana".to_string(),
                    arms: vec![
                        CaseArm {
                            patterns: vec!["apple".to_string(), "banana".to_string(), "cherry".to_string()],
                            commands: vec![make_simple_command("echo", vec!["fruit"])],
                        },
                    ],
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "fruit\n"); // Second pattern matches
    }

    #[test] 
    fn test_subshell_execution() {
        let mut interpreter = Interpreter::new();

        // Test: (echo "in subshell")
        let program = Program {
            commands: vec![Spanned::new(
                Command::Subshell {
                    commands: vec![make_simple_command("echo", vec!["in subshell"])],
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "in subshell\n");
    }

    #[test]
    fn test_brace_group_execution() {
        let mut interpreter = Interpreter::new();

        // Test: { echo "in brace group"; }
        let program = Program {
            commands: vec![Spanned::new(
                Command::BraceGroup {
                    commands: vec![make_simple_command("echo", vec!["in brace group"])],
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "in brace group\n");
    }

    #[test]
    fn test_nested_compound_commands() {
        let mut interpreter = Interpreter::new();

        // Test: if true; then { echo "nested"; }; fi
        let program = Program {
            commands: vec![Spanned::new(
                Command::If {
                    condition: Box::new(make_simple_command("true", vec![])),
                    then_body: vec![Spanned::new(
                        Command::BraceGroup {
                            commands: vec![make_simple_command("echo", vec!["nested"])],
                        },
                        Span::dummy(),
                    )],
                    elif_clauses: vec![],
                    else_body: None,
                },
                Span::dummy(),
            )],
        };

        let result = interpreter.execute(program).unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "nested\n");
    }
}
