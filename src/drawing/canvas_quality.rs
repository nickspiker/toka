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
//!
//! Pixel buffer stores linear RGBA as [ScalarF4E4; 4].
//! sRGB OETF + error diffusion downconversion applied at to_rgba_bytes().

use spirix::{CircleF4E4, ScalarF4E4};

/// Linear RGBA pixel: [R, G, B, A] in S44, all channels [0.0, 1.0]
pub type Pixel = [ScalarF4E4; 4];

/// Opaque black in linear S44 RGBA
pub const BLACK: Pixel = [
    ScalarF4E4::ZERO,
    ScalarF4E4::ZERO,
    ScalarF4E4::ZERO,
    ScalarF4E4::ONE,
];

/// Canvas with fixed pixel resolution and RU-based coordinate system
pub struct CanvasQuality {
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

    /// Pixel buffer: linear RGBA S44 per pixel
    /// Composited in linear light; sRGB OETF applied at to_rgba_bytes()
    pixels: Vec<Pixel>,
}

impl CanvasQuality {
    /// Create a new canvas with the given pixel dimensions
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            span: ScalarF4E4::from(width * height) / (width + height),
            ru: ScalarF4E4::ONE,
            half_dims: CircleF4E4::from((width, height)) >> 1,
            pixels: vec![BLACK; width * height],
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

    /// Get half dimensions (for center-origin calculations)
    pub(crate) fn half_dims(&self) -> CircleF4E4 {
        self.half_dims
    }

    /// Convert RU X coordinate to pixel coordinate
    pub(crate) fn ru_to_px_x(&self, x: ScalarF4E4) -> isize {
        let px = self.half_dims.r() + x * self.span * self.ru;
        px.to_isize()
    }

    /// Convert RU Y coordinate to pixel coordinate
    pub(crate) fn ru_to_px_y(&self, y: ScalarF4E4) -> isize {
        let py = self.half_dims.i() + y * self.span * self.ru;
        py.to_isize()
    }

    /// Convert RU width to pixel width
    pub(crate) fn ru_to_px_w(&self, w: ScalarF4E4) -> isize {
        let pw = w * self.span * self.ru;
        pw.to_isize()
    }

    /// Convert RU height to pixel height
    pub(crate) fn ru_to_px_h(&self, h: ScalarF4E4) -> isize {
        let ph = h * self.span * self.ru;
        ph.to_isize()
    }

    /// Clear entire canvas to a VSF colour
    pub fn clear(&mut self, colour: &vsf::VsfType) -> Result<(), String> {
        let pixel = crate::renderer::extract_colour_linear(colour)?;
        self.pixels.fill(pixel);
        Ok(())
    }

    /// Get canvas dimensions
    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Get pixel buffer (linear S44 RGBA)
    pub fn pixels(&self) -> &[Pixel] {
        &self.pixels
    }

    /// Get mutable pixel buffer
    pub(crate) fn pixels_mut(&mut self) -> &mut [Pixel] {
        &mut self.pixels
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
    /// Pipeline per pixel:
    ///   linear S44 → gamma-2 OETF (sqrt) per channel → scaled to [0, 255]
    ///
    /// Gamma-2 is self-consistent with the gamma-2 EOTF used on input (ra squaring).
    /// Error diffusion downconversion: cast to u8, compute remainder,
    /// carry remainder forward to next pixel independently per channel.
    /// Alpha is kept linear (no OETF — alpha is not a light quantity).
    pub fn to_rgba_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.pixels.len() * 4);

        // Per-channel error accumulators (carried across the scanline)
        let mut err_r = ScalarF4E4::ZERO;
        let mut err_g = ScalarF4E4::ZERO;
        let mut err_b = ScalarF4E4::ZERO;
        let mut err_a = ScalarF4E4::ZERO;

        for (i, pixel) in self.pixels.iter().enumerate() {
            // Reset error at start of each row
            if i % self.width == 0 {
                err_r = ScalarF4E4::ZERO;
                err_g = ScalarF4E4::ZERO;
                err_b = ScalarF4E4::ZERO;
                err_a = ScalarF4E4::ZERO;
            }

            // Apply gamma-2 OETF (sqrt) to RGB channels, alpha stays linear
            let r_enc: ScalarF4E4 = (pixel[0].sqrt() << 8) + err_r;
            let g_enc: ScalarF4E4 = (pixel[1].sqrt() << 8) + err_g;
            let b_enc: ScalarF4E4 = (pixel[2].sqrt() << 8) + err_b;
            let a_enc: ScalarF4E4 = (pixel[3] << 8) + err_a;

            // Truncate to u8
            let r_u8 = r_enc.to_u8();
            let g_u8 = g_enc.to_u8();
            let b_u8 = b_enc.to_u8();
            let a_u8 = a_enc.to_u8();

            // Compute remainder and carry forward
            err_r = r_enc - ScalarF4E4::from(r_u8);
            err_g = g_enc - ScalarF4E4::from(g_u8);
            err_b = b_enc - ScalarF4E4::from(b_u8);
            err_a = a_enc - ScalarF4E4::from(a_u8);

            bytes.push(r_u8);
            bytes.push(g_u8);
            bytes.push(b_u8);
            bytes.push(a_u8);
        }

        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canvas_creation() {
        let canvas = CanvasQuality::new(100, 100);
        assert_eq!(canvas.dimensions(), (100, 100));
        assert_eq!(canvas.pixels().len(), 10000);
    }

    #[test]
    fn test_clear() {
        let mut canvas = CanvasQuality::new(10, 10);
        let red = vsf::VsfType::rcr;
        canvas.clear(&red).unwrap();

        // All pixels should be the same after clear
        let first_pixel = canvas.pixels()[0];
        assert!(canvas.pixels().iter().all(|p| p == &first_pixel));
    }
}
