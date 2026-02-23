#![allow(missing_docs)]
//! Circle rasterization for CanvasFast (u32 sRGB)

use crate::drawing::canvas_fast::CanvasFast;
use spirix::{CircleF4E4, ScalarF4E4};

impl CanvasFast {
    /// Fill a circle (RU coordinates, center-origin)
    pub fn fill_circle(&mut self, center: CircleF4E4, radius: ScalarF4E4, colour: u32) {
        let cx = self.ru_to_px_x(center.r());
        let cy = self.ru_to_px_y(center.i());
        let r  = self.ru_to_px_w(radius);

        for py in (cy - r)..=(cy + r) {
            for px in (cx - r)..=(cx + r) {
                let dx = px - cx;
                let dy = py - cy;
                if dx * dx + dy * dy <= r * r {
                    if (px as usize) < self.coords.width && (py as usize) < self.coords.height {
                        let idx = (py as usize) * self.coords.width + (px as usize);
                        self.pixels[idx] = Self::blend(colour, self.pixels[idx]);
                    }
                }
            }
        }
    }

    /// Stroke a circle outline (RU coordinates, center-origin)
    pub fn stroke_circle(
        &mut self,
        center: CircleF4E4,
        radius: ScalarF4E4,
        stroke_width: ScalarF4E4,
        colour: u32,
    ) {
        let cx      = self.ru_to_px_x(center.r());
        let cy      = self.ru_to_px_y(center.i());
        let r_outer = self.ru_to_px_w(radius + stroke_width / ScalarF4E4::from(2));
        let r_inner = self.ru_to_px_w(radius - stroke_width >> 1).max(0);

        for py in (cy - r_outer)..=(cy + r_outer) {
            for px in (cx - r_outer)..=(cx + r_outer) {
                let dx = px - cx;
                let dy = py - cy;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq >= r_inner * r_inner && dist_sq <= r_outer * r_outer {
                    if (px as usize) < self.coords.width && (py as usize) < self.coords.height {
                        let idx = (py as usize) * self.coords.width + (px as usize);
                        self.pixels[idx] = Self::blend(colour, self.pixels[idx]);
                    }
                }
            }
        }
    }
}
