//! LALRPOP-based parser for Shex shell - Phase 0.5 with LALRPOP
//!
//! Uses LALRPOP parser generator with logos lexer for improved maintainability.

#![allow(unused_imports)]
#![allow(unused_variables)] // Allow unused variables in generated LALRPOP code
#![allow(clippy::all, clippy::pedantic, clippy::nursery)]

use shex_ast::{Command, Program, ShexError, SourceMap, Span};
use shex_lexer::{Lexer, SpannedToken, Token};

// Include the generated LALRPOP parser
lalrpop_util::lalrpop_mod!(pub shex);

// String processing utilities
pub mod string_utils;

// Variable resolution infrastructure
pub mod variable_resolver;

// Helper functions for POSIX grammar implementation
pub fn combine_args(prefix: Vec<SpannedToken>, suffix: Vec<SpannedToken>) -> Vec<String> {
    string_utils::combine_args(&prefix, &suffix)
}

pub fn extract_assignments(prefix: Vec<SpannedToken>) -> Vec<(String, String)> {
    string_utils::extract_assignments(&prefix)
}

pub fn token_to_string(token: SpannedToken) -> String {
    string_utils::token_to_string(&token)
}

pub struct Parser {
    input: String,
    source_map: SourceMap,
    filename: String,
    tokens: Vec<SpannedToken>,
}

impl Parser {
    /// Create a new parser for the given input
    ///
    /// # Errors
    ///
    /// Returns `ShexError` if there are lexical errors in the input
    pub fn new(input: &str) -> Result<Self, ShexError> {
        Self::new_with_filename(input, "<input>")
    }

    /// Create a new parser for the given input with a filename
    ///
    /// # Errors
    ///
    /// Returns `ShexError` if there are lexical errors in the input
    pub fn new_with_filename(input: &str, filename: &str) -> Result<Self, ShexError> {
        let source_map = SourceMap::new(input);

        // Tokenize input using logos
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        // Check for lexer errors
        for token in &tokens {
            if token.token == Token::Error {
                return Err(ShexError::syntax(
                    format!("Unexpected character: {}", token.text),
                    token.span,
                    &source_map,
                    filename,
                ));
            }
        }

        Ok(Self {
            input: input.to_string(),
            source_map,
            filename: filename.to_string(),
            tokens,
        })
    }

    /// Parse the input into a program AST
    ///
    /// # Errors
    ///
    /// Returns `ShexError` if there are syntax errors during parsing
    pub fn parse(&self) -> Result<Program, ShexError> {
        // Filter out newlines and empty commands, keep only meaningful tokens
        let filtered_tokens: Vec<SpannedToken> = self
            .tokens
            .iter()
            .filter(|token| token.token != Token::Newline)
            .cloned()
            .collect();

        // Convert tokens to the format LALRPOP expects
        let lalrpop_tokens: Vec<Result<(usize, SpannedToken, usize), ()>> = filtered_tokens
            .into_iter()
            .map(|token| {
                let start = token.span.start;
                let end = token.span.end;
                Ok((start, token, end))
            })
            .collect();

        // Use LALRPOP parser
        let parser = shex::ProgramParser::new();
        match parser.parse(lalrpop_tokens) {
            Ok(mut program) => {
                // Filter out empty commands (from newlines)
                program.commands.retain(|cmd| match &cmd.node {
                    Command::Simple { name, .. } => !name.is_empty(),
                    _ => true,
                });
                Ok(program)
            }
            Err(err) => {
                // Convert LALRPOP error to ShexError
                let error_msg = format!("Parse error: {err:?}");
                Err(ShexError::syntax(
                    error_msg,
                    Span::new(0, self.input.len()),
                    &self.source_map,
                    &self.filename,
                ))
            }
        }
    }

    /// Get access to the source map for error reporting
    #[must_use]
    pub const fn source_map(&self) -> &SourceMap {
        &self.source_map
    }

    /// Get access to the filename
    #[must_use]
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// Get access to the original input
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }

    /// Get access to the tokens (useful for debugging)
    #[must_use]
    pub fn tokens(&self) -> &[SpannedToken] {
        &self.tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shex_ast::Command;

    #[test]
    fn test_simple_command() {
        let parser = Parser::new("echo hello").unwrap();
        let program = parser.parse().unwrap();

        assert_eq!(program.commands.len(), 1);
        match &program.commands[0].node {
            Command::Simple {
                name,
                args,
                assignments,
                redirections: _,
            } => {
                assert_eq!(name, "echo");
                assert_eq!(args, &["hello"]);
                assert_eq!(assignments, &[]);
            }
            _ => panic!("Expected simple command"),
        }
    }

    #[test]
    fn test_command_arguments() {
        // Test multiple argument types in one test
        let parser = Parser::new(r#"echo hello "world test" $var ${other:-default}"#).unwrap();
        let program = parser.parse().unwrap();

        assert_eq!(program.commands.len(), 1);
        match &program.commands[0].node {
            Command::Simple {
                name,
                args,
                assignments,
                redirections: _,
            } => {
                assert_eq!(name, "echo");
                assert_eq!(args, &["hello", "world test", "$var", "${other:-default}"]);
                assert_eq!(assignments, &[]);
            }
            _ => panic!("Expected simple command"),
        }
    }

    #[test]
    fn test_command_with_assignments() {
        let parser = Parser::new("name=world echo hello $name").unwrap();
        let program = parser.parse().unwrap();

        assert_eq!(program.commands.len(), 1);
        match &program.commands[0].node {
            Command::Simple {
                name,
                args,
                assignments,
                redirections: _,
            } => {
                assert_eq!(name, "echo");
                assert_eq!(args, &["hello", "$name"]);
                assert_eq!(assignments, &[("name".to_string(), "world".to_string())]);
            }
            _ => panic!("Expected simple command"),
        }
    }

    #[test]
    fn test_empty_input() {
        let parser = Parser::new("").unwrap();
        let program = parser.parse().unwrap();

        assert_eq!(program.commands.len(), 0);
    }

    // Pipeline test disabled for Phase 0.5 - will re-enable in Phase 1
    #[test]
    #[ignore]
    fn test_pipeline() {
        let parser = Parser::new("echo hello | wc").unwrap();
        let program = parser.parse().unwrap();

        assert_eq!(program.commands.len(), 1);
        match &program.commands[0].node {
            Command::Pipeline { commands, redirections: _ } => {
                assert_eq!(commands.len(), 2);
                // First command should be "echo hello"
                match &commands[0].node {
                    Command::Simple { name, args, .. } => {
                        assert_eq!(name, "echo");
                        assert_eq!(args, &["hello"]);
                    }
                    _ => panic!("Expected simple command"),
                }
                // Second command should be "wc"
                match &commands[1].node {
                    Command::Simple { name, args, .. } => {
                        assert_eq!(name, "wc");
                        assert_eq!(args.len(), 0);
                    }
                    _ => panic!("Expected simple command"),
                }
            }
            _ => panic!("Expected pipeline"),
        }
    }
}
