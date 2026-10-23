//! Prints an AST back as canonical GML: C-style braces and operators, one statement per line, and only the
//! parentheses that GM8's precedence needs. Used for round-trip tests and for quoting GML in generated
//! code and diagnostics.

use std::fmt::Write;

use crate::ast::*;

pub fn program(program: &Program) -> String {
    let mut out = String::new();
    for stmt in &program.body {
        write_stmt(&mut out, stmt, 0);
    }
    out
}

pub fn expr(expr: &Expr) -> String {
    let mut out = String::new();
    write_expr(&mut out, expr);
    out
}

/// One statement on a single line without its nested bodies, e.g. `if (a > 1)` or `x += 1;`. Useful as a
/// source comment next to translated code.
pub fn stmt_head(stmt: &Stmt) -> String {
    match &stmt.kind {
        StmtKind::If { cond, .. } => format!("if ({})", expr(cond)),
        StmtKind::While { cond, .. } => format!("while ({})", expr(cond)),
        StmtKind::DoUntil { cond, .. } => format!("do ... until ({})", expr(cond)),
        StmtKind::For { cond, .. } => format!("for (...; {}; ...)", expr(cond)),
        StmtKind::Repeat { count, .. } => format!("repeat ({})", expr(count)),
        StmtKind::With { target, .. } => format!("with ({})", expr(target)),
        StmtKind::Switch { value, .. } => format!("switch ({})", expr(value)),
        StmtKind::Block(_) => "{ ... }".to_owned(),
        _ => {
            let mut out = String::new();
            write_stmt(&mut out, stmt, 0);
            out.trim_end().to_owned()
        }
    }
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("    ");
    }
}

/// Writes a statement body after its header, leaving the cursor at the end of the body's last line. Blocks
/// stay blocks; any other statement goes on its own indented line without braces, so re-parsing the output
/// gives back the same tree.
fn write_body(out: &mut String, body: &Stmt, depth: usize) {
    if let StmtKind::Block(stmts) = &body.kind {
        out.push_str(" {\n");
        for s in stmts {
            write_stmt(out, s, depth + 1);
        }
        indent(out, depth);
        out.push('}');
    } else {
        out.push('\n');
        write_stmt(out, body, depth + 1);
        out.pop();
    }
}

/// Continues after a body written by [`write_body`] with a keyword such as `else` or `until`.
fn continue_after(out: &mut String, body: &Stmt, depth: usize, keyword: &str) {
    if matches!(body.kind, StmtKind::Block(_)) {
        out.push(' ');
    } else {
        out.push('\n');
        indent(out, depth);
    }
    out.push_str(keyword);
}

fn names(list: &[Ident]) -> String {
    list.iter()
        .map(|n| n.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn write_stmt(out: &mut String, stmt: &Stmt, depth: usize) {
    indent(out, depth);
    match &stmt.kind {
        StmtKind::Block(body) => {
            out.push_str("{\n");
            for s in body {
                write_stmt(out, s, depth + 1);
            }
            indent(out, depth);
            out.push('}');
        }
        StmtKind::Var(list) => write!(out, "var {};", names(list)).unwrap(),
        StmtKind::GlobalVar(list) => write!(out, "globalvar {};", names(list)).unwrap(),
        StmtKind::Assign { place, op, value } => {
            write!(out, "{} {} {};", expr(place), op.as_str(), expr(value)).unwrap();
        }
        StmtKind::Call(call) => write!(out, "{};", expr(call)).unwrap(),
        StmtKind::If {
            cond,
            then,
            otherwise,
        } => {
            write!(out, "if ({})", expr(cond)).unwrap();
            write_body(out, then, depth);
            if let Some(o) = otherwise {
                continue_after(out, then, depth, "else");
                write_body(out, o, depth);
            }
        }
        StmtKind::While { cond, body } => {
            write!(out, "while ({})", expr(cond)).unwrap();
            write_body(out, body, depth);
        }
        StmtKind::DoUntil { body, cond } => {
            out.push_str("do");
            write_body(out, body, depth);
            continue_after(out, body, depth, "until");
            write!(out, " ({});", expr(cond)).unwrap();
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
        } => {
            let head = |s: &Stmt| {
                let mut t = String::new();
                write_stmt(&mut t, s, 0);
                t.trim_end().trim_end_matches(';').to_owned()
            };
            write!(out, "for ({}; {}; {})", head(init), expr(cond), head(step)).unwrap();
            write_body(out, body, depth);
        }
        StmtKind::Repeat { count, body } => {
            write!(out, "repeat ({})", expr(count)).unwrap();
            write_body(out, body, depth);
        }
        StmtKind::With { target, body } => {
            write!(out, "with ({})", expr(target)).unwrap();
            write_body(out, body, depth);
        }
        StmtKind::Switch { value, body } => {
            writeln!(out, "switch ({}) {{", expr(value)).unwrap();
            for s in body {
                write_stmt(out, s, depth + 1);
            }
            indent(out, depth);
            out.push('}');
        }
        StmtKind::Case(value) => write!(out, "case {}:", expr(value)).unwrap(),
        StmtKind::Default => out.push_str("default:"),
        StmtKind::Break => out.push_str("break;"),
        StmtKind::Continue => out.push_str("continue;"),
        StmtKind::Exit => out.push_str("exit;"),
        StmtKind::Return(value) => write!(out, "return {};", expr(value)).unwrap(),
        StmtKind::Empty => out.push(';'),
    }
    out.push('\n');
}

fn write_real(out: &mut String, v: f64) {
    // `{}` prints the shortest text that parses back to the same f64.
    write!(out, "{v}").unwrap();
}

fn write_str(out: &mut String, s: &str) {
    // GM8 strings have no escapes, so pick the quote the text doesn't contain.
    let q = if s.contains('"') { '\'' } else { '"' };
    write!(out, "{q}{s}{q}").unwrap();
}

/// Writes `e`, parenthesised if it is a binary or unary expression (as an operand of a postfix or unary
/// operator).
fn write_tight(out: &mut String, e: &Expr) {
    if matches!(e.kind, ExprKind::Binary { .. } | ExprKind::Unary { .. }) {
        out.push('(');
        write_expr(out, e);
        out.push(')');
    } else {
        write_expr(out, e);
    }
}

fn write_operand(out: &mut String, e: &Expr, parent: u8, right: bool) {
    let needs_parens = match &e.kind {
        // Left-associative: an equal-precedence right operand needs parentheses.
        ExprKind::Binary { op, .. } => {
            op.precedence() < parent || (right && op.precedence() == parent)
        }
        _ => false,
    };
    if needs_parens {
        out.push('(');
        write_expr(out, e);
        out.push(')');
    } else {
        write_expr(out, e);
    }
}

fn write_expr(out: &mut String, e: &Expr) {
    match &e.kind {
        ExprKind::Real(v) => write_real(out, *v),
        ExprKind::Str(s) => write_str(out, s),
        ExprKind::Const(c) => out.push_str(c.as_str()),
        ExprKind::Ident(name) => out.push_str(name),
        ExprKind::Field { base, name } => {
            // A number base needs parentheses so `1.x` doesn't lex as a decimal.
            if matches!(base.kind, ExprKind::Real(_)) {
                out.push('(');
                write_expr(out, base);
                out.push(')');
            } else {
                write_tight(out, base);
            }
            out.push('.');
            out.push_str(&name.name);
        }
        ExprKind::Index { base, indices } => {
            write_tight(out, base);
            out.push('[');
            for (i, idx) in indices.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_expr(out, idx);
            }
            out.push(']');
        }
        ExprKind::Call { name, args } => {
            out.push_str(&name.name);
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_expr(out, a);
            }
            out.push(')');
        }
        ExprKind::Unary { op, expr } => {
            out.push_str(op.as_str());
            write_tight(out, expr);
        }
        ExprKind::Binary { op, lhs, rhs } => {
            let p = op.precedence();
            write_operand(out, lhs, p, false);
            write!(out, " {} ", op.as_str()).unwrap();
            write_operand(out, rhs, p, true);
        }
    }
}
