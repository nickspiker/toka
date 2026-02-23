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
//! - [`rect_fast`] - Rectangle rasterization
//! - [`circle_fast`] - Circle rasterization
//! - [`triangle_fast`] - Triangle rasterization (internal, for rotated rects)
//! - [`text_fast`] - Text rendering (placeholder)
//!
//! Quality pipeline:
//! - [`canvas_quality`] - CanvasQuality struct and pixel ops
//! - [`rect_quality`] - Rectangle rasterization
//! - [`circle_quality`] - Circle rasterization
//! - [`triangle_quality`] - Triangle rasterization
//! - [`text_quality`] - Text rendering (placeholder)

pub mod shared;

pub mod canvas_fast;
pub mod rect_fast;
pub mod circle_fast;
pub mod triangle_fast;
pub mod text_fast;

pub mod canvas_quality;
pub mod pixel_quality;
pub mod rect_quality;
pub mod circle_quality;
pub mod triangle_quality;
pub mod text_quality;

pub use canvas_fast::CanvasFast;
pub use canvas_quality::{CanvasQuality, Pixel};

/// Default canvas type — fast u32 sRGB pipeline
pub type Canvas = CanvasFast;
