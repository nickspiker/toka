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

pub mod canvas_quality;
pub mod pixel_quality;
pub mod rect_quality;
pub mod circle_quality;
pub mod text_quality;

pub use canvas_fast::CanvasFast;
pub use canvas_quality::{CanvasQuality, Pixel};

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

    pub fn set_ru(&mut self, ru: ScalarF4E4) {
        match self {
            Canvas::Fast(c) => c.set_ru(ru),
            Canvas::Quality(c) => c.set_ru(ru),
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
        align: u8,
    ) -> Result<(), String> {
        match self {
            Canvas::Fast(c) => {
                let u32_colour = crate::renderer::extract_colour_u32(colour)?;
                c.draw_text(font_cache, font_key, font_bytes, pos, size, text, u32_colour, align);
                Ok(())
            }
            Canvas::Quality(c) => {
                let pixel = crate::renderer::extract_colour_linear(colour)?;
                c.draw_text(font_cache, font_key, font_bytes, pos, size, text, pixel, align);
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
}
