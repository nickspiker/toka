//! Pixel operations and alpha blending
//!
//! All blending in linear S44 light — no gamma-space math.
//! Porter-Duff "src over dst" compositing.

use crate::drawing::canvas_quality::{CanvasQuality, Pixel};
use spirix::{CircleF4E4, ScalarF4E4};

impl CanvasQuality {
    /// Set a single pixel (centered pixel coordinates), no blending
    pub fn set_pixel_px(&mut self, x: isize, y: isize, colour: Pixel) {
        let center_x = (self.width() >> 1) as isize;
        let center_y = (self.height() >> 1) as isize;
        let px = center_x + x;
        let py = center_y + y;
        if (px as usize) < self.width() && (py as usize) < self.height() {
            let idx = (py as usize) * self.width() + (px as usize);
            self.pixels_mut()[idx] = colour;
        }
    }

    /// Set a single pixel (RU coordinates), no blending
    pub fn set_pixel_ru(&mut self, pos: CircleF4E4, colour: Pixel) {
        let x = self.ru_to_px_x(pos.r());
        let y = self.ru_to_px_y(pos.i());
        self.set_pixel_px(x, y, colour);
    }

    /// Blend src over dst using src alpha (Porter-Duff src-over, linear light)
    ///
    /// out_rgb = src_a * src_rgb + (1 - src_a) * dst_rgb
    /// out_a   = src_a + (1 - src_a) * dst_a
    #[inline]
    pub(crate) fn blend(src: Pixel, dst: Pixel) -> Pixel {
        let src_a = src[3];
        let inv_a = ScalarF4E4::ONE - src_a;
        [
            src_a * src[0] + inv_a * dst[0],
            src_a * src[1] + inv_a * dst[1],
            src_a * src[2] + inv_a * dst[2],
            src_a + inv_a * dst[3],
        ]
    }

    /// Blend src over dst at canvas position, scaling alpha by AA coverage weight
    pub(crate) fn blend_pixel(&mut self, x: isize, y: isize, src: Pixel, weight: ScalarF4E4) {
        if x >= 0 && (x as usize) < self.width() && y >= 0 && (y as usize) < self.height() {
            let idx = (y as usize) * self.width() + (x as usize);
            if idx < self.pixels().len() {
                let mut weighted_src = src;
                weighted_src[3] = src[3] * weight;
                let dst = self.pixels()[idx];
                self.pixels_mut()[idx] = Self::blend(weighted_src, dst);
            }
        }
    }
}
