//! Recursive-descent parser.
//!
//! ```text
//! program  ::= command*
//! command  ::= forward | turn | pen | color | set | print
//!            | dotimes | to | IDENT
//! block    ::= '{' command* '}'
//! expr     ::= NUMBER | IDENT | '(' expr OP expr ')'
//! ```
//!
//! Error messages include the 1-based source line of the offending token.

use crate::ast::{Command, Expr, Op};
use crate::lexer::{Token, TokenStream};

pub struct Parser {
    tokens: Vec<Token>,
    lines:  Vec<usize>,
    pos:    usize,
}

impl Parser {
    pub fn new(stream: TokenStream) -> Self {
        Parser { tokens: stream.tokens, lines: stream.lines, pos: 0 }
    }

    // ── Cursor helpers ────────────────────────────────────────────────────────

    fn current_line(&self) -> usize {
        self.lines.get(self.pos.saturating_sub(1)).copied().unwrap_or(1)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn consume(&mut self) -> Option<Token> {
        let tok = self.tokens.get(self.pos).cloned();
        if tok.is_some() { self.pos += 1; }
        tok
    }

    fn err(&self, msg: impl Into<String>) -> String {
        format!("Line {}: {}", self.current_line(), msg.into())
    }

    // ── Grammar ───────────────────────────────────────────────────────────────

    pub fn parse_program(&mut self) -> Result<Command, String> {
        let mut cmds = Vec::new();
        while self.pos < self.tokens.len() {
            cmds.push(self.parse_command()?);
        }
        Ok(Command::Block(cmds))
    }

    fn parse_command(&mut self) -> Result<Command, String> {
        match self.consume() {
            Some(Token::Forward) => Ok(Command::Forward(self.parse_expr()?)),
            Some(Token::Turn)    => Ok(Command::Turn(self.parse_expr()?)),
            Some(Token::Pen)     => Ok(Command::Pen(self.parse_expr()?)),

            // color <r> <g> <b>
            Some(Token::Color) => {
                let r = self.parse_expr()?;
                let g = self.parse_expr()?;
                let b = self.parse_expr()?;
                Ok(Command::Color(r, g, b))
            }

            Some(Token::Set) => {
                let name = match self.consume() {
                    Some(Token::Ident(s)) => s,
                    other => return Err(self.err(format!(
                        "Expected variable name after 'set', got {:?}", other))),
                };
                Ok(Command::SetVar(name, self.parse_expr()?))
            }

            Some(Token::Print) => Ok(Command::Print(self.parse_expr()?)),

            Some(Token::DoTimes) => {
                let count = self.parse_expr()?;
                let body  = self.parse_block()?;
                Ok(Command::DoTimes(count, body))
            }

            Some(Token::To) => {
                let name = match self.consume() {
                    Some(Token::Ident(s)) => s,
                    other => return Err(self.err(format!(
                        "Expected procedure name after 'to', got {:?}", other))),
                };
                let body = self.parse_block()?;
                Ok(Command::Procedure(name, body))
            }

            // Any bare identifier is a procedure call.
            Some(Token::Ident(name)) => Ok(Command::Call(name)),

            other => Err(self.err(format!("Unexpected token: {:?}", other))),
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Command>, String> {
        match self.consume() {
            Some(Token::LBrace) => {}
            other => return Err(self.err(format!(
                "Expected '{{' to open block, got {:?}", other))),
        }
        let mut cmds = Vec::new();
        while !matches!(self.peek(), Some(Token::RBrace) | None) {
            cmds.push(self.parse_command()?);
        }
        match self.consume() {
            Some(Token::RBrace) => {}
            other => return Err(self.err(format!(
                "Expected '}}' to close block, got {:?}", other))),
        }
        Ok(cmds)
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        match self.consume() {
            Some(Token::Number(n))  => Ok(Expr::Literal(n)),
            Some(Token::Ident(s))   => Ok(Expr::Variable(s)),
            Some(Token::LParen) => {
                let left  = self.parse_expr()?;
                let op    = self.parse_op()?;
                let right = self.parse_expr()?;
                match self.consume() {
                    Some(Token::RParen) => {}
                    other => return Err(self.err(format!(
                        "Expected ')' to close expression, got {:?}", other))),
                }
                Ok(Expr::BinOp(Box::new(left), op, Box::new(right)))
            }
            other => Err(self.err(format!(
                "Expected expression (number / variable / '('), got {:?}", other))),
        }
    }

    fn parse_op(&mut self) -> Result<Op, String> {
        match self.consume() {
            Some(Token::Plus)  => Ok(Op::Add),
            Some(Token::Minus) => Ok(Op::Sub),
            Some(Token::Star)  => Ok(Op::Mul),
            Some(Token::Slash) => Ok(Op::Div),
            other => Err(self.err(format!(
                "Expected operator (+,-,*,/), got {:?}", other))),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::Parser;
    use crate::ast::{Command, Expr, Op};
    use crate::lexer::tokenize;

    /// Parse `src` and return the single command inside the top-level Block.
    /// Panics if there is not exactly one command.
    fn parse_one(src: &str) -> Command {
        let mut p = Parser::new(tokenize(src));
        let Command::Block(mut cmds) = p.parse_program().unwrap() else {
            panic!("expected Block");
        };
        assert_eq!(cmds.len(), 1, "expected exactly 1 command, got {}", cmds.len());
        cmds.remove(0)
    }

    // ── Leaf drawing commands ─────────────────────────────────────────────────

    #[test]
    fn parse_forward_literal() {
        let cmd = parse_one("forward 100");
        assert!(matches!(cmd, Command::Forward(Expr::Literal(v)) if (v - 100.0).abs() < 1e-12));
    }

    #[test]
    fn parse_turn_negative() {
        let cmd = parse_one("turn -45");
        assert!(matches!(cmd, Command::Turn(Expr::Literal(v)) if (v + 45.0).abs() < 1e-12));
    }

    #[test]
    fn parse_pen_zero() {
        let cmd = parse_one("pen 0");
        assert!(matches!(cmd, Command::Pen(Expr::Literal(v)) if v.abs() < 1e-12));
    }

    #[test]
    fn parse_color_three_args() {
        let cmd = parse_one("color 255 0 128");
        let Command::Color(r, g, b) = cmd else { panic!("expected Color"); };
        assert!(matches!(r, Expr::Literal(v) if (v - 255.0).abs() < 1e-12));
        assert!(matches!(g, Expr::Literal(v) if v.abs() < 1e-12));
        assert!(matches!(b, Expr::Literal(v) if (v - 128.0).abs() < 1e-12));
    }

    // ── Variables ─────────────────────────────────────────────────────────────

    #[test]
    fn parse_set_variable() {
        let cmd = parse_one("set n 42");
        let Command::SetVar(name, expr) = cmd else { panic!("expected SetVar"); };
        assert_eq!(name, "n");
        assert!(matches!(expr, Expr::Literal(v) if (v - 42.0).abs() < 1e-12));
    }

    #[test]
    fn parse_print_command() {
        let cmd = parse_one("print 3.14");
        assert!(matches!(cmd, Command::Print(Expr::Literal(v)) if (v - 3.14).abs() < 1e-9));
    }

    // ── Expressions ───────────────────────────────────────────────────────────

    #[test]
    fn parse_expr_variable_reference() {
        let cmd = parse_one("forward n");
        assert!(matches!(cmd, Command::Forward(Expr::Variable(s)) if s == "n"));
    }

    #[test]
    fn parse_expr_binop_add() {
        let cmd = parse_one("forward (n + 5)");
        let Command::Forward(Expr::BinOp(left, op, right)) = cmd else {
            panic!("expected Forward(BinOp)");
        };
        assert!(matches!(*left, Expr::Variable(s) if s == "n"));
        assert!(matches!(op, Op::Add));
        assert!(matches!(*right, Expr::Literal(v) if (v - 5.0).abs() < 1e-12));
    }

    #[test]
    fn parse_expr_binop_mul() {
        let cmd = parse_one("forward (n * 2)");
        let Command::Forward(Expr::BinOp(_, op, _)) = cmd else { panic!(); };
        assert!(matches!(op, Op::Mul));
    }

    #[test]
    fn parse_expr_binop_div() {
        let cmd = parse_one("forward (n / 4)");
        let Command::Forward(Expr::BinOp(_, op, _)) = cmd else { panic!(); };
        assert!(matches!(op, Op::Div));
    }

    // ── Control flow ──────────────────────────────────────────────────────────

    #[test]
    fn parse_dotimes_with_body() {
        let cmd = parse_one("dotimes 4 { forward 100 turn 90 }");
        let Command::DoTimes(count, body) = cmd else { panic!("expected DoTimes"); };
        assert!(matches!(count, Expr::Literal(v) if (v - 4.0).abs() < 1e-12));
        assert_eq!(body.len(), 2);
        assert!(matches!(body[0], Command::Forward(_)));
        assert!(matches!(body[1], Command::Turn(_)));
    }

    #[test]
    fn parse_nested_dotimes() {
        let cmd = parse_one("dotimes 3 { dotimes 4 { forward 10 } }");
        let Command::DoTimes(_, outer_body) = cmd else { panic!(); };
        assert!(matches!(outer_body[0], Command::DoTimes(_, _)));
    }

    // ── Procedures ────────────────────────────────────────────────────────────

    #[test]
    fn parse_to_defines_procedure() {
        let cmd = parse_one("to square { forward 100 turn 90 }");
        let Command::Procedure(name, body) = cmd else { panic!("expected Procedure"); };
        assert_eq!(name, "square");
        assert_eq!(body.len(), 2);
    }

    #[test]
    fn parse_bare_identifier_is_call() {
        let cmd = parse_one("square");
        assert!(matches!(cmd, Command::Call(s) if s == "square"));
    }

    // ── Multi-command programs ────────────────────────────────────────────────

    #[test]
    fn parse_empty_program() {
        let mut p = Parser::new(tokenize(""));
        let Command::Block(cmds) = p.parse_program().unwrap() else { panic!(); };
        assert!(cmds.is_empty());
    }

    #[test]
    fn parse_multiple_commands() {
        let mut p = Parser::new(tokenize("forward 10\nturn 90\nforward 10"));
        let Command::Block(cmds) = p.parse_program().unwrap() else { panic!(); };
        assert_eq!(cmds.len(), 3);
    }

    // ── Error cases ───────────────────────────────────────────────────────────

    #[test]
    fn parse_error_unclosed_block() {
        let mut p = Parser::new(tokenize("dotimes 4 { forward 100"));
        let err = p.parse_program().unwrap_err();
        assert!(err.contains("Line"), "error should name a line: {}", err);
    }

    #[test]
    fn parse_error_missing_expression() {
        // 'forward' with no argument
        let mut p = Parser::new(tokenize("forward"));
        let err = p.parse_program().unwrap_err();
        assert!(err.contains("expression") || err.contains("Line"), "{}", err);
    }

    #[test]
    fn parse_error_set_without_name() {
        let mut p = Parser::new(tokenize("set 42"));
        let err = p.parse_program().unwrap_err();
        assert!(err.contains("variable name") || err.contains("Line"), "{}", err);
    }

    #[test]
    fn parse_error_to_without_name() {
        let mut p = Parser::new(tokenize("to { forward 1 }"));
        let err = p.parse_program().unwrap_err();
        assert!(err.contains("procedure name") || err.contains("Line"), "{}", err);
    }

    #[test]
    fn parse_error_includes_line_number() {
        // Error on line 3.
        let src = "forward 10\nturn 90\nforward";
        let mut p = Parser::new(tokenize(src));
        let err = p.parse_program().unwrap_err();
        assert!(err.starts_with("Line 3:"), "expected 'Line 3:' prefix, got: {}", err);
    }
}
