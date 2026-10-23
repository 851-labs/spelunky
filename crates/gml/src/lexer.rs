//! The GM8 GML lexer.
//!
//! GM8 lexing quirks handled here:
//! - `begin`/`end` are the same tokens as `{`/`}`.
//! - `and`/`or`/`xor`/`not` are the same tokens as `&&`/`||`/`^^`/`!`; `<>` is `!=`; `:=` is `=`.
//! - Strings use `"` or `'` and have **no escapes**; they may span lines.
//! - Hex literals are `$1F`. `#1F` is accepted too (some GML dialects allow it); Spelunky never uses it.
//! - `//` and `/* */` comments. An unterminated `/*` runs to the end of the unit, which is how GM8's
//!   lexer behaves as far as OpenGMK models it (`gml-parser/src/lexer.rs`); verify against oracle.
//! - Keywords are case-sensitive, like the rest of GML.

use crate::span::Span;
use crate::{Error, Result};

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Ident(String),
    Real(f64),
    Str(String),
    Kw(Kw),
    Op(Op),
    /// `{` or `begin`.
    LBrace,
    /// `}` or `end`.
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Semi,
    Colon,
    Dot,
    Eof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kw {
    If,
    Then,
    Else,
    While,
    Do,
    Until,
    For,
    Repeat,
    Switch,
    Case,
    Default,
    Break,
    Continue,
    Exit,
    Return,
    With,
    Var,
    GlobalVar,
    True,
    False,
    SelfKw,
    Other,
    All,
    Noone,
    Global,
}

/// Operator tokens. `Assign` is `=` (or `:=`); whether it means assignment or equality depends on where it
/// appears, which the parser decides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    AndAssign,
    OrAssign,
    XorAssign,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Mod,
    And,
    Or,
    Xor,
    Not,
    BitAnd,
    BitOr,
    BitXor,
    BitNot,
    Shl,
    Shr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

fn keyword(word: &str) -> Option<Tok> {
    Some(match word {
        "if" => Tok::Kw(Kw::If),
        "then" => Tok::Kw(Kw::Then),
        "else" => Tok::Kw(Kw::Else),
        "while" => Tok::Kw(Kw::While),
        "do" => Tok::Kw(Kw::Do),
        "until" => Tok::Kw(Kw::Until),
        "for" => Tok::Kw(Kw::For),
        "repeat" => Tok::Kw(Kw::Repeat),
        "switch" => Tok::Kw(Kw::Switch),
        "case" => Tok::Kw(Kw::Case),
        "default" => Tok::Kw(Kw::Default),
        "break" => Tok::Kw(Kw::Break),
        "continue" => Tok::Kw(Kw::Continue),
        "exit" => Tok::Kw(Kw::Exit),
        "return" => Tok::Kw(Kw::Return),
        "with" => Tok::Kw(Kw::With),
        "var" => Tok::Kw(Kw::Var),
        "globalvar" => Tok::Kw(Kw::GlobalVar),
        "true" => Tok::Kw(Kw::True),
        "false" => Tok::Kw(Kw::False),
        "self" => Tok::Kw(Kw::SelfKw),
        "other" => Tok::Kw(Kw::Other),
        "all" => Tok::Kw(Kw::All),
        "noone" => Tok::Kw(Kw::Noone),
        "global" => Tok::Kw(Kw::Global),
        "begin" => Tok::LBrace,
        "end" => Tok::RBrace,
        "and" => Tok::Op(Op::And),
        "or" => Tok::Op(Op::Or),
        "xor" => Tok::Op(Op::Xor),
        "not" => Tok::Op(Op::Not),
        "div" => Tok::Op(Op::IntDiv),
        "mod" => Tok::Op(Op::Mod),
        _ => return None,
    })
}

/// Splits a whole unit into tokens, ending with [`Tok::Eof`].
pub fn tokenize(source: &str) -> Result<Vec<Token>> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    loop {
        // Whitespace: GM8 treats every byte at or below space as blank.
        while i < bytes.len() && bytes[i] <= b' ' {
            i += 1;
        }
        if i >= bytes.len() {
            tokens.push(Token {
                tok: Tok::Eof,
                span: Span::new(i, i),
            });
            return Ok(tokens);
        }
        let start = i;
        let b = bytes[i];
        let next = bytes.get(i + 1).copied();

        // Comments.
        if b == b'/' && next == Some(b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b == b'/' && next == Some(b'*') {
            i += 2;
            while i < bytes.len() && !(bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/')) {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }

        let tok = match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => {
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let word = &source[start..i];
                keyword(word).unwrap_or_else(|| Tok::Ident(word.to_owned()))
            }
            b'0'..=b'9' => lex_decimal(source, &mut i),
            // `.5` is a number; `.x` is a field access.
            b'.' if next.is_some_and(|c| c.is_ascii_digit()) => lex_decimal(source, &mut i),
            b'$' | b'#' => {
                i += 1;
                let digits = i;
                while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
                    i += 1;
                }
                if digits == i {
                    return Err(Error::new(
                        format!("`{}` without hex digits", b as char),
                        Span::new(start, i),
                    ));
                }
                // u64 holds 16 hex digits; GM8 reals can't hold more exactly anyway.
                let value = u64::from_str_radix(&source[digits..i], 16)
                    .map_err(|_| Error::new("hex literal too long", Span::new(start, i)))?;
                Tok::Real(value as f64)
            }
            b'"' | b'\'' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b {
                    i += 1;
                }
                if i >= bytes.len() {
                    return Err(Error::new("unterminated string", Span::new(start, i)));
                }
                i += 1;
                Tok::Str(source[start + 1..i - 1].to_owned())
            }
            _ => {
                let (tok, len) = lex_punct(b, next).ok_or_else(|| {
                    // Report the whole character, so the span stays on a UTF-8 boundary.
                    let c = source[i..].chars().next().expect("i < len");
                    Error::new(
                        format!("unexpected character {c:?}"),
                        Span::new(i, i + c.len_utf8()),
                    )
                })?;
                i += len;
                tok
            }
        };
        tokens.push(Token {
            tok,
            span: Span::new(start, i),
        });
    }
}

/// Digits with at most one decimal point. A second `.` ends the literal.
fn lex_decimal(source: &str, i: &mut usize) -> Tok {
    let bytes = source.as_bytes();
    let start = *i;
    let mut seen_point = false;
    while *i < bytes.len() {
        match bytes[*i] {
            b'0'..=b'9' => {}
            // A point only belongs to the number if a digit follows, so `1.x` is a field access.
            b'.' if !seen_point && bytes.get(*i + 1).is_some_and(u8::is_ascii_digit) => {
                seen_point = true
            }
            _ => break,
        }
        *i += 1;
    }
    // Rust's parser is correctly rounded; GM8 (Delphi) may round differently in the last bit for long
    // decimals. Spelunky's literals are short, so this is noted rather than handled; verify against oracle.
    Tok::Real(
        source[start..*i]
            .parse()
            .expect("digits and one point always parse"),
    )
}

fn lex_punct(b: u8, next: Option<u8>) -> Option<(Tok, usize)> {
    let two = |tok| Some((tok, 2));
    let one = |tok| Some((tok, 1));
    match (b, next) {
        (b'=', Some(b'=')) => two(Tok::Op(Op::Eq)),
        (b'!', Some(b'=')) => two(Tok::Op(Op::Ne)),
        (b'<', Some(b'>')) => two(Tok::Op(Op::Ne)),
        (b'<', Some(b'=')) => two(Tok::Op(Op::Le)),
        (b'>', Some(b'=')) => two(Tok::Op(Op::Ge)),
        (b'<', Some(b'<')) => two(Tok::Op(Op::Shl)),
        (b'>', Some(b'>')) => two(Tok::Op(Op::Shr)),
        (b'&', Some(b'&')) => two(Tok::Op(Op::And)),
        (b'|', Some(b'|')) => two(Tok::Op(Op::Or)),
        (b'^', Some(b'^')) => two(Tok::Op(Op::Xor)),
        (b'+', Some(b'=')) => two(Tok::Op(Op::AddAssign)),
        (b'-', Some(b'=')) => two(Tok::Op(Op::SubAssign)),
        (b'*', Some(b'=')) => two(Tok::Op(Op::MulAssign)),
        (b'/', Some(b'=')) => two(Tok::Op(Op::DivAssign)),
        (b'&', Some(b'=')) => two(Tok::Op(Op::AndAssign)),
        (b'|', Some(b'=')) => two(Tok::Op(Op::OrAssign)),
        (b'^', Some(b'=')) => two(Tok::Op(Op::XorAssign)),
        (b':', Some(b'=')) => two(Tok::Op(Op::Assign)),
        (b'=', _) => one(Tok::Op(Op::Assign)),
        (b'<', _) => one(Tok::Op(Op::Lt)),
        (b'>', _) => one(Tok::Op(Op::Gt)),
        (b'+', _) => one(Tok::Op(Op::Add)),
        (b'-', _) => one(Tok::Op(Op::Sub)),
        (b'*', _) => one(Tok::Op(Op::Mul)),
        (b'/', _) => one(Tok::Op(Op::Div)),
        (b'%', _) => one(Tok::Op(Op::Mod)),
        (b'!', _) => one(Tok::Op(Op::Not)),
        (b'&', _) => one(Tok::Op(Op::BitAnd)),
        (b'|', _) => one(Tok::Op(Op::BitOr)),
        (b'^', _) => one(Tok::Op(Op::BitXor)),
        (b'~', _) => one(Tok::Op(Op::BitNot)),
        (b'{', _) => one(Tok::LBrace),
        (b'}', _) => one(Tok::RBrace),
        (b'(', _) => one(Tok::LParen),
        (b')', _) => one(Tok::RParen),
        (b'[', _) => one(Tok::LBracket),
        (b']', _) => one(Tok::RBracket),
        (b',', _) => one(Tok::Comma),
        (b';', _) => one(Tok::Semi),
        (b':', _) => one(Tok::Colon),
        (b'.', _) => one(Tok::Dot),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Tok> {
        tokenize(src).unwrap().into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn word_operators_alias_symbols() {
        assert_eq!(toks("a and b or c xor not d"), toks("a && b || c ^^ ! d"),);
        assert_eq!(toks("begin end"), vec![Tok::LBrace, Tok::RBrace, Tok::Eof]);
        assert_eq!(toks("a <> b")[1], Tok::Op(Op::Ne));
        assert_eq!(toks("a := b")[1], Tok::Op(Op::Assign));
        assert_eq!(toks("a div b mod c")[1], Tok::Op(Op::IntDiv));
    }

    #[test]
    fn numbers() {
        assert_eq!(
            toks("$1F #ff 1.5 .25 3"),
            vec![
                Tok::Real(31.0),
                Tok::Real(255.0),
                Tok::Real(1.5),
                Tok::Real(0.25),
                Tok::Real(3.0),
                Tok::Eof
            ]
        );
        assert_eq!(
            toks("100001.x")[..3],
            [Tok::Real(100001.0), Tok::Dot, Tok::Ident("x".into())]
        );
        // A second point starts a new token (here, a field access on the literal).
        assert_eq!(
            toks("1.5.x")[..3],
            [Tok::Real(1.5), Tok::Dot, Tok::Ident("x".into())]
        );
    }

    #[test]
    fn strings_have_no_escapes() {
        // The backslash is an ordinary character, so the first string ends at the second `"`.
        assert_eq!(
            toks(r#""a\" 'b'"#),
            vec![Tok::Str("a\\".into()), Tok::Str("b".into()), Tok::Eof]
        );
        assert_eq!(toks("'it\"s'"), vec![Tok::Str("it\"s".into()), Tok::Eof]);
        assert_eq!(
            toks("\"two\r\nlines\""),
            vec![Tok::Str("two\r\nlines".into()), Tok::Eof]
        );
    }

    #[test]
    fn comments() {
        assert_eq!(
            toks("a // b\r\nc /* d\n e */ f /* open"),
            vec![
                Tok::Ident("a".into()),
                Tok::Ident("c".into()),
                Tok::Ident("f".into()),
                Tok::Eof
            ]
        );
    }

    #[test]
    fn errors_have_spans() {
        let err = tokenize("x = \"open").unwrap_err();
        assert_eq!(err.span, Span::new(4, 9));
        assert!(tokenize("a @ b").is_err());
        // A stray multi-byte character is reported whole.
        let src = "a é b";
        let err = tokenize(src).unwrap_err();
        assert_eq!(err.span.text(src), "é");
    }
}
