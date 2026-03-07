//! Line drawing for CanvasQuality
//!
//! - 1px hairline (default / pixel mode): Wu's anti-aliased line algorithm
//! - Weighted lines: single-pass SDF with 1px AA fringe, per-endpoint cap styles

use crate::drawing::canvas_quality::{CanvasQuality, Pixel};
use crate::drawing::shared::{Cap, LineSettings};
use spirix::{CircleF4E4, ScalarF4E4};

impl CanvasQuality {
    /// Draw a line between two RU-space points.
    pub fn draw_line(
        &mut self,
        start: CircleF4E4,
        end: CircleF4E4,
        colour: Pixel,
        settings: &LineSettings,
    ) {
        match settings.weight {
            None => self.draw_line_wu(start, end, colour),
            Some(weight) => self.draw_line_sdf(start, end, colour, weight, settings.cap_start, settings.cap_end),
        }
    }

    /// Wu's anti-aliased 1px line (quality pipeline, linear Pixel)
    fn draw_line_wu(
        &mut self,
        start: CircleF4E4,
        end: CircleF4E4,
        colour: Pixel,
    ) {
        let (mut x0, mut y0, mut x1, mut y1) = (
            ScalarF4E4::from(self.ru_to_px_x(start.r())),
            ScalarF4E4::from(self.ru_to_px_y(start.i())),
            ScalarF4E4::from(self.ru_to_px_x(end.r())),
            ScalarF4E4::from(self.ru_to_px_y(end.i())),
        );

        let width = self.width() as isize;
        let height = self.height() as isize;
        let steep = (y1 - y0).magnitude() > (x1 - x0).magnitude();

        if steep {
            std::mem::swap(&mut x0, &mut y0);
            std::mem::swap(&mut x1, &mut y1);
        }
        if x0 > x1 {
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut y0, &mut y1);
        }

        let dx = x1 - x0;
        let dy = y1 - y0;
        let gradient = if dx == ScalarF4E4::ZERO { ScalarF4E4::ONE } else { dy / dx };
        let half = ScalarF4E4::from_f32(0.5);

        let pixels = self.pixels_mut();
        let mut plot = |x: isize, y: isize, coverage: ScalarF4E4| {
            let (px, py) = if steep { (y, x) } else { (x, y) };
            if px < 0 || px >= width || py < 0 || py >= height { return; }
            let idx = py as usize * width as usize + px as usize;
            let alpha = coverage * colour[3];
            let inv = ScalarF4E4::ONE - alpha;
            let bg = pixels[idx];
            pixels[idx] = [
                colour[0] * alpha + bg[0] * inv,
                colour[1] * alpha + bg[1] * inv,
                colour[2] * alpha + bg[2] * inv,
                colour[3] * alpha + bg[3] * inv,
            ];
        };

        // First endpoint
        let xend = x0.round();
        let yend = y0 + gradient * (xend - x0);
        let xgap = ScalarF4E4::ONE - (x0 + half).frac();
        let xpxl1 = xend.to_isize();
        let ypxl1 = yend.floor().to_isize();
        plot(xpxl1, ypxl1, (ScalarF4E4::ONE - yend.frac()) * xgap);
        plot(xpxl1, ypxl1 + 1, yend.frac() * xgap);
        let mut intery = yend + gradient;

        // Second endpoint
        let xend = x1.round();
        let yend = y1 + gradient * (xend - x1);
        let xgap = (x1 + half).frac();
        let xpxl2 = xend.to_isize();
        let ypxl2 = yend.floor().to_isize();
        plot(xpxl2, ypxl2, (ScalarF4E4::ONE - yend.frac()) * xgap);
        plot(xpxl2, ypxl2 + 1, yend.frac() * xgap);

        // Main loop
        for x in (xpxl1 + 1)..xpxl2 {
            let y = intery.floor().to_isize();
            let frac = intery.frac();
            plot(x, y, ScalarF4E4::ONE - frac);
            plot(x, y + 1, frac);
            intery = intery + gradient;
        }
    }

    /// SDF-based thick line: single pass, per-endpoint caps, AA fringe.
    fn draw_line_sdf(
        &mut self,
        start: CircleF4E4,
        end: CircleF4E4,
        colour: Pixel,
        weight: ScalarF4E4,
        cap_start: Cap,
        cap_end: Cap,
    ) {
        let half = ScalarF4E4::from_f32(0.5);
        let radius = weight * half;

        let x0 = ScalarF4E4::from(self.ru_to_px_x(start.r()));
        let y0 = ScalarF4E4::from(self.ru_to_px_y(start.i()));
        let x1 = ScalarF4E4::from(self.ru_to_px_x(end.r()));
        let y1 = ScalarF4E4::from(self.ru_to_px_y(end.i()));
        let r_px = (weight * self.span() * self.ru() * half).to_isize().max(1);

        let dx = x1 - x0;
        let dy = y1 - y0;
        let seg_len_sq = dx * dx + dy * dy;
        if seg_len_sq == ScalarF4E4::ZERO { return; }
        let seg_len = seg_len_sq.sqrt();
        let extend = radius / seg_len;

        let canvas_w = self.width() as isize;
        let canvas_h = self.height() as isize;

        let margin = r_px + 2;
        let min_x = (x0.to_isize().min(x1.to_isize()) - margin).max(0);
        let max_x = (x0.to_isize().max(x1.to_isize()) + margin).min(canvas_w - 1);
        let min_y = (y0.to_isize().min(y1.to_isize()) - margin).max(0);
        let max_y = (y0.to_isize().max(y1.to_isize()) + margin).min(canvas_h - 1);

        let far = radius + ScalarF4E4::ONE + ScalarF4E4::ONE;
        let pixels = self.pixels_mut();

        for py in min_y..=max_y {
            let pf_y = ScalarF4E4::from(py);
            for px in min_x..=max_x {
                let pf_x = ScalarF4E4::from(px);

                let ap_x = pf_x - x0;
                let ap_y = pf_y - y0;
                let t = (ap_x * dx + ap_y * dy) / seg_len_sq;

                let dist = if t.is_negative() {
                    match cap_start {
                        Cap::Round => (ap_x * ap_x + ap_y * ap_y).sqrt(),
                        Cap::Square => {
                            let tc = t.clamp(-extend, ScalarF4E4::ZERO);
                            let d_x = pf_x - (x0 + dx * tc);
                            let d_y = pf_y - (y0 + dy * tc);
                            (d_x * d_x + d_y * d_y).sqrt()
                        }
                        Cap::Butt => far,
                    }
                } else if t > ScalarF4E4::ONE {
                    match cap_end {
                        Cap::Round => {
                            let bp_x = pf_x - x1;
                            let bp_y = pf_y - y1;
                            (bp_x * bp_x + bp_y * bp_y).sqrt()
                        }
                        Cap::Square => {
                            let tc = t.clamp(ScalarF4E4::ONE, ScalarF4E4::ONE + extend);
                            let d_x = pf_x - (x0 + dx * tc);
                            let d_y = pf_y - (y0 + dy * tc);
                            (d_x * d_x + d_y * d_y).sqrt()
                        }
                        Cap::Butt => far,
                    }
                } else {
                    let d_x = pf_x - (x0 + dx * t);
                    let d_y = pf_y - (y0 + dy * t);
                    (d_x * d_x + d_y * d_y).sqrt()
                };

                let coverage = (radius + half - dist).clamp(ScalarF4E4::ZERO, ScalarF4E4::ONE);
                if !coverage.is_positive() { continue; }

                let alpha = coverage * colour[3];
                let inv = ScalarF4E4::ONE - alpha;
                let idx = py as usize * canvas_w as usize + px as usize;
                let bg = pixels[idx];
                pixels[idx] = [
                    colour[0] * alpha + bg[0] * inv,
                    colour[1] * alpha + bg[1] * inv,
                    colour[2] * alpha + bg[2] * inv,
                    colour[3] * alpha + bg[3] * inv,
                ];
            }
        }
    }
}
