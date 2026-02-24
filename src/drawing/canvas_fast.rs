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

    pub fn span(&self) -> ScalarF4E4 {
        self.coords.span()
    }
    pub fn ru(&self) -> ScalarF4E4 {
        self.coords.ru()
    }
    pub fn width(&self) -> usize {
        self.coords.width()
    }
    pub fn height(&self) -> usize {
        self.coords.height()
    }
    pub fn dimensions(&self) -> (usize, usize) {
        (self.coords.width(), self.coords.height())
    }
    pub fn half_dims(&self) -> CircleF4E4 {
        self.coords.half_dims()
    }
    pub fn set_ru(&mut self, ru: ScalarF4E4) {
        self.coords.set_ru(ru);
    }
    pub fn adjust_zoom(&mut self, steps: ScalarF4E4) {
        self.coords.adjust_zoom(steps);
    }
    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    #[inline]
    pub(crate) fn ru_to_px_x(&self, x: ScalarF4E4) -> isize {
        self.coords.ru_to_px_x(x)
    }
    #[inline]
    pub(crate) fn ru_to_px_y(&self, y: ScalarF4E4) -> isize {
        self.coords.ru_to_px_y(y)
    }
    #[inline]
    pub(crate) fn ru_to_px_w(&self, w: ScalarF4E4) -> isize {
        self.coords.ru_to_px_w(w)
    }
    #[inline]
    pub(crate) fn ru_to_px_h(&self, h: ScalarF4E4) -> isize {
        self.coords.ru_to_px_h(h)
    }

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
            bytes.push((pixel >> 8) as u8); // B
            bytes.push(0xFF); // A — always opaque
        }
        bytes
    }

    /// Overlay fg over bg using provided inverse alpha
    #[inline]
    pub(crate) fn overlay(fg: u32, bg: u32, inv_alpha: u8) -> u32 {
        let mut b = bg as u64;
        b = (b | (b << 16)) & 0x0000FFFF0000FFFF;
        b = (b | (b << 8)) & 0x00FF00FF00FF00FF;

        let mut f = fg as u64;
        f = (f | (f << 16)) & 0x0000FFFF0000FFFF;
        f = (f | (f << 8)) & 0x00FF00FF00FF00FF;

        let mut out = b * inv_alpha as u64 + f;
        out = (out >> 8) & 0x00FF00FF00FF00FF;
        out = (out | (out >> 8)) & 0x0000FFFF0000FFFF;
        out = out | (out >> 16) | 0xFF;

        out as u32
    }

    /// Blend fg over bg using provided inverse alpha
    #[inline]
    pub(crate) fn blend(fg: u32, bg: u32, alpha: u8) -> u32 {
        let mut b = bg as u64;
        b = (b | (b << 16)) & 0x0000FFFF0000FFFF;
        b = (b | (b << 8)) & 0x00FF00FF00FF00FF;

        let mut f = fg as u64;
        f = (f | (f << 16)) & 0x0000FFFF0000FFFF;
        f = (f | (f << 8)) & 0x00FF00FF00FF00FF;

        let mut out = b * (255 - alpha) as u64 + f * alpha as u64;
        out = (out >> 8) & 0x00FF00FF00FF00FF;
        out = (out | (out >> 8)) & 0x0000FFFF0000FFFF;
        out = out | (out >> 16) | 0xFF;

        out as u32
    }
}
