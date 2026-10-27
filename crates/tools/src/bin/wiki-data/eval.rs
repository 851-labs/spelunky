//! Evaluates the small part of GML a stat needs: arithmetic on numbers and on the variables a table binds,
//! so a constant like `-1 * 2.5` reads as -2.5, `rand(2, 5)` as a range, and a price formula as a price.
//! Anything else (a function call, an unbound variable) is unknown, and the caller says so rather than
//! guessing.

use std::collections::BTreeMap;

use gml::Expr;
use gml::ast::{BinOp, Const, ExprKind, UnOp};

/// A GML value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Real(f64),
    Str(String),
}

impl Value {
    pub fn real(&self) -> Option<f64> {
        match *self {
            Value::Real(v) => Some(v),
            Value::Str(_) => None,
        }
    }
}

/// Variables by their GML text (`cost`, `global.currLevel`, `gem.value`, `shopType`).
pub type Env = BTreeMap<String, Value>;

/// A number literal's value as the source writes it. GM8's runner reads some decimals 1 ulp off
/// (`gml::lexer::decimal_value`: `0.6` is 0.6000000000000001); a wiki shows the number in the code, and
/// the difference never reaches a stat.
fn literal(expr: &Expr, source: &str) -> f64 {
    match expr.kind {
        ExprKind::Real(v) => expr
            .span
            .text(source)
            .trim_matches(|c: char| c == '(' || c == ')' || c.is_whitespace())
            .parse::<f64>()
            .ok()
            .filter(|p| p.is_finite())
            .unwrap_or(v),
        _ => unreachable!("not a literal"),
    }
}

/// GML truthiness: a real of at least 0.5 is true.
pub(crate) fn truthy(v: f64) -> bool {
    v >= 0.5
}

fn bool_value(b: bool) -> f64 {
    f64::from(u8::from(b))
}

/// The value of `expr` with the variables in `env`, or `None` if it needs something else.
pub fn eval(expr: &Expr, source: &str, env: &Env) -> Option<Value> {
    let real = |e: &Expr| eval(e, source, env)?.real();
    Some(match &expr.kind {
        ExprKind::Real(_) => Value::Real(literal(expr, source)),
        ExprKind::Str(s) => Value::Str(s.clone()),
        ExprKind::Const(Const::True) => Value::Real(1.0),
        ExprKind::Const(Const::False) => Value::Real(0.0),
        ExprKind::Const(_) => return None,
        ExprKind::Ident(_) | ExprKind::Field { .. } | ExprKind::Index { .. } => {
            env.get(&gml::print::expr(expr))?.clone()
        }
        ExprKind::Call { name, args } => {
            // A call an area binds by its text, as it binds a variable (Olmec's Lair, `isRoom("rOlmec")`).
            if let Some(v) = env.get(&gml::print::expr(expr)) {
                return Some(v.clone());
            }
            let args: Option<Vec<f64>> = args.iter().map(real).collect();
            let args = args?;
            Value::Real(match (name.name.as_str(), args.as_slice()) {
                ("floor", [a]) => a.floor(),
                ("ceil", [a]) => a.ceil(),
                ("abs", [a]) => a.abs(),
                ("min", [_, ..]) => args.iter().copied().fold(f64::INFINITY, f64::min),
                ("max", [_, ..]) => args.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                _ => return None,
            })
        }
        ExprKind::Unary { op, expr } => {
            let v = real(expr)?;
            Value::Real(match op {
                UnOp::Neg => -v,
                UnOp::Plus => v,
                UnOp::Not => bool_value(!truthy(v)),
                UnOp::BitNot => return None,
            })
        }
        ExprKind::Binary { op, lhs, rhs } => {
            let (a, b) = (eval(lhs, source, env)?, eval(rhs, source, env)?);
            if let (Value::Str(a), Value::Str(b)) = (&a, &b) {
                return Some(match op {
                    BinOp::Add => Value::Str(format!("{a}{b}")),
                    BinOp::Eq => Value::Real(bool_value(a == b)),
                    BinOp::Ne => Value::Real(bool_value(a != b)),
                    _ => return None,
                });
            }
            let (a, b) = (a.real()?, b.real()?);
            Value::Real(match op {
                BinOp::Add => a + b,
                BinOp::Sub => a - b,
                BinOp::Mul => a * b,
                BinOp::Div if b != 0.0 => a / b,
                BinOp::Eq => bool_value(a == b),
                BinOp::Ne => bool_value(a != b),
                BinOp::Lt => bool_value(a < b),
                BinOp::Le => bool_value(a <= b),
                BinOp::Gt => bool_value(a > b),
                BinOp::Ge => bool_value(a >= b),
                BinOp::And => bool_value(truthy(a) && truthy(b)),
                BinOp::Or => bool_value(truthy(a) || truthy(b)),
                BinOp::Xor => bool_value(truthy(a) != truthy(b)),
                // The rest (integer division, mod, bit operations) don't come up in a stat.
                _ => return None,
            })
        }
    })
}

/// Whether a condition holds with the variables in `env`, or `None` when that depends on something else.
/// Unlike [`eval`], `&&` is false when either side is, and `||` true when either side is, whatever the
/// other: `global.darkLevel && rand(1, 40) == 1` is false on a level that isn't dark.
pub fn truth(expr: &Expr, source: &str, env: &Env) -> Option<bool> {
    match &expr.kind {
        ExprKind::Binary {
            op: op @ (BinOp::And | BinOp::Or),
            lhs,
            rhs,
        } => {
            // The value that decides it alone: false for `&&`, true for `||`.
            let decisive = *op == BinOp::Or;
            match (truth(lhs, source, env), truth(rhs, source, env)) {
                (Some(a), _) if a == decisive => Some(decisive),
                (_, Some(b)) if b == decisive => Some(decisive),
                (Some(_), Some(_)) => Some(!decisive),
                _ => None,
            }
        }
        ExprKind::Unary {
            op: UnOp::Not,
            expr,
        } => truth(expr, source, env).map(|t| !t),
        _ => eval(expr, source, env)?.real().map(truthy),
    }
}

/// The values a random expression can take.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct Range {
    #[serde(serialize_with = "crate::serialize_number")]
    pub min: f64,
    #[serde(serialize_with = "crate::serialize_number")]
    pub max: f64,
    /// Whether it's a whole number: `rand(a, b)` (the game's script, `floor(random(b - a + 1)) + a`)
    /// gives every integer from `a` to `b`; `random(n)` gives a real from 0 up to but not including `n`.
    pub integer: bool,
}

/// The range of an expression built from `rand` and `random` with constant arguments and arithmetic
/// with constants, or `None` if it isn't one (or isn't random at all).
pub fn range(expr: &Expr, source: &str) -> Option<Range> {
    let constant = |e: &Expr| eval(e, source, &Env::new())?.real();
    match &expr.kind {
        ExprKind::Call { name, args } => match (name.name.as_str(), args.as_slice()) {
            ("rand", [a, b]) => {
                let (a, b) = (constant(a)?, constant(b)?);
                Some(Range {
                    min: a.min(b),
                    max: a.max(b),
                    integer: a.fract() == 0.0 && b.fract() == 0.0,
                })
            }
            ("random", [n]) => {
                let n = constant(n)?;
                Some(Range {
                    min: n.min(0.0),
                    max: n.max(0.0),
                    integer: false,
                })
            }
            _ => None,
        },
        ExprKind::Unary {
            op: UnOp::Neg,
            expr,
        } => {
            let r = range(expr, source)?;
            Some(Range {
                min: -r.max,
                max: -r.min,
                ..r
            })
        }
        ExprKind::Binary { op, lhs, rhs } => {
            // Each side is a range or a constant (a range of one value), and at least one is random.
            let side = |e: &Expr| {
                range(e, source).map(|r| (r, true)).or_else(|| {
                    let v = constant(e)?;
                    Some((
                        Range {
                            min: v,
                            max: v,
                            integer: v.fract() == 0.0,
                        },
                        false,
                    ))
                })
            };
            let ((a, ra), (b, rb)) = (side(lhs)?, side(rhs)?);
            if !ra && !rb {
                return None;
            }
            let integer = a.integer && b.integer;
            let (min, max) = match op {
                BinOp::Add => (a.min + b.min, a.max + b.max),
                BinOp::Sub => (a.min - b.max, a.max - b.min),
                BinOp::Mul => {
                    let products = [a.min * b.min, a.min * b.max, a.max * b.min, a.max * b.max];
                    (
                        products.iter().copied().fold(f64::INFINITY, f64::min),
                        products.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    )
                }
                _ => return None,
            };
            Some(Range { min, max, integer })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval_str(code: &str, env: &[(&str, f64)]) -> Option<Value> {
        let expr = gml::parse_expression(code).unwrap();
        let env = env
            .iter()
            .map(|&(k, v)| (k.to_owned(), Value::Real(v)))
            .collect();
        eval(&expr, code, &env)
    }

    fn range_str(code: &str) -> Option<Range> {
        range(&gml::parse_expression(code).unwrap(), code)
    }

    #[test]
    fn evaluates_constants_and_bound_variables() {
        assert_eq!(eval_str("-1 * 2.5", &[]), Some(Value::Real(-2.5)));
        // As written, not as GM8's runner reads it (0.6000000000000001).
        assert_eq!(eval_str("0.6", &[]), Some(Value::Real(0.6)));
        assert_eq!(eval_str("( 0.6 ) * 2", &[]), Some(Value::Real(1.2)));
        assert_eq!(eval_str("true", &[]), Some(Value::Real(1.0)));
        assert_eq!(eval_str("\"Snake\"", &[]), Some(Value::Str("Snake".into())));
        assert_eq!(eval_str("x + 1", &[]), None);
        assert_eq!(eval_str("rand(1, 2)", &[]), None);
        let price = "cost + (cost / 100) * 10 * (global.currLevel - 2)";
        let env = [("cost", 15000.0), ("global.currLevel", 4.0)];
        assert_eq!(eval_str(price, &env), Some(Value::Real(18000.0)));
        let gem = "gem.value + ceil(gem.value / 4) * global.levelType";
        let env = [("gem.value", 500.0), ("global.levelType", 3.0)];
        assert_eq!(eval_str(gem, &env), Some(Value::Real(875.0)));
        assert_eq!(
            eval_str("global.currLevel > 2", &[("global.currLevel", 2.0)]),
            Some(Value::Real(0.0))
        );
    }

    #[test]
    fn truth_decides_what_it_can() {
        let truth_str = |code: &str, env: &[(&str, Value)]| {
            let expr = gml::parse_expression(code).unwrap();
            let env = env
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect();
            truth(&expr, code, &env)
        };
        let dark = [("global.darkLevel", Value::Real(0.0))];
        let cond = "global.darkLevel and rand(1,40) == 1";
        assert_eq!(truth_str(cond, &dark), Some(false));
        assert_eq!(truth_str("not global.darkLevel or x", &dark), Some(true));
        assert_eq!(truth_str("global.darkLevel or x", &dark), None);
        let shop = [("shopType", Value::Str("Weapon".into()))];
        assert_eq!(truth_str("shopType == \"Weapon\"", &shop), Some(true));
        assert_eq!(truth_str("shopType == \"Bomb\"", &shop), Some(false));
        assert_eq!(truth_str("rand(1, 2) == 1", &shop), None);
        // A call bound by its text, as `--find` prints it.
        let lair = [("isRoom(\"rOlmec\")", Value::Real(1.0))];
        assert_eq!(
            truth_str("isLevel() and not isRoom(\"rOlmec\")", &lair),
            Some(false)
        );
        assert_eq!(
            truth_str("isLevel() and not isRoom(\"rLoadLevel\")", &lair),
            None
        );
    }

    #[test]
    fn ranges_follow_rand_and_random() {
        let r = |min, max, integer| Some(Range { min, max, integer });
        assert_eq!(range_str("rand(100, 1000)"), r(100.0, 1000.0, true));
        assert_eq!(range_str("-1 * rand(2, 5)"), r(-5.0, -2.0, true));
        assert_eq!(range_str("-rand(4, 6)"), r(-6.0, -4.0, true));
        assert_eq!(range_str("random(3) + 1"), r(1.0, 4.0, false));
        assert_eq!(range_str("random(1) - random(1)"), r(-1.0, 1.0, false));
        assert_eq!(range_str("rand(6, 8) + xVel"), None);
        assert_eq!(range_str("2 + 3"), None);
    }
}
