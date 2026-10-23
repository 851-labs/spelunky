//! Prototype GML-to-Rust transpiler (851-3121). Emits Rust against the runtime API sketched in
//! `proto_rt.rs` / `docs/design/runtime.md`. Not the production translator: it exists to measure coverage,
//! output size, and readability, and to check that the target API is expressive enough.
//!
//! ```sh
//! # Print the translation of some units (unit-name prefixes; `name/` for a whole object):
//! scripts/cargo run -p gml --example transpile -- oSnake/ scripts/rand
//! # Translate the whole game into a standalone crate and type-check it:
//! scripts/cargo run -p gml --example transpile -- --crate /tmp/proto-game
//! scripts/cargo check --manifest-path /tmp/proto-game/Cargo.toml
//! ```

#[path = "../common/mod.rs"]
mod common;

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use common::{NameKind, Source, locals_of, spelunky};
use gml::ast::*;
use gml::{LineIndex, print};

const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
    "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro", "override",
    "priv", "typeof", "unsized", "virtual", "yield", "try", "gen",
];

/// A GML name as a Rust identifier: unchanged where possible so generated code greps like the GML.
fn rid(name: &str) -> String {
    if RUST_KEYWORDS.contains(&name) {
        format!("r#{name}")
    } else {
        name.to_owned()
    }
}

/// Names the generated code refers to, collected while emitting, so the crate mode can declare them.
#[derive(Default)]
struct Symbols {
    vars: BTreeSet<String>,
    gvars: BTreeSet<String>,
    builtins: BTreeSet<String>,
    constants: BTreeSet<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum Ty {
    Value,
    Bool,
}

/// What `break`/`continue` refer to at a point in the code. Every loop is labeled, because Rust needs a
/// label on any `break` that passes through a labeled block.
#[derive(Clone)]
enum Scope {
    /// `while`, `repeat`, `with`: `break 'l` / `continue 'l`.
    Loop(String),
    /// `for` and `do-until`: `continue` must still run the step / the condition, so it breaks out of a
    /// labeled block around the body (the second label).
    LoopWithTail(String, String),
    /// `switch`: `break` leaves the labeled switch block; `continue` goes to the enclosing loop.
    Switch(String),
}

struct Emitter<'a> {
    src: &'a Source,
    unit: &'a str,
    lines: LineIndex,
    locals: BTreeSet<String>,
    is_script: bool,
    /// Scripts that can end without `return` give back the last assigned or called value (GM8 quirk).
    track_ret: bool,
    scopes: Vec<Scope>,
    label: usize,
    tmp: usize,
    out: String,
    syms: &'a mut Symbols,
}

impl Emitter<'_> {
    fn line(&mut self, depth: usize, text: &str) {
        for _ in 0..depth {
            self.out.push_str("    ");
        }
        self.out.push_str(text);
        self.out.push('\n');
    }

    fn fresh(&mut self, prefix: &str) -> String {
        self.tmp += 1;
        format!("{prefix}{}", self.tmp)
    }

    fn exit_code(&self) -> &'static str {
        if self.is_script {
            "return Ok(cx.ret.clone());"
        } else {
            "return Ok(());"
        }
    }

    // ---- expressions ----

    fn value(&mut self, e: &Expr) -> String {
        let (code, ty) = self.expr(e);
        match ty {
            Ty::Value => code,
            Ty::Bool => format!("b2v({code})"),
        }
    }

    fn cond(&mut self, e: &Expr) -> String {
        let (code, ty) = self.expr(e);
        match ty {
            Ty::Bool => code,
            Ty::Value => format!("truthy(&{code})"),
        }
    }

    /// Array indices as `u32` expressions; literal indices are emitted as plain numbers.
    fn index(&mut self, indices: &[Expr]) -> (String, String) {
        let mut one = |e: &Expr| match e.kind {
            ExprKind::Real(v) if v >= 0.0 && v.fract() == 0.0 => format!("{v}"),
            _ => format!("ix({})?", self.value(e)),
        };
        let i = one(&indices[0]);
        let j = indices.get(1).map_or("0".to_owned(), one);
        (i, j)
    }

    fn var(&mut self, name: &str) -> String {
        self.syms.vars.insert(name.to_owned());
        format!("v::{}", rid(name))
    }

    fn gvar(&mut self, name: &str) -> String {
        self.syms.gvars.insert(name.to_owned());
        format!("gv::{}", rid(name))
    }

    fn ident(&mut self, name: &str) -> String {
        match self.src.classify(name, &self.locals) {
            NameKind::Local => format!("l_{name}.get()?"),
            NameKind::Argument(n) => format!("cx.arg({n})?"),
            NameKind::InstanceBuiltin | NameKind::InstanceVar => {
                let v = self.var(name);
                format!("g.get(cx.this, {v})?")
            }
            NameKind::GlobalBuiltin => {
                let v = self.var(name);
                format!("g.bget({v}, 0)?")
            }
            NameKind::GlobalVar => {
                let v = self.gvar(name);
                format!("g.gget({v})?")
            }
            NameKind::Constant => {
                self.syms.constants.insert(name.to_owned());
                format!("k::{name}")
            }
            NameKind::Object => format!("obj::{name}"),
            NameKind::Sprite => format!("spr::{name}"),
            NameKind::Room => format!("rm::{name}"),
            NameKind::Background => format!("bgr::{name}"),
            NameKind::Script => format!("scr_id::{name}"),
        }
    }

    /// A variable read: `name`, `base.name`, `name[i, j]` or `base.name[i, j]`. `other`/`self`/`global`
    /// bases resolve statically; any other base is resolved at run time. GM8 evaluates the base, then the
    /// indices. When those contain calls (which need `&mut g`) they are bound first.
    fn read(&mut self, base: Option<&Expr>, name: &str, indices: Option<&[Expr]>) -> String {
        let hoist = base.is_some_and(has_call) || indices.is_some_and(|ix| ix.iter().any(has_call));
        let mut lets = Vec::new();
        let target = match base.map(|b| &b.kind) {
            None => None,
            Some(ExprKind::Const(Const::Global)) => Some("global".to_owned()),
            Some(ExprKind::Const(Const::Other)) => Some("cx.other".to_owned()),
            Some(ExprKind::Const(Const::SelfKw)) => Some("cx.this".to_owned()),
            Some(_) => {
                let b = self.value(base.unwrap());
                if hoist {
                    lets.push(format!("let t = {b};"));
                    Some("&t".to_owned())
                } else {
                    Some(format!("&{b}"))
                }
            }
        };
        let idx = indices.map(|ix| {
            let (i, j) = self.index(ix);
            if hoist {
                lets.push(format!("let i = {i};"));
                lets.push(format!("let j = {j};"));
                ("i".to_owned(), "j".to_owned())
            } else {
                (i, j)
            }
        });
        let (sfx, ix_args) = match &idx {
            Some((i, j)) => ("_a", format!(", {i}, {j}")),
            None => ("", String::new()),
        };
        let call = match target.as_deref() {
            Some("global") => {
                let v = self.gvar(name);
                format!("g.gget{sfx}({v}{ix_args})?")
            }
            Some(who @ ("cx.other" | "cx.this")) => {
                let v = self.var(name);
                format!("g.get{sfx}({who}, {v}{ix_args})?")
            }
            Some(t) => {
                let v = self.var(name);
                format!("g.get_on{sfx}(cx, {t}, {v}{ix_args})?")
            }
            None => match self.src.classify(name, &self.locals) {
                NameKind::Local => match &idx {
                    None => format!("l_{name}.get()?"),
                    Some((i, j)) => format!("l_{name}.get_a({i}, {j})?"),
                },
                NameKind::GlobalBuiltin => {
                    let v = self.var(name);
                    let i = idx.as_ref().map_or("0", |(i, _)| i.as_str());
                    format!("g.bget({v}, {i})?")
                }
                NameKind::GlobalVar => {
                    let v = self.gvar(name);
                    format!("g.gget{sfx}({v}{ix_args})?")
                }
                NameKind::InstanceBuiltin | NameKind::InstanceVar => {
                    let v = self.var(name);
                    format!("g.get{sfx}(cx.this, {v}{ix_args})?")
                }
                _ if idx.is_none() => self.ident(name),
                other => panic!("{}: cannot index {other:?} `{name}`", self.unit),
            },
        };
        if lets.is_empty() {
            call
        } else {
            format!("{{ {} {call} }}", lets.join(" "))
        }
    }

    fn expr(&mut self, e: &Expr) -> (String, Ty) {
        let code = match &e.kind {
            ExprKind::Real(v) => format!("r({v:?})"),
            ExprKind::Str(s) => format!("s({s:?})"),
            ExprKind::Const(c) => format!("r({:?})", c.value()),
            ExprKind::Ident(name) => self.ident(name),
            ExprKind::Field { base, name } => self.read(Some(base), &name.name, None),
            ExprKind::Index { base, indices } => match &base.kind {
                ExprKind::Field { base, name } => self.read(Some(base), &name.name, Some(indices)),
                ExprKind::Ident(name) => self.read(None, name, Some(indices)),
                _ => panic!("{}: indexing a non-variable", self.unit),
            },
            ExprKind::Call { name, args } => self.call(&name.name, args),
            ExprKind::Unary { op, expr } => match op {
                UnOp::Not => {
                    let c = self.cond(expr);
                    return (format!("!{c}"), Ty::Bool);
                }
                // Negating a literal can't fail, so fold it: `-1` is `r(-1.0)`.
                UnOp::Neg => match expr.kind {
                    ExprKind::Real(v) => format!("r({:?})", -v),
                    _ => format!("neg({})?", self.value(expr)),
                },
                UnOp::BitNot => format!("bitnot({})?", self.value(expr)),
                UnOp::Plus => self.value(expr),
            },
            ExprKind::Binary { op, lhs, rhs } => {
                let f = match op {
                    BinOp::And | BinOp::Or | BinOp::Xor => {
                        // Plain functions: both operands are evaluated, as in GM8.
                        let (l, r) = (self.cond(lhs), self.cond(rhs));
                        let f = match op {
                            BinOp::And => "and",
                            BinOp::Or => "or",
                            _ => "xor",
                        };
                        return (format!("{f}({l}, {r})"), Ty::Bool);
                    }
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        let (l, r) = (self.value(lhs), self.value(rhs));
                        let f = format!("{op:?}").to_lowercase();
                        return (format!("{f}(&{l}, &{r})?"), Ty::Bool);
                    }
                    _ => arith_fn(*op),
                };
                let (l, r) = (self.value(lhs), self.value(rhs));
                format!("{f}({l}, {r})?")
            }
        };
        (code, Ty::Value)
    }

    fn call(&mut self, name: &str, args: &[Expr]) -> String {
        let vals: Vec<String> = args.iter().map(|a| self.value(a)).collect();
        // Arguments are bound first so the call can borrow `g` mutably; the order is GM8's left to right.
        let callee = if self.src.scripts.contains(name) {
            format!("scr::{}", rid(name))
        } else {
            self.syms.builtins.insert(name.to_owned());
            format!("b::{}", rid(name))
        };
        if vals.is_empty() {
            format!("{callee}(g, cx, &[])?")
        } else {
            format!("{{ let a = [{}]; {callee}(g, cx, &a)? }}", vals.join(", "))
        }
    }

    // ---- statements ----

    fn assign(&mut self, place: &Expr, op: AssignOp, value: &Expr, depth: usize) {
        // GM8 order (OpenGMK `compiler.rs` make_set/make_modify, `runtime.rs` SetField): resolve the target,
        // then the array index, then (for `op=`) read the old value, then evaluate the right-hand side.
        let (target, name, indices): (Option<&Expr>, &str, Option<&[Expr]>) = match &place.kind {
            ExprKind::Ident(n) => (None, n, None),
            ExprKind::Field { base, name } => (Some(base), &name.name, None),
            ExprKind::Index { base, indices } => match &base.kind {
                ExprKind::Ident(n) => (None, n, Some(indices)),
                ExprKind::Field { base, name } => (Some(base), &name.name, Some(indices)),
                _ => panic!("{}: bad assignment target", self.unit),
            },
            _ => panic!("{}: bad assignment target", self.unit),
        };
        let mut pre = Vec::new();
        let idx = indices.map(|ix| {
            let (i, j) = self.index(ix);
            pre.push(format!("let i = {i};"));
            pre.push(format!("let j = {j};"));
            ("i".to_owned(), "j".to_owned())
        });
        let kind = match target {
            None => self.src.classify(name, &self.locals),
            Some(base) => match &base.kind {
                ExprKind::Const(Const::Global) => NameKind::GlobalVar,
                _ => NameKind::InstanceVar,
            },
        };
        // Built-ins keep strict arithmetic for `op=`; user variables and locals ignore type errors.
        let builtin = matches!(kind, NameKind::InstanceBuiltin | NameKind::GlobalBuiltin)
            || (target.is_some() && common::INSTANCE_BUILTINS.contains(&name));
        let (read, write) = match (kind, target.map(|b| &b.kind)) {
            (NameKind::Local, None) => match &idx {
                None => (format!("l_{name}.get()?"), format!("l_{name}.set(x)")),
                Some((i, j)) => (
                    format!("l_{name}.get_a({i}, {j})?"),
                    format!("l_{name}.set_a({i}, {j}, x)"),
                ),
            },
            (NameKind::Argument(n), None) => {
                (format!("cx.arg({n})?"), format!("cx.set_arg({n}, x)?"))
            }
            (NameKind::GlobalBuiltin, None) => {
                let v = self.var(name);
                let i = idx.as_ref().map_or("0".to_owned(), |(i, _)| i.clone());
                (
                    format!("g.bget({v}, {i})?"),
                    format!("g.bset({v}, {i}, x)?"),
                )
            }
            (NameKind::GlobalVar, _) => {
                let v = self.gvar(name);
                match &idx {
                    None => (format!("g.gget({v})?"), format!("g.gset({v}, x)?")),
                    Some((i, j)) => (
                        format!("g.gget_a({v}, {i}, {j})?"),
                        format!("g.gset_a({v}, {i}, {j}, x)?"),
                    ),
                }
            }
            (
                _,
                None | Some(ExprKind::Const(Const::SelfKw)) | Some(ExprKind::Const(Const::Other)),
            ) => {
                let who = if matches!(target.map(|b| &b.kind), Some(ExprKind::Const(Const::Other)))
                {
                    "cx.other"
                } else {
                    "cx.this"
                };
                let v = self.var(name);
                match &idx {
                    None => (
                        format!("g.get({who}, {v})?"),
                        format!("g.set({who}, {v}, x)?"),
                    ),
                    Some((i, j)) => (
                        format!("g.get_a({who}, {v}, {i}, {j})?"),
                        format!("g.set_a({who}, {v}, {i}, {j}, x)?"),
                    ),
                }
            }
            (_, Some(_)) => {
                // `obj.x = ...` writes every instance of `obj`; `obj.x += ...` reads the first one. GM8
                // evaluates the base twice for `op=` (once to read, once to write).
                let base = target.unwrap();
                let t = self.value(base);
                pre.insert(0, format!("let t = {t};"));
                let v = self.var(name);
                match &idx {
                    None => (
                        format!("{{ let t2 = {t}; g.get_on(cx, &t2, {v})? }}"),
                        format!("g.set_on(cx, &t, {v}, x)?"),
                    ),
                    Some((i, j)) => (
                        format!("{{ let t2 = {t}; g.get_on_a(cx, &t2, {v}, {i}, {j})? }}"),
                        format!("g.set_on_a(cx, &t, {v}, {i}, {j}, x)?"),
                    ),
                }
            }
        };
        let rhs = self.value(value);
        let compute = match op {
            AssignOp::Set => format!("let x = {rhs};"),
            _ => {
                let f = arith_fn(match op {
                    AssignOp::Add => BinOp::Add,
                    AssignOp::Sub => BinOp::Sub,
                    AssignOp::Mul => BinOp::Mul,
                    AssignOp::Div => BinOp::Div,
                    AssignOp::BitAnd => BinOp::BitAnd,
                    AssignOp::BitOr => BinOp::BitOr,
                    AssignOp::BitXor => BinOp::BitXor,
                    AssignOp::Set => unreachable!(),
                });
                pre.push(format!("let old = {read};"));
                if builtin {
                    format!("let x = {f}(old, {rhs})?;")
                } else {
                    format!("let x = lenient({f}, old, {rhs});")
                }
            }
        };
        pre.push(compute);
        if self.track_ret {
            pre.push("cx.ret = x.clone();".to_owned());
        }
        pre.push(format!("{write};"));
        if pre.len() == 2 && op == AssignOp::Set && !self.track_ret {
            // The common case `a = b;`: one line.
            let line = format!("{{ {} {} }}", pre[0], pre[1]);
            self.line(depth, &line);
        } else {
            self.line(depth, "{");
            for p in pre {
                self.line(depth + 1, &p);
            }
            self.line(depth, "}");
        }
    }

    fn body(&mut self, stmt: &Stmt, depth: usize) {
        match &stmt.kind {
            StmtKind::Block(stmts) => {
                for s in stmts {
                    self.stmt(s, depth);
                }
            }
            _ => self.stmt(stmt, depth),
        }
    }

    fn stmt(&mut self, stmt: &Stmt, depth: usize) {
        let line_no = self.lines.line_col(stmt.span.start).line;
        if !matches!(
            stmt.kind,
            StmtKind::Block(_) | StmtKind::Var(_) | StmtKind::Empty
        ) {
            let head = print::stmt_head(stmt);
            let head: String = head
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(100)
                .collect();
            self.line(depth, &format!("// {line_no}: {head}"));
            let at = format!("g.at({:?}, {line_no});", self.unit);
            self.line(depth, &at);
        }
        match &stmt.kind {
            StmtKind::Block(stmts) => {
                self.line(depth, "{");
                for s in stmts {
                    self.stmt(s, depth + 1);
                }
                self.line(depth, "}");
            }
            StmtKind::Var(_) | StmtKind::Empty => {}
            StmtKind::GlobalVar(_) => {
                // Resolved statically: every `globalvar` name is a global everywhere (see runtime.md).
            }
            StmtKind::Assign { place, op, value } => self.assign(place, *op, value, depth),
            StmtKind::Call(call) => {
                let c = self.value(call);
                if self.track_ret {
                    self.line(depth, &format!("cx.ret = {c};"));
                } else {
                    self.line(depth, &format!("{c};"));
                }
            }
            StmtKind::If {
                cond,
                then,
                otherwise,
            } => {
                let c = self.cond(cond);
                self.line(depth, &format!("if {c} {{"));
                self.body(then, depth + 1);
                if let Some(o) = otherwise {
                    self.line(depth, "} else {");
                    self.body(o, depth + 1);
                }
                self.line(depth, "}");
            }
            StmtKind::While { cond, body } => {
                let c = self.cond(cond);
                let l = self.next_label("while");
                self.line(depth, &format!("{l}: while {c} {{"));
                self.scopes.push(Scope::Loop(l));
                self.body(body, depth + 1);
                self.scopes.pop();
                self.line(depth, "}");
            }
            StmtKind::Repeat { count, body } => {
                let n = self.value(count);
                let l = self.next_label("repeat");
                self.line(depth, &format!("{l}: for _ in 0..repeat_count({n})? {{"));
                self.scopes.push(Scope::Loop(l));
                self.body(body, depth + 1);
                self.scopes.pop();
                self.line(depth, "}");
            }
            StmtKind::DoUntil { body, cond } => {
                let l = self.next_label("do");
                let label = self.next_label("body");
                self.line(depth, &format!("{l}: loop {{"));
                self.line(depth + 1, &format!("{label}: {{"));
                self.scopes.push(Scope::LoopWithTail(l.clone(), label));
                self.body(body, depth + 2);
                self.scopes.pop();
                self.line(depth + 1, "}");
                let c = self.cond(cond);
                self.line(depth + 1, &format!("if {c} {{ break {l}; }}"));
                self.line(depth, "}");
            }
            StmtKind::For {
                init,
                cond,
                step,
                body,
            } => {
                self.stmt(init, depth);
                let l = self.next_label("for");
                let label = self.next_label("body");
                self.line(depth, &format!("{l}: loop {{"));
                let c = self.cond(cond);
                self.line(depth + 1, &format!("if !{c} {{ break {l}; }}"));
                self.line(depth + 1, &format!("{label}: {{"));
                self.scopes.push(Scope::LoopWithTail(l, label));
                self.body(body, depth + 2);
                self.scopes.pop();
                self.line(depth + 1, "}");
                self.stmt(step, depth + 1);
                self.line(depth, "}");
            }
            StmtKind::With { target, body } => {
                let t = self.value(target);
                let it = self.fresh("it");
                let h = self.fresh("h");
                let l = self.next_label("with");
                self.line(depth, "{");
                self.line(depth + 1, &format!("let t = {t};"));
                self.line(depth + 1, &format!("let mut {it} = g.with_iter(cx, &t)?;"));
                self.line(
                    depth + 1,
                    &format!("{l}: while let Some({h}) = {it}.next(g) {{"),
                );
                // Shadow `cx`: inside `with`, self is the target and other is the outer self.
                self.line(depth + 2, &format!("let cx = &mut cx.with_self({h});"));
                self.scopes.push(Scope::Loop(l));
                self.body(body, depth + 2);
                self.scopes.pop();
                self.line(depth + 1, "}");
                self.line(depth, "}");
            }
            StmtKind::Switch { value, body } => self.switch(value, body, depth),
            StmtKind::Case(_) | StmtKind::Default => {
                panic!("{}: case label outside switch", self.unit)
            }
            StmtKind::Break => match self.scopes.last() {
                Some(Scope::Switch(label) | Scope::Loop(label) | Scope::LoopWithTail(label, _)) => {
                    let l = format!("break {label};");
                    self.line(depth, &l);
                }
                // GM8 treats a stray `break` like the end of the unit.
                None => self.line(depth, self.exit_code()),
            },
            StmtKind::Continue => {
                match self
                    .scopes
                    .iter()
                    .rev()
                    .find(|s| !matches!(s, Scope::Switch(_)))
                {
                    Some(Scope::LoopWithTail(_, body)) => {
                        let l = format!("break {body};");
                        self.line(depth, &l);
                    }
                    Some(Scope::Loop(label)) => {
                        let l = format!("continue {label};");
                        self.line(depth, &l);
                    }
                    Some(Scope::Switch(_)) => unreachable!(),
                    None => self.line(depth, self.exit_code()),
                }
            }
            StmtKind::Exit => self.line(depth, self.exit_code()),
            StmtKind::Return(value) => {
                let v = self.value(value);
                if self.is_script {
                    self.line(depth, &format!("return Ok({v});"));
                } else {
                    // `return` in an event evaluates its value and ends the event.
                    self.line(depth, &format!("let _ = {v};"));
                    self.line(depth, "return Ok(());");
                }
            }
        }
    }

    fn next_label(&mut self, kind: &str) -> String {
        self.label += 1;
        format!("'{kind}{}", self.label)
    }

    /// GM8 `switch`: evaluate the cases in order until one is exactly equal, jump to that label, and fall
    /// through. Emitted as an entry index plus one `if entry <= k` per section.
    fn switch(&mut self, value: &Expr, body: &[Stmt], depth: usize) {
        let label = self.next_label("switch");
        let mut sections: Vec<(Option<&Expr>, Vec<&Stmt>)> = Vec::new();
        for s in body {
            match &s.kind {
                StmtKind::Case(e) => sections.push((Some(e), Vec::new())),
                StmtKind::Default => sections.push((None, Vec::new())),
                // Statements before the first label are skipped by the jump.
                _ => {
                    if let Some(last) = sections.last_mut() {
                        last.1.push(s);
                    }
                }
            }
        }
        let v = self.value(value);
        self.line(depth, &format!("{label}: {{"));
        self.line(depth + 1, &format!("let sw = {v};"));
        let default = sections
            .iter()
            .position(|(c, _)| c.is_none())
            .unwrap_or(sections.len());
        let mut chain = String::from("let entry: usize = ");
        for (k, (case, _)) in sections.iter().enumerate() {
            if let Some(e) = case {
                let c = self.value(e);
                write!(chain, "if case_eq(&sw, &{c}) {{ {k} }} else ").unwrap();
            }
        }
        write!(chain, "{{ {default} }};").unwrap();
        self.line(depth + 1, &chain);
        self.scopes.push(Scope::Switch(label));
        for (k, (_, stmts)) in sections.iter().enumerate() {
            self.line(depth + 1, &format!("if entry <= {k} {{"));
            for s in stmts {
                self.stmt(s, depth + 2);
            }
            self.line(depth + 1, "}");
        }
        self.scopes.pop();
        self.line(depth, "}");
    }
}

fn has_call(e: &Expr) -> bool {
    common::calls_any(e, &|_| true)
}

fn arith_fn(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "add",
        BinOp::Sub => "sub",
        BinOp::Mul => "mul",
        BinOp::Div => "div",
        BinOp::IntDiv => "idiv",
        BinOp::Mod => "modulo",
        BinOp::BitAnd => "bitand",
        BinOp::BitOr => "bitor",
        BinOp::BitXor => "bitxor",
        BinOp::Shl => "shl",
        BinOp::Shr => "shr",
        _ => unreachable!("not arithmetic: {op:?}"),
    }
}

/// Whether a script can finish without `return` (see `analyze.rs`).
fn can_fall_off(body: &[Stmt]) -> bool {
    fn ends(stmt: &Stmt) -> bool {
        match &stmt.kind {
            StmtKind::Return(_) | StmtKind::Exit => true,
            StmtKind::Block(b) => b.last().is_some_and(ends),
            StmtKind::If {
                then,
                otherwise: Some(o),
                ..
            } => ends(then) && ends(o),
            _ => false,
        }
    }
    !body.last().is_some_and(ends)
}

/// Translates one code unit into a Rust function named `fn_name`.
fn emit_unit(src: &Source, unit: &str, fn_name: &str, syms: &mut Symbols) -> String {
    let program = &src.programs[unit];
    let source = &src.units.iter().find(|u| u.name == unit).unwrap().source;
    let is_script = unit.starts_with("scripts/");
    let locals = locals_of(program);
    let mut e = Emitter {
        src,
        unit,
        lines: LineIndex::new(source),
        locals: locals.clone(),
        is_script,
        track_ret: is_script && can_fall_off(&program.body),
        scopes: Vec::new(),
        label: 0,
        tmp: 0,
        out: String::new(),
        syms,
    };
    e.line(0, &format!("/// GML: `{unit}`."));
    if is_script {
        e.line(
            0,
            &format!(
                "pub fn {}(g: &mut Game, cx: &mut Ctx, args: &[Value]) -> R<Value> {{",
                rid(fn_name)
            ),
        );
        e.line(1, "let cx = &mut cx.call(args);");
    } else {
        e.line(
            0,
            &format!("pub fn {fn_name}(g: &mut Game, cx: &mut Ctx) -> R<()> {{"),
        );
    }
    for l in &locals {
        e.line(1, &format!("let mut l_{l} = Local::default();"));
    }
    for s in &program.body {
        e.stmt(s, 1);
    }
    e.line(
        1,
        if is_script {
            "Ok(cx.ret.clone())"
        } else {
            "Ok(())"
        },
    );
    e.line(0, "}");
    e.out
}

/// Composes an object event from its action list: code actions call the functions emitted for them,
/// drag-and-drop actions are translated inline.
fn emit_event(
    src: &Source,
    object: &gmk::Object,
    event: &gmk::Event,
    syms: &mut Symbols,
) -> String {
    let ev = spelunky::event_name(&src.gmk, event);
    let mut body = Vec::new();
    let mut pending_if: Option<String> = None;
    for (i, action) in event.actions.iter().enumerate() {
        let unit = format!("{}/{ev}#{i}", object.name);
        assert_eq!(
            action.applies_to, -1,
            "{unit}: only `applies to self` is handled"
        );
        let arg = |n: usize, syms: &mut Symbols| -> String {
            let name = format!("{unit}:arg{n}");
            let StmtKind::Call(expr) = &src.programs[&name].body[0].kind else {
                unreachable!()
            };
            let mut e = Emitter {
                src,
                unit: &name,
                lines: LineIndex::new(""),
                locals: BTreeSet::new(),
                is_script: false,
                track_ret: false,
                scopes: Vec::new(),
                label: 0,
                tmp: 0,
                out: String::new(),
                syms,
            };
            e.value(expr)
        };
        let action_code = if action.code().is_some() {
            format!("{ev}_a{i}(g, cx)?;")
        } else {
            match (action.library_id, action.action_id) {
                (1, 604) => "g.event_inherited(cx)?;".to_owned(),
                (1, 203) => "b::instance_destroy(g, cx, &[])?;".to_owned(),
                // A comment.
                (1, 605) => continue,
                (1, 611) => {
                    let name = action.args[0].value.clone();
                    syms.vars.insert(name.clone());
                    format!("g.set(cx.this, v::{}, {})?;", rid(&name), arg(1, syms))
                }
                (1, 601) => {
                    let id: usize = action.args[0].value.parse().expect("script id");
                    let script = &src.gmk.scripts[id].as_ref().expect("script exists").name;
                    let args: Vec<_> = (1..action.args.len()).map(|n| arg(n, syms)).collect();
                    format!(
                        "{{ let a = [{}]; scr::{script}(g, cx, &a)?; }}",
                        args.join(", ")
                    )
                }
                (1, 408) => {
                    let c = format!("truthy(&{})", arg(0, syms));
                    pending_if = Some(if action.invert { format!("!{c}") } else { c });
                    continue;
                }
                (1, id @ (524 | 531)) => {
                    let name = if id == 524 {
                        "action_color"
                    } else {
                        "action_fullscreen"
                    };
                    syms.builtins.insert(name.to_owned());
                    format!("b::{name}(g, cx, &[r({}.0)])?;", action.args[0].value)
                }
                (lib, id) => panic!("{unit}: unsupported DnD action {lib}/{id}"),
            }
        };
        // A DnD `if` guards exactly the next action.
        match pending_if.take() {
            Some(c) => body.push(format!("if {c} {{ {action_code} }}")),
            None => body.push(action_code),
        }
    }
    let mut out = format!(
        "/// Event `{ev}` of `{}`: its actions in order.\npub fn {ev}(g: &mut Game, cx: &mut Ctx) -> R<()> {{\n",
        object.name
    );
    for b in body {
        writeln!(out, "    {b}").unwrap();
    }
    out.push_str("    Ok(())\n}\n");
    out
}

const PRELUDE: &str =
    "use crate::rt::*;\nuse crate::{b, bgr, gv, k, obj, rm, scr, scr_id, spr, v};\n";

/// Every code action of one object, plus its event functions.
fn emit_object(src: &Source, object: &gmk::Object, syms: &mut Symbols) -> String {
    let mut out = String::from(PRELUDE);
    for event in &object.events {
        let ev = spelunky::event_name(&src.gmk, event);
        for (i, action) in event.actions.iter().enumerate() {
            if action.code().is_some() {
                let unit = format!("{}/{ev}#{i}", object.name);
                out.push('\n');
                out.push_str(&emit_unit(src, &unit, &format!("{ev}_a{i}"), syms));
            }
        }
    }
    for event in &object.events {
        out.push('\n');
        out.push_str(&emit_event(src, object, event, syms));
    }
    out
}

fn write_crate(src: &Source, out: &Path) {
    let gen_dir = out.join("src");
    fs::create_dir_all(gen_dir.join("objects")).unwrap();
    let mut syms = Symbols::default();
    let mut lib = String::from(
        "#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types, unused, unreachable_code, \
         clippy::all)]\n//! Generated by the gml transpiler prototype. Do not edit.\npub mod rt;\npub mod scripts;\n\
         pub use scripts as scr;\npub mod objects {\n",
    );
    let mut rust_lines = 0;
    for object in src
        .gmk
        .objects
        .iter()
        .flatten()
        .filter(|o| !o.events.is_empty())
    {
        let code = emit_object(src, object, &mut syms);
        rust_lines += code.lines().count();
        fs::write(
            gen_dir.join("objects").join(format!("{}.rs", object.name)),
            code,
        )
        .unwrap();
        writeln!(lib, "    pub mod {};", object.name).unwrap();
    }
    lib.push_str("}\n");
    let mut scripts = String::from(PRELUDE);
    for unit in src.units.iter().filter(|u| u.name.starts_with("scripts/")) {
        scripts.push('\n');
        scripts.push_str(&emit_unit(
            src,
            &unit.name,
            &unit.name["scripts/".len()..],
            &mut syms,
        ));
    }
    rust_lines += scripts.lines().count();
    fs::write(gen_dir.join("scripts.rs"), scripts).unwrap();

    // Declarations the generated code refers to. In the real port these come from the runtime (`v`,
    // `b`, `k`) and from the extracted assets (`obj`, `spr`, ...).
    let decl = |items: &mut dyn Iterator<Item = String>, body: &dyn Fn(usize, &str) -> String| {
        items
            .enumerate()
            .map(|(i, n)| body(i, &n))
            .collect::<String>()
    };
    let module =
        |name: &str, body: String| format!("pub mod {name} {{\n    use crate::rt::*;\n{body}}}\n");
    lib += &module(
        "v",
        decl(&mut syms.vars.iter().cloned(), &|i, n| {
            format!("    pub const {}: Var = Var({i});\n", rid(n))
        }),
    );
    lib += &module(
        "gv",
        decl(&mut syms.gvars.iter().cloned(), &|i, n| {
            format!("    pub const {}: GVar = GVar({i});\n", rid(n))
        }),
    );
    lib += &module(
        "k",
        decl(&mut syms.constants.iter().cloned(), &|_, n| {
            format!("    pub const {n}: Value = r(0.0);\n")
        }),
    );
    lib += &module(
        "b",
        decl(&mut syms.builtins.iter().cloned(), &|_, n| {
            format!(
                "    pub fn {}(g: &mut Game, cx: &mut Ctx, a: &[Value]) -> R<Value> {{ unimplemented!() }}\n",
                rid(n)
            )
        }),
    );
    let resources = |name: &str, set: &mut dyn Iterator<Item = String>| {
        module(
            name,
            decl(set, &|i, n| {
                format!("    pub const {n}: Value = r({i}.0);\n")
            }),
        )
    };
    lib += &resources("obj", &mut src.parents.keys().cloned());
    lib += &resources("spr", &mut src.sprites.iter().cloned());
    lib += &resources("rm", &mut src.rooms.iter().cloned());
    lib += &resources("bgr", &mut src.backgrounds.iter().cloned());
    lib += &resources("scr_id", &mut src.scripts.iter().cloned());
    fs::write(gen_dir.join("lib.rs"), lib).unwrap();
    fs::write(gen_dir.join("rt.rs"), include_str!("proto_rt.rs")).unwrap();
    fs::write(
        out.join("Cargo.toml"),
        "[package]\nname = \"proto-game\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n",
    )
    .unwrap();
    let gml_lines: usize = src.units.iter().map(|u| u.source.lines().count()).sum();
    println!(
        "wrote {} units ({gml_lines} GML lines) as {rust_lines} Rust lines to {}",
        src.units.len(),
        out.display()
    );
    println!(
        "{} variables, {} globals, {} built-in functions, {} constants referenced",
        syms.vars.len(),
        syms.gvars.len(),
        syms.builtins.len(),
        syms.constants.len()
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let src = Source::load();
    if args.first().map(String::as_str) == Some("--crate") {
        write_crate(
            &src,
            Path::new(args.get(1).expect("--crate needs an output dir")),
        );
        return;
    }
    let mut syms = Symbols::default();
    for prefix in &args {
        if let Some(name) = prefix.strip_suffix('/') {
            let object = src
                .gmk
                .objects
                .iter()
                .flatten()
                .find(|o| o.name == name)
                .expect("no such object");
            println!("{}", emit_object(&src, object, &mut syms));
            continue;
        }
        for unit in src
            .units
            .iter()
            .filter(|u| u.name.starts_with(prefix.as_str()))
        {
            let name = unit.name.rsplit('/').next().unwrap().replace('#', "_a");
            println!("{}", emit_unit(&src, &unit.name, &name, &mut syms));
        }
    }
}
