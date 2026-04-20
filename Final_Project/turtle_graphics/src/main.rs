//! Turtle Graphics Interpreter — entry point.
//!
//! ## Pipeline
//! ```text
//! Source → lexer → parser → interpreter → Vec<Line> → animated renderer
//! ```
//!
//! ## Usage
//! ```
//! cargo run -- examples/rainbow.tg
//! cargo run                          # built-in demo
//! ```
//!
//! ## Keyboard Controls
//! | Key        | Action                                  |
//! |------------|-----------------------------------------|
//! | Space      | Pause / Resume                          |
//! | R          | Restart animation from the beginning    |
//! | F / Enter  | Finish — jump to the fully drawn state  |
//! | ↑ / +      | Double the drawing speed                |
//! | ↓ / -      | Halve the drawing speed (min 1)         |
//! | Q / Escape | Quit                                    |

use lib::{interpreter, lexer, parser, renderer, turtle};

use std::time::{Duration, Instant};

use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

// ── Constants ─────────────────────────────────────────────────────────────────

const WIDTH:  u32 = 900;
const HEIGHT: u32 = 700;

/// Dark background — enough contrast for both colored and default-green lines.
const BG_COLOR: [u8; 4] = [0x12, 0x12, 0x16, 0xff];

/// Target frame duration (~60 fps).
const FRAME_DURATION: Duration = Duration::from_millis(16);

// ── Built-in demo ─────────────────────────────────────────────────────────────

const DEMO_PROGRAM: &str = r#"
# Built-in demo: a color-cycling rose pattern built from a procedure.
# The 'petal' procedure draws one teardrop arc;
# 36 petals arranged at 10-degree intervals form the rose.
# Color cycles through the rainbow as petals are drawn.

to petal {
    set r 6
    dotimes 20 {
        forward r
        turn 9
        set r (r + 2.5)
    }
    turn 180
    set r 6
    dotimes 20 {
        forward r
        turn 9
        set r (r + 2.5)
    }
    turn 180
}

pen 1
set hue 0
dotimes 36 {
    # Cycle through R→G→B using three overlapping triangle waves.
    # hue runs 0→360 over the 36 petals (10 degrees each).
    set r (255)
    set g (0)
    set b (128)
    color r g b
    petal
    turn 10
    set hue (hue + 10)
}
"#;

// ── Animation state ───────────────────────────────────────────────────────────

struct Anim {
    drawn:     usize,   // segments revealed so far
    total:     usize,   // total segments
    paused:    bool,
    speed:     usize,   // segments to reveal per frame tick
    last_tick: Instant,
}

impl Anim {
    fn new(total: usize) -> Self {
        Anim {
            drawn: 0,
            total,
            paused: false,
            speed: 3,
            last_tick: Instant::now(),
        }
    }

    /// Advance the animation if enough time has passed.
    /// Returns `true` if the frame needs to be redrawn.
    fn tick(&mut self) -> bool {
        if self.paused || self.drawn >= self.total {
            return false;
        }
        let now = Instant::now();
        if now.duration_since(self.last_tick) >= FRAME_DURATION {
            self.last_tick = now;
            self.drawn = (self.drawn + self.speed).min(self.total);
            true
        } else {
            false
        }
    }

    fn speed_up(&mut self) {
        self.speed = (self.speed * 2).min(2000);
        println!("Speed: {} lines/frame", self.speed);
    }

    fn speed_down(&mut self) {
        self.speed = (self.speed / 2).max(1);
        println!("Speed: {} lines/frame", self.speed);
    }

    fn restart(&mut self) {
        self.drawn = 0;
        self.paused = false;
    }

    fn finish(&mut self) {
        self.drawn = self.total;
    }

    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        println!("{}", if self.paused { "Paused." } else { "Resumed." });
    }
}

// ── Pipeline ──────────────────────────────────────────────────────────────────

fn run_program(source: &str) -> Result<Vec<turtle::Line>, String> {
    let stream  = lexer::tokenize(source);
    let mut p   = parser::Parser::new(stream);
    let program = p.parse_program()?;

    let mut state   = turtle::TurtleState::new();
    let mut symbols = interpreter::SymbolTable::new();
    let mut procs   = interpreter::ProcTable::new();

    interpreter::execute(&program, &mut state, &mut symbols, &mut procs)?;
    Ok(state.lines)
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    // ── 1. Load and execute ───────────────────────────────────────────────────
    let (source, title) = match std::env::args().nth(1) {
        Some(path) => {
            let src = std::fs::read_to_string(&path).unwrap_or_else(|e| {
                eprintln!("Error reading '{}': {}", path, e);
                std::process::exit(1);
            });
            let t = format!("Turtle Graphics — {}", path);
            (src, t)
        }
        None => {
            println!("No file given — running built-in demo.");
            println!("Usage: cargo run -- <file.tg>");
            (DEMO_PROGRAM.to_string(), "Turtle Graphics — Demo".to_string())
        }
    };

    let lines = match run_program(&source) {
        Ok(l) => l,
        Err(e) => { eprintln!("Error: {}", e); std::process::exit(1); }
    };

    let total = lines.len();
    println!(
        "{} line segments total.  Space=pause  R=restart  F=finish  \
         ↑/↓=speed  Q=quit",
        total
    );

    // ── 2. Compute viewport (auto-scale + center) ─────────────────────────────
    let (scale, offset_x, offset_y) = renderer::fit_viewport(&lines, WIDTH, HEIGHT);

    // ── 3. Open window ────────────────────────────────────────────────────────
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title(&title)
        .with_inner_size(LogicalSize::new(WIDTH, HEIGHT))
        .with_resizable(false)
        .build(&event_loop)
        .expect("Failed to create window");

    let size    = window.inner_size();
    let surface = SurfaceTexture::new(size.width, size.height, &window);
    let mut pixels = Pixels::new(WIDTH, HEIGHT, surface)
        .expect("Failed to create pixel buffer");

    // ── 4. Animation state ────────────────────────────────────────────────────
    let mut anim = Anim::new(total);

    // ── 5. Event loop ─────────────────────────────────────────────────────────
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::RedrawRequested(_) => {
                let frame = pixels.frame_mut();
                renderer::clear(frame, BG_COLOR);
                for line in &lines[..anim.drawn] {
                    renderer::draw_line(frame, WIDTH, HEIGHT, line, scale, offset_x, offset_y);
                }
                if let Err(e) = pixels.render() {
                    eprintln!("Render error: {e}");
                    *control_flow = ControlFlow::Exit;
                }
            }

            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,

                WindowEvent::KeyboardInput {
                    input: KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(key),
                        ..
                    },
                    ..
                } => match key {
                    VirtualKeyCode::Q | VirtualKeyCode::Escape => {
                        *control_flow = ControlFlow::Exit;
                    }
                    VirtualKeyCode::Space => anim.toggle_pause(),
                    VirtualKeyCode::R => {
                        anim.restart();
                        window.request_redraw();
                    }
                    VirtualKeyCode::F | VirtualKeyCode::Return => {
                        anim.finish();
                        window.request_redraw();
                    }
                    VirtualKeyCode::Up   | VirtualKeyCode::Equals => anim.speed_up(),
                    VirtualKeyCode::Down | VirtualKeyCode::Minus   => anim.speed_down(),
                    _ => {}
                },

                _ => {}
            },

            Event::MainEventsCleared => {
                if anim.tick() {
                    window.request_redraw();
                }
            }

            _ => {}
        }
    });
}
