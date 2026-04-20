# Turtle Graphics Interpreter in Rust

A complete turtle graphics interpreter built in Rust, using `winit` for windowing
and `pixels` for a software-rendered framebuffer.  Implements the **Composite**
and **Interpreter** design patterns.

---

## Building and Running

```bash
cargo build --release

cargo run -- examples/rainbow.tg     # color spiral
cargo run -- examples/sunset.tg      # warm concentric arcs
cargo run -- examples/mandala.tg     # mandala from a procedure
cargo run -- examples/galaxy.tg      # spiral arms
cargo run -- examples/snowflake.tg   # geometric snowflake
cargo run                            # built-in demo
```

### Keyboard Controls

| Key | Action |
|---|---|
| **Space** | Pause / Resume animation |
| **R** | Restart from the beginning |
| **F** or **Enter** | Finish — jump to fully drawn |
| **↑** or **=** | Double the drawing speed |
| **↓** or **-** | Halve the drawing speed |
| **Q** or **Escape** | Quit |

---

## The Language

### Drawing Commands

| Syntax | Description |
|---|---|
| `forward <expr>` | Move forward the given distance |
| `turn <expr>` | Rotate clockwise (positive) or counter-clockwise (negative) |
| `pen 1` | Lower the pen — movement draws |
| `pen 0` | Lift the pen — movement does not draw |
| `color <r> <g> <b>` | Set the drawing color (0–255 per channel) |

### Variables and Control Flow

| Syntax | Description |
|---|---|
| `set name <expr>` | Assign a value to a variable |
| `dotimes <expr> { ... }` | Repeat the block that many times |
| `print <expr>` | Print a value to stdout (debugging) |

### Procedures

```mermaid 
to square {
    dotimes 4 {
        forward 100
        turn 90
    }
}

square     # call it anywhere by name
```

`to <name> { <body> }` defines a procedure.  Bare identifiers in command position
are procedure calls.  Procedures can call other procedures.

### Expressions

| Form | Example |
|---|---|
| Literal | `100`, `-45.5`, `3.14` |
| Variable | `n`, `step` |
| Arithmetic | `(n + 5)`, `(n * 2)`, `(n - 1)`, `(n / 4)` |

Arithmetic must be parenthesized.  Comments start with `#`.

---

## Design Patterns

### Composite Pattern — `src/ast.rs`

The AST is a tree of `Command` nodes.

**Composite nodes** (hold `Vec<Command>` children):

| Node | Description |
|---|---|
| `Block(Vec<Command>)` | Root of every program; sequential execution |
| `DoTimes(Expr, Vec<Command>)` | Counted loop |
| `Procedure(String, Vec<Command>)` | Named definition |

**Leaf nodes** (no children, only expression arguments):
`Forward`, `Turn`, `Pen`, `Color`, `SetVar`, `Print`, `Call`

The interpreter processes all nodes through the same `execute()` call without
ever branching on "leaf vs. composite."  The tree structure drives recursion.

### Interpreter Pattern — `src/interpreter.rs`

`execute()` dispatches on each `Command` variant — one grammar rule per match arm:

- **Leaf nodes** evaluate their expression and act directly on the turtle.
- **`Block`** iterates children and calls `execute` on each.
- **`DoTimes`** evaluates its count, then loops the child list.
- **`Procedure`** inserts its body into `procs` — no execution yet.
- **`Call`** clones the body from `procs` (to free the borrow), then executes it.

Three mutable pieces of state thread through every call:

| State | Type | Contents |
|---|---|---|
| `turtle` | `TurtleState` | Position, heading, pen, current color, recorded `Line` segments |
| `symbols` | `HashMap<String, f64>` | Variable values |
| `procs` | `HashMap<String, Vec<Command>>` | Procedure bodies |

---

## Architecture

```mermaid
src/
  lexer.rs        Tokenizer → TokenStream (tokens + 1-based line numbers)
  ast.rs          Command / Expr enums — the grammar
  parser.rs       Recursive descent: TokenStream → Command tree
  turtle.rs       TurtleState: geometry, pen, color, Vec<Line>
  interpreter.rs  execute(): Interpreter pattern dispatcher
  renderer.rs     fit_viewport() + Bresenham line drawing
  main.rs         Anim state, keyboard handler, winit event loop

examples/
  square.tg       Simple square
  star.tg         5-pointed star
  spiral.tg       Expanding square spiral (variable)
  nested.tg       Nested dotimes loops
  pen_demo.tg     Pen up/down — disconnected shapes
  mandala.tg      Mandala using a procedure
  galaxy.tg       Spiral arms with a growing variable
  snowflake.tg    Two procedures cooperating
  lace.tg         Variable shared across procedure calls
  rainbow.tg      Color spiral — color command demo
  sunset.tg       Warm arc bands — color + procedure
```

### Data flow

```mermaid
.tg source
    │ lexer::tokenize()
TokenStream
    │ parser::Parser::parse_program()
Command tree  (Block at root)
    │ interpreter::execute(…, &mut turtle, &mut symbols, &mut procs)
Vec<Line>  (each Line has its own color)
    │ renderer::fit_viewport()  →  scale, offset_x, offset_y
    │ renderer::draw_line()  per Anim::drawn lines
Pixel framebuffer  →  window via pixels + winit
```

### Animation design

The interpreter runs to completion before the window opens, producing the full
`Vec<Line>`.  The `Anim` struct then controls how many segments are revealed:

```rust
for line in &lines[..anim.drawn] {
    renderer::draw_line(…);
}
```

`Anim::tick()` advances `drawn` by `speed` once per 16 ms frame tick, giving a
smooth ~60 fps animation at any speed setting.  Because the interpreter already
ran, the animation is just a rendering effect — no interpreter work happens in
the event loop at all.

### Auto-scaling

`renderer::fit_viewport` inspects every endpoint in `Vec<Line>`, computes the
bounding box, and returns `(scale, offset_x, offset_y)` that maps the whole
drawing into 88 % of the window with uniform margins.  Programs that draw a
tiny 10-unit square and programs that draw a 2000-unit spiral both fill the
window the same way.
