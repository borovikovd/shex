//! Abstract Syntax Tree definitions for Shex
//!
//! Every AST node preserves location information for error reporting.

/// Source location information for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub const fn dummy() -> Self {
        Self { start: 0, end: 0 }
    }
}

/// Line and column position in source text
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    #[must_use]
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Convert byte span to line/column positions
pub struct SourceMap {
    line_starts: Vec<usize>,
}

impl SourceMap {
    #[must_use]
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (pos, ch) in source.char_indices() {
            if ch == '\n' {
                line_starts.push(pos + 1);
            }
        }
        Self { line_starts }
    }

    #[must_use]
    pub fn position(&self, byte_offset: usize) -> Position {
        match self.line_starts.binary_search(&byte_offset) {
            Ok(line) => Position::new(line + 1, 1),
            Err(line) => {
                let line_start = self.line_starts[line - 1];
                Position::new(line, byte_offset - line_start + 1)
            }
        }
    }

    #[must_use]
    pub fn span_to_positions(&self, span: Span) -> (Position, Position) {
        (self.position(span.start), self.position(span.end))
    }
}

/// AST node with location information
#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    #[must_use]
    pub const fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}

/// Top-level program
#[derive(Debug, Clone)]
pub struct Program {
    pub commands: Vec<Spanned<Command>>,
}

/// A shell command - follows POSIX command hierarchy
#[derive(Debug, Clone)]
pub enum Command {
    /// Simple command: echo hello (with optional prefix assignments)
    Simple {
        name: String,
        args: Vec<String>,
        assignments: Vec<(String, String)>,
    },
    /// Pipeline: cmd1 | cmd2 | cmd3
    Pipeline { commands: Vec<Spanned<Command>> },
    /// Variable assignment(s): var1=value1 var2=value2
    Assignment { assignments: Vec<(String, String)> },
    /// Logical AND: cmd1 && cmd2
    AndIf {
        left: Box<Spanned<Command>>,
        right: Box<Spanned<Command>>,
    },
    /// Logical OR: cmd1 || cmd2  
    OrIf {
        left: Box<Spanned<Command>>,
        right: Box<Spanned<Command>>,
    },
    /// Sequential execution: cmd1; cmd2
    Sequence { commands: Vec<Spanned<Command>> },
    /// Background execution: cmd &
    Background { command: Box<Spanned<Command>> },
}

/// Error types with location information
#[derive(thiserror::Error, Debug)]
pub enum ShexError {
    #[error("Shex:{filename}:{line}:{column}: ERR_SYNTAX: {message}")]
    Syntax {
        message: String,
        span: Span,
        filename: String,
        line: usize,
        column: usize,
    },

    #[error("Shex:{filename}:{line}:{column}: ERR_UNDEF_VAR: {var} is not set")]
    UndefinedVariable {
        var: String,
        span: Span,
        filename: String,
        line: usize,
        column: usize,
    },

    #[error("Shex:{filename}:{line}:{column}: ERR_COMMAND_NOT_FOUND: {command} not found")]
    CommandNotFound {
        command: String,
        span: Span,
        filename: String,
        line: usize,
        column: usize,
    },
}

impl ShexError {
    #[must_use]
    pub fn syntax(message: String, span: Span, source_map: &SourceMap, filename: &str) -> Self {
        let pos = source_map.position(span.start);
        Self::Syntax {
            message,
            span,
            filename: filename.to_string(),
            line: pos.line,
            column: pos.column,
        }
    }

    #[must_use]
    pub fn undefined_variable(
        var: String,
        span: Span,
        source_map: &SourceMap,
        filename: &str,
    ) -> Self {
        let pos = source_map.position(span.start);
        Self::UndefinedVariable {
            var,
            span,
            filename: filename.to_string(),
            line: pos.line,
            column: pos.column,
        }
    }

    #[must_use]
    pub fn command_not_found(
        command: String,
        span: Span,
        source_map: &SourceMap,
        filename: &str,
    ) -> Self {
        let pos = source_map.position(span.start);
        Self::CommandNotFound {
            command,
            span,
            filename: filename.to_string(),
            line: pos.line,
            column: pos.column,
        }
    }

    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Syntax { span, .. }
            | Self::UndefinedVariable { span, .. }
            | Self::CommandNotFound { span, .. } => *span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_creation() {
        let span = Span::new(10, 20);
        assert_eq!(span.start, 10);
        assert_eq!(span.end, 20);
    }

    #[test]
    fn test_spanned_node() {
        let cmd = Command::Simple {
            name: "echo".to_string(),
            args: vec!["hello".to_string()],
            assignments: vec![],
        };
        let spanned = Spanned::new(cmd, Span::new(0, 10));
        assert_eq!(spanned.span.start, 0);
        assert_eq!(spanned.span.end, 10);
    }

    #[test]
    fn test_source_map() {
        let source = "echo hello\necho world\n";
        let source_map = SourceMap::new(source);

        // Test position at start
        let pos = source_map.position(0);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 1);

        // Test position after first word
        let pos = source_map.position(4);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 5);

        // Test position on second line
        let pos = source_map.position(11);
        assert_eq!(pos.line, 2);
        assert_eq!(pos.column, 1);
    }

    #[test]
    fn test_error_with_proper_format() {
        let source = "echo hello\nnonexistent";
        let source_map = SourceMap::new(source);
        let span = Span::new(11, 22); // "nonexistent" on line 2

        let error =
            ShexError::command_not_found("nonexistent".to_string(), span, &source_map, "test.sh");

        let error_str = format!("{error}");
        assert!(error_str.contains("Shex:test.sh:2:1"));
        assert!(error_str.contains("ERR_COMMAND_NOT_FOUND"));
    }
}
