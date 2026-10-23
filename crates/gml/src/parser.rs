//! Recursive-descent parser for GM8 GML.
//!
//! GM8 parsing quirks handled here:
//! - Semicolons are optional between statements, and extra ones are ignored.
//! - `if cond [then] stmt`, `while cond [do] stmt`, `with target [do] stmt`; conditions need no parentheses.
//! - A statement-level `=` (or `:=`) is assignment; inside an expression it is equality.
//! - `var a, b` and `var a b` both declare two locals.
//! - `case x:` and `default:` are statements inside the switch body.
//! - `for (init; cond; step)` where the semicolons are optional.
//! - An assignment target can start with a parenthesised expression: `(obj).x = 1`.

use crate::ast::*;
use crate::lexer::{Kw, Op, Tok, Token, tokenize};
use crate::span::Span;
use crate::{Error, Result};

pub(crate) struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub(crate) fn new(source: &str) -> Result<Self> {
        Ok(Self {
            tokens: tokenize(source)?,
            pos: 0,
        })
    }

    pub(crate) fn program(&mut self) -> Result<Program> {
        let mut body = Vec::new();
        loop {
            self.skip_semis();
            if self.at(&Tok::Eof) {
                return Ok(Program { body });
            }
            body.push(self.statement()?);
        }
    }

    /// A single expression filling the whole input (a DnD argument).
    pub(crate) fn whole_expression(&mut self) -> Result<Expr> {
        let expr = self.expr()?;
        if !self.at(&Tok::Eof) {
            return Err(self.unexpected("end of expression"));
        }
        Ok(expr)
    }

    // ---- token helpers ----

    fn peek(&self) -> &Tok {
        &self.tokens[self.pos].tok
    }

    fn peek_at(&self, n: usize) -> &Tok {
        let i = (self.pos + n).min(self.tokens.len() - 1);
        &self.tokens[i].tok
    }

    fn span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn prev_span(&self) -> Span {
        self.tokens[self.pos.saturating_sub(1)].span
    }

    fn at(&self, tok: &Tok) -> bool {
        self.peek() == tok
    }

    fn bump(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        token
    }

    fn eat(&mut self, tok: &Tok) -> bool {
        if self.at(tok) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, tok: &Tok, what: &str) -> Result<Span> {
        if self.at(tok) {
            Ok(self.bump().span)
        } else {
            Err(self.unexpected(what))
        }
    }

    fn unexpected(&self, expected: &str) -> Error {
        Error::new(
            format!("expected {expected}, found {}", describe(self.peek())),
            self.span(),
        )
    }

    fn skip_semis(&mut self) {
        while self.eat(&Tok::Semi) {}
    }

    // ---- statements ----

    /// One statement, plus any semicolons after it.
    fn statement(&mut self) -> Result<Stmt> {
        let stmt = self.statement_inner()?;
        self.skip_semis();
        Ok(stmt)
    }

    /// The body of `if`/`while`/`with`/...: a lone `;` is an empty statement.
    fn body(&mut self) -> Result<Box<Stmt>> {
        if self.at(&Tok::Semi) {
            let span = self.bump().span;
            self.skip_semis();
            return Ok(Box::new(Stmt {
                kind: StmtKind::Empty,
                span,
            }));
        }
        if self.at(&Tok::Eof) {
            return Err(self.unexpected("a statement"));
        }
        Ok(Box::new(self.statement()?))
    }

    fn statement_inner(&mut self) -> Result<Stmt> {
        let start = self.span();
        let kind = match self.peek().clone() {
            Tok::LBrace => {
                self.bump();
                let body = self.block_items()?;
                StmtKind::Block(body)
            }
            Tok::Kw(kw) => match kw {
                Kw::Var | Kw::GlobalVar => {
                    self.bump();
                    let names = self.var_names()?;
                    if kw == Kw::Var {
                        StmtKind::Var(names)
                    } else {
                        StmtKind::GlobalVar(names)
                    }
                }
                Kw::If => {
                    self.bump();
                    let cond = self.expr()?;
                    self.eat(&Tok::Kw(Kw::Then));
                    let then = self.body()?;
                    let otherwise = if self.eat(&Tok::Kw(Kw::Else)) {
                        Some(self.body()?)
                    } else {
                        None
                    };
                    StmtKind::If {
                        cond,
                        then,
                        otherwise,
                    }
                }
                Kw::While => {
                    self.bump();
                    let cond = self.expr()?;
                    self.eat(&Tok::Kw(Kw::Do));
                    StmtKind::While {
                        cond,
                        body: self.body()?,
                    }
                }
                Kw::Do => {
                    self.bump();
                    let body = self.body()?;
                    self.expect(&Tok::Kw(Kw::Until), "`until`")?;
                    StmtKind::DoUntil {
                        body,
                        cond: self.expr()?,
                    }
                }
                Kw::For => {
                    self.bump();
                    self.expect(&Tok::LParen, "`(` after `for`")?;
                    let init = Box::new(self.statement_inner()?);
                    self.skip_semis();
                    let cond = self.expr()?;
                    self.skip_semis();
                    let step = Box::new(self.statement_inner()?);
                    self.skip_semis();
                    self.expect(&Tok::RParen, "`)` closing `for`")?;
                    StmtKind::For {
                        init,
                        cond,
                        step,
                        body: self.body()?,
                    }
                }
                Kw::Repeat => {
                    self.bump();
                    let count = self.expr()?;
                    StmtKind::Repeat {
                        count,
                        body: self.body()?,
                    }
                }
                Kw::With => {
                    self.bump();
                    let target = self.expr()?;
                    self.eat(&Tok::Kw(Kw::Do));
                    StmtKind::With {
                        target,
                        body: self.body()?,
                    }
                }
                Kw::Switch => {
                    self.bump();
                    let value = self.expr()?;
                    self.expect(&Tok::LBrace, "`{` opening the switch body")?;
                    StmtKind::Switch {
                        value,
                        body: self.block_items()?,
                    }
                }
                Kw::Case => {
                    self.bump();
                    let value = self.expr()?;
                    self.expect(&Tok::Colon, "`:` after `case` value")?;
                    StmtKind::Case(value)
                }
                Kw::Default => {
                    self.bump();
                    self.expect(&Tok::Colon, "`:` after `default`")?;
                    StmtKind::Default
                }
                Kw::Break => {
                    self.bump();
                    StmtKind::Break
                }
                Kw::Continue => {
                    self.bump();
                    StmtKind::Continue
                }
                Kw::Exit => {
                    self.bump();
                    StmtKind::Exit
                }
                Kw::Return => {
                    self.bump();
                    StmtKind::Return(self.expr()?)
                }
                Kw::SelfKw
                | Kw::Other
                | Kw::All
                | Kw::Noone
                | Kw::Global
                | Kw::True
                | Kw::False => self.assignment_or_call()?,
                Kw::Then | Kw::Else | Kw::Until => return Err(self.unexpected("a statement")),
            },
            Tok::Ident(_) | Tok::LParen | Tok::Real(_) => self.assignment_or_call()?,
            _ => return Err(self.unexpected("a statement")),
        };
        Ok(Stmt {
            kind,
            span: start.to(self.prev_span()),
        })
    }

    /// Statements up to the closing `}`/`end` (the opening brace is already consumed).
    fn block_items(&mut self) -> Result<Vec<Stmt>> {
        let mut body = Vec::new();
        loop {
            self.skip_semis();
            if self.eat(&Tok::RBrace) {
                return Ok(body);
            }
            if self.at(&Tok::Eof) {
                return Err(self.unexpected("`}` or `end`"));
            }
            body.push(self.statement()?);
        }
    }

    /// Names after `var`/`globalvar`. Commas are optional, so a following identifier is another name
    /// unless it starts the next statement (it is followed by `(`, `.`, `[` or an assignment operator).
    fn var_names(&mut self) -> Result<Vec<Ident>> {
        let mut names = Vec::new();
        loop {
            let Tok::Ident(name) = self.peek().clone() else {
                if names.is_empty() {
                    return Err(self.unexpected("a variable name"));
                }
                return Ok(names);
            };
            if !names.is_empty() && starts_statement(self.peek_at(1)) {
                return Ok(names);
            }
            let span = self.bump().span;
            names.push(Ident { name, span });
            if self.eat(&Tok::Comma) && !matches!(self.peek(), Tok::Ident(_)) {
                return Err(self.unexpected("a variable name after `,`"));
            }
        }
    }

    fn assignment_or_call(&mut self) -> Result<StmtKind> {
        let place = self.postfix()?;
        if let Some(op) = assign_op(self.peek()) {
            if !matches!(
                place.kind,
                ExprKind::Ident(_) | ExprKind::Field { .. } | ExprKind::Index { .. }
            ) {
                return Err(Error::new("cannot assign to this expression", place.span));
            }
            self.bump();
            let value = self.expr()?;
            return Ok(StmtKind::Assign { place, op, value });
        }
        if matches!(place.kind, ExprKind::Call { .. }) {
            return Ok(StmtKind::Call(place));
        }
        Err(self.unexpected("an assignment operator"))
    }

    // ---- expressions ----

    pub(crate) fn expr(&mut self) -> Result<Expr> {
        self.binary(0)
    }

    /// Precedence climbing; every level is left-associative.
    fn binary(&mut self, min_prec: u8) -> Result<Expr> {
        let mut lhs = self.unary()?;
        while let Some(op) = binary_op(self.peek()) {
            let prec = op.precedence();
            if prec < min_prec {
                break;
            }
            self.bump();
            let rhs = self.binary(prec + 1)?;
            let span = lhs.span.to(rhs.span);
            lhs = Expr {
                kind: ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            };
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr> {
        let op = match self.peek() {
            Tok::Op(Op::Not) => UnOp::Not,
            Tok::Op(Op::Sub) => UnOp::Neg,
            Tok::Op(Op::Add) => UnOp::Plus,
            Tok::Op(Op::BitNot) => UnOp::BitNot,
            _ => return self.postfix(),
        };
        let start = self.bump().span;
        let expr = self.unary()?;
        let span = start.to(expr.span);
        Ok(Expr {
            kind: ExprKind::Unary {
                op,
                expr: Box::new(expr),
            },
            span,
        })
    }

    /// A primary expression followed by any number of `.field` and `[index]` suffixes.
    fn postfix(&mut self) -> Result<Expr> {
        let mut expr = self.primary()?;
        loop {
            if self.eat(&Tok::Dot) {
                let Tok::Ident(name) = self.peek().clone() else {
                    return Err(self.unexpected("a field name after `.`"));
                };
                let name_span = self.bump().span;
                let span = expr.span.to(name_span);
                let name = Ident {
                    name,
                    span: name_span,
                };
                expr = Expr {
                    kind: ExprKind::Field {
                        base: Box::new(expr),
                        name,
                    },
                    span,
                };
            } else if self.eat(&Tok::LBracket) {
                let mut indices = vec![self.expr()?];
                if self.eat(&Tok::Comma) {
                    indices.push(self.expr()?);
                }
                let end = self.expect(&Tok::RBracket, "`]`")?;
                let span = expr.span.to(end);
                expr = Expr {
                    kind: ExprKind::Index {
                        base: Box::new(expr),
                        indices,
                    },
                    span,
                };
            } else {
                return Ok(expr);
            }
        }
    }

    fn primary(&mut self) -> Result<Expr> {
        // Check before consuming, so the error points at the offending token (or the end of the code).
        let starts_expr = matches!(
            self.peek(),
            Tok::Real(_)
                | Tok::Str(_)
                | Tok::Ident(_)
                | Tok::LParen
                | Tok::Kw(
                    Kw::True
                        | Kw::False
                        | Kw::SelfKw
                        | Kw::Other
                        | Kw::All
                        | Kw::Noone
                        | Kw::Global
                )
        );
        if !starts_expr {
            return Err(self.unexpected("an expression"));
        }
        let token = self.bump();
        let span = token.span;
        let kind = match token.tok {
            Tok::Real(v) => ExprKind::Real(v),
            Tok::Str(s) => ExprKind::Str(s),
            Tok::Kw(Kw::True) => ExprKind::Const(Const::True),
            Tok::Kw(Kw::False) => ExprKind::Const(Const::False),
            Tok::Kw(Kw::SelfKw) => ExprKind::Const(Const::SelfKw),
            Tok::Kw(Kw::Other) => ExprKind::Const(Const::Other),
            Tok::Kw(Kw::All) => ExprKind::Const(Const::All),
            Tok::Kw(Kw::Noone) => ExprKind::Const(Const::Noone),
            Tok::Kw(Kw::Global) => ExprKind::Const(Const::Global),
            Tok::Ident(name) if self.at(&Tok::LParen) => {
                self.bump();
                let mut args = Vec::new();
                if !self.at(&Tok::RParen) {
                    loop {
                        args.push(self.expr()?);
                        if !self.eat(&Tok::Comma) {
                            break;
                        }
                    }
                }
                let end = self.expect(&Tok::RParen, "`,` or `)` in argument list")?;
                return Ok(Expr {
                    kind: ExprKind::Call {
                        name: Ident { name, span },
                        args,
                    },
                    span: span.to(end),
                });
            }
            Tok::Ident(name) => ExprKind::Ident(name),
            Tok::LParen => {
                let inner = self.expr()?;
                let end = self.expect(&Tok::RParen, "`)`")?;
                // Parentheses only group; keep the inner node but widen its span to include them.
                return Ok(Expr {
                    kind: inner.kind,
                    span: span.to(end),
                });
            }
            _ => unreachable!("checked by `starts_expr`"),
        };
        Ok(Expr { kind, span })
    }
}

fn starts_statement(next: &Tok) -> bool {
    matches!(next, Tok::LParen | Tok::Dot | Tok::LBracket) || assign_op(next).is_some()
}

fn assign_op(tok: &Tok) -> Option<AssignOp> {
    Some(match tok {
        Tok::Op(Op::Assign) => AssignOp::Set,
        Tok::Op(Op::AddAssign) => AssignOp::Add,
        Tok::Op(Op::SubAssign) => AssignOp::Sub,
        Tok::Op(Op::MulAssign) => AssignOp::Mul,
        Tok::Op(Op::DivAssign) => AssignOp::Div,
        Tok::Op(Op::AndAssign) => AssignOp::BitAnd,
        Tok::Op(Op::OrAssign) => AssignOp::BitOr,
        Tok::Op(Op::XorAssign) => AssignOp::BitXor,
        _ => return None,
    })
}

fn binary_op(tok: &Tok) -> Option<BinOp> {
    let Tok::Op(op) = tok else { return None };
    Some(match op {
        Op::And => BinOp::And,
        Op::Or => BinOp::Or,
        Op::Xor => BinOp::Xor,
        // `=` inside an expression is comparison.
        Op::Assign | Op::Eq => BinOp::Eq,
        Op::Ne => BinOp::Ne,
        Op::Lt => BinOp::Lt,
        Op::Le => BinOp::Le,
        Op::Gt => BinOp::Gt,
        Op::Ge => BinOp::Ge,
        Op::BitAnd => BinOp::BitAnd,
        Op::BitOr => BinOp::BitOr,
        Op::BitXor => BinOp::BitXor,
        Op::Shl => BinOp::Shl,
        Op::Shr => BinOp::Shr,
        Op::Add => BinOp::Add,
        Op::Sub => BinOp::Sub,
        Op::Mul => BinOp::Mul,
        Op::Div => BinOp::Div,
        Op::IntDiv => BinOp::IntDiv,
        Op::Mod => BinOp::Mod,
        _ => return None,
    })
}

fn describe(tok: &Tok) -> String {
    match tok {
        Tok::Ident(name) => format!("identifier `{name}`"),
        Tok::Real(v) => format!("number {v}"),
        Tok::Str(_) => "a string".to_owned(),
        Tok::Kw(kw) => format!("keyword `{}`", format!("{kw:?}").to_lowercase()),
        Tok::Op(op) => format!("operator {op:?}"),
        Tok::Eof => "end of code".to_owned(),
        other => format!("{other:?}"),
    }
}
