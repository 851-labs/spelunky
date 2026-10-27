//! Finds a pattern in GML: `hp = ?`, `global.plife -= ?`, `dist < ?`, `repeat (?) {}`. A pattern is a
//! statement or an expression in which `?` stands for any expression; it matches code with the same
//! syntax tree, whatever the spacing, parentheses or comments. Each match comes with what has to be true
//! for the code to run: the `if`s around it, the loops, `with`s and `switch` cases it's in.

use gml::ast::{AssignOp, ExprKind, StmtKind};
use gml::{Expr, Program, Stmt};
use serde::Serialize;

/// What `?` becomes before the pattern is parsed: an identifier no game code uses.
const HOLE: &str = "__hole__";

/// A parsed pattern.
#[derive(Debug)]
pub enum Pattern {
    /// A statement other than a call: an assignment, or a compound statement's head (its body `{}`).
    Stmt(Stmt),
    /// An expression, or a call (which matches as a statement too).
    Expr(Expr),
}

impl Pattern {
    pub fn parse(text: &str) -> Result<Pattern, String> {
        let code = text.replace('?', HOLE);
        // A statement first: `hp = ?` is an assignment, though as an expression it would read as `==`.
        if let Ok(program) = gml::parse_program(&code)
            && let [stmt] = program.body.as_slice()
        {
            return match &stmt.kind {
                StmtKind::Call(call) => Ok(Pattern::Expr(call.clone())),
                StmtKind::Assign { .. } => Ok(Pattern::Stmt(stmt.clone())),
                kind if body(kind).is_some_and(is_empty_block) => Ok(Pattern::Stmt(stmt.clone())),
                _ => Err(format!(
                    "pattern {text:?}: a statement pattern is an assignment, a call, or a head like \
                     `repeat (?) {{}}` (its body empty)"
                )),
            };
        }
        let expr = gml::parse_expression(&code).map_err(|e| format!("pattern {text:?}: {e}"))?;
        if is_hole(&expr) {
            return Err(format!("pattern {text:?} would match anything"));
        }
        Ok(Pattern::Expr(expr))
    }

    /// The variable a plain `name = ?` pattern sets (its place has no hole), if it's one.
    pub fn sets(&self) -> Option<String> {
        match self {
            Pattern::Stmt(Stmt {
                kind:
                    StmtKind::Assign {
                        place,
                        op: AssignOp::Set,
                        ..
                    },
                ..
            }) => {
                let text = gml::print::expr(place);
                (!text.contains(HOLE)).then_some(text)
            }
            _ => None,
        }
    }
}

/// A condition a match depends on, as GML.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum When {
    /// Inside `if (cond)`.
    If(String),
    /// In the `else` of `if (cond)`: the condition was false.
    Not(String),
    /// In a loop's body, given by its head (`repeat (6)`, `while (true)`).
    Loop(String),
    /// Run by the instances `with (target)` picks.
    With(String),
    /// In a `switch` case: `value == case` (several, `||`-joined, when cases fall through).
    Case(String),
    /// After a statement that may `exit` the event, given by its head: reached only if it didn't.
    NotExited(String),
    /// After a drag-and-drop question action (by its function), which decides whether the rest runs.
    Question(String),
}

impl When {
    /// The GML it gives.
    pub fn text(&self) -> &str {
        match self {
            When::If(t)
            | When::Not(t)
            | When::Loop(t)
            | When::With(t)
            | When::Case(t)
            | When::NotExited(t)
            | When::Question(t) => t,
        }
    }
}

/// One match: the node, what was in the holes, and its conditions.
#[derive(Debug)]
pub struct Match<'a> {
    /// The matched statement, or `None` for an expression.
    pub stmt: Option<&'a Stmt>,
    /// The matched expression; for a statement, its value side (an assignment's right-hand side, or
    /// the compound statement's head expression).
    pub expr: &'a Expr,
    /// Where it starts in the unit's source.
    pub start: u32,
    pub end: u32,
    /// What each `?` matched, in order.
    pub holes: Vec<&'a Expr>,
    pub when: Vec<When>,
}

/// Every match of `pattern` in `program`, in source order.
pub fn find<'a>(pattern: &Pattern, program: &'a Program) -> Vec<Match<'a>> {
    let mut finder = Finder {
        pattern,
        when: Vec::new(),
        out: Vec::new(),
    };
    finder.block(&program.body);
    finder.out
}

/// Every statement of `program`, with its conditions, for analyses that look at all of them.
pub fn statements(program: &Program) -> Vec<(&Stmt, Vec<When>)> {
    let mut walker = Walker {
        when: Vec::new(),
        out: Vec::new(),
    };
    walker.block(&program.body);
    walker.out
}

struct Finder<'p, 'a> {
    pattern: &'p Pattern,
    when: Vec<When>,
    out: Vec<Match<'a>>,
}

struct Walker<'a> {
    when: Vec<When>,
    out: Vec<(&'a Stmt, Vec<When>)>,
}

/// How a walk over statements tracks conditions; the walkers above only differ in what they do at each
/// statement and expression.
trait Walk<'a> {
    fn when(&mut self) -> &mut Vec<When>;
    fn at_stmt(&mut self, _stmt: &'a Stmt) {}
    fn at_expr(&mut self, _expr: &'a Expr) {}

    fn block(&mut self, stmts: &'a [Stmt]) {
        let depth = self.when().len();
        for stmt in stmts {
            self.stmt(stmt);
            if contains_exit(stmt) {
                self.when()
                    .push(When::NotExited(gml::print::stmt_head(stmt)));
            }
        }
        self.when().truncate(depth);
    }

    /// Visits `stmt` under one more condition.
    fn under(&mut self, when: When, stmt: &'a Stmt) {
        self.when().push(when);
        self.stmt(stmt);
        self.when().pop();
    }

    fn exprs(&mut self, expr: &'a Expr) {
        self.at_expr(expr);
        match &expr.kind {
            ExprKind::Field { base, .. } => self.exprs(base),
            ExprKind::Index { base, indices } => {
                self.exprs(base);
                indices.iter().for_each(|e| self.exprs(e));
            }
            ExprKind::Call { args, .. } => args.iter().for_each(|e| self.exprs(e)),
            ExprKind::Unary { expr, .. } => self.exprs(expr),
            ExprKind::Binary { lhs, rhs, .. } => {
                self.exprs(lhs);
                self.exprs(rhs);
            }
            ExprKind::Real(_) | ExprKind::Str(_) | ExprKind::Const(_) | ExprKind::Ident(_) => {}
        }
    }

    fn stmt(&mut self, stmt: &'a Stmt) {
        self.at_stmt(stmt);
        let head = || gml::print::stmt_head(stmt);
        match &stmt.kind {
            StmtKind::Block(stmts) => self.block(stmts),
            StmtKind::Assign { place, value, .. } => {
                self.exprs(place);
                self.exprs(value);
            }
            StmtKind::Call(e) | StmtKind::Return(e) | StmtKind::Case(e) => self.exprs(e),
            StmtKind::If {
                cond,
                then,
                otherwise,
            } => {
                self.exprs(cond);
                let text = gml::print::expr(cond);
                self.under(When::If(text.clone()), then);
                if let Some(otherwise) = otherwise {
                    self.under(When::Not(text), otherwise);
                }
            }
            StmtKind::While { cond, body } => {
                self.exprs(cond);
                self.under(When::Loop(head()), body);
            }
            StmtKind::DoUntil { body, cond } => {
                self.under(When::Loop(head()), body);
                self.exprs(cond);
            }
            StmtKind::For {
                init,
                cond,
                step,
                body,
            } => {
                self.stmt(init);
                self.exprs(cond);
                let head = format!("for (...; {}; ...)", gml::print::expr(cond));
                self.under(When::Loop(head.clone()), body);
                self.under(When::Loop(head), step);
            }
            StmtKind::Repeat { count, body } => {
                self.exprs(count);
                self.under(When::Loop(head()), body);
            }
            StmtKind::With { target, body } => {
                self.exprs(target);
                self.under(When::With(gml::print::expr(target)), body);
            }
            StmtKind::Switch { value, body } => {
                self.exprs(value);
                let value = gml::print::expr(value);
                let depth = self.when().len();
                // Cases since the last `break` (control falls through from one to the next).
                let mut cases: Vec<String> = Vec::new();
                for s in body {
                    match &s.kind {
                        StmtKind::Case(e) => {
                            cases.push(format!("{value} == {}", gml::print::expr(e)))
                        }
                        StmtKind::Default => cases.push("default".to_owned()),
                        _ => {
                            self.when().truncate(depth);
                            if !cases.is_empty() {
                                self.when().push(When::Case(cases.join(" || ")));
                            }
                            self.stmt(s);
                            if matches!(s.kind, StmtKind::Break) {
                                cases.clear();
                            }
                        }
                    }
                }
                self.when().truncate(depth);
            }
            StmtKind::Var(_)
            | StmtKind::GlobalVar(_)
            | StmtKind::Default
            | StmtKind::Break
            | StmtKind::Continue
            | StmtKind::Exit
            | StmtKind::Empty => {}
        }
    }
}

impl<'a> Walk<'a> for Walker<'a> {
    fn when(&mut self) -> &mut Vec<When> {
        &mut self.when
    }

    fn at_stmt(&mut self, stmt: &'a Stmt) {
        self.out.push((stmt, self.when.clone()));
    }
}

impl<'a> Walk<'a> for Finder<'_, 'a> {
    fn when(&mut self) -> &mut Vec<When> {
        &mut self.when
    }

    fn at_stmt(&mut self, stmt: &'a Stmt) {
        let Pattern::Stmt(pattern) = self.pattern else {
            return;
        };
        let mut holes = Vec::new();
        if !stmt_matches(pattern, stmt, &mut holes) {
            return;
        }
        let expr = match &stmt.kind {
            StmtKind::Assign { value, .. } => value,
            kind => head_expr(kind).expect("a statement pattern is an assignment or has a head"),
        };
        self.out.push(Match {
            stmt: Some(stmt),
            expr,
            start: stmt.span.start,
            end: stmt.span.end,
            holes,
            when: self.when.clone(),
        });
    }

    fn at_expr(&mut self, expr: &'a Expr) {
        let Pattern::Expr(pattern) = self.pattern else {
            return;
        };
        let mut holes = Vec::new();
        if expr_matches(pattern, expr, &mut holes) {
            self.out.push(Match {
                stmt: None,
                expr,
                start: expr.span.start,
                end: expr.span.end,
                holes,
                when: self.when.clone(),
            });
        }
    }
}

fn is_hole(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::Ident(n) if n == HOLE)
}

fn is_empty_block(stmt: &Stmt) -> bool {
    matches!(&stmt.kind, StmtKind::Block(b) if b.is_empty())
}

/// A compound statement's body (the first, for `if`).
fn body(kind: &StmtKind) -> Option<&Stmt> {
    match kind {
        StmtKind::If { then: body, .. }
        | StmtKind::While { body, .. }
        | StmtKind::DoUntil { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Repeat { body, .. }
        | StmtKind::With { body, .. } => Some(body),
        _ => None,
    }
}

/// A compound statement's head expression: its condition, count or target.
fn head_expr(kind: &StmtKind) -> Option<&Expr> {
    match kind {
        StmtKind::If { cond: e, .. }
        | StmtKind::While { cond: e, .. }
        | StmtKind::DoUntil { cond: e, .. }
        | StmtKind::For { cond: e, .. }
        | StmtKind::Repeat { count: e, .. }
        | StmtKind::With { target: e, .. } => Some(e),
        _ => None,
    }
}

/// Whether `stmt` may end the event with `exit`, anywhere inside it.
fn contains_exit(stmt: &Stmt) -> bool {
    let mut walker = Walker {
        when: Vec::new(),
        out: Vec::new(),
    };
    walker.stmt(stmt);
    walker
        .out
        .iter()
        .any(|(s, _)| matches!(s.kind, StmtKind::Exit))
}

fn stmt_matches<'a>(pattern: &Stmt, stmt: &'a Stmt, holes: &mut Vec<&'a Expr>) -> bool {
    match (&pattern.kind, &stmt.kind) {
        (
            StmtKind::Assign {
                place: pp,
                op: po,
                value: pv,
            },
            StmtKind::Assign { place, op, value },
        ) => po == op && expr_matches(pp, place, holes) && expr_matches(pv, value, holes),
        (p, s) if std::mem::discriminant(p) == std::mem::discriminant(s) => {
            match (head_expr(p), head_expr(s)) {
                (Some(ph), Some(sh)) => expr_matches(ph, sh, holes),
                _ => false,
            }
        }
        _ => false,
    }
}

fn expr_matches<'a>(pattern: &Expr, expr: &'a Expr, holes: &mut Vec<&'a Expr>) -> bool {
    if is_hole(pattern) {
        holes.push(expr);
        return true;
    }
    let all = |ps: &[Expr], es: &'a [Expr], holes: &mut Vec<&'a Expr>| {
        ps.len() == es.len() && ps.iter().zip(es).all(|(p, e)| expr_matches(p, e, holes))
    };
    match (&pattern.kind, &expr.kind) {
        (ExprKind::Real(a), ExprKind::Real(b)) => a == b,
        (ExprKind::Str(a), ExprKind::Str(b)) => a == b,
        (ExprKind::Const(a), ExprKind::Const(b)) => a == b,
        (ExprKind::Ident(a), ExprKind::Ident(b)) => a == b,
        (ExprKind::Field { base: pb, name: pn }, ExprKind::Field { base, name }) => {
            pn.name == name.name && expr_matches(pb, base, holes)
        }
        (
            ExprKind::Index {
                base: pb,
                indices: pi,
            },
            ExprKind::Index { base, indices },
        ) => expr_matches(pb, base, holes) && all(pi, indices, holes),
        (ExprKind::Call { name: pn, args: pa }, ExprKind::Call { name, args }) => {
            pn.name == name.name && all(pa, args, holes)
        }
        (ExprKind::Unary { op: po, expr: pe }, ExprKind::Unary { op, expr }) => {
            po == op && expr_matches(pe, expr, holes)
        }
        (
            ExprKind::Binary {
                op: po,
                lhs: pl,
                rhs: pr,
            },
            ExprKind::Binary { op, lhs, rhs },
        ) => po == op && expr_matches(pl, lhs, holes) && expr_matches(pr, rhs, holes),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each match as (matched source, holes, conditions).
    fn matches(pattern: &str, code: &str) -> Vec<(String, Vec<String>, Vec<When>)> {
        let program = gml::parse_program(code).unwrap();
        find(&Pattern::parse(pattern).unwrap(), &program)
            .into_iter()
            .map(|m| {
                (
                    code[m.start as usize..m.end as usize].to_owned(),
                    m.holes.iter().map(|h| gml::print::expr(h)).collect(),
                    m.when,
                )
            })
            .collect()
    }

    #[test]
    fn statement_patterns_match_assignments_with_their_conditions() {
        let code = "hp = 1;\nif (a > 1) { hp = 3 } else if b hp = rand(1,2)\nhp += 1;";
        let found = matches("hp = ?", code);
        assert_eq!(found.len(), 3);
        assert_eq!(found[0], ("hp = 1".into(), vec!["1".into()], vec![]));
        assert_eq!(found[1].1, ["3"]);
        assert_eq!(found[1].2, [When::If("a > 1".into())]);
        assert_eq!(found[2].1, ["rand(1, 2)"]);
        assert_eq!(
            found[2].2,
            [When::Not("a > 1".into()), When::If("b".into())]
        );
        // `+=` is another statement.
        assert_eq!(matches("hp += ?", code).len(), 1);
    }

    #[test]
    fn expression_patterns_match_anywhere() {
        let code = "if (dist < 90 and oPlayer1.y > y + 16) status = ATTACK;\nx = (dist<160);";
        let found = matches("dist < ?", code);
        assert_eq!(
            found.iter().map(|m| m.1[0].as_str()).collect::<Vec<_>>(),
            ["90", "160"]
        );
        assert_eq!(found[0].2, []);
        // A call pattern finds calls used as statements and as values.
        assert_eq!(
            matches("playSound(?)", "playSound(a); x = playSound(b)").len(),
            2
        );
        assert!(Pattern::parse("?").is_err());
    }

    #[test]
    fn heads_loops_withs_switches_and_exits() {
        let code = "repeat (6) { with (obj) { xVel = 1 } }\nswitch (s) { case 1: case 2: y = 1; break; default: y = 2 }\nif (a) exit;\ny = 3;";
        let found = matches("repeat (?) {}", code);
        assert_eq!(found[0].1, ["6"]);
        let found = matches("xVel = ?", code);
        assert_eq!(
            found[0].2,
            [When::Loop("repeat (6)".into()), When::With("obj".into())]
        );
        let found = matches("y = ?", code);
        assert_eq!(found[0].2, [When::Case("s == 1 || s == 2".into())]);
        assert_eq!(found[1].2, [When::Case("default".into())]);
        assert_eq!(found[2].2, [When::NotExited("if (a)".into())]);
        assert!(Pattern::parse("if (?) { x = 1 }").is_err());
    }
}
