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

    /// Draw a 1px horizontal line (no AA) — RU coordinates, center-origin
    pub fn hline_ru(&mut self, y: ScalarF4E4, x0: ScalarF4E4, x1: ScalarF4E4, colour: u32) {
        let span_ru = self.coords.span * self.coords.ru;
        let half_w = self.coords.half_dims.r();
        let half_h = self.coords.half_dims.i();
        let py = (half_h + y * span_ru).to_isize();
        if py < 0 || py >= self.coords.height as isize { return; }
        let px0 = (half_w + x0 * span_ru).to_isize().clamp(0, self.coords.width as isize) as usize;
        let px1 = (half_w + x1 * span_ru).to_isize().clamp(0, self.coords.width as isize) as usize;
        let row = py as usize * self.coords.width;
        let alpha = colour as u8;
        if alpha == 255 {
            self.pixels[row + px0..row + px1].fill(colour);
        } else {
            for px in px0..px1 {
                self.pixels[row + px] = Self::blend(colour, self.pixels[row + px], alpha);
            }
        }
    }

    /// Draw a 1px vertical line (no AA) — RU coordinates, center-origin
    pub fn vline_ru(&mut self, x: ScalarF4E4, y0: ScalarF4E4, y1: ScalarF4E4, colour: u32) {
        let span_ru = self.coords.span * self.coords.ru;
        let half_w = self.coords.half_dims.r();
        let half_h = self.coords.half_dims.i();
        let px = (half_w + x * span_ru).to_isize();
        if px < 0 || px >= self.coords.width as isize { return; }
        let py0 = (half_h + y0 * span_ru).to_isize().clamp(0, self.coords.height as isize) as usize;
        let py1 = (half_h + y1 * span_ru).to_isize().clamp(0, self.coords.height as isize) as usize;
        let px = px as usize;
        let w = self.coords.width;
        let alpha = colour as u8;
        if alpha == 255 {
            for py in py0..py1 {
                self.pixels[py * w + px] = colour;
            }
        } else {
            for py in py0..py1 {
                let idx = py * w + px;
                self.pixels[idx] = Self::blend(colour, self.pixels[idx], alpha);
            }
        }
    }

    /// Draw a 1px axis-aligned rectangle outline (no AA) — RU coordinates, center-origin
    pub fn stroke_rect_ru(&mut self, pos: CircleF4E4, size: CircleF4E4, colour: u32) {
        let half_w = size.r() >> 1usize;
        let half_h = size.i() >> 1usize;
        let left = pos.r() - half_w;
        let right = pos.r() + half_w;
        let top = pos.i() - half_h;
        let bottom = pos.i() + half_h;
        self.hline_ru(top, left, right, colour);     // top
        self.hline_ru(bottom, left, right, colour);   // bottom
        self.vline_ru(left, top, bottom, colour);     // left
        self.vline_ru(right, top, bottom, colour);    // right
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
