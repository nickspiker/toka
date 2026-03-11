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
            Canvas::Quality(c) => Canvas::Quality(CanvasQuality::new_layer(c.width(), c.height(), c.ru())),
        }
    }

    /// Composite a layer onto this canvas with opacity and blend mode.
    ///
    /// Fast path: if opacity is 1.0 and mode is Normal, this is a no-op
    /// (caller should have rendered directly into self instead).
    pub fn composite_layer(&mut self, layer: &Canvas, opacity: ScalarF4E4, mode: BlendMode) {
        match (self, layer) {
            (Canvas::Fast(dst), Canvas::Fast(src)) => {
                let a = (opacity * ScalarF4E4::from(255)).to_isize().clamp(0, 255) as u8;
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
}
