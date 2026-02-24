#![allow(missing_docs)]
//! Rectangle rasterization for CanvasQuality (linear S44 RGBA)

use crate::drawing::canvas_quality::{CanvasQuality, Pixel};
use spirix::{CircleF4E4, ScalarF4E4};

impl CanvasQuality {
    /// Fill an axis-aligned rectangle (RU coordinates, center-origin)
    pub fn fill_rect_ru(&mut self, pos: CircleF4E4, size: CircleF4E4, colour: Pixel) {
        let cx = self.ru_to_px_x(pos.r());
        let cy = self.ru_to_px_y(pos.i());
        let pw = self.ru_to_px_w(size.r());
        let ph = self.ru_to_px_h(size.i());

        let center_x = (self.width() >> 1) as isize;
        let center_y = (self.height() >> 1) as isize;
        let px = center_x + cx - pw >> 1;
        let py = center_y + cy - ph >> 1;

        let x1 = px.clamp(0, self.width() as isize) as usize;
        let y1 = py.clamp(0, self.height() as isize) as usize;
        let x2 = (px + pw).clamp(0, self.width() as isize) as usize;
        let y2 = (py + ph).clamp(0, self.height() as isize) as usize;

        for row in y1..y2 {
            for col in x1..x2 {
                let idx = row * self.width() + col;
                let dst = self.pixels()[idx];
                self.pixels_mut()[idx] = Self::blend(colour, dst);
            }
        }
    }

    /// Fill a rotated rectangle (RU coordinates, center-origin)
    pub fn fill_rotated_rect_ru(
        &mut self,
        pos: CircleF4E4,
        size: CircleF4E4,
        angle: ScalarF4E4,
        colour: Pixel,
    ) {
        let center = self.half_dims() + pos * self.span() * self.ru();
        let half: CircleF4E4 = (size * self.span() * self.ru()) >> 1;

        self.fill_rect_aa(center, half, angle, colour);
    }

    /// Fill a rectangle using signed distance field — handles all rotations and aspect ratios.
    ///
    /// Iterates the AABB of the rotated rect. For each pixel, transforms into rect-local
    /// space and computes SDF. Coverage scales the src alpha for sub-pixel AA on all edges.
    fn fill_rect_aa(
        &mut self,
        center: CircleF4E4,
        half: CircleF4E4,
        angle: ScalarF4E4,
        colour: Pixel,
    ) {
        let cos = angle.cos();
        let sin = angle.sin();

        let hw = half.r();
        let hh = half.i();

        // AABB of the rotated rect — clamped to canvas
        let aabb_half_w = (hw * cos.magnitude() + hh * sin.magnitude()).ceil();
        let aabb_half_h = (hw * sin.magnitude() + hh * cos.magnitude()).ceil();

        let cx = center.r();
        let cy = center.i();

        let x0 = (cx - aabb_half_w).to_isize().clamp(0, self.width() as isize);
        let x1 = (cx + aabb_half_w).to_isize().clamp(0, self.width() as isize);
        let y0 = (cy - aabb_half_h).to_isize().clamp(0, self.height() as isize);
        let y1 = (cy + aabb_half_h).to_isize().clamp(0, self.height() as isize);

        for py in y0..y1 {
            for px in x0..x1 {
                // Translate to rect-center-relative
                let dx = ScalarF4E4::from(px) - cx;
                let dy = ScalarF4E4::from(py) - cy;

                // Rotate into rect-local space (inverse rotation = transpose)
                let lx = dx * cos + dy * sin;
                let ly = dy * cos - dx * sin;

                // SDF: distance inside the rect boundary
                let sdf = -(lx.magnitude() - hw).max(ly.magnitude() - hh);
                if sdf.is_negative() { continue; }

                let idx = (py as usize) * self.width() + (px as usize);
                let dst = self.pixels()[idx];
                let coverage = sdf.min(ScalarF4E4::ONE);
                self.pixels_mut()[idx] = Self::blend_weighted(colour, dst, coverage);
            }
        }
    }
}
