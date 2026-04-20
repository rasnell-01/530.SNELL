//! Abstract Syntax Tree for the Turtle Graphics language.
//!
//! ## Composite Pattern
//!
//! `Command` is a tree.  Internal (composite) nodes hold `Vec<Command>`:
//!   - `Block`     — every parsed program at its root.
//!   - `DoTimes`   — a counted loop.
//!   - `Procedure` — a named, reusable definition.
//!
//! Leaf nodes carry only expression arguments:
//!   `Forward`, `Turn`, `Pen`, `Color`, `SetVar`, `Print`, `Call`.
//!
//! The interpreter (`interpreter::execute`) walks the tree uniformly through
//! one recursive function; the tree structure drives control flow.
//!
//! ## Interpreter Pattern
//!
//! Each `Command` variant is one grammar production rule.  `execute` dispatches
//! on each variant and "interprets" it against live turtle/symbol/proc state.

/// A value that evaluates to `f64` at runtime.
#[derive(Debug, Clone)]
pub enum Expr {
    Literal(f64),
    Variable(String),
    BinOp(Box<Expr>, Op, Box<Expr>),
}

/// Arithmetic operators, used inside parenthesised expressions.
#[derive(Debug, Clone, Copy)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

/// One node in the command tree.
#[derive(Debug, Clone)]
pub enum Command {
    // ── Leaf: drawing ────────────────────────────────────────────────────────

    /// Move the turtle forward.
    Forward(Expr),
    /// Rotate the turtle.  Positive = clockwise, negative = counter-clockwise.
    Turn(Expr),
    /// `0` lifts the pen (move without drawing); any other value lowers it.
    Pen(Expr),
    /// Set the drawing color: `color <r> <g> <b>` (values 0–255).
    Color(Expr, Expr, Expr),

    // ── Leaf: variables / I-O ────────────────────────────────────────────────

    /// Assign `expr` to a named variable.
    SetVar(String, Expr),
    /// Print the evaluated expression to stdout.
    Print(Expr),

    // ── Leaf: procedure call ─────────────────────────────────────────────────

    /// Invoke a named procedure.
    Call(String),

    // ── Composite: loops ─────────────────────────────────────────────────────

    /// Repeat the body `count` times.
    DoTimes(Expr, Vec<Command>),

    // ── Composite: definition ────────────────────────────────────────────────

    /// Define a named procedure.  Body is stored in the proc table; not run yet.
    Procedure(String, Vec<Command>),

    // ── Composite: sequence ──────────────────────────────────────────────────

    /// An ordered list of commands.  The root of every parsed program.
    Block(Vec<Command>),
}
