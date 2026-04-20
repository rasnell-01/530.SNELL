//! Interpreter pattern implementation.
//!
//! `execute` is the single recursive dispatch function.  It accepts three
//! mutable state objects and one `Command` node:
//!
//! | Arg       | Type                         | Purpose                        |
//! |-----------|------------------------------|--------------------------------|
//! | `turtle`  | `&mut TurtleState`           | position, heading, pen, color  |
//! | `symbols` | `&mut SymbolTable`           | variable name → f64            |
//! | `procs`   | `&mut ProcTable`             | proc name → Vec<Command>       |
//!
//! Variables and procedures are global in scope: a `set` inside a procedure
//! is visible back in the caller, and a `to` definition inside a `dotimes`
//! loop takes effect immediately for subsequent iterations.

use std::collections::HashMap;
use crate::ast::{Command, Expr, Op};
use crate::turtle::TurtleState;

pub type SymbolTable = HashMap<String, f64>;
pub type ProcTable   = HashMap<String, Vec<Command>>;

// ── Expression evaluation ─────────────────────────────────────────────────────

pub fn eval_expr(expr: &Expr, symbols: &SymbolTable) -> Result<f64, String> {
    match expr {
        Expr::Literal(n) => Ok(*n),

        Expr::Variable(name) => symbols
            .get(name)
            .copied()
            .ok_or_else(|| format!("Undefined variable '{}'", name)),

        Expr::BinOp(left, op, right) => {
            let l = eval_expr(left, symbols)?;
            let r = eval_expr(right, symbols)?;
            match op {
                Op::Add => Ok(l + r),
                Op::Sub => Ok(l - r),
                Op::Mul => Ok(l * r),
                Op::Div => {
                    if r == 0.0 { Err("Division by zero".into()) }
                    else { Ok(l / r) }
                }
            }
        }
    }
}

// ── Command execution ─────────────────────────────────────────────────────────

pub fn execute(
    cmd:     &Command,
    turtle:  &mut TurtleState,
    symbols: &mut SymbolTable,
    procs:   &mut ProcTable,
) -> Result<(), String> {
    match cmd {
        Command::Forward(e) => turtle.forward(eval_expr(e, symbols)?),
        Command::Turn(e)    => turtle.turn(eval_expr(e, symbols)?),
        Command::Pen(e)     => turtle.set_pen(eval_expr(e, symbols)? != 0.0),

        Command::Color(r, g, b) => turtle.set_color(
            eval_expr(r, symbols)?,
            eval_expr(g, symbols)?,
            eval_expr(b, symbols)?,
        ),

        Command::SetVar(name, e) => {
            let val = eval_expr(e, symbols)?;
            symbols.insert(name.clone(), val);
        }

        Command::Print(e) => println!("{}", eval_expr(e, symbols)?),

        // Clone the body to release the &procs borrow before the recursive call.
        Command::Call(name) => {
            let body = procs
                .get(name)
                .cloned()
                .ok_or_else(|| format!("Unknown procedure '{}'", name))?;
            for child in &body {
                execute(child, turtle, symbols, procs)?;
            }
        }

        Command::DoTimes(count_expr, body) => {
            let count = eval_expr(count_expr, symbols)? as u64;
            for _ in 0..count {
                for child in body {
                    execute(child, turtle, symbols, procs)?;
                }
            }
        }

        Command::Procedure(name, body) => {
            procs.insert(name.clone(), body.clone());
        }

        Command::Block(cmds) => {
            for child in cmds {
                execute(child, turtle, symbols, procs)?;
            }
        }
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{eval_expr, execute, ProcTable, SymbolTable};
    use crate::ast::Expr;
    use crate::lexer::tokenize;
    use crate::parser::Parser;
    use crate::turtle::TurtleState;

    /// Run a turtle program and return all three state objects on success.
    fn exec(src: &str) -> Result<(TurtleState, SymbolTable, ProcTable), String> {
        let mut p = Parser::new(tokenize(src));
        let prog = p.parse_program()?;
        let mut turtle  = TurtleState::new();
        let mut symbols = SymbolTable::new();
        let mut procs   = ProcTable::new();
        execute(&prog, &mut turtle, &mut symbols, &mut procs)?;
        Ok((turtle, symbols, procs))
    }

    // ── eval_expr ─────────────────────────────────────────────────────────────

    #[test]
    fn eval_literal() {
        let sym = SymbolTable::new();
        let v = eval_expr(&Expr::Literal(42.0), &sym).unwrap();
        assert!((v - 42.0).abs() < 1e-12);
    }

    #[test]
    fn eval_defined_variable() {
        let mut sym = SymbolTable::new();
        sym.insert("n".into(), 7.5);
        let v = eval_expr(&Expr::Variable("n".into()), &sym).unwrap();
        assert!((v - 7.5).abs() < 1e-12);
    }

    #[test]
    fn eval_undefined_variable_is_error() {
        let err = eval_expr(&Expr::Variable("x".into()), &SymbolTable::new()).unwrap_err();
        assert!(err.contains("'x'"), "{}", err);
    }

    // ── Basic drawing commands ────────────────────────────────────────────────

    #[test]
    fn exec_forward_moves_north() {
        let (t, _, _) = exec("forward 100").unwrap();
        assert!((t.y + 100.0).abs() < 1e-9);
        assert!(t.x.abs() < 1e-9);
    }

    #[test]
    fn exec_turn_changes_angle() {
        let (t, _, _) = exec("turn 90").unwrap();
        assert!((t.angle - 90.0).abs() < 1e-9);
    }

    #[test]
    fn exec_pen_zero_suppresses_drawing() {
        let (t, _, _) = exec("pen 0\nforward 100").unwrap();
        assert!(t.lines.is_empty());
    }

    #[test]
    fn exec_pen_one_restores_drawing() {
        let (t, _, _) = exec("pen 0\npen 1\nforward 100").unwrap();
        assert_eq!(t.lines.len(), 1);
    }

    #[test]
    fn exec_color_applied_to_next_lines() {
        let (t, _, _) = exec("color 255 0 128\nforward 10").unwrap();
        assert_eq!(t.lines[0].color, [255, 0, 128, 255]);
    }

    // ── Variables and arithmetic ──────────────────────────────────────────────

    #[test]
    fn exec_set_stores_value() {
        let (_, sym, _) = exec("set n 99").unwrap();
        assert!((sym["n"] - 99.0).abs() < 1e-12);
    }

    #[test]
    fn exec_variable_used_as_distance() {
        let (t, _, _) = exec("set dist 75\nforward dist").unwrap();
        assert!((t.y + 75.0).abs() < 1e-9);
    }

    #[test]
    fn exec_arithmetic_add() {
        let (t, _, _) = exec("set a 30\nset b 20\nforward (a + b)").unwrap();
        assert!((t.y + 50.0).abs() < 1e-9);
    }

    #[test]
    fn exec_arithmetic_sub() {
        let (t, _, _) = exec("set a 80\nforward (a - 30)").unwrap();
        assert!((t.y + 50.0).abs() < 1e-9);
    }

    #[test]
    fn exec_arithmetic_mul() {
        let (t, _, _) = exec("set n 5\nforward (n * 20)").unwrap();
        assert!((t.y + 100.0).abs() < 1e-9);
    }

    #[test]
    fn exec_arithmetic_div() {
        let (t, _, _) = exec("forward (100 / 4)").unwrap();
        assert!((t.y + 25.0).abs() < 1e-9);
    }

    #[test]
    fn exec_division_by_zero_is_error() {
        let err = exec("forward (1 / 0)").unwrap_err();
        assert!(err.to_lowercase().contains("zero"), "{}", err);
    }

    #[test]
    fn exec_undefined_variable_is_error() {
        let err = exec("forward x").unwrap_err();
        assert!(err.contains("'x'"), "{}", err);
    }

    // ── DoTimes ───────────────────────────────────────────────────────────────

    #[test]
    fn exec_dotimes_repeats_body_n_times() {
        let (t, _, _) = exec("dotimes 6 { forward 10 }").unwrap();
        assert!((t.y + 60.0).abs() < 1e-9);
        assert_eq!(t.lines.len(), 6);
    }

    #[test]
    fn exec_dotimes_zero_runs_no_iterations() {
        let (t, _, _) = exec("dotimes 0 { forward 999 }").unwrap();
        assert!(t.lines.is_empty());
    }

    #[test]
    fn exec_dotimes_closes_square() {
        let (t, _, _) = exec("dotimes 4 { forward 100 turn 90 }").unwrap();
        assert!(t.x.abs() < 1e-9, "x={}", t.x);
        assert!(t.y.abs() < 1e-9, "y={}", t.y);
        assert_eq!(t.lines.len(), 4);
    }

    #[test]
    fn exec_nested_dotimes_multiplies_iterations() {
        // 3 outer × 4 inner = 12 forward calls.
        let (t, _, _) = exec("dotimes 3 { dotimes 4 { forward 1 } }").unwrap();
        assert_eq!(t.lines.len(), 12);
    }

    #[test]
    fn exec_dotimes_variable_accumulates() {
        let (_, sym, _) = exec("set n 0\ndotimes 5 { set n (n + 1) }").unwrap();
        assert!((sym["n"] - 5.0).abs() < 1e-12);
    }

    // ── Procedures ────────────────────────────────────────────────────────────

    #[test]
    fn exec_to_registers_in_proc_table() {
        let (_, _, procs) = exec("to myproc { forward 10 }").unwrap();
        assert!(procs.contains_key("myproc"));
    }

    #[test]
    fn exec_procedure_call_executes_body() {
        let (t, _, _) = exec("to step { forward 50 }\nstep").unwrap();
        assert!((t.y + 50.0).abs() < 1e-9);
    }

    #[test]
    fn exec_procedure_called_multiple_times() {
        let (t, _, _) = exec("to go { forward 10 }\ngo\ngo\ngo").unwrap();
        assert!((t.y + 30.0).abs() < 1e-9);
        assert_eq!(t.lines.len(), 3);
    }

    #[test]
    fn exec_procedure_reads_outer_variable() {
        let src = "set dist 40\nto step { forward dist }\nstep";
        let (t, _, _) = exec(src).unwrap();
        assert!((t.y + 40.0).abs() < 1e-9);
    }

    #[test]
    fn exec_procedure_writes_shared_variable() {
        let src = "set n 0\nto inc { set n (n + 1) }\ninc\ninc\ninc";
        let (_, sym, _) = exec(src).unwrap();
        assert!((sym["n"] - 3.0).abs() < 1e-12);
    }

    #[test]
    fn exec_procedure_calling_another_procedure() {
        let src = "to move { forward 20 }\nto go { move move }\ngo";
        let (t, _, _) = exec(src).unwrap();
        assert!((t.y + 40.0).abs() < 1e-9);
    }

    #[test]
    fn exec_unknown_procedure_is_error() {
        let err = exec("ghost").unwrap_err();
        assert!(err.contains("'ghost'"), "{}", err);
    }

    #[test]
    fn exec_print_does_not_error() {
        assert!(exec("set x 42\nprint x").is_ok());
    }
}
