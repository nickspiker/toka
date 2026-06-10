//! Drawing primitives for Toka canvas
//!
//! Two pipeline variants:
//! - **Fast** (`CanvasFast`): packed u32 sRGB, colours pre-converted at build time,
//!   blending via SIMD-in-register u64, zero-copy output. Default pipeline.
//! - **Quality** (`CanvasQuality`): linear S44 RGBA, Porter-Duff compositing in
//!   linear light, gamma-2 OETF + error diffusion at output.
//!
//! Shared:
//! - [`shared`] - RU coordinate system (span, zoom, px conversions)
//!
//! Fast pipeline:
//! - [`canvas_fast`] - CanvasFast struct and pixel ops
//! - [`rect_fast`] - Rectangle rasterization (SDF, all rotations)
//! - [`circle_fast`] - Circle rasterization
//! - [`text_fast`] - Text rendering (placeholder)
//!
//! Quality pipeline:
//! - [`canvas_quality`] - CanvasQuality struct and pixel ops
//! - [`rect_quality`] - Rectangle rasterization
//! - [`circle_quality`] - Circle rasterization
//! - [`text_quality`] - Text rendering (placeholder)

pub mod shared;

pub mod canvas_fast;
pub mod rect_fast;
pub mod circle_fast;
pub mod text_fast;
pub mod line_fast;

pub mod canvas_quality;
pub mod pixel_quality;
pub mod rect_quality;
pub mod circle_quality;
pub mod text_quality;
pub mod line_quality;

pub use canvas_fast::CanvasFast;
pub use canvas_quality::{CanvasQuality, Pixel};
pub use shared::{BlendMode, TextSettings, LineSettings, TableSettings};

use crate::vm::FontCache;
use spirix::{CircleF4E4, ScalarF4E4};

/// Runtime-selectable canvas — both pipelines compiled in, toggled at runtime.
pub enum Canvas {
    /// Fast u32 sRGB pipeline — pre-gamma, SIMD-in-register blending
    Fast(CanvasFast),
    /// Quality linear S44 RGBA pipeline — Porter-Duff, gamma-2 OETF at output
    Quality(CanvasQuality),
}

#[allow(missing_docs)]
impl Canvas {
    /// Create a fast (u32 sRGB) canvas
    pub fn new_fast(width: usize, height: usize) -> Self {
        Canvas::Fast(CanvasFast::new(width, height))
    }

    /// Create a quality (linear S44 RGBA) canvas
    pub fn new_quality(width: usize, height: usize) -> Self {
        Canvas::Quality(CanvasQuality::new(width, height))
    }

    /// Pipeline name: "fast" or "quality"
    pub fn pipeline_name(&self) -> &'static str {
        match self {
            Canvas::Fast(_) => "fast",
            Canvas::Quality(_) => "quality",
        }
    }

    pub fn span(&self) -> ScalarF4E4 {
        match self {
            Canvas::Fast(c) => c.span(),
            Canvas::Quality(c) => c.span(),
        }
    }

    pub fn ru(&self) -> ScalarF4E4 {
        match self {
            Canvas::Fast(c) => c.ru(),
            Canvas::Quality(c) => c.ru(),
        }
    }

    pub fn ru_to_px_x(&self, x: ScalarF4E4) -> usize {
        match self {
            Canvas::Fast(c) => c.ru_to_px_x(x).max(0) as usize,
            Canvas::Quality(c) => c.ru_to_px_x(x).max(0) as usize,
        }
    }

    pub fn ru_to_px_y(&self, y: ScalarF4E4) -> usize {
        match self {
            Canvas::Fast(c) => c.ru_to_px_y(y).max(0) as usize,
            Canvas::Quality(c) => c.ru_to_px_y(y).max(0) as usize,
        }
    }

    pub fn set_ru(&mut self, ru: ScalarF4E4) {
        match self {
            Canvas::Fast(c) => c.set_ru(ru),
            Canvas::Quality(c) => c.set_ru(ru),
        }
    }

    pub fn set_scroll_y(&mut self, scroll_y: ScalarF4E4) {
        match self {
            Canvas::Fast(c) => c.coords.set_scroll_y(scroll_y),
            Canvas::Quality(c) => c.set_scroll_y(scroll_y),
        }
    }

    pub fn set_clip_y(&mut self, min: usize, max: usize) {
        match self {
            Canvas::Fast(c) => c.coords.set_clip_y(min, max),
            Canvas::Quality(c) => { c.set_clip_y(min, max); }
        }
    }

    pub fn clear_clip_y(&mut self) {
        match self {
            Canvas::Fast(c) => c.coords.clear_clip_y(),
            Canvas::Quality(c) => { c.clear_clip_y(); }
        }
    }

    pub fn adjust_zoom(&mut self, steps: ScalarF4E4) {
        match self {
            Canvas::Fast(c) => c.adjust_zoom(steps),
            Canvas::Quality(c) => c.adjust_zoom(steps),
        }
    }

    pub fn width(&self) -> usize {
        match self {
            Canvas::Fast(c) => c.width(),
            Canvas::Quality(c) => c.width(),
        }
    }

    pub fn height(&self) -> usize {
        match self {
            Canvas::Fast(c) => c.height(),
            Canvas::Quality(c) => c.height(),
        }
    }

    pub fn dimensions(&self) -> (usize, usize) {
        match self {
            Canvas::Fast(c) => c.dimensions(),
            Canvas::Quality(c) => c.dimensions(),
        }
    }

    pub fn half_dims(&self) -> CircleF4E4 {
        match self {
            Canvas::Fast(c) => c.half_dims(),
            Canvas::Quality(c) => c.half_dims(),
        }
    }

    pub fn clear(&mut self, colour: &vsf::VsfType) -> Result<(), String> {
        match self {
            Canvas::Fast(c) => c.clear(colour),
            Canvas::Quality(c) => c.clear(colour),
        }
    }

    /// Shift pixel buffer by delta_y rows. Exposed strip filled with bg colour.
    pub fn scroll_pixels(&mut self, delta_y: isize, bg: u32) {
        match self {
            Canvas::Fast(c) => c.scroll_pixels(delta_y, bg),
            Canvas::Quality(_c) => {} // TODO: quality pipeline scroll
        }
    }

    /// Convert canvas pixels to RGBA bytes for browser ImageData
    pub fn to_rgba_bytes(&self) -> Vec<u8> {
        match self {
            Canvas::Fast(c) => c.to_rgba_bytes(),
            Canvas::Quality(c) => c.to_rgba_bytes(),
        }
    }

    pub fn fill_rect_ru(&mut self, pos: CircleF4E4, size: CircleF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        match self {
            Canvas::Fast(c) => {
                let u32_colour = crate::renderer::extract_colour_u32(colour)?;
                c.fill_rect_ru(pos, size, u32_colour);
                Ok(())
            }
            Canvas::Quality(c) => {
                let pixel = crate::renderer::extract_colour_linear(colour)?;
                c.fill_rect_ru(pos, size, pixel);
                Ok(())
            }
        }
    }

    /// Draw a 1px horizontal line (no AA — fast path)
    pub fn hline_ru(&mut self, y: ScalarF4E4, x0: ScalarF4E4, x1: ScalarF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        match self {
            Canvas::Fast(c) => {
                let u32_colour = crate::renderer::extract_colour_u32(colour)?;
                c.hline_ru(y, x0, x1, u32_colour);
                Ok(())
            }
            Canvas::Quality(_c) => Ok(()),
        }
    }

    /// Draw a 1px vertical line (no AA — fast path)
    pub fn vline_ru(&mut self, x: ScalarF4E4, y0: ScalarF4E4, y1: ScalarF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        match self {
            Canvas::Fast(c) => {
                let u32_colour = crate::renderer::extract_colour_u32(colour)?;
                c.vline_ru(x, y0, y1, u32_colour);
                Ok(())
            }
            Canvas::Quality(_c) => Ok(()),
        }
    }

    /// Draw a 1px axis-aligned rectangle outline (no AA — fast path for borders)
    pub fn stroke_rect_ru(&mut self, pos: CircleF4E4, size: CircleF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        match self {
            Canvas::Fast(c) => {
                let u32_colour = crate::renderer::extract_colour_u32(colour)?;
                c.stroke_rect_ru(pos, size, u32_colour);
                Ok(())
            }
            Canvas::Quality(_c) => {
                // TODO: quality path for stroke_rect_ru
                Ok(())
            }
        }
    }

    pub fn fill_rotated_rect_ru(&mut self, pos: CircleF4E4, size: CircleF4E4, angle: ScalarF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        match self {
            Canvas::Fast(c) => {
                let u32_colour = crate::renderer::extract_colour_u32(colour)?;
                c.fill_rotated_rect_ru(pos, size, angle, u32_colour);
                Ok(())
            }
            Canvas::Quality(c) => {
                let pixel = crate::renderer::extract_colour_linear(colour)?;
                c.fill_rotated_rect_ru(pos, size, angle, pixel);
                Ok(())
            }
        }
    }

    pub fn draw_text(
        &mut self,
        font_cache: &mut FontCache,
        font_key: [u8; 32],
        font_bytes: &[u8],
        pos: CircleF4E4,
        size: ScalarF4E4,
        text: &str,
        colour: &vsf::VsfType,
        settings: &TextSettings,
    ) -> Result<(), String> {
        match self {
            Canvas::Fast(c) => {
                let u32_colour = crate::renderer::extract_colour_u32(colour)?;
                c.draw_text(font_cache, font_key, font_bytes, pos, size, text, u32_colour, settings);
                Ok(())
            }
            Canvas::Quality(c) => {
                let pixel = crate::renderer::extract_colour_linear(colour)?;
                c.draw_text(font_cache, font_key, font_bytes, pos, size, text, pixel, settings);
                Ok(())
            }
        }
    }

    pub fn draw_line(
        &mut self,
        start: CircleF4E4,
        end: CircleF4E4,
        colour: &vsf::VsfType,
        settings: &LineSettings,
    ) -> Result<(), String> {
        match self {
            Canvas::Fast(c) => {
                let u32_colour = crate::renderer::extract_colour_u32(colour)?;
                c.draw_line(start, end, u32_colour, settings);
                Ok(())
            }
            Canvas::Quality(c) => {
                let pixel = crate::renderer::extract_colour_linear(colour)?;
                c.draw_line(start, end, pixel, settings);
                Ok(())
            }
        }
    }

    pub fn fill_circle(&mut self, center: CircleF4E4, radius: ScalarF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        match self {
            Canvas::Fast(c) => {
                let u32_colour = crate::renderer::extract_colour_u32(colour)?;
                c.fill_circle(center, radius, u32_colour);
                Ok(())
            }
            Canvas::Quality(c) => {
                let pixel = crate::renderer::extract_colour_linear(colour)?;
                c.fill_circle(center, radius, pixel);
                Ok(())
            }
        }
    }

    pub fn fill_ellipse(&mut self, center: CircleF4E4, radii: CircleF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        match self {
            Canvas::Fast(c) => {
                let u32_colour = crate::renderer::extract_colour_u32(colour)?;
                c.fill_ellipse(center, radii, u32_colour);
                Ok(())
            }
            Canvas::Quality(c) => {
                let pixel = crate::renderer::extract_colour_linear(colour)?;
                c.fill_ellipse(center, radii, pixel);
                Ok(())
            }
        }
    }

    pub fn stroke_ellipse(
        &mut self,
        center: CircleF4E4,
        radii: CircleF4E4,
        stroke_width: ScalarF4E4,
        colour: &vsf::VsfType,
    ) -> Result<(), String> {
        match self {
            Canvas::Fast(c) => {
                let u32_colour = crate::renderer::extract_colour_u32(colour)?;
                c.stroke_ellipse(center, radii, stroke_width, u32_colour);
                Ok(())
            }
            Canvas::Quality(c) => {
                let pixel = crate::renderer::extract_colour_linear(colour)?;
                c.stroke_ellipse(center, radii, stroke_width, pixel);
                Ok(())
            }
        }
    }

    /// Create a transparent layer canvas matching this canvas's pipeline and dimensions.
    pub fn new_layer(&self) -> Canvas {
        match self {
            Canvas::Fast(c) => Canvas::Fast(CanvasFast::new_layer(c.width(), c.height(), &c.coords)),
            Canvas::Quality(c) => {
                let mut layer = CanvasQuality::new_layer(c.width(), c.height(), c.ru());
                layer.set_scroll_y(c.scroll_y());
                Canvas::Quality(layer)
            }
        }
    }

    /// Composite a layer onto this canvas with opacity and blend mode.
    ///
    /// Fast path: if opacity is 1.0 and mode is Normal, this is a no-op
    /// (caller should have rendered directly into self instead).
    pub fn composite_layer(&mut self, layer: &Canvas, opacity: ScalarF4E4, mode: BlendMode) {
        match (self, layer) {
            (Canvas::Fast(dst), Canvas::Fast(src)) => {
                let a = (opacity * 255i32).to_isize().clamp(0, 255) as u8;
                dst.composite_from(src, a, mode);
            }
            (Canvas::Quality(dst), Canvas::Quality(src)) => {
                dst.composite_from(src, opacity, mode);
            }
            _ => {} // mismatched pipelines — silently skip
        }
    }

    /// Returns true if the given opacity + blend mode would be a passthrough
    /// (no temp layer needed — render children directly).
    pub fn is_layer_passthrough(opacity: ScalarF4E4, mode: BlendMode) -> bool {
        mode.is_passthrough() && opacity >= ScalarF4E4::ONE
    }

    /// Save a rectangular pixel region (for differential rerender).
    /// Returns (pixels, px_x, px_y, px_w, px_h) in pixel coords.
    pub fn save_region_ru(&self, pos: CircleF4E4, size: CircleF4E4) -> (Vec<u32>, usize, usize, usize, usize) {
        match self {
            Canvas::Fast(c) => {
                let half_w = size.r() >> 1usize;
                let half_h = size.i() >> 1usize;
                let left = c.coords.ru_to_px_x(pos.r() - half_w).max(0) as usize;
                let top = c.coords.ru_to_px_y(pos.i() - half_h).max(0) as usize;
                let right = (c.coords.ru_to_px_x(pos.r() + half_w).max(0) as usize).min(c.coords.width);
                let bottom = (c.coords.ru_to_px_y(pos.i() + half_h).max(0) as usize).min(c.coords.height);
                let w = right.saturating_sub(left);
                let h = bottom.saturating_sub(top);
                let mut pixels = Vec::with_capacity(w * h);
                for row in top..bottom {
                    let start = row * c.coords.width + left;
                    pixels.extend_from_slice(&c.pixels[start..start + w]);
                }
                (pixels, left, top, w, h)
            }
            Canvas::Quality(_) => (Vec::new(), 0, 0, 0, 0), // TODO
        }
    }

    /// Restore a rectangular pixel region (for differential rerender).
    pub fn restore_region(&mut self, pixels: &[u32], px_x: usize, px_y: usize, px_w: usize, px_h: usize) {
        match self {
            Canvas::Fast(c) => {
                let canvas_w = c.coords.width;
                for row in 0..px_h {
                    let dst_y = px_y + row;
                    if dst_y >= c.coords.height { break; }
                    let dst_start = dst_y * canvas_w + px_x;
                    let src_start = row * px_w;
                    if src_start + px_w <= pixels.len() && dst_start + px_w <= c.pixels.len() {
                        c.pixels[dst_start..dst_start + px_w].copy_from_slice(&pixels[src_start..src_start + px_w]);
                    }
                }
            }
            Canvas::Quality(_) => {} // TODO
        }
    }

    /// Blinkey cursor brightness constant (integer, maps to 0x01010100 * wave)
    const BLINKEY_BRIGHTNESS: i32 = 100;
    /// Horizontal smear half-width in pixels
    const BLINKEY_SMEAR: i32 = 7;

    /// Add blinkey wave (top-bright variant): wave = (1 - t²)(1 - t)² * BRIGHTNESS
    /// Height spans full textbox (tip to tip). Horizontal smear attenuates with distance.
    pub fn blinkey_add_top(&mut self, px_x: usize, px_y: usize, px_h: usize) {
        let Canvas::Fast(c) = self else { return };
        let w = c.coords.width;
        let h = c.coords.height;
        if px_h < 2 { return; }
        let half = px_h as i32 / 2;
        for row in 0..px_h {
            let y = px_y + row;
            if y >= h { break; }
            // t in [-half, half] mapped to [-1, 1] via fixed-point (scale by 1024)
            let t_1024 = (row as i32 - half) * 1024 / half;
            // (1 - t²)(1 - t)² all in fixed-point /1024
            let one_minus_t2 = 1024 - (t_1024 * t_1024 / 1024);
            let one_minus_t = 1024 - t_1024;
            let wave = one_minus_t2 * one_minus_t / 1024 * one_minus_t / 1024;
            let bright = (wave * Self::BLINKEY_BRIGHTNESS / 1024).max(0) as u32;
            if bright == 0 { continue; }
            for dx in -Self::BLINKEY_SMEAR..=Self::BLINKEY_SMEAR {
                let x = px_x as i32 + dx;
                if x < 0 || x as usize >= w { continue; }
                let idx = y * w + x as usize;
                let atten = bright >> dx.unsigned_abs();
                if atten > 0 {
                    c.pixels[idx] = c.pixels[idx].saturating_add(0x01010100 * atten);
                }
            }
        }
    }

    /// Add blinkey wave (bottom-bright variant): wave = (1 - t²)(1 + t)² * BRIGHTNESS
    pub fn blinkey_add_bottom(&mut self, px_x: usize, px_y: usize, px_h: usize) {
        let Canvas::Fast(c) = self else { return };
        let w = c.coords.width;
        let h = c.coords.height;
        if px_h < 2 { return; }
        let half = px_h as i32 / 2;
        for row in 0..px_h {
            let y = px_y + row;
            if y >= h { break; }
            let t_1024 = (row as i32 - half) * 1024 / half;
            let one_minus_t2 = 1024 - (t_1024 * t_1024 / 1024);
            let one_plus_t = 1024 + t_1024;
            let wave = one_minus_t2 * one_plus_t / 1024 * one_plus_t / 1024;
            let bright = (wave * Self::BLINKEY_BRIGHTNESS / 1024).max(0) as u32;
            if bright == 0 { continue; }
            for dx in -Self::BLINKEY_SMEAR..=Self::BLINKEY_SMEAR {
                let x = px_x as i32 + dx;
                if x < 0 || x as usize >= w { continue; }
                let idx = y * w + x as usize;
                let atten = bright >> dx.unsigned_abs();
                if atten > 0 {
                    c.pixels[idx] = c.pixels[idx].saturating_add(0x01010100 * atten);
                }
            }
        }
    }

    /// Subtract blinkey wave (top-bright variant) — exact inverse of add_top
    pub fn blinkey_sub_top(&mut self, px_x: usize, px_y: usize, px_h: usize) {
        let Canvas::Fast(c) = self else { return };
        let w = c.coords.width;
        let h = c.coords.height;
        if px_h < 2 { return; }
        let half = px_h as i32 / 2;
        for row in 0..px_h {
            let y = px_y + row;
            if y >= h { break; }
            let t_1024 = (row as i32 - half) * 1024 / half;
            let one_minus_t2 = 1024 - (t_1024 * t_1024 / 1024);
            let one_minus_t = 1024 - t_1024;
            let wave = one_minus_t2 * one_minus_t / 1024 * one_minus_t / 1024;
            let bright = (wave * Self::BLINKEY_BRIGHTNESS / 1024).max(0) as u32;
            if bright == 0 { continue; }
            for dx in -Self::BLINKEY_SMEAR..=Self::BLINKEY_SMEAR {
                let x = px_x as i32 + dx;
                if x < 0 || x as usize >= w { continue; }
                let idx = y * w + x as usize;
                let atten = bright >> dx.unsigned_abs();
                if atten > 0 {
                    c.pixels[idx] = c.pixels[idx].saturating_sub(0x01010100 * atten);
                }
            }
        }
    }

    /// Subtract blinkey wave (bottom-bright variant) — exact inverse of add_bottom
    pub fn blinkey_sub_bottom(&mut self, px_x: usize, px_y: usize, px_h: usize) {
        let Canvas::Fast(c) = self else { return };
        let w = c.coords.width;
        let h = c.coords.height;
        if px_h < 2 { return; }
        let half = px_h as i32 / 2;
        for row in 0..px_h {
            let y = px_y + row;
            if y >= h { break; }
            let t_1024 = (row as i32 - half) * 1024 / half;
            let one_minus_t2 = 1024 - (t_1024 * t_1024 / 1024);
            let one_plus_t = 1024 + t_1024;
            let wave = one_minus_t2 * one_plus_t / 1024 * one_plus_t / 1024;
            let bright = (wave * Self::BLINKEY_BRIGHTNESS / 1024).max(0) as u32;
            if bright == 0 { continue; }
            for dx in -Self::BLINKEY_SMEAR..=Self::BLINKEY_SMEAR {
                let x = px_x as i32 + dx;
                if x < 0 || x as usize >= w { continue; }
                let idx = y * w + x as usize;
                let atten = bright >> dx.unsigned_abs();
                if atten > 0 {
                    c.pixels[idx] = c.pixels[idx].saturating_sub(0x01010100 * atten);
                }
            }
        }
    }
}
