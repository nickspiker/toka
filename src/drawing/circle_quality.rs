//! Circle and ellipse rasterization (filled and stroked)
//!
//! 1px AA fringe using dist² — no sqrt needed.
//! Circle: edge band r_inner² < dist² ≤ r_outer², coverage interpolated linearly.
//! Ellipse: gradient-magnitude estimate for per-pixel AA band width.

use crate::drawing::canvas_quality::{CanvasQuality, Pixel};
use spirix::{CircleF4E4, ScalarF4E4};

impl CanvasQuality {
    /// Fill a circle with 1px AA fringe (RU coordinates, center-origin)
    pub fn fill_circle(&mut self, center: CircleF4E4, radius: ScalarF4E4, colour: Pixel) {
        let cx = self.ru_to_px_x(center.r());
        let cy = self.ru_to_px_y(center.i());
        let r = self.ru_to_px_w(radius);
        if r <= 0 { return; }

        let r_outer = r;
        let r_inner = r - 1;
        let r_outer2 = r_outer * r_outer;
        let r_inner2 = r_inner * r_inner;
        let edge_range = r_outer2 - r_inner2; // = 2r - 1

        let w = self.width() as isize;
        let h = self.height() as isize;

        let min_y = (cy - r_outer).max(0);
        let max_y = (cy + r_outer).min(h - 1);

        for py in min_y..=max_y {
            let dy = py - cy;
            let dy2 = dy * dy;
            if dy2 > r_outer2 { continue; }

            let row = py as usize * w as usize;
            let min_x = (cx - r_outer).max(0);
            let max_x = (cx + r_outer).min(w - 1);

            for px in min_x..=max_x {
                let dx = px - cx;
                let dist2 = dx * dx + dy2;
                if dist2 > r_outer2 { continue; }

                let idx = row + px as usize;
                let dst = self.pixels()[idx];
                if dist2 <= r_inner2 {
                    self.pixels_mut()[idx] = Self::blend(colour, dst);
                } else {
                    let coverage = ScalarF4E4::from(r_outer2 - dist2) / edge_range;
                    self.pixels_mut()[idx] = Self::blend_weighted(colour, dst, coverage);
                }
            }
        }
    }

    /// Stroke a circle outline with 1px AA fringe on both edges (RU coordinates)
    pub fn stroke_circle(
        &mut self,
        center: CircleF4E4,
        radius: ScalarF4E4,
        stroke_width: ScalarF4E4,
        colour: Pixel,
    ) {
        let cx = self.ru_to_px_x(center.r());
        let cy = self.ru_to_px_y(center.i());
        let half_w = self.ru_to_px_w(stroke_width) >> 1;
        let r_px = self.ru_to_px_w(radius);

        let r_outer = r_px + half_w;
        let r_outer_in = (r_outer - 1).max(0);
        let r_outer2 = r_outer * r_outer;
        let r_outer_in2 = r_outer_in * r_outer_in;
        let edge_outer = r_outer2 - r_outer_in2;

        let r_inner = (r_px - half_w).max(0);
        let r_inner_out = r_inner + 1;
        let r_inner2 = r_inner * r_inner;
        let r_inner_out2 = r_inner_out * r_inner_out;
        let edge_inner = r_inner_out2 - r_inner2;

        let w = self.width() as isize;
        let h = self.height() as isize;

        let min_y = (cy - r_outer).max(0);
        let max_y = (cy + r_outer).min(h - 1);

        for py in min_y..=max_y {
            let dy = py - cy;
            let dy2 = dy * dy;
            if dy2 > r_outer2 { continue; }

            let row = py as usize * w as usize;
            let min_x = (cx - r_outer).max(0);
            let max_x = (cx + r_outer).min(w - 1);

            for px in min_x..=max_x {
                let dx = px - cx;
                let dist2 = dx * dx + dy2;
                if dist2 > r_outer2 || dist2 < r_inner2 { continue; }

                let idx = row + px as usize;
                let dst = self.pixels()[idx];

                if dist2 >= r_inner_out2 && dist2 <= r_outer_in2 {
                    self.pixels_mut()[idx] = Self::blend(colour, dst);
                } else if dist2 > r_outer_in2 && edge_outer > 0 {
                    let coverage = ScalarF4E4::from(r_outer2 - dist2) / edge_outer;
                    self.pixels_mut()[idx] = Self::blend_weighted(colour, dst, coverage);
                } else if dist2 < r_inner_out2 && edge_inner > 0 {
                    let coverage = ScalarF4E4::from(dist2 - r_inner2) / edge_inner;
                    self.pixels_mut()[idx] = Self::blend_weighted(colour, dst, coverage);
                }
            }
        }
    }

    /// Fill an ellipse with 1px AA fringe (RU coordinates, center-origin)
    ///
    /// Uses i64 intermediates to avoid overflow.
    /// AA via gradient-magnitude estimate — no sqrt.
    pub fn fill_ellipse(&mut self, center: CircleF4E4, radii: CircleF4E4, colour: Pixel) {
        let cx = self.ru_to_px_x(center.r());
        let cy = self.ru_to_px_y(center.i());
        let rx = self.ru_to_px_w(radii.r()).max(1);
        let ry = self.ru_to_px_h(radii.i()).max(1);

        let rx2 = rx as i64 * rx as i64;
        let ry2 = ry as i64 * ry as i64;
        let boundary = rx2 * ry2;

        let w = self.width() as isize;
        let h = self.height() as isize;

        let min_y = (cy - ry).max(0);
        let max_y = (cy + ry).min(h - 1);

        for py in min_y..=max_y {
            let dy = (py - cy) as i64;
            let dy2_rx2 = dy * dy * rx2;
            if dy2_rx2 > boundary { continue; }

            let row = py as usize * w as usize;
            let min_x = (cx - rx).max(0);
            let max_x = (cx + rx).min(w - 1);

            for px in min_x..=max_x {
                let dx = (px - cx) as i64;
                let q = dx * dx * ry2 + dy2_rx2;
                if q > boundary { continue; }

                let grad = 2 * (dx.abs() * ry2).max(dy.abs() * rx2);
                let inner = boundary - grad;

                let idx = row + px as usize;
                let dst = self.pixels()[idx];
                if q <= inner || grad == 0 {
                    self.pixels_mut()[idx] = Self::blend(colour, dst);
                } else {
                    let coverage = ScalarF4E4::from(boundary - q) / grad;
                    self.pixels_mut()[idx] = Self::blend_weighted(colour, dst, coverage);
                }
            }
        }
    }

    /// Stroke an ellipse outline with 1px AA fringe on both edges (RU coordinates)
    ///
    /// Stroke width applied per-axis (scales with curvature — thinner at narrow ends).
    pub fn stroke_ellipse(
        &mut self,
        center: CircleF4E4,
        radii: CircleF4E4,
        stroke_width: ScalarF4E4,
        colour: Pixel,
    ) {
        let cx = self.ru_to_px_x(center.r());
        let cy = self.ru_to_px_y(center.i());
        let rx = self.ru_to_px_w(radii.r()).max(1);
        let ry = self.ru_to_px_h(radii.i()).max(1);
        let sw = (self.ru_to_px_w(stroke_width) >> 1).max(1);

        let orx = rx + sw;
        let ory = ry + sw;
        let irx = (rx - sw).max(0);
        let iry = (ry - sw).max(0);

        let orx2 = orx as i64 * orx as i64;
        let ory2 = ory as i64 * ory as i64;
        let b_out = orx2 * ory2;

        let irx2 = irx as i64 * irx as i64;
        let iry2 = iry as i64 * iry as i64;
        let b_in = irx2 * iry2;

        let w = self.width() as isize;
        let h = self.height() as isize;

        let min_y = (cy - ory).max(0);
        let max_y = (cy + ory).min(h - 1);

        for py in min_y..=max_y {
            let dy = (py - cy) as i64;
            if dy * dy * orx2 > b_out { continue; }

            let row = py as usize * w as usize;
            let min_x = (cx - orx).max(0);
            let max_x = (cx + orx).min(w - 1);

            for px in min_x..=max_x {
                let dx = (px - cx) as i64;

                let q_out = dx * dx * ory2 + dy * dy * orx2;
                if q_out > b_out { continue; }

                let q_in = if irx > 0 && iry > 0 {
                    dx * dx * iry2 + dy * dy * irx2
                } else { 0 };
                if q_in < b_in && irx > 0 && iry > 0 { continue; }

                let grad_out = 2 * (dx.abs() * ory2).max(dy.abs() * orx2);
                let grad_in = if irx > 0 && iry > 0 {
                    2 * (dx.abs() * iry2).max(dy.abs() * irx2)
                } else { 0 };

                let idx = row + px as usize;
                let dst = self.pixels()[idx];

                if q_out > b_out - grad_out && grad_out > 0 {
                    let coverage = ScalarF4E4::from(b_out - q_out) / grad_out;
                    self.pixels_mut()[idx] = Self::blend_weighted(colour, dst, coverage);
                } else if irx > 0 && iry > 0 && q_in < b_in + grad_in && grad_in > 0 {
                    let coverage = ScalarF4E4::from(q_in - b_in) / grad_in;
                    self.pixels_mut()[idx] = Self::blend_weighted(colour, dst, coverage);
                } else {
                    self.pixels_mut()[idx] = Self::blend(colour, dst);
                }
            }
        }
    }
}
