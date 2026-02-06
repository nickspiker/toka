//! Canvas backend for Relative Unit (RU) rendering
//!
//! RU (Relative Units): Resolution-independent coordinate system
//! - span = 2wh/(w+h) - harmonic mean, base unit for all measurements
//! - 1 RU from center reaches edge of smaller dimension
//! - `ru` multiplier: user-adjustable zoom (scales all GUI without layout changes)
//! - Same bytecode renders correctly at any resolution
//!
//! Coordinate system:
//! - (0, 0) = center of canvas
//! - +X = right, +Y = down
//! - All coordinates in RU space, converted to pixels internally
//!
//! All math uses Spirix ScalarF4E4 (no IEEE-754 floats).

use spirix::{CircleF4E4, ScalarF4E4};

/// Canvas with fixed pixel resolution and RU-based coordinate system
pub struct Canvas {
    /// Width in pixels (usize for array indexing)
    width: usize,

    /// Height in pixels (usize for array indexing)
    height: usize,

    /// Span: harmonic mean = 2wh/(w+h), base unit for RU system
    span: ScalarF4E4,

    /// User zoom multiplier (default 1), scales all RU measurements
    ru: ScalarF4E4,

    /// Half dimensions (width, height) for center-origin coordinate calculations
    half_dims: CircleF4E4,

    /// Pixel buffer: s44 RGBA as [R, G, B, A], row-major
    /// VSF RGB colour space - conversion to sRGB happens in WASM wrapper
    pixels: Vec<[ScalarF4E4; 4]>,
}

impl Canvas {
    /// Create a new canvas with the given pixel dimensions
    pub fn new(width: usize, height: usize) -> Self {
        // Span: harmonic mean = 2wh/(w+h)
        // Smooth at w==h, biased toward smaller dimension, finite slope at axes
        // Opaque black in s44 RGBA: [0, 0, 0, 1]
        let black = [
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ONE,
        ];

        Self {
            width,
            height,
            span: ScalarF4E4::from(width * height) / (width + height),
            ru: ScalarF4E4::ONE,
            half_dims: CircleF4E4::from((width, height)) >> 1,
            pixels: vec![black; width * height],
        }
    }

    /// Get current span (harmonic mean of width/height) as pixel count
    pub fn span(&self) -> ScalarF4E4 {
        self.span
    }

    /// Get current RU multiplier
    pub fn ru(&self) -> ScalarF4E4 {
        self.ru
    }

    /// Set RU multiplier
    pub fn set_ru(&mut self, ru: ScalarF4E4) {
        self.ru = ru.clamp(0.125, 8);
    }

    /// Adjust zoom by steps (positive = zoom in, negative = zoom out)
    /// Uses logarithmic scaling: each step multiplies by 33/32 (in) or 32/33 (out)
    pub fn adjust_zoom(&mut self, steps: ScalarF4E4) {
        // For positive steps: multiply by (33/32)^steps
        // For negative steps: multiply by (32/33)^|steps|
        // Approximation using repeated multiplication for integer steps
        let steps_i = steps.to_isize();
        let step_count = steps_i.unsigned_abs() as usize;
        let is_zoom_in = steps_i > 0;

        let mut factor = ScalarF4E4::ONE;
        let zoom_in_ratio = ScalarF4E4::from(33) / ScalarF4E4::from(32);
        let zoom_out_ratio = ScalarF4E4::from(32) / ScalarF4E4::from(33);

        for _ in 0..step_count {
            if is_zoom_in {
                factor = factor * zoom_in_ratio;
            } else {
                factor = factor * zoom_out_ratio;
            }
        }

        self.set_ru(self.ru * factor);
    }

    /// Convert RU X coordinate to pixel coordinate
    /// Origin is center of canvas, positive X = right
    fn ru_to_px_x(&self, x: ScalarF4E4) -> isize {
        // center_x + x * span * ru
        // isize allows negative coords for off-screen rendering (clipped later)
        let px = self.half_dims.r() + x * self.span * self.ru;
        px.to_isize()
    }

    /// Convert RU Y coordinate to pixel coordinate
    /// Origin is center of canvas, positive Y = down
    fn ru_to_px_y(&self, y: ScalarF4E4) -> isize {
        // center_y + y * span * ru
        let py = self.half_dims.i() + y * self.span * self.ru;
        py.to_isize()
    }

    /// Convert RU width to pixel width
    fn ru_to_px_w(&self, w: ScalarF4E4) -> isize {
        // w * span * ru
        let pw = w * self.span * self.ru;
        pw.to_isize()
    }

    /// Convert RU height to pixel height
    fn ru_to_px_h(&self, h: ScalarF4E4) -> isize {
        // h * span * ru
        let ph = h * self.span * self.ru;
        ph.to_isize()
    }

    /// Clear entire canvas to a colour (s44 RGBA in VSF RGB colour space)
    pub fn clear(&mut self, r: ScalarF4E4, g: ScalarF4E4, b: ScalarF4E4, a: ScalarF4E4) {
        self.pixels.fill([r, g, b, a]);
    }

    /// Fill a rectangle (centered pixel coordinates)
    ///
    /// - cx, cy: center of rectangle in pixels relative to canvas center
    /// - w, h: width and height in pixels
    /// - colour: RGBA as [ScalarF4E4; 4]
    pub fn fill_rect_px(
        &mut self,
        cx: isize,
        cy: isize,
        w: isize,
        h: isize,
        colour: [ScalarF4E4; 4],
    ) {
        // Canvas center
        let center_x = (self.width >> 1) as isize;
        let center_y = (self.height >> 1) as isize;

        // Convert centered coords to top-left coords
        let px = center_x + cx - w >> 1;
        let py = center_y + cy - h >> 1;

        // Clamp to canvas bounds
        let x1 = px.clamp(0, self.width as isize) as usize;
        let y1 = py.clamp(0, self.height as isize) as usize;
        let x2 = (px + w).clamp(0, self.width as isize) as usize;
        let y2 = (py + h).clamp(0, self.height as isize) as usize;

        // Fill pixels (internal TL usize indexing)
        for row in y1..y2 {
            for col in x1..x2 {
                let idx = row * self.width + col;
                self.pixels[idx] = colour;
            }
        }
    }

    /// Fill a rectangle (RU coordinates, center-origin)
    ///
    /// - pos: center of rectangle (x, y) in RU as CircleF4E4
    /// - size: dimensions (w, h) in RU as CircleF4E4
    /// - colour: RGBA as [ScalarF4E4; 4]
    /// - 1 RU = span * ru pixels
    pub fn fill_rect_ru(&mut self, pos: CircleF4E4, size: CircleF4E4, colour: [ScalarF4E4; 4]) {
        let x = pos.r();
        let y = pos.i();
        let w = size.r();
        let h = size.i();

        // Convert RU to centered pixels
        let cx = self.ru_to_px_x(x);
        let cy = self.ru_to_px_y(y);
        let pw = self.ru_to_px_w(w);
        let ph = self.ru_to_px_h(h);

        self.fill_rect_px(cx, cy, pw, ph, colour);
    }

    /// Set a single pixel (centered pixel coordinates)
    ///
    /// - x, y: pixel position relative to canvas center
    /// - colour: RGBA as [ScalarF4E4; 4]
    pub fn set_pixel_px(&mut self, x: isize, y: isize, colour: [ScalarF4E4; 4]) {
        let center_x = (self.width >> 1) as isize;
        let center_y = (self.height >> 1) as isize;

        let px = center_x + x;
        let py = center_y + y;

        if (px as usize) < self.width && (py as usize) < self.height {
            let idx = (py as usize) * self.width + (px as usize);
            self.pixels[idx] = colour;
        }
    }

    /// Set a single pixel (RU coordinates, center-origin)
    ///
    /// - pos: pixel position (x, y) in RU as CircleF4E4
    /// - colour: RGBA as [ScalarF4E4; 4]
    pub fn set_pixel_ru(&mut self, pos: CircleF4E4, colour: [ScalarF4E4; 4]) {
        let x = self.ru_to_px_x(pos.r());
        let y = self.ru_to_px_y(pos.i());
        self.set_pixel_px(x, y, colour);
    }

    /// Draw a single pixel (viewport coordinates)
    /// - pos: pixel position (x, y) in viewport coords as CircleF4E4
    /// - colour: RGBA as [ScalarF4E4; 4]
    pub fn draw_pixel(&mut self, pos: CircleF4E4, colour: [ScalarF4E4; 4]) {
        let x = pos.r();
        let y = pos.i();

        let px = (x * self.width).to_isize();
        let py = (y * self.height).to_isize();

        // Unsigned bounds check: negative values wrap to huge positive, fail automatically
        if (px as usize) < self.width && (py as usize) < self.height {
            let idx = (py as usize) * self.width + (px as usize);
            self.pixels[idx] = colour;
        }
    }

    /// Draw text (placeholder - draws coloured rectangle for text bounds)
    /// - pos: text position (x, y) as CircleF4E4
    /// - size: text size as ScalarF4E4
    /// - text: string to render
    /// - colour: RGBA as [ScalarF4E4; 4]
    pub fn draw_text(
        &mut self,
        pos: CircleF4E4,
        size: ScalarF4E4,
        text: &str,
        colour: [ScalarF4E4; 4],
    ) {
        // Placeholder: Draw a coloured rectangle representing text bounds
        // Height is based on size, width is proportional to text length
        let char_width = size * ScalarF4E4::from(6) / ScalarF4E4::from(10);
        let text_width = char_width * ScalarF4E4::from(text.len());

        let text_size = CircleF4E4::from((text_width, size));
        self.fill_rect_ru(pos, text_size, colour);
    }

    /// Fill a circle (RU coordinates, center-origin)
    /// - center: center point (x, y) in RU as CircleF4E4
    /// - radius: radius in RU as ScalarF4E4
    /// - colour: RGBA as [ScalarF4E4; 4]
    pub fn fill_circle(&mut self, center: CircleF4E4, radius: ScalarF4E4, colour: [ScalarF4E4; 4]) {
        let cx = self.ru_to_px_x(center.r());
        let cy = self.ru_to_px_y(center.i());
        let r = self.ru_to_px_w(radius);

        // Midpoint circle algorithm with flood fill
        for py in (cy - r)..=(cy + r) {
            for px in (cx - r)..=(cx + r) {
                // Check if pixel is within circle radius
                let dx = px - cx;
                let dy = py - cy;
                if dx * dx + dy * dy <= r * r {
                    // Bounds check
                    if (px as usize) < self.width && (py as usize) < self.height {
                        let idx = (py as usize) * self.width + (px as usize);
                        self.pixels[idx] = colour;
                    }
                }
            }
        }
    }

    /// Stroke a circle outline (RU coordinates, center-origin)
    /// - center: center point (x, y) in RU as CircleF4E4
    /// - radius: radius in RU as ScalarF4E4
    /// - stroke_width: line width in RU as ScalarF4E4
    /// - colour: RGBA as [ScalarF4E4; 4]
    pub fn stroke_circle(
        &mut self,
        center: CircleF4E4,
        radius: ScalarF4E4,
        stroke_width: ScalarF4E4,
        colour: [ScalarF4E4; 4],
    ) {
        let cx = self.ru_to_px_x(center.r());
        let cy = self.ru_to_px_y(center.i());
        let r_outer = self.ru_to_px_w(radius + stroke_width / ScalarF4E4::from(2));
        let r_inner = self.ru_to_px_w(radius - stroke_width >> 1).max(0);

        // Draw pixels in the annulus between inner and outer radius
        for py in (cy - r_outer)..=(cy + r_outer) {
            for px in (cx - r_outer)..=(cx + r_outer) {
                let dx = px - cx;
                let dy = py - cy;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq >= r_inner * r_inner && dist_sq <= r_outer * r_outer {
                    // Bounds check
                    if (px as usize) < self.width && (py as usize) < self.height {
                        let idx = (py as usize) * self.width + (px as usize);
                        self.pixels[idx] = colour;
                    }
                }
            }
        }
    }

    /// Draw an anti-aliased line (pixel coordinates)
    /// - start: start point (x, y) in pixels
    /// - end: end point (x, y) in pixels
    /// - colour_start: RGBA colour at line start
    /// - colour_end: RGBA colour at line end
    pub fn draw_line(
        &mut self,
        start: CircleF4E4,
        end: CircleF4E4,
        colour_start: [ScalarF4E4; 4],
        colour_end: [ScalarF4E4; 4],
    ) {
        crate::drawing::line::draw_line_s44(
            &mut self.pixels,
            self.width,
            self.height,
            start.r(),
            start.i(),
            end.r(),
            end.i(),
            colour_start,
            colour_end,
        );
    }

    /// Get canvas dimensions
    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Get pixel buffer (s44 RGBA arrays)
    pub fn pixels(&self) -> &[[ScalarF4E4; 4]] {
        &self.pixels
    }

    /// Get canvas width
    pub fn width(&self) -> usize {
        self.width
    }

    /// Get canvas height
    pub fn height(&self) -> usize {
        self.height
    }

    /// Convert canvas pixels to RGBA byte array for browser ImageData
    ///
    /// Proper VSF RGB → sRGB color pipeline (pure S44, no IEEE-754):
    /// 1. Decode VSF gamma 2 (square to linearize)
    /// 2. Transform linear VSF RGB → linear sRGB using matrix
    /// 3. Encode with sRGB OETF (piecewise gamma)
    /// 4. Quantize S44 [0-1] → u8 [0-255]
    pub fn to_rgba_bytes(&self) -> Vec<u8> {
        use spirix::ScalarF4E4;
        use vsf::colour::convert::{
            apply_matrix_3x3_s44, linearize_gamma2_s44, srgb_oetf_s44, vsf_rgb2srgb_s44,
        };

        self.pixels
            .iter()
            .flat_map(|&[r_vsf, g_vsf, b_vsf, a]| {
                // 1. Decode VSF gamma 2: encoded^2 → linear
                let r_lin_vsf = linearize_gamma2_s44(r_vsf);
                let g_lin_vsf = linearize_gamma2_s44(g_vsf);
                let b_lin_vsf = linearize_gamma2_s44(b_vsf);

                // 2. Color space transform: linear VSF RGB → linear sRGB
                let [r_lin_srgb, g_lin_srgb, b_lin_srgb] =
                    apply_matrix_3x3_s44(&vsf_rgb2srgb_s44(), &[r_lin_vsf, g_lin_vsf, b_lin_vsf]);

                // 3. Apply sRGB OETF (gamma encoding for display)
                let r_srgb = srgb_oetf_s44(r_lin_srgb);
                let g_srgb = srgb_oetf_s44(g_lin_srgb);
                let b_srgb = srgb_oetf_s44(b_lin_srgb);

                [
                    (r_srgb << 8isize).to_u8(),
                    (g_srgb << 8isize).to_u8(),
                    (b_srgb << 8isize).to_u8(),
                    (a << 8isize).to_u8(),
                ]
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canvas_creation() {
        let canvas = Canvas::new(100, 100);
        assert_eq!(canvas.dimensions(), (100, 100));
        assert_eq!(canvas.pixels().len(), 10000);
    }

    #[test]
    fn test_clear() {
        let mut canvas = Canvas::new(10, 10);
        // Opaque red: R=1, G=0, B=0, A=1
        canvas.clear(
            ScalarF4E4::ONE,
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ONE,
        );

        let red_rgba = [
            ScalarF4E4::ONE,
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ONE,
        ];
        assert!(canvas.pixels().iter().all(|&p| p == red_rgba));
    }

    #[test]
    fn test_fill_rect_ru() {
        let mut canvas = Canvas::new(100, 100);
        // Opaque black
        canvas.clear(
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ONE,
        );

        // Fill center with white using RU coordinates
        // (0,0) = center, 0.5 RU wide/tall = 50 pixels (span=100, ru=1.0)
        let pos = CircleF4E4::from((ScalarF4E4::ZERO, ScalarF4E4::ZERO)); // center (x, y)
        let size = CircleF4E4::from((
            ScalarF4E4::from(1) / ScalarF4E4::from(2), // w = 0.5 = 50 pixels
            ScalarF4E4::from(1) / ScalarF4E4::from(2), // h = 0.5 = 50 pixels
        ));
        let white = [
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
        ];

        canvas.fill_rect(pos, size, white);

        // Check center pixel is white (R=1, G=1, B=1, A=1)
        let center = 50 * 100 + 50;
        let white_rgba = [
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
        ];
        assert_eq!(canvas.pixels()[center], white_rgba);

        // Check corner pixel is still black (R=0, G=0, B=0, A=1)
        let black_rgba = [
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ONE,
        ];
        assert_eq!(canvas.pixels()[0], black_rgba);
    }

    #[test]
    fn test_fill_rect_vp() {
        let mut canvas = Canvas::new(100, 100);
        // Opaque black
        canvas.clear(
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ONE,
        );

        // Fill center quarter with white using viewport coordinates
        let pos = CircleF4E4::from((
            ScalarF4E4::from(1) / ScalarF4E4::from(4), // x = 0.25
            ScalarF4E4::from(1) / ScalarF4E4::from(4), // y = 0.25
        ));
        let size = CircleF4E4::from((
            ScalarF4E4::from(1) / ScalarF4E4::from(2), // w = 0.5
            ScalarF4E4::from(1) / ScalarF4E4::from(2), // h = 0.5
        ));
        let white = [
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
        ];

        canvas.fill_rect_vp(pos, size, white);

        // Check center pixel is white (R=1, G=1, B=1, A=1)
        let center = 50 * 100 + 50;
        let white_rgba = [
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
        ];
        assert_eq!(canvas.pixels()[center], white_rgba);

        // Check corner pixel is still black (R=0, G=0, B=0, A=1)
        let black_rgba = [
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ONE,
        ];
        assert_eq!(canvas.pixels()[0], black_rgba);
    }

    #[test]
    fn test_fill_circle() {
        let mut canvas = Canvas::new(100, 100);
        // Opaque black
        canvas.clear(
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ONE,
        );

        // Fill circle at center with radius = 0.25 RU (25 pixels)
        let center = CircleF4E4::from((ScalarF4E4::ZERO, ScalarF4E4::ZERO));
        let radius = ScalarF4E4::from(1) / ScalarF4E4::from(4); // 0.25 = 25 pixels
        let white = [
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
        ];

        canvas.fill_circle(center, radius, white);

        // Check center pixel is white
        let center_px = 50 * 100 + 50;
        assert_eq!(canvas.pixels()[center_px], white);

        // Check corner pixel is still black
        let black = [
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ONE,
        ];
        assert_eq!(canvas.pixels()[0], black);
    }

    #[test]
    fn test_stroke_circle() {
        let mut canvas = Canvas::new(100, 100);
        // Opaque black
        canvas.clear(
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ONE,
        );

        // Stroke circle at center
        let center = CircleF4E4::from((ScalarF4E4::ZERO, ScalarF4E4::ZERO));
        let radius = ScalarF4E4::from(1) / ScalarF4E4::from(4); // 0.25 = 25 pixels
        let stroke_width = ScalarF4E4::from(1) / ScalarF4E4::from(20); // 0.05 = 5 pixels
        let white = [
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
        ];

        canvas.stroke_circle(center, radius, stroke_width, white);

        // The exact pixel values depend on implementation, just verify no crash
        // and that some pixels changed
        let white_count = canvas.pixels().iter().filter(|&&p| p == white).count();
        assert!(
            white_count > 0,
            "Expected some white pixels from circle stroke"
        );
    }
}
