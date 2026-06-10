#![allow(missing_docs)]
//! Fast canvas: packed u32 sRGB pixel buffer
//!
//! Colours are pre-converted to sRGB u32 at scene graph build time.
//! Primary blend extracts channels individually and preserves bg alpha.
//! AA edge blend uses SIMD-in-register u64 trick — 4 channels in one multiply.
//! Output is manual byte extraction to browser ImageData [R, G, B, A].
//!
//! Pixel format: R<<24 | G<<16 | B<<8 | A

use crate::drawing::shared::{BlendMode, RuCoords};
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
        let w = self.coords.width;
        let y0 = self.coords.clip_y_min;
        let y1 = self.coords.clip_y_max;
        self.pixels[y0 * w .. y1 * w].fill(u32_colour);
        Ok(())
    }

    /// Shift pixel buffer up or down by `delta_y` rows. Exposed strip filled with `bg`.
    /// Positive delta = content moves up (scroll down). Negative = content moves down.
    /// copy_within uses memmove internally — safe for overlapping regions.
    pub fn scroll_pixels(&mut self, delta_y: isize, bg: u32) {
        let h = self.coords.height;
        let w = self.coords.width;
        let d = delta_y.unsigned_abs().min(h);
        if d == 0 { return; }
        if d >= h { self.pixels.fill(bg); return; }

        if delta_y > 0 {
            // Scroll down: copy rows [d..h] → [0..h-d], fill bottom strip
            self.pixels.copy_within(d * w .. h * w, 0);
            self.pixels[((h - d) * w)..].fill(bg);
        } else {
            // Scroll up: copy rows [0..h-d] → [d..h], fill top strip
            self.pixels.copy_within(0 .. (h - d) * w, d * w);
            self.pixels[..d * w].fill(bg);
        }

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

    /// Blend fg over bg using provided alpha
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

    /// Apply a blend mode per-channel then alpha-composite the result.
    ///
    /// Unpacks to i16 intermediates for headroom (Add can exceed 255,
    /// Subtract can go negative). Saturates to 0..255 on repack.
    #[inline]
    pub(crate) fn blend_mode(src: u32, dst: u32, alpha: u8, mode: BlendMode) -> u32 {
        use BlendMode::*;
        if matches!(mode, Normal) {
            return Self::blend(src, dst, alpha);
        }

        let sr = (src >> 24) as i16;
        let sg = ((src >> 16) & 0xFF) as i16;
        let sb = ((src >> 8) & 0xFF) as i16;

        let dr = (dst >> 24) as i16;
        let dg = ((dst >> 16) & 0xFF) as i16;
        let db = ((dst >> 8) & 0xFF) as i16;

        let blend_ch = |s: i16, d: i16| -> i16 {
            match mode {
                Normal => s,
                Multiply => (s * d) >> 8,
                Screen => s + d - ((s * d) >> 8),
                Overlay => {
                    if d < 128 { (s * d) >> 7 }
                    else { 255 - (((255 - s) * (255 - d)) >> 7) }
                }
                Darken => s.min(d),
                Lighten => s.max(d),
                ColorDodge => {
                    if s >= 255 { 255 } else { ((d << 8) / (256 - s)).min(255) }
                }
                ColorBurn => {
                    if s <= 0 { 0 } else { 255 - (((255 - d) << 8) / s).min(255) }
                }
                HardLight => {
                    if s < 128 { (s * d) >> 7 }
                    else { 255 - (((255 - s) * (255 - d)) >> 7) }
                }
                SoftLight => {
                    // Pegtop: (1-2s)·d² + 2s·d  (i32 intermediates to avoid overflow)
                    let s32 = s as i32;
                    let d32 = d as i32;
                    (((255 - 2 * s32) * d32 * d32 / 65025 + 2 * s32 * d32 / 255) as i16).min(255)
                }
                Difference => (s - d).abs(),
                Exclusion => s + d - ((s * d) >> 7),
                Add => s + d,
                Subtract => d - s,
                Divide => {
                    if s <= 0 { 255 } else { ((d << 8) / s).min(255) }
                }
            }
        };

        let br = blend_ch(sr, dr);
        let bg = blend_ch(sg, dg);
        let bb = blend_ch(sb, db);

        // Alpha-composite: out = alpha/255 * blended + (1-alpha/255) * dst
        let a = alpha as i16;
        let ia = 255 - a;
        let mix = |b: i16, d: i16| -> u8 {
            ((b * a + d * ia + 127) / 255).clamp(0, 255) as u8
        };

        let da = (dst & 0xFF) as u8;
        let or = mix(br, dr);
        let og = mix(bg, dg);
        let ob = mix(bb, db);

        ((or as u32) << 24) | ((og as u32) << 16) | ((ob as u32) << 8) | (da as u32)
    }

    /// Create a transparent layer canvas matching this canvas's dimensions and RU state
    pub fn new_layer(width: usize, height: usize, coords: &RuCoords) -> Self {
        let mut layer = Self {
            coords: RuCoords::new(width, height),
            pixels: vec![0u32; width * height], // fully transparent
        };
        layer.coords.set_ru(coords.ru());
        layer.coords.set_scroll_y(coords.scroll_y);
        layer
    }

    /// Composite a source layer onto this canvas with opacity and blend mode.
    ///
    /// Skips fully transparent source pixels (alpha channel == 0).
    pub fn composite_from(&mut self, src: &CanvasFast, opacity: u8, mode: BlendMode) {
        for i in 0..self.pixels.len().min(src.pixels.len()) {
            let s = src.pixels[i];
            if s & 0xFF == 0 { continue; } // transparent source pixel
            let a = ((s & 0xFF) as u16 * opacity as u16 / 255) as u8;
            self.pixels[i] = Self::blend_mode(s, self.pixels[i], a, mode);
        }
    }
}
