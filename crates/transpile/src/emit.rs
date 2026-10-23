//! Translating one GML unit (a script or a code action) or one object event into Rust against the `gm`
//! runtime API (`docs/design/runtime.md`, section 12). The GM8 rules each construct follows are listed in
//! `docs/design/translation.md`; the comments here say how the generated code gets them.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use gmk::ActionKind;
use gml::ast::*;
use gml::{LineIndex, print};

use crate::source::{NameKind, Source, can_end_without_return, event_name, locals_of};

const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
    "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro", "override",
    "priv", "typeof", "unsized", "virtual", "yield", "try", "gen",
];

/// A GML name as a Rust identifier: unchanged where possible, so generated code greps like the GML.
/// Rust keywords become raw identifiers (`v::r#type`).
pub fn rid(name: &str) -> String {
    assert!(
        !matches!(name, "self" | "Self" | "super" | "crate" | "_"),
        "`{name}` can't be a Rust identifier"
    );
    if RUST_KEYWORDS.contains(&name) {
        format!("r#{name}")
    } else {
        name.to_owned()
    }
}

/// Names the generated code refers to, collected while emitting, so the generator can declare them.
#[derive(Default)]
pub struct Symbols {
    /// Instance variables (built-in and user) and game-wide built-ins, by GML name.
    pub vars: BTreeSet<String>,
    pub gvars: BTreeSet<String>,
    /// Built-in functions called, with the argument counts they're called with.
    pub builtins: BTreeMap<String, BTreeSet<usize>>,
    pub constants: BTreeSet<String>,
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

pub struct Emitter<'a> {
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

/// `code` as a method receiver: a block needs parentheses (`({ ... }).add(..)`).
fn recv(code: String) -> String {
    if code.starts_with('{') {
        format!("({code})")
    } else {
        code
    }
}

impl<'a> Emitter<'a> {
    fn new(
        src: &'a Source,
        unit: &'a str,
        source: &str,
        locals: BTreeSet<String>,
        syms: &'a mut Symbols,
    ) -> Emitter<'a> {
        Emitter {
            src,
            unit,
            lines: LineIndex::new(source),
            locals,
            is_script: unit.starts_with("scripts/"),
            track_ret: false,
            scopes: Vec::new(),
            label: 0,
            tmp: 0,
            out: String::new(),
            syms,
        }
    }

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
            Ty::Value => format!("{}.is_truthy()", recv(code)),
        }
    }

    /// One array index as a `u32` expression; a literal index in range is emitted as a plain number.
    fn index1(&mut self, e: &Expr) -> String {
        match e.kind {
            ExprKind::Real(v) if (0.0..32000.0).contains(&v) && v.fract() == 0.0 => {
                format!("{}", v as u32)
            }
            _ => format!("ix(&{})?", self.value(e)),
        }
    }

    /// Array indices as `(i, j)`: GM8 stores `a[i, j]` at row `i`, and `a[j]` is `a[0, j]`.
    fn index(&mut self, indices: &[Expr]) -> (String, String) {
        match indices {
            [j] => ("0".to_owned(), self.index1(j)),
            [i, j] => {
                let i = self.index1(i);
                (i, self.index1(j))
            }
            _ => panic!("{}: {} array indices", self.unit, indices.len()),
        }
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
            NameKind::ArgumentArray => "cx.arg_ix(0)?".to_owned(),
            NameKind::ArgumentCount => "cx.argument_count()".to_owned(),
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
            NameKind::Object => format!("obj::{}", rid(name)),
            NameKind::Sprite => format!("spr::{}", rid(name)),
            NameKind::Room => format!("rm::{}", rid(name)),
            NameKind::Background => format!("bgr::{}", rid(name)),
            NameKind::Script => format!("scr_id::{}", rid(name)),
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
        let flat = |idx: &Option<(String, String)>| one_d(self.unit, indices, idx);
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
                    format!("g.bget({v}, {})?", flat(&idx))
                }
                NameKind::ArgumentArray => format!("cx.arg_ix({})?", flat(&idx)),
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
            ExprKind::Str(s) => {
                // GML strings are Windows-1252 bytes; `Value::from(&str)` stores UTF-8. They agree on
                // ASCII, which is all the game's literals use.
                assert!(
                    s.is_ascii(),
                    "{}: non-ASCII string literal {s:?}",
                    self.unit
                );
                format!("s!({s:?})")
            }
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
                // GM8's `!` is an operator on values that rejects strings (`Value::not`), not the negation
                // of truthiness. On a comparison's result (always a real) the two agree.
                UnOp::Not => match self.expr(expr) {
                    (c, Ty::Bool) => return (format!("!({c})"), Ty::Bool),
                    (v, Ty::Value) => format!("{}.not()?", recv(v)),
                },
                // Negating a literal can't fail, so fold it: `-1` is `r(-1.0)`.
                UnOp::Neg => match expr.kind {
                    ExprKind::Real(v) => format!("r({:?})", -v),
                    _ => format!("{}.neg()?", recv(self.value(expr))),
                },
                UnOp::BitNot => format!("{}.bit_not()?", recv(self.value(expr))),
                UnOp::Plus => self.value(expr),
            },
            ExprKind::Binary { op, lhs, rhs } => {
                let f = match op {
                    BinOp::And | BinOp::Or | BinOp::Xor => {
                        // Plain functions on evaluated operands: both sides always run, as in GM8.
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
                        let f = format!("gml_{op:?}").to_lowercase();
                        return (format!("{}.{f}(&{r})?", recv(l)), Ty::Bool);
                    }
                    _ => arith_fn(*op),
                };
                let (l, r) = (self.value(lhs), self.value(rhs));
                format!("{}.{f}(&{r})?", recv(l))
            }
        };
        (code, Ty::Value)
    }

    fn call(&mut self, name: &str, args: &[Expr]) -> String {
        // Arguments run left to right, as in GM8. Any that touch the world are bound first, so the call
        // can borrow `g` mutably.
        let bind = args.iter().any(|a| self.needs_world(a));
        let vals: Vec<String> = args.iter().map(|a| self.value(a)).collect();
        let list = vals.join(", ");
        let (callee, args) = if self.src.scripts.contains(name) {
            (format!("scr::{}", rid(name)), ("&a", format!("&[{list}]")))
        } else {
            self.syms
                .builtins
                .entry(name.to_owned())
                .or_default()
                .insert(args.len());
            (format!("b::{}", rid(name)), ("a", format!("[{list}]")))
        };
        if bind {
            format!("{{ let a = [{list}]; {callee}(g, cx, {})? }}", args.0)
        } else {
            format!("{callee}(g, cx, {})?", args.1)
        }
    }

    /// Whether evaluating `e` reads or changes the world (anything but literals, locals, arguments,
    /// constants and resource ids).
    fn needs_world(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Real(_) | ExprKind::Str(_) | ExprKind::Const(_) => false,
            ExprKind::Ident(name) => matches!(
                self.src.classify(name, &self.locals),
                NameKind::InstanceBuiltin
                    | NameKind::InstanceVar
                    | NameKind::GlobalBuiltin
                    | NameKind::GlobalVar
            ),
            ExprKind::Unary { expr, .. } => self.needs_world(expr),
            ExprKind::Binary { lhs, rhs, .. } => self.needs_world(lhs) || self.needs_world(rhs),
            _ => true,
        }
    }

    // ---- statements ----

    /// `place op= value`. GM8's order (OpenGMK `compiler.rs` make_set/make_modify, `runtime.rs` SetField):
    /// resolve the target, then the array index, then (for `op=`) read the old value, then evaluate the
    /// right-hand side. Computed indices and a base expression are bound first to keep that order; the
    /// value is bound when it calls anything (which needs `&mut g`), or when a script tracks its return
    /// value.
    pub fn assign(&mut self, place: &Expr, op: AssignOp, value: &Expr, depth: usize) {
        /// Where the new value goes in a write template.
        const X: &str = "@X@";
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
        let unit = self.unit;
        let base_value = match target.map(|b| &b.kind) {
            None | Some(ExprKind::Const(Const::Global | Const::SelfKw | Const::Other)) => None,
            Some(_) => {
                let t = self.value(target.unwrap());
                pre.push(format!("let t = {t};"));
                Some(t)
            }
        };
        let idx = indices.map(|ix| {
            let (i, j) = self.index(ix);
            let mut bind = |s: String, n: &str| {
                if s.parse::<u32>().is_ok() {
                    s
                } else {
                    pre.push(format!("let {n} = {s};"));
                    n.to_owned()
                }
            };
            (bind(i, "i"), bind(j, "j"))
        });
        let kind = match target {
            None => self.src.classify(name, &self.locals),
            Some(base) => match &base.kind {
                ExprKind::Const(Const::Global) => NameKind::GlobalVar,
                _ => NameKind::InstanceVar,
            },
        };
        let flat = |idx: &Option<(String, String)>| one_d(unit, indices, idx);
        // Built-ins (arguments included) keep strict arithmetic for `op=`; user variables, globals and
        // locals ignore type errors (OpenGMK `compiler.rs` make_modify_instruction: `type_unsafe` is set
        // only for fields). `other.x` is the built-in `x`; `global.x` is a user global named "x".
        let builtin = builtin_assign(kind, target.is_some(), name);
        let (read, write) = match (kind, target.map(|b| &b.kind)) {
            (NameKind::Local, None) => match &idx {
                None => (format!("l_{name}.get()?"), format!("l_{name}.set({X})")),
                Some((i, j)) => (
                    format!("l_{name}.get_a({i}, {j})?"),
                    format!("l_{name}.set_a({i}, {j}, {X})"),
                ),
            },
            (NameKind::Argument(n), None) => {
                assert!(idx.is_none(), "{unit}: cannot index `{name}`");
                (format!("cx.arg({n})?"), format!("cx.set_arg({n}, {X})?"))
            }
            (NameKind::ArgumentArray, None) => {
                let i = flat(&idx);
                (
                    format!("cx.arg_ix({i})?"),
                    format!("cx.set_arg_ix({i}, {X})?"),
                )
            }
            (NameKind::ArgumentCount, None) => {
                panic!("{}: `argument_count` is read-only", self.unit)
            }
            (NameKind::GlobalBuiltin, None) => {
                let v = self.var(name);
                let i = flat(&idx);
                (
                    format!("g.bget({v}, {i})?"),
                    format!("g.bset({v}, {i}, {X})?"),
                )
            }
            (NameKind::GlobalVar, _) => {
                let v = self.gvar(name);
                match &idx {
                    None => (format!("g.gget({v})?"), format!("g.gset({v}, {X})?")),
                    Some((i, j)) => (
                        format!("g.gget_a({v}, {i}, {j})?"),
                        format!("g.gset_a({v}, {i}, {j}, {X})?"),
                    ),
                }
            }
            (_, None | Some(ExprKind::Const(Const::SelfKw | Const::Other))) => {
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
                        format!("g.set({who}, {v}, {X})?"),
                    ),
                    Some((i, j)) => (
                        format!("g.get_a({who}, {v}, {i}, {j})?"),
                        format!("g.set_a({who}, {v}, {i}, {j}, {X})?"),
                    ),
                }
            }
            (_, Some(_)) => {
                // `obj.x = ...` writes every instance of `obj`; `obj.x += ...` reads the first one. GM8
                // evaluates the base twice for `op=` (once to read, once to write).
                let t = base_value.expect("bound above");
                let v = self.var(name);
                match &idx {
                    None => (
                        format!("{{ let t2 = {t}; g.get_on(cx, &t2, {v})? }}"),
                        format!("g.set_on(cx, &t, {v}, {X})?"),
                    ),
                    Some((i, j)) => (
                        format!("{{ let t2 = {t}; g.get_on_a(cx, &t2, {v}, {i}, {j})? }}"),
                        format!("g.set_on_a(cx, &t, {v}, {i}, {j}, {X})?"),
                    ),
                }
            }
        };
        let rhs = self.value(value);
        let modify = |old: &str| -> String {
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
            if builtin {
                format!("{}.{f}(&{rhs})?", recv(old.to_owned()))
            } else {
                format!("lenient(Value::{f}, {old}, &{rhs})")
            }
        };
        // Anything that calls a function needs `&mut g`, so it can't be an argument of the write.
        let bind = self.track_ret || has_call(value) || target.is_some_and(has_call);
        if bind {
            if op == AssignOp::Set {
                pre.push(format!("let x = {rhs};"));
            } else {
                pre.push(format!("let old = {read};"));
                pre.push(format!("let x = {};", modify("old")));
            }
            if self.track_ret {
                pre.push("cx.ret = x.clone();".to_owned());
            }
            pre.push(format!("{};", write.replace(X, "x")));
        } else {
            let new = if op == AssignOp::Set {
                rhs
            } else {
                modify(&read)
            };
            pre.push(format!("{};", write.replace(X, &new)));
        }
        match pre.as_slice() {
            [one] => self.line(depth, one),
            [a, b] if a.len() + b.len() < 100 => self.line(depth, &format!("{{ {a} {b} }}")),
            _ => {
                self.line(depth, "{");
                for p in pre {
                    self.line(depth + 1, &p);
                }
                self.line(depth, "}");
            }
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
                // Resolved statically: every `globalvar` name is a global everywhere (see
                // `docs/design/translation.md`, "Risks and open questions").
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
                // The count is evaluated once.
                let n = self.value(count);
                let l = self.next_label("repeat");
                self.line(depth, &format!("{l}: for _ in 0..repeat_count(&{n}) {{"));
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
                self.line(depth + 1, &format!("if !({c}) {{ break {l}; }}"));
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
                // Inside `with`, self is the target and other is the outer self. They are swapped in
                // place (not in a new context) so `cx.ret` stays shared, and restored after the loop.
                // `exit` leaves the unit without restoring, which is why every unit with a `with`
                // works on its own copy of the context.
                let saved = self.fresh("outer");
                self.line(depth + 1, &format!("let {saved} = (cx.this, cx.other);"));
                self.line(
                    depth + 1,
                    &format!("{l}: while let Some({h}) = {it}.next(g) {{"),
                );
                self.line(
                    depth + 2,
                    &format!("(cx.this, cx.other) = ({h}, {saved}.0);"),
                );
                self.scopes.push(Scope::Loop(l));
                self.body(body, depth + 2);
                self.scopes.pop();
                self.line(depth + 1, "}");
                self.line(depth + 1, &format!("(cx.this, cx.other) = {saved};"));
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

/// The index of a 1-D array (a game-wide built-in such as `view_xview[i]`, or `argument[i]`): the column of
/// row 0. `idx` holds the index code as emitted, possibly bound to a name.
fn one_d(unit: &str, indices: Option<&[Expr]>, idx: &Option<(String, String)>) -> String {
    assert!(
        indices.is_none_or(|ix| ix.len() == 1),
        "{unit}: 2-D index on a 1-D array"
    );
    idx.as_ref().map_or("0".to_owned(), |(_, j)| j.clone())
}

/// The function a code action is emitted as: `step_a1` for action 1 of the step event.
pub fn action_fn(event: &str, action: usize) -> String {
    format!("{event}_a{action}")
}

/// Whether an `op=` on this target is a built-in variable's, with strict arithmetic. `kind` is the target's
/// classification (`InstanceVar` or `GlobalVar` when it has a base); `has_base` says it is `base.name`.
fn builtin_assign(kind: NameKind, has_base: bool, name: &str) -> bool {
    match kind {
        NameKind::InstanceBuiltin
        | NameKind::GlobalBuiltin
        | NameKind::Argument(_)
        | NameKind::ArgumentArray
        | NameKind::ArgumentCount => true,
        // `other.x`, `obj.sprite_index`: the built-in on that instance.
        NameKind::InstanceVar => has_base && gm::Builtin::from_name(name).is_some(),
        _ => false,
    }
}

fn has_call(e: &Expr) -> bool {
    crate::source::calls_any(e, &|_| true)
}

/// The `gm::Value` method for an arithmetic or bitwise operator.
fn arith_fn(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "add",
        BinOp::Sub => "sub",
        BinOp::Mul => "mul",
        BinOp::Div => "div",
        BinOp::IntDiv => "int_div",
        BinOp::Mod => "modulo",
        BinOp::BitAnd => "bit_and",
        BinOp::BitOr => "bit_or",
        BinOp::BitXor => "bit_xor",
        BinOp::Shl => "shl",
        BinOp::Shr => "shr",
        _ => unreachable!("not arithmetic: {op:?}"),
    }
}

fn contains_with(program: &Program) -> bool {
    struct W(bool);
    impl gml::visit::Visitor for W {
        fn visit_stmt(&mut self, stmt: &Stmt) {
            self.0 |= matches!(stmt.kind, StmtKind::With { .. });
            gml::visit::walk_stmt(self, stmt);
        }
    }
    let mut w = W(false);
    gml::visit::walk_program(&mut w, program);
    w.0
}

/// Translates a parsed program as unit `unit` (a script if it starts with `scripts/`) into a Rust function
/// named `fn_name`.
pub fn emit_program(
    src: &Source,
    unit: &str,
    program: &Program,
    source: &str,
    fn_name: &str,
    syms: &mut Symbols,
) -> String {
    let locals = locals_of(program);
    let mut e = Emitter::new(src, unit, source, locals.clone(), syms);
    e.track_ret = e.is_script && can_end_without_return(&program.body);
    e.line(0, &format!("/// GML: `{unit}`."));
    if e.is_script {
        e.line(
            0,
            &format!(
                "pub fn {}(g: &mut World, cx: &Ctx, args: &[Value]) -> R<Value> {{",
                rid(fn_name)
            ),
        );
        e.line(1, "let cx = &mut cx.call(args);");
    } else {
        e.line(
            0,
            &format!("pub fn {fn_name}(g: &mut World, cx: &mut Ctx) -> R<()> {{"),
        );
        if contains_with(program) {
            // `with` changes cx.this/other in place; an `exit` inside it must not leak that change into
            // the event's next action.
            e.line(1, "let cx = &mut cx.clone();");
        }
    }
    for l in &locals {
        e.line(1, &format!("let mut l_{l} = Local::default();"));
    }
    for s in &program.body {
        e.stmt(s, 1);
    }
    e.line(
        1,
        if e.is_script {
            "Ok(cx.ret.clone())"
        } else {
            "Ok(())"
        },
    );
    e.line(0, "}");
    e.out
}

/// Translates one parsed unit of `src`.
pub fn emit_unit(src: &Source, unit: &str, fn_name: &str, syms: &mut Symbols) -> String {
    emit_program(
        src,
        unit,
        &src.programs[unit],
        src.source(unit),
        fn_name,
        syms,
    )
}

/// Wraps an action in the pending `action_if` conditions, outermost first, and clears them.
fn guard(pending: &mut Vec<String>, action: &str) -> String {
    pending.drain(..).rev().fold(action.to_owned(), |inner, c| {
        format!("if {c} {{ {inner} }}")
    })
}

/// The name of a drag-and-drop action, for comments.
fn dnd_name(lib: i32, id: i32) -> &'static str {
    match (lib, id) {
        (1, 604) => "action_inherited",
        (1, 203) => "action_kill_object",
        (1, 611) => "action_set_variable",
        (1, 601) => "action_execute_script",
        (1, 408) => "action_if",
        (1, 524) => "action_color",
        (1, 531) => "action_fullscreen",
        _ => "?",
    }
}

/// Composes an object event from its action list: code actions call the functions emitted for them,
/// drag-and-drop actions are translated inline.
pub fn emit_event(
    src: &Source,
    object: &gmk::Object,
    event: &gmk::Event,
    syms: &mut Symbols,
) -> String {
    let ev = event_name(&src.gmk, event);
    let mut body = Vec::new();
    // Conditions of `action_if`s waiting for the action they guard. GM8 applies a question to the next
    // action, so consecutive questions nest: [if A, if B, X] is `if A { if B { X } }`, and B is only
    // evaluated when A holds (OpenGMK `action.rs` exec_slice).
    let mut pending_ifs: Vec<String> = Vec::new();
    for (i, action) in event.actions.iter().enumerate() {
        let unit = format!("{}/{ev}#{i}", object.name);
        assert_eq!(
            action.applies_to, -1,
            "{unit}: only `applies to self` is handled"
        );
        let arg = |n: usize, syms: &mut Symbols| -> String {
            let name = format!("{unit}:arg{n}");
            let mut e = Emitter::new(src, &name, "", BTreeSet::new(), syms);
            e.value(&src.dnd_exprs[&name])
        };
        // Blocks, `else`, `repeat` and `exit` actions change how questions apply; Spelunky uses none.
        assert!(
            matches!(
                action.kind,
                ActionKind::Normal | ActionKind::Code | ActionKind::Variable
            ),
            "{unit}: unsupported DnD action kind {:?}",
            action.kind
        );
        if action.code().is_some() {
            let f = action_fn(&ev, i);
            body.push(guard(&mut pending_ifs, &format!("{f}(g, cx)?;")));
            continue;
        }
        let (lib, id) = (action.library_id, action.action_id);
        let args: Vec<&str> = action.args.iter().map(|a| a.value.as_str()).collect();
        let not = if action.invert { "not " } else { "" };
        body.push(format!(
            "// #{i} {not}{}({})",
            dnd_name(lib, id),
            args.join(", ")
        ));
        // DnD actions have no GML lines; errors in them report line 0 of the action's unit.
        let at = format!("g.at({unit:?}, 0);");
        let action_code = match (lib, id) {
            (1, 604) => "g.event_inherited(cx)?;".to_owned(),
            (1, 203) => {
                syms.builtins
                    .entry("instance_destroy".to_owned())
                    .or_default()
                    .insert(0);
                "b::instance_destroy(g, cx, [])?;".to_owned()
            }
            // A comment. After a question, GM8 would skip just the comment when the condition is false
            // (OpenGMK `action.rs` skip_actions) and run the next action anyway; Spelunky never does
            // this, so refuse rather than guess.
            (1, 605) => {
                assert!(
                    pending_ifs.is_empty(),
                    "{unit}: comment after an `action_if`"
                );
                body.pop();
                continue;
            }
            // Set variable: `name = value`, or `name += value` when "relative" is ticked. The name is
            // resolved like any assignment target (it may be `global.x` or `other.x`).
            (1, 611) => {
                let name = format!("{unit}:arg0");
                let place = &src.dnd_exprs[&name];
                let value = &src.dnd_exprs[&format!("{unit}:arg1")];
                let op = if action.relative {
                    AssignOp::Add
                } else {
                    AssignOp::Set
                };
                let mut e = Emitter::new(src, &name, "", BTreeSet::new(), syms);
                e.assign(place, op, value, 0);
                e.out.lines().map(str::trim).collect::<Vec<_>>().join(" ")
            }
            (1, 601) => {
                let id: usize = action.args[0].value.parse().expect("script id");
                let script = &src.gmk.scripts[id].as_ref().expect("script exists").name;
                let args: Vec<_> = (1..action.args.len()).map(|n| arg(n, syms)).collect();
                format!(
                    "{{ let a = [{}]; scr::{}(g, cx, &a)?; }}",
                    args.join(", "),
                    rid(script)
                )
            }
            (1, 408) => {
                let c = format!("{}.is_truthy()", recv(arg(0, syms)));
                let c = if action.invert { format!("!{c}") } else { c };
                // The condition runs before the action it guards, so it records its own location.
                pending_ifs.push(format!("{{ {at} {c} }}"));
                continue;
            }
            (1, id @ (524 | 531)) => {
                let name = if id == 524 {
                    "action_color"
                } else {
                    "action_fullscreen"
                };
                syms.builtins.entry(name.to_owned()).or_default().insert(1);
                let v: f64 = action.args[0].value.parse().expect("a number");
                format!("b::{name}(g, cx, [r({v:?})])?;")
            }
            (lib, id) => panic!("{unit}: unsupported DnD action {lib}/{id}"),
        };
        body.push(guard(&mut pending_ifs, &format!("{at} {action_code}")));
    }
    // Questions at the end of the event guard nothing, but their conditions still run (they may call
    // scripts or draw from the RNG).
    if !pending_ifs.is_empty() {
        body.push(guard(&mut pending_ifs, ""));
    }
    let mut out = format!(
        "/// Event `{ev}` of `{}`: its actions in order.\npub fn {ev}(g: &mut World, cx: &mut Ctx) -> R<()> {{\n",
        object.name
    );
    for b in body {
        writeln!(out, "    {b}").unwrap();
    }
    out.push_str("    Ok(())\n}\n");
    out
}

/// The prelude of every generated file.
pub const PRELUDE: &str = "use crate::rt::*;\n";

/// Every code action of one object, plus its event functions.
pub fn emit_object(src: &Source, object: &gmk::Object, syms: &mut Symbols) -> String {
    let mut out = String::new();
    for event in &object.events {
        let ev = event_name(&src.gmk, event);
        for (i, action) in event.actions.iter().enumerate() {
            if action.code().is_some() {
                let unit = format!("{}/{ev}#{i}", object.name);
                out.push('\n');
                out.push_str(&emit_unit(src, &unit, &action_fn(&ev, i), syms));
            }
        }
    }
    for event in &object.events {
        out.push('\n');
        out.push_str(&emit_event(src, object, event, syms));
    }
    out
}
