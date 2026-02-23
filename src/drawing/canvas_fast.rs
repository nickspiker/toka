#![allow(missing_docs)]
//! Fast canvas: packed u32 sRGB pixel buffer
//!
//! Colours are pre-converted to sRGB u32 at scene graph build time.
//! Primary blend extracts channels individually and preserves bg alpha.
//! AA edge blend uses SIMD-in-register u64 trick — 4 channels in one multiply.
//! Output is manual byte extraction to browser ImageData [R, G, B, A].
//!
//! Pixel format: R<<24 | G<<16 | B<<8 | A

use crate::drawing::shared::RuCoords;
use spirix::{CircleF4E4, ScalarF4E4};

/// Opaque black in packed u32 sRGB (R=0, G=0, B=0, A=255)
pub const BLACK_U32: u32 = 0x000000FF;

/// Fast canvas with pre-gamma-encoded u32 pixel buffer
pub struct CanvasFast {
    pub(crate) coords: RuCoords,
    pub(crate) pixels: Vec<u32>,
}

impl CanvasFast {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            coords: RuCoords::new(width, height),
            pixels: vec![BLACK_U32; width * height],
        }
    }

    pub fn span(&self) -> ScalarF4E4 { self.coords.span() }
    pub fn ru(&self) -> ScalarF4E4 { self.coords.ru() }
    pub fn width(&self) -> usize { self.coords.width() }
    pub fn height(&self) -> usize { self.coords.height() }
    pub fn dimensions(&self) -> (usize, usize) { (self.coords.width(), self.coords.height()) }
    pub fn half_dims(&self) -> CircleF4E4 { self.coords.half_dims() }
    pub fn set_ru(&mut self, ru: ScalarF4E4) { self.coords.set_ru(ru); }
    pub fn adjust_zoom(&mut self, steps: ScalarF4E4) { self.coords.adjust_zoom(steps); }
    pub fn pixels(&self) -> &[u32] { &self.pixels }

    #[inline] pub(crate) fn ru_to_px_x(&self, x: ScalarF4E4) -> isize { self.coords.ru_to_px_x(x) }
    #[inline] pub(crate) fn ru_to_px_y(&self, y: ScalarF4E4) -> isize { self.coords.ru_to_px_y(y) }
    #[inline] pub(crate) fn ru_to_px_w(&self, w: ScalarF4E4) -> isize { self.coords.ru_to_px_w(w) }
    #[inline] pub(crate) fn ru_to_px_h(&self, h: ScalarF4E4) -> isize { self.coords.ru_to_px_h(h) }

    /// Clear canvas to a VSF colour (pre-converts to sRGB u32)
    pub fn clear(&mut self, colour: &vsf::VsfType) -> Result<(), String> {
        let u32_colour = crate::renderer::extract_colour_u32(colour)?;
        self.pixels.fill(u32_colour);
        Ok(())
    }

    /// Pixel buffer as RGBA bytes for browser ImageData
    ///
    /// Extracts R<<24|G<<16|B<<8|A → [R, G, B, A] bytes.
    /// Alpha is forced to 0xFF — the canvas is an opaque output surface.
    pub fn to_rgba_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.pixels.len() * 4);
        for &pixel in &self.pixels {
            bytes.push((pixel >> 24) as u8); // R
            bytes.push((pixel >> 16) as u8); // G
            bytes.push((pixel >> 8)  as u8); // B
            bytes.push(0xFF);                // A — always opaque
        }
        bytes
    }

    /// Blend fg over bg using fg alpha (low byte)
    ///
    /// Output alpha is taken from bg (stays 255 for opaque canvas output).
    #[inline]
    pub(crate) fn blend(fg: u32, bg: u32) -> u32 {
        let alpha     = (fg & 0xFF) as u64;
        let inv_alpha = 255 - alpha;

        let fg_r = (fg >> 24) as u64;
        let fg_g = ((fg >> 16) & 0xFF) as u64;
        let fg_b = ((fg >> 8)  & 0xFF) as u64;

        let bg_r = (bg >> 24) as u64;
        let bg_g = ((bg >> 16) & 0xFF) as u64;
        let bg_b = ((bg >> 8)  & 0xFF) as u64;

        let r = ((bg_r * inv_alpha + fg_r * alpha) >> 8) as u32;
        let g = ((bg_g * inv_alpha + fg_g * alpha) >> 8) as u32;
        let b = ((bg_b * inv_alpha + fg_b * alpha) >> 8) as u32;

        // Preserve bg alpha (opaque canvas output to browser)
        let out_a = bg & 0xFF;
        (r << 24) | (g << 16) | (b << 8) | out_a
    }

    /// Blend fg over bg with explicit coverage weight (0-255), SIMD-in-register
    ///
    /// Used for AA edge pixels. All 4 channels blended; bg alpha preserved via input.
    #[inline]
    pub(crate) fn blend_weighted(fg: u32, bg: u32, weight_fg: u8) -> u32 {
        let weight_bg = 255 - weight_fg as u64;
        let weight_fg = weight_fg as u64;

        let mut b = bg as u64;
        b = (b | (b << 16)) & 0x0000FFFF0000FFFF;
        b = (b | (b << 8))  & 0x00FF00FF00FF00FF;

        let mut f = fg as u64;
        f = (f | (f << 16)) & 0x0000FFFF0000FFFF;
        f = (f | (f << 8))  & 0x00FF00FF00FF00FF;

        let mut out = b * weight_bg + f * weight_fg;
        out = (out >> 8)         & 0x00FF00FF00FF00FF;
        out = (out | (out >> 8)) & 0x0000FFFF0000FFFF;
        out = out | (out >> 16);

        out as u32
    }

    /// Blend a single pixel at canvas coordinates with AA coverage weight
    #[inline]
    pub(crate) fn blend_pixel(&mut self, x: isize, y: isize, fg: u32, weight: u8) {
        if x >= 0 && (x as usize) < self.coords.width && y >= 0 && (y as usize) < self.coords.height {
            let idx = (y as usize) * self.coords.width + (x as usize);
            if idx < self.pixels.len() {
                self.pixels[idx] = Self::blend_weighted(fg, self.pixels[idx], weight);
            }
        }
    }

    /// Set a single pixel (centered pixel coordinates), no blending
    pub fn set_pixel_px(&mut self, x: isize, y: isize, colour: u32) {
        let px = self.coords.half_dims.r().to_isize() + x;
        let py = self.coords.half_dims.i().to_isize() + y;
        if (px as usize) < self.coords.width && (py as usize) < self.coords.height {
            self.pixels[(py as usize) * self.coords.width + (px as usize)] = colour;
        }
    }

    /// Set a single pixel (RU coordinates), no blending
    pub fn set_pixel_ru(&mut self, pos: CircleF4E4, colour: u32) {
        let x = self.ru_to_px_x(pos.r());
        let y = self.ru_to_px_y(pos.i());
        self.set_pixel_px(x, y, colour);
    }
}
