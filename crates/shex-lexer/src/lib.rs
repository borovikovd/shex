//! Lexical analysis for Shex shell
//!
//! Implements POSIX shell tokenization plus Shex extensions using logos.

use logos::Logos;
use shex_ast::Span;

/// Shell tokens - Complete POSIX token set
#[derive(Logos, Debug, PartialEq, Eq, Clone)]
pub enum Token {
    // POSIX Basic Tokens
    /// Assignment word (var=value) - must come before Word to take precedence  
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*=[^\s]*", priority = 2)]
    AssignmentWord,

    /// A word token (shell words, can contain various characters including paths)
    #[regex(r"[a-zA-Z_/][a-zA-Z0-9_./-]*")]
    Word,

    /// Special single character tokens
    #[token("[")]
    LeftBracket,

    #[token("]")]
    RightBracket,

    #[token("-")]
    Dash,

    #[token(".")]
    Dot,

    /// Number token (can be IO_NUMBER in context)
    #[regex(r"[0-9]+")]
    Number,

    /// String literal with quotes
    #[regex(r#""([^"\\]|\\.)*""#)]
    #[regex(r#"'([^'\\]|\\.)*'"#)]
    String,

    /// Newline
    #[token("\n")]
    Newline,

    // POSIX Multi-character Operators
    /// Logical AND operator (&&)
    #[token("&&")]
    AndIf,

    /// Logical OR operator (||)
    #[token("||")]
    OrIf,

    /// Double semicolon (;;)
    #[token(";;")]
    Dsemi,

    /// Here-document (<<)
    #[token("<<")]
    Dless,

    /// Append redirection (>>)
    #[token(">>")]
    Dgreat,

    /// Input redirection from file descriptor (<&)
    #[token("<&")]
    Lessand,

    /// Output redirection to file descriptor (>&)
    #[token(">&")]
    Greatand,

    /// Input/output redirection (<>)
    #[token("<>")]
    Lessgreat,

    /// Here-document with tab removal (<<-)
    #[token("<<-")]
    Dlessdash,

    /// Force redirection override (>|)
    #[token(">|")]
    Clobber,

    // POSIX Reserved Words
    /// if keyword
    #[token("if")]
    If,

    /// then keyword
    #[token("then")]
    Then,

    /// else keyword
    #[token("else")]
    Else,

    /// elif keyword
    #[token("elif")]
    Elif,

    /// fi keyword
    #[token("fi")]
    Fi,

    /// do keyword
    #[token("do")]
    Do,

    /// done keyword
    #[token("done")]
    Done,

    /// case keyword
    #[token("case")]
    Case,

    /// esac keyword
    #[token("esac")]
    Esac,

    /// while keyword
    #[token("while")]
    While,

    /// until keyword
    #[token("until")]
    Until,

    /// for keyword
    #[token("for")]
    For,

    /// in keyword
    #[token("in")]
    In,

    /// Left brace ({)
    #[token("{")]
    Lbrace,

    /// Right brace (})
    #[token("}")]
    Rbrace,

    /// Bang (!)
    #[token("!")]
    Bang,

    // Single-character operators
    /// Pipe operator (|)
    #[token("|")]
    Pipe,

    /// Semicolon separator (;)
    #[token(";")]
    Semicolon,

    /// Background operator (&)
    #[token("&")]
    Ampersand,

    /// Input redirection (<)
    #[token("<")]
    Less,

    /// Output redirection (>)
    #[token(">")]
    Great,

    /// Left parenthesis (()
    #[token("(")]
    Lparen,

    /// Right parenthesis ())
    #[token(")")]
    Rparen,

    // Shex Extensions (from Phase 1.1)
    /// Parameter expansion with braces: ${var}, ${var:-default}, etc.
    /// Higher priority than simple parameter expansion
    #[regex(r"\$\{[^}]+\}", priority = 3)]
    ParameterExpansion,

    /// Simple parameter expansion: $var
    /// Must come after `ParameterExpansion` to avoid conflicts
    #[regex(r"\$[a-zA-Z_][a-zA-Z0-9_]*", priority = 2)]
    SimpleParameterExpansion,

    /// Whitespace (ignored)
    #[regex(r"[ \t\f]+", logos::skip)]
    Whitespace,

    /// End of input
    Eof,

    /// Lexer error
    Error,
}

/// Token with location information
#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
    pub text: String,
}

/// Lexer that produces tokens with spans
pub struct Lexer<'input> {
    lexer: logos::Lexer<'input, Token>,
    input: &'input str,
}

impl<'input> Lexer<'input> {
    #[must_use]
    pub fn new(input: &'input str) -> Self {
        Self {
            lexer: Token::lexer(input),
            input,
        }
    }

    /// Get the next token with span information
    pub fn next_token(&mut self) -> SpannedToken {
        match self.lexer.next() {
            Some(Ok(token)) => {
                let span = self.lexer.span();
                let text = self.input[span.clone()].to_string();
                SpannedToken {
                    token,
                    span: Span::new(span.start, span.end),
                    text,
                }
            }
            Some(Err(())) => {
                let span = self.lexer.span();
                let text = self.input[span.clone()].to_string();
                SpannedToken {
                    token: Token::Error,
                    span: Span::new(span.start, span.end),
                    text,
                }
            }
            None => SpannedToken {
                token: Token::Eof,
                span: Span::new(self.input.len(), self.input.len()),
                text: String::new(),
            },
        }
    }

    /// Tokenize the entire input
    pub fn tokenize(&mut self) -> Vec<SpannedToken> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = token.token == Token::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let mut lexer = Lexer::new("echo hello");
        let tokens = lexer.tokenize();

        assert_eq!(tokens.len(), 3); // echo, hello, EOF
        assert_eq!(tokens[0].token, Token::Word);
        assert_eq!(tokens[0].text, "echo");
        assert_eq!(tokens[1].token, Token::Word);
        assert_eq!(tokens[1].text, "hello");
        assert_eq!(tokens[2].token, Token::Eof);
    }

    #[test]
    fn test_pipeline() {
        let mut lexer = Lexer::new("echo hello | wc");
        let tokens = lexer.tokenize();

        // Should have: echo, hello, |, wc, EOF
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].text, "echo");
        assert_eq!(tokens[1].text, "hello");
        assert_eq!(tokens[2].token, Token::Pipe);
        assert_eq!(tokens[3].text, "wc");
        assert_eq!(tokens[4].token, Token::Eof);
    }

    #[test]
    fn test_basic_tokenization() {
        let mut lexer = Lexer::new("echo hello");
        let tokens = lexer.tokenize();

        assert_eq!(tokens.len(), 3); // echo, hello, EOF
        assert_eq!(tokens[0].token, Token::Word);
        assert_eq!(tokens[0].text, "echo");
        assert_eq!(tokens[1].token, Token::Word);
        assert_eq!(tokens[1].text, "hello");
        assert_eq!(tokens[2].token, Token::Eof);
    }

    #[test]
    fn test_span_tracking() {
        let mut lexer = Lexer::new("echo hello");
        let tokens = lexer.tokenize();

        // Check that spans are correct
        assert_eq!(tokens[0].span.start, 0);
        assert_eq!(tokens[0].span.end, 4); // "echo"
        assert_eq!(tokens[1].span.start, 5);
        assert_eq!(tokens[1].span.end, 10); // "hello"
    }

    #[test]
    fn test_string_literals() {
        let mut lexer = Lexer::new(r#"echo "hello world" 'test'"#);
        let tokens = lexer.tokenize();

        assert_eq!(tokens.len(), 4); // echo, "hello world", 'test', EOF
        assert_eq!(tokens[0].text, "echo");
        assert_eq!(tokens[1].token, Token::String);
        assert_eq!(tokens[1].text, r#""hello world""#);
        assert_eq!(tokens[2].token, Token::String);
        assert_eq!(tokens[2].text, "'test'");
    }

    #[test]
    fn test_parameter_expansions() {
        let mut lexer = Lexer::new("echo $var ${other:-default}");
        let tokens = lexer.tokenize();

        assert_eq!(tokens.len(), 4); // echo, $var, ${other:-default}, EOF
        assert_eq!(tokens[0].token, Token::Word);
        assert_eq!(tokens[1].token, Token::SimpleParameterExpansion);
        assert_eq!(tokens[1].text, "$var");
        assert_eq!(tokens[2].token, Token::ParameterExpansion);
        assert_eq!(tokens[2].text, "${other:-default}");
    }

    #[test]
    fn test_logical_operators() {
        let mut lexer = Lexer::new("cmd1 && cmd2 || cmd3");
        let tokens = lexer.tokenize();

        assert_eq!(tokens.len(), 6); // cmd1, &&, cmd2, ||, cmd3, EOF
        assert_eq!(tokens[1].token, Token::AndIf);
        assert_eq!(tokens[1].text, "&&");
        assert_eq!(tokens[3].token, Token::OrIf);
        assert_eq!(tokens[3].text, "||");
    }

    #[test]
    fn test_posix_operators() {
        // Test key POSIX multi-character operators
        let test_cases = vec![
            ("<<", Token::Dless),
            (">>", Token::Dgreat),
            ("<&", Token::Lessand),
            (">&", Token::Greatand),
            ("<>", Token::Lessgreat),
            ("<<-", Token::Dlessdash),
            (">|", Token::Clobber),
            (";;", Token::Dsemi),
        ];

        for (input, expected_token) in test_cases {
            let mut lexer = Lexer::new(input);
            let tokens = lexer.tokenize();
            assert_eq!(tokens[0].token, expected_token);
            assert_eq!(tokens[0].text, input);
        }
    }

    #[test]
    fn test_posix_keywords() {
        // Test essential POSIX keywords
        let test_cases = vec![
            ("if", Token::If),
            ("then", Token::Then),
            ("else", Token::Else),
            ("fi", Token::Fi),
            ("for", Token::For),
            ("while", Token::While),
            ("do", Token::Do),
            ("done", Token::Done),
        ];

        for (input, expected_token) in test_cases {
            let mut lexer = Lexer::new(input);
            let tokens = lexer.tokenize();
            assert_eq!(tokens.len(), 2); // keyword, EOF
            assert_eq!(tokens[0].token, expected_token);
            assert_eq!(tokens[0].text, input);
        }
    }

    #[test]
    fn test_operator_precedence() {
        // Test that multi-character operators take precedence over single characters
        let mut lexer = Lexer::new("&& ||");
        let tokens = lexer.tokenize();

        assert_eq!(tokens.len(), 3); // &&, ||, EOF
        assert_eq!(tokens[0].token, Token::AndIf);
        assert_eq!(tokens[0].text, "&&");
        assert_eq!(tokens[1].token, Token::OrIf);
        assert_eq!(tokens[1].text, "||");
    }
}
