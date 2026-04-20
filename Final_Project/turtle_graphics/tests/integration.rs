//! Integration tests for the Turtle Graphics interpreter.
//!
//! Each test runs a complete source program through the full pipeline
//! (lex → parse → interpret) and asserts on the resulting turtle state.
//! These tests exercise the modules working together, not in isolation.

use lib::{interpreter, lexer, parser, turtle};

// ── Test helper ───────────────────────────────────────────────────────────────

/// Run `src` through the complete pipeline.
/// Returns the finished `TurtleState` on success, or an error string.
fn run(src: &str) -> Result<turtle::TurtleState, String> {
    let stream  = lexer::tokenize(src);
    let mut p   = parser::Parser::new(stream);
    let program = p.parse_program()?;

    let mut state   = turtle::TurtleState::new();
    let mut symbols = interpreter::SymbolTable::new();
    let mut procs   = interpreter::ProcTable::new();

    interpreter::execute(&program, &mut state, &mut symbols, &mut procs)?;
    Ok(state)
}

// ── Geometric correctness ─────────────────────────────────────────────────────

#[test]
fn square_produces_four_lines_and_closes() {
    let t = run("dotimes 4 { forward 100 turn 90 }").unwrap();
    assert_eq!(t.lines.len(), 4, "a square has 4 sides");
    assert!(t.x.abs() < 1e-9, "turtle should return to x=0, got {}", t.x);
    assert!(t.y.abs() < 1e-9, "turtle should return to y=0, got {}", t.y);
}

#[test]
fn equilateral_triangle_closes() {
    let t = run("dotimes 3 { forward 100 turn 120 }").unwrap();
    assert!(t.x.abs() < 1e-9);
    assert!(t.y.abs() < 1e-9);
    assert_eq!(t.lines.len(), 3);
}

#[test]
fn five_pointed_star_produces_five_lines() {
    let t = run("dotimes 5 { forward 200 turn 144 }").unwrap();
    assert_eq!(t.lines.len(), 5);
}

#[test]
fn forward_only_moves_north() {
    let t = run("forward 150").unwrap();
    assert!(t.x.abs() < 1e-9);
    assert!((t.y + 150.0).abs() < 1e-9);
}

// ── Pen control ───────────────────────────────────────────────────────────────

#[test]
fn pen_up_produces_no_lines() {
    let t = run("pen 0\nforward 200\nforward 200").unwrap();
    assert!(t.lines.is_empty());
}

#[test]
fn pen_down_after_pen_up_resumes_drawing() {
    let src = "pen 0\nforward 100\npen 1\nforward 100";
    let t = run(src).unwrap();
    assert_eq!(t.lines.len(), 1, "only the second segment should be drawn");
}

#[test]
fn alternating_pen_produces_correct_line_count() {
    // 3 pen-down segments, 2 pen-up moves.
    let src = r#"
        forward 10
        pen 0  forward 10
        pen 1  forward 10
        pen 0  forward 10
        pen 1  forward 10
    "#;
    let t = run(src).unwrap();
    assert_eq!(t.lines.len(), 3);
}

// ── Variables ─────────────────────────────────────────────────────────────────

#[test]
fn variable_controls_forward_distance() {
    let t = run("set n 60\nforward n").unwrap();
    assert!((t.y + 60.0).abs() < 1e-9);
}

#[test]
fn variable_incremented_in_loop() {
    // Each of 10 iterations adds 5 north; total = 50 north.
    let src = "set d 5\ndotimes 10 { forward d }";
    let t = run(src).unwrap();
    assert!((t.y + 50.0).abs() < 1e-9);
}

#[test]
fn arithmetic_grows_side_in_spiral() {
    // Expanding spiral: each forward is 2 units longer than the last.
    let src = "set n 2\ndotimes 20 { forward n turn 90 set n (n + 2) }";
    let t = run(src).unwrap();
    assert_eq!(t.lines.len(), 20);
    // Last line should be longer than the first.
    let first_len = {
        let l = &t.lines[0];
        ((l.x2 - l.x1).powi(2) + (l.y2 - l.y1).powi(2)).sqrt()
    };
    let last_len = {
        let l = &t.lines[19];
        ((l.x2 - l.x1).powi(2) + (l.y2 - l.y1).powi(2)).sqrt()
    };
    assert!(last_len > first_len, "spiral should grow: first={} last={}", first_len, last_len);
}

// ── Procedures ────────────────────────────────────────────────────────────────

#[test]
fn procedure_definition_and_single_call() {
    let src = "to side { forward 80 }\nside";
    let t = run(src).unwrap();
    assert!((t.y + 80.0).abs() < 1e-9);
    assert_eq!(t.lines.len(), 1);
}

#[test]
fn procedure_called_in_loop() {
    // A square drawn via a procedure.
    let src = "to side { forward 60 turn 90 }\ndotimes 4 { side }";
    let t = run(src).unwrap();
    assert_eq!(t.lines.len(), 4);
    assert!(t.x.abs() < 1e-9);
    assert!(t.y.abs() < 1e-9);
}

#[test]
fn two_procedures_cooperate() {
    let src = r#"
        to small { forward 20 }
        to big   { small small small }
        big
    "#;
    let t = run(src).unwrap();
    assert!((t.y + 60.0).abs() < 1e-9);
    assert_eq!(t.lines.len(), 3);
}

#[test]
fn procedure_shares_variables_with_caller() {
    // Procedure reads 'size' set by caller; caller reads 'count' set by procedure.
    let src = r#"
        set size 30
        set count 0
        to one_step {
            forward size
            set count (count + 1)
        }
        one_step
        one_step
    "#;
    let t = run(src).unwrap();
    assert!((t.y + 60.0).abs() < 1e-9);
    // count should be 2; verify via line count since we can't read sym here.
    assert_eq!(t.lines.len(), 2);
}

// ── Color ─────────────────────────────────────────────────────────────────────

#[test]
fn color_command_sets_line_color() {
    let t = run("color 200 50 100\nforward 1").unwrap();
    assert_eq!(t.lines[0].color, [200, 50, 100, 255]);
}

#[test]
fn color_changes_mid_program() {
    let src = "forward 1\ncolor 255 0 0\nforward 1";
    let t = run(src).unwrap();
    assert_ne!(t.lines[0].color, t.lines[1].color,
        "color should have changed between the two segments");
    assert_eq!(t.lines[1].color, [255, 0, 0, 255]);
}

#[test]
fn color_clamped_in_source() {
    // Values outside 0–255 are clamped, not errors.
    let t = run("color -10 300 128\nforward 1").unwrap();
    assert_eq!(t.lines[0].color, [0, 255, 128, 255]);
}

// ── Error handling ────────────────────────────────────────────────────────────

#[test]
fn error_undefined_variable_names_it() {
    let err = run("forward undefined_var").unwrap_err();
    assert!(err.contains("'undefined_var'"), "{}", err);
}

#[test]
fn error_unknown_procedure_names_it() {
    let err = run("no_such_proc").unwrap_err();
    assert!(err.contains("'no_such_proc'"), "{}", err);
}

#[test]
fn error_division_by_zero() {
    let err = run("forward (10 / 0)").unwrap_err();
    assert!(err.to_lowercase().contains("zero"), "{}", err);
}

#[test]
fn error_parse_includes_line_number() {
    // Syntax error on line 3.
    let src = "forward 10\nturn 90\nforward"; // missing argument on line 3
    let err = run(src).unwrap_err();
    assert!(err.contains("Line 3"), "expected line 3 in error: {}", err);
}

// ── Full programs from the examples directory ─────────────────────────────────

#[test]
fn example_star_runs_without_error() {
    let src = std::fs::read_to_string("examples/star.tg").expect("examples/star.tg not found");
    let t = run(&src).unwrap();
    assert_eq!(t.lines.len(), 5, "a 5-pointed star has 5 sides");
}

#[test]
fn example_square_runs_without_error() {
    let src = std::fs::read_to_string("examples/square.tg").expect("examples/square.tg not found");
    let t = run(&src).unwrap();
    // square.tg: dotimes 4 { forward 150 turn 90 } — 4 lines, closed path
    assert_eq!(t.lines.len(), 4);
    assert!(t.x.abs() < 1e-9);
    assert!(t.y.abs() < 1e-9);
}

#[test]
fn example_spiral_runs_and_grows() {
    let src = std::fs::read_to_string("examples/spiral.tg").expect("examples/spiral.tg not found");
    let t = run(&src).unwrap();
    assert!(!t.lines.is_empty(), "spiral should produce lines");
    // The spiral grows: last segment should be longer than first.
    if t.lines.len() >= 2 {
        let len = |l: &turtle::Line| {
            ((l.x2 - l.x1).powi(2) + (l.y2 - l.y1).powi(2)).sqrt()
        };
        assert!(len(t.lines.last().unwrap()) > len(&t.lines[0]),
            "spiral should grow");
    }
}

#[test]
fn example_mandala_runs_without_error() {
    let src = std::fs::read_to_string("examples/mandala.tg").expect("examples/mandala.tg not found");
    assert!(run(&src).is_ok());
}

#[test]
fn example_rainbow_runs_without_error() {
    let src = std::fs::read_to_string("examples/rainbow.tg").expect("examples/rainbow.tg not found");
    let t = run(&src).unwrap();
    assert!(!t.lines.is_empty());
}

#[test]
fn example_galaxy_runs_without_error() {
    let src = std::fs::read_to_string("examples/galaxy.tg").expect("examples/galaxy.tg not found");
    assert!(run(&src).is_ok());
}
