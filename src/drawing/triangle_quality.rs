//! Triangle rasterization with anti-aliasing
//!
//! Used internally for rotated rectangle decomposition.
//! Future user-facing AA polygon primitive.

use crate::drawing::canvas_quality::{CanvasQuality, Pixel};
use spirix::{CircleF4E4, ScalarF4E4};

impl CanvasQuality {
    /// Fill a triangle with AA on the outer edge (p1 → p2)
    pub(crate) fn fill_triangle_aa(
        &mut self,
        center: CircleF4E4,
        p1: CircleF4E4,
        p2: CircleF4E4,
        colour: Pixel,
    ) {
        let diff = p2 - p1;
        let dx = diff.r().magnitude();
        let dy = diff.i().magnitude();
        if dx > dy {
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
        colour: Pixel,
    ) {
        let min_y = p1.i().min(p2.i()).min(center.i());
        let max_y = p1.i().max(p2.i()).max(center.i());

        let y_start = min_y.clamp(0, self.height());
        let y_end = max_y.clamp(0, self.height());

        for y_px in y_start.to_usize()..=y_end.to_usize() {
            let y = ScalarF4E4::from(y_px);
            let mut intersections = Vec::new();

            if let Some(x) = Self::line_intersect_y(center, p1, y) { intersections.push(x); }
            if let Some(x) = Self::line_intersect_y(center, p2, y) { intersections.push(x); }
            if let Some(x) = Self::line_intersect_y(p1, p2, y)     { intersections.push(x); }

            if intersections.len() >= 2 {
                intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let x_left  = intersections[0];
                let x_right = *intersections.last().unwrap();

                for x in (x_left + ScalarF4E4::ONE).to_isize()..x_right.to_isize() {
                    if x >= 0 && (x as usize) < self.width() {
                        let idx = y_px * self.width() + (x as usize);
                        if idx < self.pixels().len() {
                            self.pixels_mut()[idx] = colour;
                        }
                    }
                }

                let x_left_px = x_left.to_isize();
                if x_left_px >= 0 && (x_left_px as usize) < self.width() {
                    let coverage = ScalarF4E4::ONE - (x_left - ScalarF4E4::from(x_left_px));
                    self.blend_pixel(x_left_px, y_px as isize, colour, coverage);
                }

                let x_right_px = x_right.to_isize();
                if x_right_px >= 0 && (x_right_px as usize) < self.width() {
                    let coverage = x_right - ScalarF4E4::from(x_right_px);
                    self.blend_pixel(x_right_px, y_px as isize, colour, coverage);
                }
            }
        }
    }

    pub(crate) fn fill_triangle_vertical(
        &mut self,
        center: CircleF4E4,
        p1: CircleF4E4,
        p2: CircleF4E4,
        colour: Pixel,
    ) {
        let min_x = p1.r().min(p2.r()).min(center.r());
        let max_x = p1.r().max(p2.r()).max(center.r());

        let x_start = min_x.to_isize().clamp(0, self.width() as isize);
        let x_end   = max_x.to_isize().clamp(0, self.width() as isize);

        for x_px in x_start..=x_end {
            let x = ScalarF4E4::from(x_px);
            let mut intersections = Vec::new();

            if let Some(y) = Self::line_intersect_x(center, p1, x) { intersections.push(y); }
            if let Some(y) = Self::line_intersect_x(center, p2, x) { intersections.push(y); }
            if let Some(y) = Self::line_intersect_x(p1, p2, x)     { intersections.push(y); }

            if intersections.len() >= 2 {
                intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let y_top    = intersections[0];
                let y_bottom = *intersections.last().unwrap();

                for y in (y_top + ScalarF4E4::ONE).to_isize()..y_bottom.to_isize() {
                    if y >= 0 && (y as usize) < self.height() {
                        let idx = (y as usize) * self.width() + (x_px as usize);
                        if idx < self.pixels().len() {
                            self.pixels_mut()[idx] = colour;
                        }
                    }
                }

                let y_top_px = y_top.to_isize();
                if y_top_px >= 0 && (y_top_px as usize) < self.height() {
                    let coverage = ScalarF4E4::ONE - (y_top - ScalarF4E4::from(y_top_px));
                    self.blend_pixel(x_px, y_top_px, colour, coverage);
                }

                let y_bottom_px = y_bottom.to_isize();
                if y_bottom_px >= 0 && (y_bottom_px as usize) < self.height() {
                    let coverage = y_bottom - ScalarF4E4::from(y_bottom_px);
                    self.blend_pixel(x_px, y_bottom_px, colour, coverage);
                }
            }
        }
    }

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
