//! The execution context of one event or script call (runtime.md sections 6 and 12). OpenGMK
//! `gml/context.rs`.

use crate::error::R;
use crate::list::Inst;
use crate::program::{Event, ObjectId};
use crate::value::Value;

/// GM8 passes at most 16 arguments, and every context has all 16.
pub const MAX_ARGS: usize = 16;

/// `self`, `other`, the return value, the arguments, and the event being run with the object that
/// defines it (for `event_inherited`).
///
/// `with` swaps `this`/`other` in place and restores them after the loop (generated code does this), so
/// `ret` is shared with the enclosing code.
#[derive(Clone, Debug)]
pub struct Ctx {
    pub this: Inst,
    pub other: Inst,
    /// The script's return value: set by `return`, and by every assignment and call in scripts that can
    /// end without one (GM8 returns the last value computed).
    pub ret: Value,
    /// The arguments, up to the highest one passed or written; the rest read as 0. Empty for events, so
    /// starting an event allocates nothing. Measured: a fixed `[Value; 16]` made the frame benchmark 2.4x
    /// slower, and a 4-inline `SmallVec` made events 1.6x and script calls 10% slower than a `Vec`.
    args: Vec<Value>,
    argc: usize,
    event: Event,
    event_object: ObjectId,
}

impl Ctx {
    /// The context of an event: no arguments.
    pub fn event(this: Inst, other: Inst, event: Event, event_object: ObjectId) -> Ctx {
        Ctx {
            this,
            other,
            ret: Value::Real(0.0),
            args: Vec::new(),
            argc: 0,
            event,
            event_object,
        }
    }

    /// The event being run, as its defining object defines it.
    pub fn current_event(&self) -> Event {
        self.event
    }

    /// The object whose handler is running (an ancestor of `self`'s object for inherited handlers).
    pub fn event_object(&self) -> ObjectId {
        self.event_object
    }

    /// `argumentN`. Arguments that weren't passed read as 0: GM 8.0 has no "error on uninitialized
    /// arguments" setting (OpenGMK `gm8exe/src/reader.rs`), so the runner treats them as zero.
    #[inline]
    pub fn arg(&self, n: usize) -> R<Value> {
        Ok(self.args.get(n).cloned().unwrap_or_default())
    }

    /// `argumentN = v`. Writes past the 16 arguments are dropped, as GM 8.0 does.
    #[inline]
    pub fn set_arg(&mut self, n: usize, v: Value) -> R<()> {
        if n < MAX_ARGS {
            if self.args.len() <= n {
                self.args.resize(n + 1, Value::Real(0.0));
            }
            self.args[n] = v;
        }
        Ok(())
    }

    /// `argument[i]`.
    #[inline]
    pub fn arg_ix(&self, i: u32) -> R<Value> {
        self.arg(i as usize)
    }

    /// `argument[i] = v`.
    #[inline]
    pub fn set_arg_ix(&mut self, i: u32, v: Value) -> R<()> {
        self.set_arg(i as usize, v)
    }

    /// `argument_count`.
    pub fn argument_count(&self) -> Value {
        Value::Real(self.argc as f64)
    }

    /// The context of a script call: the same `self`, `other` and event, new arguments, a fresh return
    /// value. OpenGMK `gml/context.rs` (`copy_with_args`). GML can't pass more than 16 arguments; extra
    /// ones are dropped.
    pub fn call(&self, args: &[Value]) -> Ctx {
        let mut cx = Ctx::event(self.this, self.other, self.event, self.event_object);
        cx.args = args[..args.len().min(MAX_ARGS)].to_vec();
        cx.argc = cx.args.len();
        cx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_calls_keep_self_and_take_arguments() {
        let cx = Ctx::event(Inst(1), Inst(2), Event::Step, ObjectId(3));
        let mut s = cx.call(&[Value::Real(5.0), Value::from("a")]);
        assert_eq!((s.this, s.other), (Inst(1), Inst(2)));
        assert_eq!(s.current_event(), Event::Step);
        assert_eq!(s.arg(1), Ok(Value::from("a")));
        assert_eq!(s.arg(2), Ok(Value::Real(0.0)));
        assert_eq!(s.arg_ix(40), Ok(Value::Real(0.0)));
        assert_eq!(s.argument_count(), Value::Real(2.0));
        s.set_arg(15, Value::Real(1.0)).unwrap();
        assert_eq!(s.arg(15), Ok(Value::Real(1.0)));
        s.set_arg(16, Value::Real(1.0)).unwrap();
    }
}
