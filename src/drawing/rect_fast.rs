#![allow(missing_docs)]
//! Rectangle rasterization for CanvasFast (u32 sRGB)

use crate::drawing::canvas_fast::CanvasFast;
use spirix::{CircleF4E4, ScalarF4E4};

impl CanvasFast {
    /// Fill an axis-aligned rectangle (RU coordinates, center-origin)
    pub fn fill_rect_ru(&mut self, pos: CircleF4E4, size: CircleF4E4, colour: u32) {
        let cx = self.ru_to_px_x(pos.r());
        let cy = self.ru_to_px_y(pos.i());
        let pw = self.ru_to_px_w(size.r());
        let ph = self.ru_to_px_h(size.i());

        let center_x = (self.coords.width >> 1) as isize;
        let center_y = (self.coords.height >> 1) as isize;
        let px = center_x + cx - pw >> 1;
        let py = center_y + cy - ph >> 1;

        let x1 = px.clamp(0, self.coords.width as isize) as usize;
        let y1 = py.clamp(0, self.coords.height as isize) as usize;
        let x2 = (px + pw).clamp(0, self.coords.width as isize) as usize;
        let y2 = (py + ph).clamp(0, self.coords.height as isize) as usize;

        for row in y1..y2 {
            for col in x1..x2 {
                let idx = row * self.coords.width + col;
                self.pixels[idx] = Self::blend(colour, self.pixels[idx]);
            }
        }
    }

    /// Fill a rotated rectangle (RU coordinates, center-origin)
    pub fn fill_rotated_rect_ru(
        &mut self,
        pos: CircleF4E4,
        size: CircleF4E4,
        angle: ScalarF4E4,
        colour: u32,
    ) {
        let center = self.coords.half_dims + pos * self.coords.span * self.coords.ru;
        let scale: CircleF4E4 = (size * self.coords.span * self.coords.ru) >> 1;

        if angle.magnitude().is_zero() {
            self.fill_rotated_rect_axis_aligned(center, scale, colour);
        } else {
            self.fill_rotated_rect_decomposed(center, scale, angle, colour);
        }
    }

    fn fill_rotated_rect_axis_aligned(
        &mut self,
        center: CircleF4E4,
        half_extents: CircleF4E4,
        colour: u32,
    ) {
        let x1 = (center.r() - half_extents.r()).to_isize().clamp(0, self.coords.width as isize) as usize;
        let x2 = (center.r() + half_extents.r()).to_isize().clamp(0, self.coords.width as isize) as usize;
        let y1 = (center.i() - half_extents.i()).to_isize().clamp(0, self.coords.height as isize) as usize;
        let y2 = (center.i() + half_extents.i()).to_isize().clamp(0, self.coords.height as isize) as usize;

        for y in y1..y2 {
            for x in x1..x2 {
                let idx = y * self.coords.width + x;
                if idx < self.pixels.len() {
                    self.pixels[idx] = Self::blend(colour, self.pixels[idx]);
                }
            }
        }
    }

    fn fill_rotated_rect_decomposed(
        &mut self,
        center: CircleF4E4,
        half_extents: CircleF4E4,
        angle: ScalarF4E4,
        colour: u32,
    ) {
        let rot = CircleF4E4::from((angle.cos(), angle.sin()));

        let c0 = center + CircleF4E4::from((-half_extents.r(), -half_extents.i())) * rot;
        let c1 = center + CircleF4E4::from((half_extents.r(), -half_extents.i())) * rot;
        let c2 = center + CircleF4E4::from((half_extents.r(), half_extents.i())) * rot;
        let c3 = center + CircleF4E4::from((-half_extents.r(), half_extents.i())) * rot;

        let mut corners_with_angles = [
            (c0, (c0 - center).i().atan2((c0 - center).r())),
            (c1, (c1 - center).i().atan2((c1 - center).r())),
            (c2, (c2 - center).i().atan2((c2 - center).r())),
            (c3, (c3 - center).i().atan2((c3 - center).r())),
        ];
        corners_with_angles.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());

        let right  = corners_with_angles[0].0;
        let top    = corners_with_angles[1].0;
        let left   = corners_with_angles[2].0;
        let bottom = corners_with_angles[3].0;

        let mut x_sorted = [c0.r(), c1.r(), c2.r(), c3.r()];
        x_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut y_sorted = [c0.i(), c1.i(), c2.i(), c3.i()];
        y_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let tl = CircleF4E4::from((x_sorted[1], y_sorted[2]));
        let br = CircleF4E4::from((x_sorted[2], y_sorted[1]));

        self.fill_rect_axis_aligned_abs(tl, br, colour);
        self.scan_left(right, top, br.r(), colour);
        self.scan_down(top, left, tl.i(), colour);
        self.scan_right(left, bottom, tl.r(), colour);
        self.scan_up(bottom, right, br.i(), colour);
    }

    fn fill_rect_axis_aligned_abs(&mut self, top_left: CircleF4E4, bottom_right: CircleF4E4, colour: u32) {
        let x_start = top_left.r().to_isize().max(0);
        let x_end   = bottom_right.r().to_isize().min(self.coords.width as isize);
        let y_start = top_left.i().to_isize().max(0);
        let y_end   = bottom_right.i().to_isize().min(self.coords.height as isize);

        for y in y_start..=y_end {
            for x in x_start..=x_end {
                if (x as usize) < self.coords.width && (y as usize) < self.coords.height {
                    let idx = (y as usize) * self.coords.width + (x as usize);
                    self.pixels[idx] = Self::blend(colour, self.pixels[idx]);
                }
            }
        }
    }

    fn scan_up(&mut self, p0: CircleF4E4, p1: CircleF4E4, limit_y: ScalarF4E4, colour: u32) {
        let x_start = p0.r().min(p1.r()).to_isize().max(0);
        let x_end   = p0.r().max(p1.r()).to_isize().min(self.coords.width as isize);
        for x in x_start..=x_end {
            if let Some(edge_y) = Self::line_intersect_x(p0, p1, ScalarF4E4::from(x)) {
                let y_start = edge_y.to_isize().max(0);
                let y_end   = limit_y.to_isize().min(self.coords.height as isize);
                for y in y_start..=y_end {
                    if (x as usize) < self.coords.width && (y as usize) < self.coords.height {
                        let idx = (y as usize) * self.coords.width + (x as usize);
                        self.pixels[idx] = Self::blend(colour, self.pixels[idx]);
                    }
                }
            }
        }
    }

    fn scan_down(&mut self, p0: CircleF4E4, p1: CircleF4E4, limit_y: ScalarF4E4, colour: u32) {
        let x_start = p0.r().min(p1.r()).to_isize().max(0);
        let x_end   = p0.r().max(p1.r()).to_isize().min(self.coords.width as isize);
        for x in x_start..=x_end {
            if let Some(edge_y) = Self::line_intersect_x(p0, p1, ScalarF4E4::from(x)) {
                let y_start = limit_y.to_isize().max(0);
                let y_end   = edge_y.to_isize().min(self.coords.height as isize);
                for y in y_start..=y_end {
                    if (x as usize) < self.coords.width && (y as usize) < self.coords.height {
                        let idx = (y as usize) * self.coords.width + (x as usize);
                        self.pixels[idx] = Self::blend(colour, self.pixels[idx]);
                    }
                }
            }
        }
    }

    fn scan_left(&mut self, p0: CircleF4E4, p1: CircleF4E4, limit_x: ScalarF4E4, colour: u32) {
        let y_start = p0.i().min(p1.i()).to_isize().max(0);
        let y_end   = p0.i().max(p1.i()).to_isize().min(self.coords.height as isize);
        for y in y_start..=y_end {
            if let Some(edge_x) = Self::line_intersect_y(p0, p1, ScalarF4E4::from(y)) {
                let x_start = limit_x.to_isize().max(0);
                let x_end   = edge_x.to_isize().min(self.coords.width as isize);
                for x in x_start..=x_end {
                    if (x as usize) < self.coords.width && (y as usize) < self.coords.height {
                        let idx = (y as usize) * self.coords.width + (x as usize);
                        self.pixels[idx] = Self::blend(colour, self.pixels[idx]);
                    }
                }
            }
        }
    }

    fn scan_right(&mut self, p0: CircleF4E4, p1: CircleF4E4, limit_x: ScalarF4E4, colour: u32) {
        let y_start = p0.i().min(p1.i()).to_isize().max(0);
        let y_end   = p0.i().max(p1.i()).to_isize().min(self.coords.height as isize);
        for y in y_start..=y_end {
            if let Some(edge_x) = Self::line_intersect_y(p0, p1, ScalarF4E4::from(y)) {
                let x_start = edge_x.to_isize().max(0);
                let x_end   = limit_x.to_isize().min(self.coords.width as isize);
                for x in x_start..=x_end {
                    if (x as usize) < self.coords.width && (y as usize) < self.coords.height {
                        let idx = (y as usize) * self.coords.width + (x as usize);
                        self.pixels[idx] = Self::blend(colour, self.pixels[idx]);
                    }
                }
            }
        }
    }
}
