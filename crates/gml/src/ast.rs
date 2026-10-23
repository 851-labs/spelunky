//! The GML syntax tree. It is purely syntactic: names are not resolved, and `=` inside an expression is
//! already turned into [`BinOp::Eq`] (GM8 treats it as comparison there).

use crate::span::Span;

/// A parsed code unit: a script, a code action, or (via [`crate::parse_expression`]) a DnD argument.
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    /// `{ ... }` or `begin ... end`.
    Block(Vec<Stmt>),
    /// `var a, b;`: declares locals for the rest of the unit.
    Var(Vec<Ident>),
    /// `globalvar a, b;`: makes bare `a` refer to `global.a` from here on, game-wide.
    GlobalVar(Vec<Ident>),
    /// `place op value`, where `op` is `=`, `+=`, and so on. `place` is an identifier, a field, or an
    /// indexed variable.
    Assign {
        place: Expr,
        op: AssignOp,
        value: Expr,
    },
    /// A function or script call used as a statement.
    Call(Expr),
    If {
        cond: Expr,
        then: Box<Stmt>,
        otherwise: Option<Box<Stmt>>,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
    },
    DoUntil {
        body: Box<Stmt>,
        cond: Expr,
    },
    For {
        init: Box<Stmt>,
        cond: Expr,
        step: Box<Stmt>,
        body: Box<Stmt>,
    },
    Repeat {
        count: Expr,
        body: Box<Stmt>,
    },
    With {
        target: Expr,
        body: Box<Stmt>,
    },
    /// GM8 compiles `case`/`default` as labels inside the switch body, so the body is kept as a flat list
    /// of statements with [`StmtKind::Case`] and [`StmtKind::Default`] in it; control falls through.
    Switch {
        value: Expr,
        body: Vec<Stmt>,
    },
    Case(Expr),
    Default,
    Break,
    Continue,
    Exit,
    Return(Expr),
    /// A lone `;` used as a statement body, e.g. `if (x) ;`.
    Empty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignOp {
    Set,
    Add,
    Sub,
    Mul,
    Div,
    BitAnd,
    BitOr,
    BitXor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Real(f64),
    Str(String),
    /// A keyword constant: `true`, `false`, `self`, `other`, `all`, `noone`, `global`.
    Const(Const),
    /// A bare name: a local, an instance or global variable, a built-in variable, a built-in constant, or a
    /// resource name. Which one is decided later, not by the parser.
    Ident(String),
    /// `base.name`. `base` is any expression (GM8 evaluates it to an object or instance id, or one of the
    /// keyword constants).
    Field {
        base: Box<Expr>,
        name: Ident,
    },
    /// `base[i]` or `base[i, j]`. GM8 arrays are at most two-dimensional.
    Index {
        base: Box<Expr>,
        indices: Vec<Expr>,
    },
    /// `name(args)`: a built-in function or a game script. GM8 allows at most 16 arguments.
    Call {
        name: Ident,
        args: Vec<Expr>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Const {
    True,
    False,
    SelfKw,
    Other,
    All,
    Noone,
    Global,
}

impl Const {
    /// The value GM8 gives the keyword.
    pub fn value(self) -> f64 {
        match self {
            Const::True => 1.0,
            Const::False => 0.0,
            Const::SelfKw => -1.0,
            Const::Other => -2.0,
            Const::All => -3.0,
            Const::Noone => -4.0,
            Const::Global => -5.0,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Const::True => "true",
            Const::False => "false",
            Const::SelfKw => "self",
            Const::Other => "other",
            Const::All => "all",
            Const::Noone => "noone",
            Const::Global => "global",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnOp {
    /// `-x`
    Neg,
    /// `+x` (a no-op on reals)
    Plus,
    /// `!x` / `not x`
    Not,
    /// `~x`
    BitNot,
}

/// Binary operators. GM8 evaluates **both** operands of `and`/`or`/`xor` (no short-circuit, per OpenGMK's
/// `gm8emulator/src/gml/runtime.rs` `Node::Binary`; verify against oracle), so these are ordinary operators
/// here, not control flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinOp {
    And,
    Or,
    Xor,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Mod,
}

impl BinOp {
    /// GM8 binding strength, higher binds tighter. GM8 differs from C: `and`, `or` and `xor` share one
    /// level, and the bitwise operators bind tighter than comparisons. Taken from OpenGMK's
    /// `gml-parser/src/ast.rs` `get_op_precedence`; verify against oracle.
    pub fn precedence(self) -> u8 {
        match self {
            BinOp::And | BinOp::Or | BinOp::Xor => 0,
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 1,
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => 2,
            BinOp::Shl | BinOp::Shr => 3,
            BinOp::Add | BinOp::Sub => 4,
            BinOp::Mul | BinOp::Div | BinOp::IntDiv | BinOp::Mod => 5,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            BinOp::And => "&&",
            BinOp::Or => "||",
            BinOp::Xor => "^^",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::BitAnd => "&",
            BinOp::BitOr => "|",
            BinOp::BitXor => "^",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::IntDiv => "div",
            BinOp::Mod => "mod",
        }
    }
}

impl AssignOp {
    pub fn as_str(self) -> &'static str {
        match self {
            AssignOp::Set => "=",
            AssignOp::Add => "+=",
            AssignOp::Sub => "-=",
            AssignOp::Mul => "*=",
            AssignOp::Div => "/=",
            AssignOp::BitAnd => "&=",
            AssignOp::BitOr => "|=",
            AssignOp::BitXor => "^=",
        }
    }
}

impl UnOp {
    pub fn as_str(self) -> &'static str {
        match self {
            UnOp::Neg => "-",
            UnOp::Plus => "+",
            UnOp::Not => "!",
            UnOp::BitNot => "~",
        }
    }
}
