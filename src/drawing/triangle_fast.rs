#![allow(missing_docs)]
//! Triangle rasterization for CanvasFast (u32 sRGB)

use crate::drawing::canvas_fast::CanvasFast;
use spirix::{CircleF4E4, ScalarF4E4};

impl CanvasFast {
    pub(crate) fn fill_triangle_aa(
        &mut self,
        center: CircleF4E4,
        p1: CircleF4E4,
        p2: CircleF4E4,
        colour: u32,
    ) {
        let diff = p2 - p1;
        if diff.r().magnitude() > diff.i().magnitude() {
            self.fill_triangle_horizontal(center, p1, p2, colour);
        } else {
            self.fill_triangle_vertical(center, p1, p2, colour);
        }
    }

    pub(crate) fn fill_triangle_horizontal(
        &mut self,
        center: CircleF4E4,
        p1: CircleF4E4,
        p2: CircleF4E4,
        colour: u32,
    ) {
        let min_y = p1.i().min(p2.i()).min(center.i());
        let max_y = p1.i().max(p2.i()).max(center.i());
        let y_start = min_y.clamp(0, self.coords.height);
        let y_end   = max_y.clamp(0, self.coords.height);

        for y_px in y_start.to_usize()..=y_end.to_usize() {
            let y = ScalarF4E4::from(y_px);
            let mut xs = Vec::new();
            if let Some(x) = Self::line_intersect_y(center, p1, y) { xs.push(x); }
            if let Some(x) = Self::line_intersect_y(center, p2, y) { xs.push(x); }
            if let Some(x) = Self::line_intersect_y(p1, p2, y)     { xs.push(x); }

            if xs.len() >= 2 {
                xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let x_left  = xs[0];
                let x_right = *xs.last().unwrap();

                // Interior — direct write
                for x in (x_left + ScalarF4E4::ONE).to_isize()..x_right.to_isize() {
                    if x >= 0 && (x as usize) < self.coords.width {
                        let idx = y_px * self.coords.width + x as usize;
                        if idx < self.pixels.len() { self.pixels[idx] = colour; }
                    }
                }

                // AA edge pixels — coverage blend
                let xl_px = x_left.to_isize();
                if xl_px >= 0 && (xl_px as usize) < self.coords.width {
                    let weight = ((ScalarF4E4::ONE - (x_left - ScalarF4E4::from(xl_px))) * ScalarF4E4::from(255)).to_isize().clamp(0, 255) as u8;
                    self.blend_pixel(xl_px, y_px as isize, colour, weight);
                }
                let xr_px = x_right.to_isize();
                if xr_px >= 0 && (xr_px as usize) < self.coords.width {
                    let weight = ((x_right - ScalarF4E4::from(xr_px)) * ScalarF4E4::from(255)).to_isize().clamp(0, 255) as u8;
                    self.blend_pixel(xr_px, y_px as isize, colour, weight);
                }
            }
        }
    }

    pub(crate) fn fill_triangle_vertical(
        &mut self,
        center: CircleF4E4,
        p1: CircleF4E4,
        p2: CircleF4E4,
        colour: u32,
    ) {
        let min_x = p1.r().min(p2.r()).min(center.r());
        let max_x = p1.r().max(p2.r()).max(center.r());
        let x_start = min_x.to_isize().clamp(0, self.coords.width as isize);
        let x_end   = max_x.to_isize().clamp(0, self.coords.width as isize);

        for x_px in x_start..=x_end {
            let x = ScalarF4E4::from(x_px);
            let mut ys = Vec::new();
            if let Some(y) = Self::line_intersect_x(center, p1, x) { ys.push(y); }
            if let Some(y) = Self::line_intersect_x(center, p2, x) { ys.push(y); }
            if let Some(y) = Self::line_intersect_x(p1, p2, x)     { ys.push(y); }

            if ys.len() >= 2 {
                ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let y_top    = ys[0];
                let y_bottom = *ys.last().unwrap();

                // Interior — direct write
                for y in (y_top + ScalarF4E4::ONE).to_isize()..y_bottom.to_isize() {
                    if y >= 0 && (y as usize) < self.coords.height {
                        let idx = (y as usize) * self.coords.width + x_px as usize;
                        if idx < self.pixels.len() { self.pixels[idx] = colour; }
                    }
                }

                // AA edge pixels — coverage blend
                let yt_px = y_top.to_isize();
                if yt_px >= 0 && (yt_px as usize) < self.coords.height {
                    let weight = ((ScalarF4E4::ONE - (y_top - ScalarF4E4::from(yt_px))) * ScalarF4E4::from(255)).to_isize().clamp(0, 255) as u8;
                    self.blend_pixel(x_px, yt_px, colour, weight);
                }
                let yb_px = y_bottom.to_isize();
                if yb_px >= 0 && (yb_px as usize) < self.coords.height {
                    let weight = ((y_bottom - ScalarF4E4::from(yb_px)) * ScalarF4E4::from(255)).to_isize().clamp(0, 255) as u8;
                    self.blend_pixel(x_px, yb_px, colour, weight);
                }
            }
        }
    }

    /// X intersection of line segment with horizontal scanline at Y
    pub(crate) fn line_intersect_y(p1: CircleF4E4, p2: CircleF4E4, y: ScalarF4E4) -> Option<ScalarF4E4> {
        let (x1, y1, x2, y2) = (p1.r(), p1.i(), p2.r(), p2.i());
        if (y1 <= y && y < y2) || (y2 <= y && y < y1) {
            let dy = y2 - y1;
            if dy != ScalarF4E4::ZERO {
                return Some(x1 + (y - y1) / dy * (x2 - x1));
            }
        }
        None
    }

    /// Y intersection of line segment with vertical scanline at X
    pub(crate) fn line_intersect_x(p1: CircleF4E4, p2: CircleF4E4, x: ScalarF4E4) -> Option<ScalarF4E4> {
        let (x1, y1, x2, y2) = (p1.r(), p1.i(), p2.r(), p2.i());
        if (x1 <= x && x < x2) || (x2 <= x && x < x1) {
            let dx = x2 - x1;
            if dx != ScalarF4E4::ZERO {
                return Some(y1 + (x - x1) / dx * (y2 - y1));
            }
        }
        None
    }
}
