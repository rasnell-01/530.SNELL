//! Turtle state, geometry, and recorded line segments.
//!
//! Coordinate system:
//!   - Origin `(0, 0)` at the screen center.
//!   - X increases rightward.
//!   - Y increases upward (opposite to screen pixel Y).
//!   - Heading `0°` = north.  Positive angle = clockwise rotation.
//!
//! Each `Line` carries its own RGBA color so that programs using the `color`
//! command produce multi-colored output.  The renderer uses the stored color
//! directly, making no assumptions about a global palette.

/// A line segment drawn by the turtle, stored in logical coordinates.
#[derive(Debug, Clone)]
pub struct Line {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    /// RGBA color of this segment.
    pub color: [u8; 4],
}

/// Full turtle state at any point during execution.
#[derive(Debug)]
pub struct TurtleState {
    pub x: f64,
    pub y: f64,
    /// Heading in degrees.  0 = north, 90 = east, 180 = south, 270 = west.
    pub angle: f64,
    pub pen_down: bool,
    /// Color used for the next line drawn.  Default: bright green.
    pub color: [u8; 4],
    /// All line segments recorded so far.
    pub lines: Vec<Line>,
}

impl TurtleState {
    /// Create a turtle at the origin, heading north, pen down, color green.
    pub fn new() -> Self {
        TurtleState {
            x: 0.0,
            y: 0.0,
            angle: 0.0,
            pen_down: true,
            color: [0x00, 0xff, 0x88, 0xff], // default: turtle green
            lines: Vec::new(),
        }
    }

    /// Move forward by `units` in the current heading direction.
    /// Records a `Line` (using `self.color`) if the pen is down.
    pub fn forward(&mut self, units: f64) {
        let rad = self.angle.to_radians();
        let new_x = self.x + units * rad.sin();
        // Subtract from Y: turtle positive-Y is up, screen pixel-Y is down.
        let new_y = self.y - units * rad.cos();

        if self.pen_down {
            self.lines.push(Line {
                x1: self.x,
                y1: self.y,
                x2: new_x,
                y2: new_y,
                color: self.color,
            });
        }

        self.x = new_x;
        self.y = new_y;
    }

    /// Rotate by `degrees`.  Positive = clockwise, negative = counter-clockwise.
    pub fn turn(&mut self, degrees: f64) {
        self.angle = (self.angle + degrees).rem_euclid(360.0);
    }

    /// Lower (`true`) or lift (`false`) the pen.
    pub fn set_pen(&mut self, down: bool) {
        self.pen_down = down;
    }

    /// Change the current drawing color.
    /// R, G, B are clamped to `[0, 255]`.  Alpha is always 255 (fully opaque).
    pub fn set_color(&mut self, r: f64, g: f64, b: f64) {
        let clamp = |v: f64| v.clamp(0.0, 255.0) as u8;
        self.color = [clamp(r), clamp(g), clamp(b), 0xff];
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::TurtleState;

    const EPS: f64 = 1e-9;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    // ── Initial state ─────────────────────────────────────────────────────────

    #[test]
    fn new_starts_at_origin() {
        let t = TurtleState::new();
        assert!(approx(t.x, 0.0));
        assert!(approx(t.y, 0.0));
    }

    #[test]
    fn new_faces_north() {
        let t = TurtleState::new();
        assert!(approx(t.angle, 0.0));
    }

    #[test]
    fn new_pen_is_down() {
        assert!(TurtleState::new().pen_down);
    }

    #[test]
    fn new_has_no_lines() {
        assert!(TurtleState::new().lines.is_empty());
    }

    // ── forward — cardinal directions ─────────────────────────────────────────

    #[test]
    fn forward_north_decreases_y() {
        // angle=0 (north): x unchanged, y decreases (screen-Y convention)
        let mut t = TurtleState::new();
        t.forward(100.0);
        assert!(approx(t.x, 0.0));
        assert!(approx(t.y, -100.0));
    }

    #[test]
    fn forward_east_increases_x() {
        let mut t = TurtleState::new();
        t.turn(90.0); // face east
        t.forward(100.0);
        assert!(approx(t.x, 100.0));
        assert!(approx(t.y, 0.0));
    }

    #[test]
    fn forward_south_increases_y() {
        let mut t = TurtleState::new();
        t.turn(180.0);
        t.forward(100.0);
        assert!(approx(t.x, 0.0));
        assert!(approx(t.y, 100.0));
    }

    #[test]
    fn forward_west_decreases_x() {
        let mut t = TurtleState::new();
        t.turn(270.0);
        t.forward(100.0);
        assert!(approx(t.x, -100.0));
        assert!(approx(t.y, 0.0));
    }

    #[test]
    fn forward_negative_moves_backward() {
        let mut t = TurtleState::new();
        t.forward(-50.0); // backward (south when facing north)
        assert!(approx(t.y, 50.0));
    }

    // ── forward — pen and line recording ─────────────────────────────────────

    #[test]
    fn forward_pen_down_records_one_line() {
        let mut t = TurtleState::new();
        t.forward(100.0);
        assert_eq!(t.lines.len(), 1);
    }

    #[test]
    fn forward_pen_up_records_no_line() {
        let mut t = TurtleState::new();
        t.set_pen(false);
        t.forward(100.0);
        assert!(t.lines.is_empty());
    }

    #[test]
    fn line_endpoints_match_path() {
        let mut t = TurtleState::new();
        t.forward(100.0);
        let l = &t.lines[0];
        assert!(approx(l.x1, 0.0) && approx(l.y1, 0.0));
        assert!(approx(l.x2, 0.0) && approx(l.y2, -100.0));
    }

    #[test]
    fn line_inherits_turtle_color_at_draw_time() {
        let mut t = TurtleState::new();
        t.set_color(200.0, 50.0, 100.0);
        t.forward(1.0);
        assert_eq!(t.lines[0].color, [200, 50, 100, 255]);
    }

    #[test]
    fn changing_color_mid_path_affects_only_new_lines() {
        let mut t = TurtleState::new();
        t.forward(10.0);
        t.set_color(255.0, 0.0, 0.0);
        t.forward(10.0);
        // First line keeps original color, second gets red.
        assert_eq!(t.lines[0].color[0], 0);   // original: not red
        assert_eq!(t.lines[1].color, [255, 0, 0, 255]);
    }

    // ── turn ──────────────────────────────────────────────────────────────────

    #[test]
    fn turn_right_increases_angle() {
        let mut t = TurtleState::new();
        t.turn(90.0);
        assert!(approx(t.angle, 90.0));
    }

    #[test]
    fn turn_left_negative_wraps_via_rem_euclid() {
        let mut t = TurtleState::new();
        t.turn(-90.0);
        assert!(approx(t.angle, 270.0));
    }

    #[test]
    fn turn_full_circle_returns_to_zero() {
        let mut t = TurtleState::new();
        t.turn(360.0);
        assert!(approx(t.angle, 0.0));
    }

    #[test]
    fn turn_accumulates_correctly() {
        let mut t = TurtleState::new();
        t.turn(90.0);
        t.turn(90.0);
        t.turn(90.0);
        assert!(approx(t.angle, 270.0));
    }

    #[test]
    fn turn_by_zero_unchanged() {
        let mut t = TurtleState::new();
        t.turn(0.0);
        assert!(approx(t.angle, 0.0));
    }

    // ── set_color ─────────────────────────────────────────────────────────────

    #[test]
    fn set_color_normal_values() {
        let mut t = TurtleState::new();
        t.set_color(255.0, 128.0, 0.0);
        assert_eq!(t.color, [255, 128, 0, 255]);
    }

    #[test]
    fn set_color_clamps_above_255() {
        let mut t = TurtleState::new();
        t.set_color(300.0, 0.0, 0.0);
        assert_eq!(t.color[0], 255);
    }

    #[test]
    fn set_color_clamps_below_zero() {
        let mut t = TurtleState::new();
        t.set_color(-10.0, 0.0, 0.0);
        assert_eq!(t.color[0], 0);
    }

    #[test]
    fn set_color_alpha_is_always_opaque() {
        let mut t = TurtleState::new();
        t.set_color(0.0, 0.0, 0.0);
        assert_eq!(t.color[3], 255);
    }

    // ── Geometric closure ─────────────────────────────────────────────────────

    #[test]
    fn square_returns_to_origin() {
        // 4 × (forward 100 + turn 90) must close the path.
        let mut t = TurtleState::new();
        for _ in 0..4 {
            t.forward(100.0);
            t.turn(90.0);
        }
        assert!((t.x).abs() < 1e-9, "x should be ≈0, got {}", t.x);
        assert!((t.y).abs() < 1e-9, "y should be ≈0, got {}", t.y);
        assert_eq!(t.lines.len(), 4);
    }

    #[test]
    fn equilateral_triangle_returns_to_origin() {
        let mut t = TurtleState::new();
        for _ in 0..3 {
            t.forward(100.0);
            t.turn(120.0);
        }
        assert!((t.x).abs() < 1e-9);
        assert!((t.y).abs() < 1e-9);
    }
}
