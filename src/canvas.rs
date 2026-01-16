//! Canvas backend for viewport-relative rendering
//!
//! All coordinates are in viewport space (0.0-1.0):
//! - (0.0, 0.0) = top-left corner
//! - (1.0, 1.0) = bottom-right corner
//!
//! Resolution-independent rendering - same bytecode works at any resolution.

use spirix::ScalarF4E4;

/// Canvas with fixed pixel resolution
pub struct Canvas {
    /// Width in pixels
    width: usize,

    /// Height in pixels
    height: usize,

    /// Pixel buffer (RGBA, row-major)
    pixels: Vec<u32>,
}

impl Canvas {
    /// Create a new canvas with the given pixel dimensions
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0xFF000000; width * height], // Black with full alpha
        }
    }

    /// Clear entire canvas to a color
    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    /// Fill a rectangle (viewport coordinates 0.0-1.0)
    pub fn fill_rect(&mut self, x: ScalarF4E4, y: ScalarF4E4, w: ScalarF4E4, h: ScalarF4E4, color: u32) {
        // Convert viewport coords to pixel coords
        let px = self.vp_to_px_x(x);
        let py = self.vp_to_px_y(y);
        let pw = self.vp_to_px_w(w);
        let ph = self.vp_to_px_h(h);

        // Clamp to canvas bounds
        let x1 = px.max(0).min(self.width as i32);
        let y1 = py.max(0).min(self.height as i32);
        let x2 = (px + pw).max(0).min(self.width as i32);
        let y2 = (py + ph).max(0).min(self.height as i32);

        // Fill pixels
        for row in y1..y2 {
            for col in x1..x2 {
                let idx = (row as usize) * self.width + (col as usize);
                self.pixels[idx] = color;
            }
        }
    }

    /// Draw a single pixel (viewport coordinates)
    pub fn draw_pixel(&mut self, x: ScalarF4E4, y: ScalarF4E4, color: u32) {
        let px = self.vp_to_px_x(x);
        let py = self.vp_to_px_y(y);

        if px >= 0 && px < self.width as i32 && py >= 0 && py < self.height as i32 {
            let idx = (py as usize) * self.width + (px as usize);
            self.pixels[idx] = color;
        }
    }

    /// Draw text (basic rasterization - for v0, we'll just draw colored blocks)
    /// In a real implementation, this would use a font renderer
    pub fn draw_text(&mut self, x: ScalarF4E4, y: ScalarF4E4, size: ScalarF4E4, text: &str, color: u32) {
        // For v0: Draw a colored rectangle representing text
        // Height is based on size, width is proportional to text length
        let char_width = size * ScalarF4E4::from(0.6); // Approximate aspect ratio
        let text_width = char_width * ScalarF4E4::from(text.len() as f64);

        self.fill_rect(x, y, text_width, size, color);
    }

    /// Convert viewport X coordinate to pixel coordinate
    fn vp_to_px_x(&self, x: ScalarF4E4) -> i32 {
        let f: f64 = x.into();
        (f * self.width as f64) as i32
    }

    /// Convert viewport Y coordinate to pixel coordinate
    fn vp_to_px_y(&self, y: ScalarF4E4) -> i32 {
        let f: f64 = y.into();
        (f * self.height as f64) as i32
    }

    /// Convert viewport width to pixel width
    fn vp_to_px_w(&self, w: ScalarF4E4) -> i32 {
        let f: f64 = w.into();
        (f * self.width as f64) as i32
    }

    /// Convert viewport height to pixel height
    fn vp_to_px_h(&self, h: ScalarF4E4) -> i32 {
        let f: f64 = h.into();
        (f * self.height as f64) as i32
    }

    /// Get canvas dimensions
    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Get pixel buffer (RGBA format)
    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    /// Export as PPM (simple image format for testing)
    pub fn to_ppm(&self) -> String {
        let mut ppm = format!("P3\n{} {}\n255\n", self.width, self.height);

        for pixel in &self.pixels {
            let r = (pixel >> 16) & 0xFF;
            let g = (pixel >> 8) & 0xFF;
            let b = pixel & 0xFF;
            ppm.push_str(&format!("{} {} {} ", r, g, b));
        }

        ppm
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
        canvas.clear(0xFFFF0000); // Red

        assert!(canvas.pixels().iter().all(|&p| p == 0xFFFF0000));
    }

    #[test]
    fn test_fill_rect() {
        let mut canvas = Canvas::new(100, 100);
        canvas.clear(0xFF000000); // Black

        // Fill center quarter with white
        let x = ScalarF4E4::from(0.25);
        let y = ScalarF4E4::from(0.25);
        let w = ScalarF4E4::from(0.5);
        let h = ScalarF4E4::from(0.5);

        canvas.fill_rect(x, y, w, h, 0xFFFFFFFF);

        // Check center pixel is white
        let center = 50 * 100 + 50;
        assert_eq!(canvas.pixels()[center], 0xFFFFFFFF);

        // Check corner pixel is still black
        assert_eq!(canvas.pixels()[0], 0xFF000000);
    }
}
