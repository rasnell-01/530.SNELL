//! Software pixel-buffer rendering.
//!
//! All drawing is done on the CPU into the raw RGBA framebuffer managed by
//! the `pixels` crate.  Lines are rendered with Bresenham's algorithm.
//!
//! Each `Line` carries its own color, so `draw_line` reads from the segment
//! rather than accepting a global color parameter.  The caller passes a `scale`
//! factor (computed by `fit_viewport`) so every program fills the window.

use crate::turtle::Line;

// ── Viewport fitting ──────────────────────────────────────────────────────────

/// Compute the `(scale, offset_x, offset_y)` that fits all `lines` into a
/// window of `(width, height)` with an 88 % fill factor (6 % margin on each
/// side).
///
/// The returned transform maps logical turtle coordinates to pixel coordinates:
/// ```text
/// pixel_x = x * scale + offset_x
/// pixel_y = y * scale + offset_y
/// ```
pub fn fit_viewport(lines: &[Line], width: u32, height: u32) -> (f64, f64, f64) {
    if lines.is_empty() {
        return (1.0, width as f64 / 2.0, height as f64 / 2.0);
    }

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for l in lines {
        min_x = min_x.min(l.x1).min(l.x2);
        max_x = max_x.max(l.x1).max(l.x2);
        min_y = min_y.min(l.y1).min(l.y2);
        max_y = max_y.max(l.y1).max(l.y2);
    }

    let range_x = (max_x - min_x).max(1.0);
    let range_y = (max_y - min_y).max(1.0);

    const FILL: f64 = 0.88; // use 88 % of the window on each axis
    let scale = ((width  as f64 * FILL) / range_x)
               .min((height as f64 * FILL) / range_y);

    let center_x = (min_x + max_x) / 2.0;
    let center_y = (min_y + max_y) / 2.0;

    let offset_x = width  as f64 / 2.0 - scale * center_x;
    let offset_y = height as f64 / 2.0 - scale * center_y;

    (scale, offset_x, offset_y)
}

// ── Public drawing API ────────────────────────────────────────────────────────

/// Fill every pixel in `frame` with `color`.
pub fn clear(frame: &mut [u8], color: [u8; 4]) {
    for pixel in frame.chunks_exact_mut(4) {
        pixel.copy_from_slice(&color);
    }
}

/// Draw one `Line` into `frame`, scaling from logical to pixel coordinates.
/// The line's stored `color` field is used for the pixel color.
pub fn draw_line(
    frame:    &mut [u8],
    width:    u32,
    height:   u32,
    line:     &Line,
    scale:    f64,
    offset_x: f64,
    offset_y: f64,
) {
    let x1 = (line.x1 * scale + offset_x).round() as i32;
    let y1 = (line.y1 * scale + offset_y).round() as i32;
    let x2 = (line.x2 * scale + offset_x).round() as i32;
    let y2 = (line.y2 * scale + offset_y).round() as i32;
    bresenham(frame, width, height, x1, y1, x2, y2, line.color);
}

// ── Internals ─────────────────────────────────────────────────────────────────

fn bresenham(
    frame: &mut [u8],
    width: u32,
    height: u32,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: [u8; 4],
) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1i32 } else { -1i32 };
    let sy = if y0 < y1 { 1i32 } else { -1i32 };
    let mut err = dx - dy;

    loop {
        set_pixel(frame, width, height, x0, y0, color);
        if x0 == x1 && y0 == y1 { break; }
        let e2 = 2 * err;
        if e2 > -dy { err -= dy; x0 += sx; }
        if e2 <  dx { err += dx; y0 += sy; }
    }
}

#[inline]
fn set_pixel(frame: &mut [u8], width: u32, height: u32, x: i32, y: i32, color: [u8; 4]) {
    if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
        let base = ((y as u32 * width + x as u32) * 4) as usize;
        frame[base..base + 4].copy_from_slice(&color);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{clear, draw_line, fit_viewport};
    use crate::turtle::Line;

    fn make_line(x1: f64, y1: f64, x2: f64, y2: f64) -> Line {
        Line { x1, y1, x2, y2, color: [255, 255, 255, 255] }
    }

    fn pixel_at(frame: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
        let base = ((y * w + x) * 4) as usize;
        [frame[base], frame[base+1], frame[base+2], frame[base+3]]
    }

    // ── clear ─────────────────────────────────────────────────────────────────

    #[test]
    fn clear_fills_entire_frame() {
        let mut frame = vec![0u8; 4 * 4]; // 1×1 pixel
        clear(&mut frame, [10, 20, 30, 255]);
        assert_eq!(&frame, &[10, 20, 30, 255]);
    }

    #[test]
    fn clear_multi_pixel_frame() {
        let mut frame = vec![0u8; 3 * 4]; // 3 pixels
        clear(&mut frame, [255, 0, 128, 255]);
        for chunk in frame.chunks_exact(4) {
            assert_eq!(chunk, &[255, 0, 128, 255]);
        }
    }

    #[test]
    fn clear_with_black() {
        let mut frame = vec![99u8; 8]; // 2 pixels, pre-filled with noise
        clear(&mut frame, [0, 0, 0, 255]);
        assert!(frame.chunks_exact(4).all(|p| p == [0, 0, 0, 255]));
    }

    // ── fit_viewport ──────────────────────────────────────────────────────────

    #[test]
    fn fit_viewport_empty_returns_identity_centered() {
        let (scale, ox, oy) = fit_viewport(&[], 900, 700);
        assert_eq!(scale, 1.0);
        assert!((ox - 450.0).abs() < 1e-9, "ox={}", ox);
        assert!((oy - 350.0).abs() < 1e-9, "oy={}", oy);
    }

    #[test]
    fn fit_viewport_symmetric_drawing_stays_centered() {
        // A line from (-100,0) to (100,0): logical center at x=0, y=0.
        // After fitting, pixel center should still be at (450, 350).
        let lines = vec![make_line(-100.0, 0.0, 100.0, 0.0)];
        let (scale, ox, oy) = fit_viewport(&lines, 900, 700);
        let pixel_center_x = 0.0_f64 * scale + ox;
        let pixel_center_y = 0.0_f64 * scale + oy;
        assert!((pixel_center_x - 450.0).abs() < 1e-6, "center_x={}", pixel_center_x);
        assert!((pixel_center_y - 350.0).abs() < 1e-6, "center_y={}", pixel_center_y);
    }

    #[test]
    fn fit_viewport_fills_88_percent_of_constraining_axis() {
        // Square drawing: range_x = range_y = 200. Scale limited by height (700).
        let lines = vec![
            make_line(-100.0, -100.0, 100.0, -100.0),
            make_line(100.0, -100.0, 100.0,  100.0),
            make_line(100.0,  100.0, -100.0, 100.0),
            make_line(-100.0, 100.0, -100.0, -100.0),
        ];
        let (scale, _ox, _oy) = fit_viewport(&lines, 900, 700);
        // Expected scale: min(900*0.88/200, 700*0.88/200) = min(3.96, 3.08) = 3.08
        let expected = (900.0_f64 * 0.88 / 200.0).min(700.0 * 0.88 / 200.0);
        assert!((scale - expected).abs() < 1e-6, "scale={} expected≈{}", scale, expected);
    }

    #[test]
    fn fit_viewport_wide_drawing_scales_by_width() {
        // Very wide, not tall: width constrains.
        let lines = vec![make_line(-500.0, 0.0, 500.0, 0.0)];
        let (scale, _ox, _oy) = fit_viewport(&lines, 900, 700);
        let expected_w = 900.0 * 0.88 / 1000.0;  // width-constrained
        let expected_h = 700.0 * 0.88 / 1.0;     // height constraint with clamped range=1
        let expected = expected_w.min(expected_h);
        assert!((scale - expected).abs() < 1e-6);
    }

    #[test]
    fn fit_viewport_off_center_drawing_recenters() {
        // A line entirely in the positive quadrant: [100,200] × [0,0].
        // Logical center: (150, 0). Pixel center should be at (450, 350).
        let lines = vec![make_line(100.0, 0.0, 200.0, 0.0)];
        let (scale, ox, oy) = fit_viewport(&lines, 900, 700);
        let pixel_center_x = 150.0 * scale + ox;
        let pixel_center_y = 0.0 * scale + oy;
        assert!((pixel_center_x - 450.0).abs() < 1e-6, "center_x={}", pixel_center_x);
        assert!((pixel_center_y - 350.0).abs() < 1e-6, "center_y={}", pixel_center_y);
    }

    // ── draw_line ─────────────────────────────────────────────────────────────

    #[test]
    fn draw_horizontal_line_sets_midpoint_pixel() {
        let (w, h) = (20u32, 20u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        // Line from (-5, 0) to (5, 0); scale=1, offset=(10, 10) → pixels y=10, x=5..15
        let line = make_line(-5.0, 0.0, 5.0, 0.0);
        draw_line(&mut frame, w, h, &line, 1.0, 10.0, 10.0);
        // The midpoint pixel (10, 10) must be set.
        assert_eq!(pixel_at(&frame, w, 10, 10), [255, 255, 255, 255]);
    }

    #[test]
    fn draw_line_uses_line_color() {
        let (w, h) = (10u32, 10u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        let mut line = make_line(0.0, 0.0, 0.0, 0.0); // single-pixel line at origin
        line.color = [200, 100, 50, 255];
        draw_line(&mut frame, w, h, &line, 1.0, 5.0, 5.0);
        assert_eq!(pixel_at(&frame, w, 5, 5), [200, 100, 50, 255]);
    }

    #[test]
    fn draw_line_out_of_bounds_does_not_panic() {
        // Line entirely outside the framebuffer — must not panic.
        let mut frame = vec![0u8; 10 * 10 * 4];
        let line = make_line(1000.0, 1000.0, 2000.0, 2000.0);
        draw_line(&mut frame, 10, 10, &line, 1.0, 0.0, 0.0);
        // Frame should remain all-zero.
        assert!(frame.iter().all(|&b| b == 0));
    }

    #[test]
    fn draw_diagonal_line_sets_endpoints() {
        let (w, h) = (20u32, 20u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        // 45° diagonal from (0,0) to (9,9) in pixel space.
        // In logical coords with scale=1, offset=0: line from (0,0) to (9,9).
        let line = make_line(0.0, 0.0, 9.0, 9.0);
        draw_line(&mut frame, w, h, &line, 1.0, 0.0, 0.0);
        // Both endpoints must be set.
        assert_eq!(pixel_at(&frame, w, 0, 0), [255, 255, 255, 255]);
        assert_eq!(pixel_at(&frame, w, 9, 9), [255, 255, 255, 255]);
    }
}
