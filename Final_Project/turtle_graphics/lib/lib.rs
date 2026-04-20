//! Turtle Graphics — library crate.
//!
//! Exposes every module so that integration tests (and anyone embedding the
//! interpreter in another program) can reach the full public API.
//!
//! The binary entry point (`src/main.rs`) uses this library via
//! `use turtle_graphics::...` rather than declaring its own `mod` blocks.

pub mod ast;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod renderer;
pub mod turtle;
