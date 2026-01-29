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
    /// VSF RGB color space - conversion to sRGB happens in WASM wrapper
    pixels: Vec<[ScalarF4E4; 4]>,
}

impl Canvas {
    /// Create a new canvas with the given pixel dimensions
    pub fn new(width: usize, height: usize) -> Self {
        // Span: harmonic mean = 2wh/(w+h)
        // Smooth at w==h, biased toward smaller dimension, finite slope at axes
        let span_px = if width + height > 0 {
            2 * width * height / (width + height)
        } else {
            1
        };

        // Opaque black in s44 RGBA: [0, 0, 0, 1]
        let black = [ScalarF4E4::ZERO, ScalarF4E4::ZERO, ScalarF4E4::ZERO, ScalarF4E4::ONE];

        Self {
            width,
            height,
            span: ScalarF4E4::from(span_px),
            ru: ScalarF4E4::ONE,
            half_dims: CircleF4E4::from((ScalarF4E4::from(width), ScalarF4E4::from(height))) >> 1,
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

    /// Clear entire canvas to a colour (s44 RGBA in VSF RGB color space)
    pub fn clear(&mut self, r: ScalarF4E4, g: ScalarF4E4, b: ScalarF4E4, a: ScalarF4E4) {
        self.pixels.fill([r, g, b, a]);
    }

    /// Fill a rectangle (RU coordinates, center-origin)
    ///
    /// Coordinates use Relative Units with (0,0) at center:
    /// - x, y: center of rectangle in RU
    /// - w, h: width and height in RU
    /// - 1.0 RU = span * ru pixels
    pub fn fill_rect(
        &mut self,
        x: ScalarF4E4,
        y: ScalarF4E4,
        w: ScalarF4E4,
        h: ScalarF4E4,
        r: ScalarF4E4,
        g: ScalarF4E4,
        b: ScalarF4E4,
        a: ScalarF4E4,
    ) {
        let color = [r, g, b, a];
        // x,y,w,h are in RU coordinate space - convert to pixel coords
        let cx = self.ru_to_px_x(x);
        let cy = self.ru_to_px_y(y);
        let pw = self.ru_to_px_w(w);
        let ph = self.ru_to_px_h(h);

        // Compute corners (rect is centered at x,y)
        let px = cx - pw / 2;
        let py = cy - ph / 2;

        // Clamp to canvas bounds
        let x1 = px.max(0).min(self.width as isize);
        let y1 = py.max(0).min(self.height as isize);
        let x2 = (px + pw).max(0).min(self.width as isize);
        let y2 = (py + ph).max(0).min(self.height as isize);

        // Fill pixels
        for row in y1..y2 {
            for col in x1..x2 {
                let idx = (row as usize) * self.width + (col as usize);
                self.pixels[idx] = color;
            }
        }
    }

    /// Fill a rectangle (viewport coordinates 0.0-1.0)
    /// Origin at top-left, x/y specify top-left corner of rectangle.
    pub fn fill_rect_vp(
        &mut self,
        x: ScalarF4E4,
        y: ScalarF4E4,
        w: ScalarF4E4,
        h: ScalarF4E4,
        r: ScalarF4E4,
        g: ScalarF4E4,
        b: ScalarF4E4,
        a: ScalarF4E4,
    ) {
        let color = [r, g, b, a];
        // Convert viewport coords to pixel coords (Spirix math)
        let width_s = ScalarF4E4::from(self.width);
        let height_s = ScalarF4E4::from(self.height);

        let px = (x * width_s).to_isize();
        let py = (y * height_s).to_isize();
        let pw = (w * width_s).to_isize();
        let ph = (h * height_s).to_isize();

        // Clamp to canvas bounds
        let x1 = px.max(0).min(self.width as isize);
        let y1 = py.max(0).min(self.height as isize);
        let x2 = (px + pw).max(0).min(self.width as isize);
        let y2 = (py + ph).max(0).min(self.height as isize);

        // Fill pixels
        for row in y1..y2 {
            for col in x1..x2 {
                let idx = (row as usize) * self.width + (col as usize);
                self.pixels[idx] = color;
            }
        }
    }

    /// Draw a single pixel (viewport coordinates)
    pub fn draw_pixel(
        &mut self,
        x: ScalarF4E4,
        y: ScalarF4E4,
        r: ScalarF4E4,
        g: ScalarF4E4,
        b: ScalarF4E4,
        a: ScalarF4E4,
    ) {
        let color = [r, g, b, a];
        let width_s = ScalarF4E4::from(self.width);
        let height_s = ScalarF4E4::from(self.height);

        let px = (x * width_s).to_isize();
        let py = (y * height_s).to_isize();

        // Unsigned bounds check: negative values wrap to huge positive, fail automatically
        if (px as u32) < self.width as u32 && (py as u32) < self.height as u32 {
            let idx = (py as usize) * self.width + (px as usize);
            self.pixels[idx] = color;
        }
    }

    /// Draw text (placeholder - draws coloured rectangle for text bounds)
    pub fn draw_text(
        &mut self,
        x: ScalarF4E4,
        y: ScalarF4E4,
        size: ScalarF4E4,
        text: &str,
        r: ScalarF4E4,
        g: ScalarF4E4,
        b: ScalarF4E4,
        a: ScalarF4E4,
    ) {
        // Placeholder: Draw a coloured rectangle representing text bounds
        // Height is based on size, width is proportional to text length
        let char_width = size * ScalarF4E4::from(6) / ScalarF4E4::from(10);
        let text_width = char_width * ScalarF4E4::from(text.len());

        self.fill_rect(x, y, text_width, size, r, g, b, a);
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
    /// Converts s44 VSF RGB pixels to sRGB RGBA bytes [RR, GG, BB, AA]
    /// TODO: Implement VSF RGB → sRGB color space conversion
    /// Currently just quantizes s44 → u8 without color space conversion
    pub fn to_rgba_bytes(&self) -> Vec<u8> {
        self.pixels
            .iter()
            .flat_map(|&[r, g, b, a]| {
                let r8 = (r << 8isize).to_u8();
                let g8 = (g << 8isize).to_u8();
                let b8 = (b << 8isize).to_u8();
                let a8 = (a << 8isize).to_u8();
                [r8, g8, b8, a8]
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

        let red_rgba = [ScalarF4E4::ONE, ScalarF4E4::ZERO, ScalarF4E4::ZERO, ScalarF4E4::ONE];
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
        let x = ScalarF4E4::from(0); // center
        let y = ScalarF4E4::from(0); // center
        let w = ScalarF4E4::from(1) / ScalarF4E4::from(2); // 0.5 = 50 pixels
        let h = ScalarF4E4::from(1) / ScalarF4E4::from(2); // 0.5 = 50 pixels

        // Opaque white: R=1, G=1, B=1, A=1
        canvas.fill_rect(
            x,
            y,
            w,
            h,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
        );

        // Check center pixel is white (R=1, G=1, B=1, A=1)
        let center = 50 * 100 + 50;
        let white_rgba = [ScalarF4E4::ONE, ScalarF4E4::ONE, ScalarF4E4::ONE, ScalarF4E4::ONE];
        assert_eq!(canvas.pixels()[center], white_rgba);

        // Check corner pixel is still black (R=0, G=0, B=0, A=1)
        let black_rgba = [ScalarF4E4::ZERO, ScalarF4E4::ZERO, ScalarF4E4::ZERO, ScalarF4E4::ONE];
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
        let x = ScalarF4E4::from(1) / ScalarF4E4::from(4); // 0.25
        let y = ScalarF4E4::from(1) / ScalarF4E4::from(4); // 0.25
        let w = ScalarF4E4::from(1) / ScalarF4E4::from(2); // 0.5
        let h = ScalarF4E4::from(1) / ScalarF4E4::from(2); // 0.5

        // Opaque white
        canvas.fill_rect_vp(
            x,
            y,
            w,
            h,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
            ScalarF4E4::ONE,
        );

        // Check center pixel is white (R=1, G=1, B=1, A=1)
        let center = 50 * 100 + 50;
        let white_rgba = [ScalarF4E4::ONE, ScalarF4E4::ONE, ScalarF4E4::ONE, ScalarF4E4::ONE];
        assert_eq!(canvas.pixels()[center], white_rgba);

        // Check corner pixel is still black (R=0, G=0, B=0, A=1)
        let black_rgba = [ScalarF4E4::ZERO, ScalarF4E4::ZERO, ScalarF4E4::ZERO, ScalarF4E4::ONE];
        assert_eq!(canvas.pixels()[0], black_rgba);
    }
}
