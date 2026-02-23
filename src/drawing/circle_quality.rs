//! Circle rasterization (filled and stroked)

use crate::drawing::canvas_quality::{CanvasQuality, Pixel};
use spirix::{CircleF4E4, ScalarF4E4};

impl CanvasQuality {
    /// Fill a circle (RU coordinates, center-origin)
    pub fn fill_circle(&mut self, center: CircleF4E4, radius: ScalarF4E4, colour: Pixel) {
        let cx = self.ru_to_px_x(center.r());
        let cy = self.ru_to_px_y(center.i());
        let r = self.ru_to_px_w(radius);

        #[cfg(target_arch = "wasm32")]
        crate::wasm::js_log(&format!("fill_circle: center=({},{}) radius={} → px: cx={} cy={} r={}",
            center.r(), center.i(), radius, cx, cy, r), "info");

        for py in (cy - r)..=(cy + r) {
            for px in (cx - r)..=(cx + r) {
                let dx = px - cx;
                let dy = py - cy;
                if dx * dx + dy * dy <= r * r {
                    if (px as usize) < self.width() && (py as usize) < self.height() {
                        let idx = (py as usize) * self.width() + (px as usize);
                        let dst = self.pixels()[idx];
                        self.pixels_mut()[idx] = Self::blend(colour, dst);
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
        colour: Pixel,
    ) {
        let cx = self.ru_to_px_x(center.r());
        let cy = self.ru_to_px_y(center.i());
        let r_outer = self.ru_to_px_w(radius + stroke_width / ScalarF4E4::from(2));
        let r_inner = self.ru_to_px_w(radius - stroke_width >> 1).max(0);

        for py in (cy - r_outer)..=(cy + r_outer) {
            for px in (cx - r_outer)..=(cx + r_outer) {
                let dx = px - cx;
                let dy = py - cy;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq >= r_inner * r_inner && dist_sq <= r_outer * r_outer {
                    if (px as usize) < self.width() && (py as usize) < self.height() {
                        let idx = (py as usize) * self.width() + (px as usize);
                        let dst = self.pixels()[idx];
                        self.pixels_mut()[idx] = Self::blend(colour, dst);
                    }
                }
            }
        }
    }
}
