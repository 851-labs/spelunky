//! Lexer and parser for GameMaker 8 GML, producing a purely syntactic AST with byte spans.
//!
//! This is the front end for the GML-to-Rust translator and for source analyses (see
//! `docs/design/translation.md`). It accepts the GM8 dialect, including its quirks: `begin`/`end`, `then`,
//! `:=`, optional semicolons, `and`/`or`/`xor`/`not`, `<>`, `div`/`mod`, `$` hex literals, strings without
//! escapes, and `=` meaning comparison inside expressions. GM8's operator precedence differs from C's (see
//! [`ast::BinOp::precedence`]).
//!
//! ```
//! let program = gml::parse_program("if x > 0 then begin y := 1 end").unwrap();
//! assert_eq!(gml::print::program(&program), "if (x > 0) {\n    y = 1;\n}\n");
//! ```

pub mod ast;
pub mod lexer;
mod parser;
pub mod print;
pub mod span;
pub mod visit;

use std::fmt;

pub use ast::{Expr, Program, Stmt};
pub use span::{LineIndex, Span};

/// A lex or parse error, located by its byte span in the unit's source.
#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    pub message: String,
    pub span: Span,
}

impl Error {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }

    /// Formats the error as `line:col: message` for `source`.
    pub fn display(&self, source: &str) -> String {
        format!(
            "{}: {}",
            LineIndex::new(source).line_col(self.span.start),
            self.message
        )
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (bytes {}..{})",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// Parses a script, a code action, or any other sequence of statements.
pub fn parse_program(source: &str) -> Result<Program> {
    parser::Parser::new(source)?.program()
}

/// Parses a single expression, such as a drag-and-drop action argument.
pub fn parse_expression(source: &str) -> Result<Expr> {
    parser::Parser::new(source)?.whole_expression()
}
