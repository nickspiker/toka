#![allow(missing_docs)]
//! Rectangle rasterization for CanvasFast (u32 sRGB)

use crate::drawing::canvas_fast::CanvasFast;
use spirix::{CircleF4E4, ScalarF4E4};

impl CanvasFast {
    /// Fill an axis-aligned rectangle with sub-pixel AA on all edges (RU coordinates, center-origin)
    pub fn fill_rect_ru(&mut self, pos: CircleF4E4, size: CircleF4E4, colour: u32) {
        let center = self.coords.half_dims + pos * self.coords.span * self.coords.ru;
        let half: CircleF4E4 = (size * self.coords.span * self.coords.ru) >> 1;

        let hw = half.r();
        let hh = half.i();

        let cx = center.r();
        let cy = center.i();

        let x0 = (cx - hw).to_isize().clamp(0, self.coords.width as isize);
        let x1 = (cx + hw).to_isize().clamp(0, self.coords.width as isize);
        let y0 = (cy - hh).to_isize().clamp(0, self.coords.height as isize);
        let y1 = (cy + hh).to_isize().clamp(0, self.coords.height as isize);

        let base_alpha = colour as u8;
        let width = self.coords.width;

        for py in y0..y1 {
            let dy = py - cy;
            // Row span: find the AA columns at each end, fill middle with memset
            let sdf_y = hh - dy.magnitude();

            for px in x0..x1 {
                let dx = px - cx;
                let sdf = sdf_y.min(hw - dx.magnitude());

                if sdf.is_negative() {
                    continue;
                }

                let idx = (py as usize) * width + (px as usize);

                if sdf.exponent > 0 {
                    if base_alpha == 255 {
                        self.pixels[idx] = colour;
                    } else {
                        self.pixels[idx] = Self::blend(colour, self.pixels[idx], base_alpha);
                    }
                } else {
                    let coverage = (sdf << 8usize).to_u8();
                    let fg_a = (((base_alpha as u16) * coverage as u16) >> 8) as u32;
                    let fg = (colour & 0xFFFFFF00) | fg_a;
                    self.pixels[idx] = Self::blend(fg, self.pixels[idx], fg_a as u8);
                }
            }
        }
    }

    /// Fill a rotated rectangle with sub-pixel AA on all edges (RU coordinates, center-origin)
    pub fn fill_rotated_rect_ru(
        &mut self,
        pos: CircleF4E4,
        size: CircleF4E4,
        angle: ScalarF4E4,
        colour: u32,
    ) {
        let center = self.coords.half_dims + pos * self.coords.span * self.coords.ru;
        let half: CircleF4E4 = (size * self.coords.span * self.coords.ru) >> 1;

        let cos = angle.cos();
        let sin = angle.sin();

        let hw = half.r();
        let hh = half.i();

        // AABB of the rotated rect — clamped to canvas
        let aabb_half_w = (hw * cos.magnitude() + hh * sin.magnitude()).ceil();
        let aabb_half_h = (hw * sin.magnitude() + hh * cos.magnitude()).ceil();

        let cx = center.r();
        let cy = center.i();

        let x0 = (cx - aabb_half_w)
            .to_isize()
            .clamp(0, self.coords.width as isize);
        let x1 = (cx + aabb_half_w)
            .to_isize()
            .clamp(0, self.coords.width as isize);
        let y0 = (cy - aabb_half_h)
            .to_isize()
            .clamp(0, self.coords.height as isize);
        let y1 = (cy + aabb_half_h)
            .to_isize()
            .clamp(0, self.coords.height as isize);

        let base_alpha = colour as u8;
        let width = self.coords.width;

        for py in y0..y1 {
            // Hoist row-invariant rotation terms
            let dy = py - cy;
            let dy_cos = dy * cos;
            let dy_sin = dy * sin;

            for px in x0..x1 {
                let dx = px - cx;

                // Rotate into rect-local space (inverse rotation = transpose)
                let lx = dx * cos + dy_sin;
                let ly = dy_cos - dx * sin;

                // SDF: distance inside the rect boundary (positive = inside)
                let sdf = -(lx.magnitude() - hw).max(ly.magnitude() - hh);

                if sdf.is_negative() {
                    continue;
                }

                let idx = (py as usize) * width + (px as usize);

                // exponent > 0 means sdf > 1: fully inside, no AA needed
                if sdf.exponent > 0 {
                    if base_alpha == 255 {
                        self.pixels[idx] = colour;
                    } else {
                        self.pixels[idx] = Self::blend(colour, self.pixels[idx], base_alpha);
                    }
                } else {
                    // AA edge: scale alpha by sub-pixel coverage
                    let coverage = (sdf << 8usize).to_u8();
                    let fg_a = (((base_alpha as u16) * coverage as u16) >> 8) as u32;
                    let fg = (colour & 0xFFFFFF00) | fg_a;
                    self.pixels[idx] = Self::blend(fg, self.pixels[idx], fg_a as u8);
                }
            }
        }
    }
}
