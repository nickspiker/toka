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

    /// Pixel buffer: packed u32 RGBA (little-endian: R | G<<8 | B<<16 | A<<24)
    /// sRGB gamma-encoded u8 per channel, ready for Canvas API
    pixels: Vec<u32>,
}

impl Canvas {
    /// Create a new canvas with the given pixel dimensions
    pub fn new(width: usize, height: usize) -> Self {
        // Opaque black: R=0, G=0, B=0, A=255
        let black: u32 = 0xFF000000;

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

    /// Clear entire canvas to a colour (packed u32 RGBA)
    pub fn clear(&mut self, colour: u32) {
        self.pixels.fill(colour);
    }

    /// Fill a rectangle (centered pixel coordinates)
    ///
    /// - cx, cy: center of rectangle in pixels relative to canvas center
    /// - w, h: width and height in pixels
    /// - colour: packed u32 RGBA
    pub fn fill_rect_px(&mut self, cx: isize, cy: isize, w: isize, h: isize, colour: u32) {
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
    /// - colour: packed u32 RGBA
    /// - 1 RU = span * ru pixels
    pub fn fill_rect_ru(&mut self, pos: CircleF4E4, size: CircleF4E4, colour: u32) {
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

    /// Fill a rotated rectangle (RU coordinates, center-origin)
    ///
    /// - pos: center of rectangle (x, y) in RU as CircleF4E4
    /// - size: dimensions (w, h) in RU as CircleF4E4
    /// - angle: rotation angle in radians as ScalarF4E4 (positive = clockwise)
    /// - colour: packed u32 RGBA
    ///
    /// Decomposes rectangle into 4 triangles with AA on outer edges
    pub fn fill_rotated_rect_ru(
        &mut self,
        pos: CircleF4E4,
        size: CircleF4E4,
        angle: ScalarF4E4,
        colour: u32,
    ) {
        let center = self.half_dims + pos * self.span * self.ru;

        // Convert size to half-extents (scale from center to edge)
        let scale: CircleF4E4 = (size * self.span * self.ru) >> 1;

        // Rotation as complex number: cos(θ) + i*sin(θ)
        let rot = CircleF4E4::from((angle.cos(), angle.sin()));

        // Calculate 4 corners of rotated rectangle using complex multiplication
        // Unrotated corners relative to center:
        //   0(-,-) --- 1(+,-)
        //   |\  C   /|
        //   | \ + / |  (+ = center)
        //   |A |X| B|
        //   | / + \ |
        //   |/  D  \|
        //   3(-,+) --- 2(+,+)

        // Each corner offset is rotated by multiplying with rot complex number
        let offset0 = CircleF4E4::from((-scale.r(), -scale.i()));
        let offset1 = CircleF4E4::from((scale.r(), -scale.i()));
        let offset2 = CircleF4E4::from((scale.r(), scale.i()));
        let offset3 = CircleF4E4::from((-scale.r(), scale.i()));

        let c0 = center + offset0 * rot;
        let c1 = center + offset1 * rot;
        let c2 = center + offset2 * rot;
        let c3 = center + offset3 * rot;

        // Determine scan direction based on edge c0→c1 orientation
        // Scan perpendicular to dominant edge to ensure 1px-wide AA
        let edge_dx = (c1.r() - c0.r()).magnitude();
        let edge_dy = (c1.i() - c0.i()).magnitude();

        if edge_dx > edge_dy {
            // Edge c0→c1 is near-horizontal → scan vertically (X scanlines)
            self.fill_rect_polygon_vertical(c0, c1, c2, c3, colour);
        } else {
            // Edge c0→c1 is near-vertical → scan horizontally (Y scanlines)
            self.fill_rect_polygon_horizontal(c0, c1, c2, c3, colour);
        }
    }

    /// Fill a triangle with anti-aliasing on the outer edge (p1 → p2)
    ///
    /// Triangle vertices: center (exact center), p1, p2
    /// - Edges from center are interior (no AA)
    /// - Edge p1 → p2 is exterior (needs AA)
    ///
    /// Strategy: Scan based on outer edge slope
    fn fill_triangle_aa(
        &mut self,
        center: CircleF4E4,
        p1: CircleF4E4,
        p2: CircleF4E4,
        colour: u32,
    ) {
        let diff = p2 - p1;
        // Determine outer edge slope
        let dx = diff.r().magnitude();
        let dy = diff.i().magnitude();

        // Choose scan direction based on which axis is dominant
        if dx > dy {
            // Horizontal-dominant: scan in Y direction
            self.fill_triangle_horizontal(center, p1, p2, colour);
        } else {
            // Vertical-dominant: scan in X direction
            self.fill_triangle_vertical(center, p1, p2, colour);
        }
    }

    /// Fill triangle with horizontal scanlines (Y-major)
    fn fill_triangle_horizontal(
        &mut self,
        center: CircleF4E4,
        p1: CircleF4E4,
        p2: CircleF4E4,
        colour: u32,
    ) {
        // Find Y bounds
        let min_y = p1.i().min(p2.i()).min(center.i());
        let max_y = p1.i().max(p2.i()).max(center.i());

        let y_start = min_y.clamp(0, self.height);
        let y_end = max_y.clamp(0, self.height);

        // Scan each row
        for y_px in y_start.to_usize()..=y_end.to_usize() {
            let y = ScalarF4E4::from(y_px);

            // Find intersections with all 3 edges
            let mut intersections = Vec::new();

            // Edge 1: center → p1
            if let Some(x) = Self::line_intersect_y(center, p1, y) {
                intersections.push(x);
            }

            // Edge 2: center → p2
            if let Some(x) = Self::line_intersect_y(center, p2, y) {
                intersections.push(x);
            }

            // Edge 3 (outer): p1 → p2
            if let Some(x) = Self::line_intersect_y(p1, p2, y) {
                intersections.push(x);
            }

            if intersections.len() >= 2 {
                intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());

                let x_left = intersections[0];
                let x_right = *intersections.last().unwrap();

                // Fill interior pixels
                let x_start_int = (x_left + ScalarF4E4::ONE).to_isize();
                let x_end_int = x_right.to_isize();

                for x in x_start_int..x_end_int {
                    if x >= 0 && (x as usize) < self.width {
                        let idx = (y_px as usize) * self.width + (x as usize);
                        if idx < self.pixels.len() {
                            self.pixels[idx] = colour;
                        }
                    }
                }

                // AA left edge pixel
                let x_left_px = x_left.to_isize();
                if x_left_px >= 0 && (x_left_px as usize) < self.width {
                    let coverage = ScalarF4E4::ONE - (x_left - ScalarF4E4::from(x_left_px));
                    let weight = (coverage * ScalarF4E4::from(255)).to_isize().clamp(0, 255) as u8;
                    self.blend_pixel(x_left_px, y_px as isize, colour, weight);
                }

                // AA right edge pixel
                let x_right_px = x_right.to_isize();
                if x_right_px >= 0 && (x_right_px as usize) < self.width {
                    let coverage = x_right - ScalarF4E4::from(x_right_px);
                    let weight = (coverage * ScalarF4E4::from(255)).to_isize().clamp(0, 255) as u8;
                    self.blend_pixel(x_right_px, y_px as isize, colour, weight);
                }
            }
        }
    }

    /// Fill triangle with vertical scanlines (X-major)
    fn fill_triangle_vertical(
        &mut self,
        center: CircleF4E4,
        p1: CircleF4E4,
        p2: CircleF4E4,
        colour: u32,
    ) {
        // Find X bounds
        let min_x = p1.r().min(p2.r()).min(center.r());
        let max_x = p1.r().max(p2.r()).max(center.r());

        let x_start = min_x.to_isize().clamp(0, self.width as isize);
        let x_end = max_x.to_isize().clamp(0, self.width as isize);

        // Scan each column
        for x_px in x_start..=x_end {
            let x = ScalarF4E4::from(x_px);

            // Find intersections with all 3 edges
            let mut intersections = Vec::new();

            // Edge 1: center → p1
            if let Some(y) = Self::line_intersect_x(center, p1, x) {
                intersections.push(y);
            }

            // Edge 2: center → p2
            if let Some(y) = Self::line_intersect_x(center, p2, x) {
                intersections.push(y);
            }

            // Edge 3 (outer): p1 → p2
            if let Some(y) = Self::line_intersect_x(p1, p2, x) {
                intersections.push(y);
            }

            if intersections.len() >= 2 {
                intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());

                let y_top = intersections[0];
                let y_bottom = *intersections.last().unwrap();

                // Fill interior pixels
                let y_start_int = (y_top + ScalarF4E4::ONE).to_isize();
                let y_end_int = y_bottom.to_isize();

                for y in y_start_int..y_end_int {
                    if y >= 0 && (y as usize) < self.height {
                        let idx = (y as usize) * self.width + (x_px as usize);
                        if idx < self.pixels.len() {
                            self.pixels[idx] = colour;
                        }
                    }
                }

                // AA top edge pixel
                let y_top_px = y_top.to_isize();
                if y_top_px >= 0 && (y_top_px as usize) < self.height {
                    let coverage = ScalarF4E4::ONE - (y_top - ScalarF4E4::from(y_top_px));
                    let weight = (coverage * ScalarF4E4::from(255)).to_isize().clamp(0, 255) as u8;
                    self.blend_pixel(x_px, y_top_px, colour, weight);
                }

                // AA bottom edge pixel
                let y_bottom_px = y_bottom.to_isize();
                if y_bottom_px >= 0 && (y_bottom_px as usize) < self.height {
                    let coverage = y_bottom - ScalarF4E4::from(y_bottom_px);
                    let weight = (coverage * ScalarF4E4::from(255)).to_isize().clamp(0, 255) as u8;
                    self.blend_pixel(x_px, y_bottom_px, colour, weight);
                }
            }
        }
    }

    /// Calculate X intersection of line segment with horizontal scanline at Y
    fn line_intersect_y(p1: CircleF4E4, p2: CircleF4E4, y: ScalarF4E4) -> Option<ScalarF4E4> {
        let x1 = p1.r();
        let y1 = p1.i();
        let x2 = p2.r();
        let y2 = p2.i();

        // Check if scanline crosses this edge
        if (y1 <= y && y < y2) || (y2 <= y && y < y1) {
            let dy = y2 - y1;
            if dy != ScalarF4E4::ZERO {
                let t = (y - y1) / dy;
                return Some(x1 + t * (x2 - x1));
            }
        }
        None
    }

    /// Calculate Y intersection of line segment with vertical scanline at X
    fn line_intersect_x(p1: CircleF4E4, p2: CircleF4E4, x: ScalarF4E4) -> Option<ScalarF4E4> {
        let x1 = p1.r();
        let y1 = p1.i();
        let x2 = p2.r();
        let y2 = p2.i();

        // Check if scanline crosses this edge
        if (x1 <= x && x < x2) || (x2 <= x && x < x1) {
            let dx = x2 - x1;
            if dx != ScalarF4E4::ZERO {
                let t = (x - x1) / dx;
                return Some(y1 + t * (y2 - y1));
            }
        }
        None
    }

    /// Fill rectangle polygon with horizontal scanlines (Y-major)
    /// Scans perpendicular to near-vertical edges
    fn fill_rect_polygon_horizontal(
        &mut self,
        c0: CircleF4E4,
        c1: CircleF4E4,
        c2: CircleF4E4,
        c3: CircleF4E4,
        colour: u32,
    ) {
        // Find Y bounds
        let min_y = c0.i().min(c1.i()).min(c2.i()).min(c3.i());
        let max_y = c0.i().max(c1.i()).max(c2.i()).max(c3.i());

        let y_start = min_y.to_isize().clamp(0, self.height as isize);
        let y_end = max_y.to_isize().clamp(0, self.height as isize);

        // Scan each row
        for y_px in y_start..=y_end {
            let y = ScalarF4E4::from(y_px);

            // Find intersections with all 4 edges
            let mut intersections = Vec::new();

            if let Some(x) = Self::line_intersect_y(c0, c1, y) {
                intersections.push(x);
            }
            if let Some(x) = Self::line_intersect_y(c1, c2, y) {
                intersections.push(x);
            }
            if let Some(x) = Self::line_intersect_y(c2, c3, y) {
                intersections.push(x);
            }
            if let Some(x) = Self::line_intersect_y(c3, c0, y) {
                intersections.push(x);
            }

            if intersections.len() >= 2 {
                intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());

                let x_left = intersections[0];
                let x_right = *intersections.last().unwrap();

                // Fill interior pixels
                let x_start_int = (x_left + ScalarF4E4::ONE).to_isize();
                let x_end_int = x_right.to_isize();

                for x in x_start_int..x_end_int {
                    if x >= 0 && (x as usize) < self.width {
                        let idx = (y_px as usize) * self.width + (x as usize);
                        if idx < self.pixels.len() {
                            self.pixels[idx] = colour;
                        }
                    }
                }

                // AA left edge pixel
                let x_left_px = x_left.to_isize();
                if x_left_px >= 0 && (x_left_px as usize) < self.width {
                    let coverage = ScalarF4E4::ONE - (x_left - ScalarF4E4::from(x_left_px));
                    let weight =
                        (coverage * ScalarF4E4::from(255)).to_isize().clamp(0, 255) as u8;
                    self.blend_pixel(x_left_px, y_px, colour, weight);
                }

                // AA right edge pixel
                let x_right_px = x_right.to_isize();
                if x_right_px >= 0 && (x_right_px as usize) < self.width {
                    let coverage = x_right - ScalarF4E4::from(x_right_px);
                    let weight =
                        (coverage * ScalarF4E4::from(255)).to_isize().clamp(0, 255) as u8;
                    self.blend_pixel(x_right_px, y_px, colour, weight);
                }
            }
        }
    }

    /// Fill rectangle polygon with vertical scanlines (X-major)
    /// Scans perpendicular to near-horizontal edges
    fn fill_rect_polygon_vertical(
        &mut self,
        c0: CircleF4E4,
        c1: CircleF4E4,
        c2: CircleF4E4,
        c3: CircleF4E4,
        colour: u32,
    ) {
        // Find X bounds
        let min_x = c0.r().min(c1.r()).min(c2.r()).min(c3.r());
        let max_x = c0.r().max(c1.r()).max(c2.r()).max(c3.r());

        let x_start = min_x.to_isize().clamp(0, self.width as isize);
        let x_end = max_x.to_isize().clamp(0, self.width as isize);

        // Scan each column
        for x_px in x_start..=x_end {
            let x = ScalarF4E4::from(x_px);

            // Find intersections with all 4 edges
            let mut intersections = Vec::new();

            if let Some(y) = Self::line_intersect_x(c0, c1, x) {
                intersections.push(y);
            }
            if let Some(y) = Self::line_intersect_x(c1, c2, x) {
                intersections.push(y);
            }
            if let Some(y) = Self::line_intersect_x(c2, c3, x) {
                intersections.push(y);
            }
            if let Some(y) = Self::line_intersect_x(c3, c0, x) {
                intersections.push(y);
            }

            if intersections.len() >= 2 {
                intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());

                let y_top = intersections[0];
                let y_bottom = *intersections.last().unwrap();

                // Fill interior pixels
                let y_start_int = (y_top + ScalarF4E4::ONE).to_isize();
                let y_end_int = y_bottom.to_isize();

                for y in y_start_int..y_end_int {
                    if y >= 0 && (y as usize) < self.height {
                        let idx = (y as usize) * self.width + (x_px as usize);
                        if idx < self.pixels.len() {
                            self.pixels[idx] = colour;
                        }
                    }
                }

                // AA top edge pixel
                let y_top_px = y_top.to_isize();
                if y_top_px >= 0 && (y_top_px as usize) < self.height {
                    let coverage = ScalarF4E4::ONE - (y_top - ScalarF4E4::from(y_top_px));
                    let weight =
                        (coverage * ScalarF4E4::from(255)).to_isize().clamp(0, 255) as u8;
                    self.blend_pixel(x_px, y_top_px, colour, weight);
                }

                // AA bottom edge pixel
                let y_bottom_px = y_bottom.to_isize();
                if y_bottom_px >= 0 && (y_bottom_px as usize) < self.height {
                    let coverage = y_bottom - ScalarF4E4::from(y_bottom_px);
                    let weight =
                        (coverage * ScalarF4E4::from(255)).to_isize().clamp(0, 255) as u8;
                    self.blend_pixel(x_px, y_bottom_px, colour, weight);
                }
            }
        }
    }

    /// Blend a pixel with coverage-based alpha
    /// For AA edges on opaque shapes
    fn blend_pixel(&mut self, x: isize, y: isize, fg_colour: u32, weight: u8) {
        if x >= 0 && (x as usize) < self.width && y >= 0 && (y as usize) < self.height {
            let idx = (y as usize) * self.width + (x as usize);
            if idx < self.pixels.len() {
                let bg = self.pixels[idx];
                self.pixels[idx] = Self::blend_s_alpha(fg_colour, bg, weight);
            }
        }
    }

    //Note this literally blends from 0-255 out of 256 so it will not be completely opaque. Set pixels directly for 100%fg
    fn blend_alpha(fg_colour: u32, bg_colour: u32) -> u32 {
        // Extract alpha from fg_colour (low byte, bits 0-7)
        let weight_fg = fg_colour as u8 as u64;
        let weight_bg = 255 - weight_fg;

        // SIMD-in-register: spread u32 RGBA into u64, blend all channels in parallel
        let mut bg = bg_colour as u64;
        bg = (bg | (bg << 16)) & 0x0000FFFF0000FFFF;
        bg = (bg | (bg << 8)) & 0x00FF00FF00FF00FF;

        let mut fg = fg_colour as u64;
        fg = (fg | (fg << 16)) & 0x0000FFFF0000FFFF;
        fg = (fg | (fg << 8)) & 0x00FF00FF00FF00FF;

        // Blend all 4 channels
        let mut blended = bg * weight_bg + fg * weight_fg;
        blended = (blended >> 8) & 0x00FF00FF00FF00FF;
        blended = (blended | (blended >> 8)) & 0x0000FFFF0000FFFF;
        blended = blended | (blended >> 16);

        blended as u32
    }

    //Note this literally blends from 0-255 out of 256 so it will not be completely opaque. Set pixels directly for 100%fg
    fn blend_s_alpha(fg_colour: u32, bg_colour: u32, weight_fg: u8) -> u32 {
        let weight_bg = 255 - weight_fg as u64;

        // SIMD-in-register: spread u32 RGBA into u64, blend all channels in parallel
        let mut bg = bg_colour as u64;
        bg = (bg | (bg << 16)) & 0x0000FFFF0000FFFF;
        bg = (bg | (bg << 8)) & 0x00FF00FF00FF00FF;

        let mut fg = fg_colour as u64;
        fg = (fg | (fg << 16)) & 0x0000FFFF0000FFFF;
        fg = (fg | (fg << 8)) & 0x00FF00FF00FF00FF;

        // Blend all 4 channels
        let mut blended = bg * weight_bg + fg * weight_fg as u64;
        blended = (blended >> 8) & 0x00FF00FF00FF00FF;
        blended = (blended | (blended >> 8)) & 0x0000FFFF0000FFFF;
        blended = blended | (blended >> 16);

        blended as u32
    }

    /// Set a single pixel (centered pixel coordinates)
    ///
    /// - x, y: pixel position relative to canvas center
    /// - colour: packed u32 RGBA
    pub fn set_pixel_px(&mut self, x: isize, y: isize, colour: u32) {
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
    /// - colour: packed u32 RGBA
    pub fn set_pixel_ru(&mut self, pos: CircleF4E4, colour: u32) {
        let x = self.ru_to_px_x(pos.r());
        let y = self.ru_to_px_y(pos.i());
        self.set_pixel_px(x, y, colour);
    }

    /// Draw a single pixel (viewport coordinates)
    /// - pos: pixel position (x, y) in viewport coords as CircleF4E4
    /// - colour: RGBA as [ScalarF4E4; 4]
    pub fn draw_pixel(&mut self, pos: CircleF4E4, colour: u32) {
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
    pub fn draw_text(&mut self, pos: CircleF4E4, size: ScalarF4E4, text: &str, colour: u32) {
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
    pub fn fill_circle(&mut self, center: CircleF4E4, radius: ScalarF4E4, colour: u32) {
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
        colour: u32,
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

    /// Draw an anti-aliased line (pixel coordinates) - WIP
    /// - start: start point (x, y) in pixels
    /// - end: end point (x, y) in pixels
    /// - colour_start: packed u32 RGBA at line start
    /// - colour_end: packed u32 RGBA at line end
    #[allow(dead_code)]
    pub fn draw_line(
        &mut self,
        _start: CircleF4E4,
        _end: CircleF4E4,
        _colour_start: u32,
        _colour_end: u32,
    ) {
        // TODO: Implement gradient line drawing for u32 pixel format
        // For now, gradients are not used in ro* rendering
    }

    /// Get canvas dimensions
    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Get pixel buffer (packed u32 RGBA)
    pub fn pixels(&self) -> &[u32] {
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
    /// Get pixel buffer as RGBA byte array for Canvas API
    ///
    /// Zero-cost view of internal u32 buffer as bytes
    /// Pixels are already in sRGB u8 format (R | G<<8 | B<<16 | A<<24)
    pub fn to_rgba_bytes(&self) -> &[u8] {
        // Use bytemuck for safe transmute (no unsafe code)
        bytemuck::cast_slice(&self.pixels)
    }
}

/// Convert linear S44 RGBA to packed u32 sRGB
///
/// Pipeline: Linear VSF RGB → Linear sRGB → Gamma sRGB → u8 → packed u32
fn convert_colour_to_u32(rgba_linear_s44: [ScalarF4E4; 4]) -> u32 {
    use vsf::colour::convert::{
        apply_matrix_3x3_s44, linearize_gamma2_s44, srgb_oetf_s44, vsf_rgb2srgb_s44,
    };

    let [r_vsf, g_vsf, b_vsf, a] = rgba_linear_s44;

    // 1. Decode VSF gamma 2: encoded^2 → linear
    let r_lin_vsf = linearize_gamma2_s44(r_vsf);
    let g_lin_vsf = linearize_gamma2_s44(g_vsf);
    let b_lin_vsf = linearize_gamma2_s44(b_vsf);

    // 2. Colour space transform: linear VSF RGB → linear sRGB
    let [r_lin_srgb, g_lin_srgb, b_lin_srgb] =
        apply_matrix_3x3_s44(&vsf_rgb2srgb_s44(), &[r_lin_vsf, g_lin_vsf, b_lin_vsf]);

    // 3. Apply sRGB OETF (gamma encoding for display)
    let r_srgb = srgb_oetf_s44(r_lin_srgb);
    let g_srgb = srgb_oetf_s44(g_lin_srgb);
    let b_srgb = srgb_oetf_s44(b_lin_srgb);

    // 4. Quantize to u8 and pack into u32 (little-endian RGBA)
    let r = (r_srgb << 8isize).to_u8();
    let g = (g_srgb << 8isize).to_u8();
    let b = (b_srgb << 8isize).to_u8();
    let a = (a << 8isize).to_u8();

    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24)
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

        canvas.fill_rect_ru(pos, size, white);

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
    fn test_fill_rect_ru_centered() {
        let mut canvas = Canvas::new(100, 100);
        // Opaque black
        canvas.clear(
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ZERO,
            ScalarF4E4::ONE,
        );

        // Fill center quarter with white using RU coordinates
        let pos = CircleF4E4::from((
            ScalarF4E4::from(0), // x = 0 (center)
            ScalarF4E4::from(0), // y = 0 (center)
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

        canvas.fill_rect_ru(pos, size, white);

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
