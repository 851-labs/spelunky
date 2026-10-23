// Stub of the runtime API that transpiled GML targets (docs/design/runtime.md, section 12). Signatures
// only: it exists so the transpiler prototype's output can be type-checked. Its `Value` stands in for
// `gm::Value` (same shape: a real or a shared string), and its operators map one-to-one onto
// `gm::Value`'s. Written into the generated crate as `src/rt.rs`; it is not part of the workspace.
#![allow(dead_code, unused_variables, non_upper_case_globals, non_snake_case, clippy::all)]

use std::sync::Arc;

pub type R<T> = Result<T, Error>;

/// A GML runtime error (unknown variable, wrong argument type, ...), located by unit and line.
#[derive(Debug)]
pub struct Error;

/// GM8's dynamic value. `Arc<str>` keeps the world `Send` for parallel rollouts.
#[derive(Clone, Debug)]
pub enum Value {
    Real(f64),
    Str(Arc<str>),
}

pub const fn r(x: f64) -> Value {
    Value::Real(x)
}

pub fn s(x: &'static str) -> Value {
    Value::Str(x.into())
}

pub fn b2v(b: bool) -> Value {
    Value::Real(if b { 1.0 } else { 0.0 })
}

/// GM8 truthiness: a real above 0.5; strings are false.
pub fn truthy(v: &Value) -> bool {
    matches!(v, Value::Real(x) if *x >= 0.5)
}

// Operators with GM8 semantics. Arithmetic can fail on type mismatches. `and`/`or`/`xor` take already
// evaluated operands: GM8 does not short-circuit.
pub fn add(a: Value, b: Value) -> R<Value> { unimplemented!() }
pub fn sub(a: Value, b: Value) -> R<Value> { unimplemented!() }
pub fn mul(a: Value, b: Value) -> R<Value> { unimplemented!() }
pub fn div(a: Value, b: Value) -> R<Value> { unimplemented!() }
pub fn idiv(a: Value, b: Value) -> R<Value> { unimplemented!() }
pub fn modulo(a: Value, b: Value) -> R<Value> { unimplemented!() }
pub fn bitand(a: Value, b: Value) -> R<Value> { unimplemented!() }
pub fn bitor(a: Value, b: Value) -> R<Value> { unimplemented!() }
pub fn bitxor(a: Value, b: Value) -> R<Value> { unimplemented!() }
pub fn shl(a: Value, b: Value) -> R<Value> { unimplemented!() }
pub fn shr(a: Value, b: Value) -> R<Value> { unimplemented!() }
pub fn neg(a: Value) -> R<Value> { unimplemented!() }
pub fn bitnot(a: Value) -> R<Value> { unimplemented!() }
/// `!a` / `not a`: GM8 rejects strings (`gm::Value::not`), so this is not `!truthy(..)`.
pub fn not(a: Value) -> R<Value> { unimplemented!() }
/// `op=` on a user variable or local: GM8 ignores a type error and keeps the old value.
pub fn lenient(op: fn(Value, Value) -> R<Value>, old: Value, rhs: Value) -> Value { unimplemented!() }
/// Comparisons use GM8's epsilon on reals.
pub fn eq(a: &Value, b: &Value) -> R<bool> { unimplemented!() }
pub fn ne(a: &Value, b: &Value) -> R<bool> { unimplemented!() }
pub fn lt(a: &Value, b: &Value) -> R<bool> { unimplemented!() }
pub fn le(a: &Value, b: &Value) -> R<bool> { unimplemented!() }
pub fn gt(a: &Value, b: &Value) -> R<bool> { unimplemented!() }
pub fn ge(a: &Value, b: &Value) -> R<bool> { unimplemented!() }
/// `switch` matches cases by exact equality, not epsilon.
pub fn case_eq(a: &Value, b: &Value) -> bool { unimplemented!() }
pub fn and(a: bool, b: bool) -> bool { a && b }
pub fn or(a: bool, b: bool) -> bool { a || b }
pub fn xor(a: bool, b: bool) -> bool { a != b }
/// `repeat (n)` runs `round(n)` times.
pub fn repeat_count(v: Value) -> R<u32> { unimplemented!() }
/// Array index (GM8 rounds and range-checks).
pub fn ix(v: Value) -> R<u32> { unimplemented!() }

/// An instance handle (slot in the instance table, not the GM instance id).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Inst(pub u32);

/// A variable id: built-ins first, then every user variable name in the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Var(pub u16);
/// A global variable id (`global.name` or a `globalvar`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GVar(pub u16);

/// A `var` local: unset until assigned (reading it then is a GML error).
#[derive(Clone, Debug, Default)]
pub struct Local(Option<Value>);
impl Local {
    pub fn get(&self) -> R<Value> { unimplemented!() }
    pub fn set(&mut self, v: Value) { self.0 = Some(v) }
    pub fn get_a(&self, i: u32, j: u32) -> R<Value> { unimplemented!() }
    pub fn set_a(&mut self, i: u32, j: u32, v: Value) { unimplemented!() }
}

/// The execution context of one event or script call. `with` swaps `this`/`other` in place.
#[derive(Clone)]
pub struct Ctx {
    pub this: Inst,
    pub other: Inst,
    pub ret: Value,
}
impl Ctx {
    /// `argumentN`.
    pub fn arg(&self, n: usize) -> R<Value> { unimplemented!() }
    pub fn set_arg(&mut self, n: usize, v: Value) -> R<()> { unimplemented!() }
    /// `argument[i]`.
    pub fn arg_ix(&self, i: u32) -> R<Value> { unimplemented!() }
    pub fn set_arg_ix(&mut self, i: u32, v: Value) -> R<()> { unimplemented!() }
    /// `argument_count`.
    pub fn argument_count(&self) -> Value { unimplemented!() }
    /// The context for a script call: same `self`/`other`, new arguments and return value.
    pub fn call(&self, args: &[Value]) -> Ctx { unimplemented!() }
}

/// Non-borrowing iterator over a `with` target (GM8 snapshots the count, skips destroyed instances).
pub struct WithIter;
impl WithIter {
    pub fn next(&mut self, g: &Game) -> Option<Inst> { unimplemented!() }
}

pub struct Game;
impl Game {
    /// Records the executing GML unit and line, for divergence reports. Compiled out in release.
    #[inline(always)]
    pub fn at(&mut self, unit: &'static str, line: u32) {}
    pub fn get(&self, h: Inst, v: Var) -> R<Value> { unimplemented!() }
    pub fn set(&mut self, h: Inst, v: Var, x: Value) -> R<()> { unimplemented!() }
    pub fn get_a(&self, h: Inst, v: Var, i: u32, j: u32) -> R<Value> { unimplemented!() }
    pub fn set_a(&mut self, h: Inst, v: Var, i: u32, j: u32, x: Value) -> R<()> { unimplemented!() }
    /// `target.v` where `target` is any value: self/other/all/noone/global, an object, or an instance id.
    /// Reads the first matching instance.
    pub fn get_on(&self, cx: &Ctx, target: &Value, v: Var) -> R<Value> { unimplemented!() }
    /// Writes every matching instance.
    pub fn set_on(&mut self, cx: &Ctx, target: &Value, v: Var, x: Value) -> R<()> { unimplemented!() }
    pub fn get_on_a(&self, cx: &Ctx, target: &Value, v: Var, i: u32, j: u32) -> R<Value> { unimplemented!() }
    pub fn set_on_a(&mut self, cx: &Ctx, target: &Value, v: Var, i: u32, j: u32, x: Value) -> R<()> {
        unimplemented!()
    }
    pub fn gget(&self, v: GVar) -> R<Value> { unimplemented!() }
    pub fn gset(&mut self, v: GVar, x: Value) -> R<()> { unimplemented!() }
    pub fn gget_a(&self, v: GVar, i: u32, j: u32) -> R<Value> { unimplemented!() }
    pub fn gset_a(&mut self, v: GVar, i: u32, j: u32, x: Value) -> R<()> { unimplemented!() }
    /// Game-wide built-ins (`room`, `view_xview[i]`, `keyboard_key`, ...).
    pub fn bget(&self, v: Var, i: u32) -> R<Value> { unimplemented!() }
    pub fn bset(&mut self, v: Var, i: u32, x: Value) -> R<()> { unimplemented!() }
    pub fn with_iter(&self, cx: &Ctx, target: &Value) -> R<WithIter> { unimplemented!() }
    /// DnD "call parent event": runs the parent object's handler for the current event.
    pub fn event_inherited(&mut self, cx: &mut Ctx) -> R<()> { unimplemented!() }
}
