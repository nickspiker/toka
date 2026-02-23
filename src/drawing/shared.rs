#![allow(missing_docs)]
//! Shared RU coordinate system used by both CanvasFast and CanvasQuality
//!
//! RU (Relative Units): Resolution-independent coordinate system
//! - span = 2wh/(w+h) - harmonic mean, base unit for all measurements
//! - 1 RU from center reaches edge of smaller dimension
//! - `ru` multiplier: user-adjustable zoom (scales all GUI without layout changes)
//! - Same bytecode renders correctly at any resolution
//!
//! Coordinate system:
//! - (0, 0) = center of canvas
//! - +X = right, +Y = down
//! - All coordinates in RU space, converted to pixels internally

use spirix::{CircleF4E4, ScalarF4E4};

/// RU coordinate system state — embedded in both canvas types
pub struct RuCoords {
    pub width: usize,
    pub height: usize,
    pub span: ScalarF4E4,
    pub ru: ScalarF4E4,
    pub half_dims: CircleF4E4,
}

impl RuCoords {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            span: ScalarF4E4::from(width * height) / (width + height),
            ru: ScalarF4E4::ONE,
            half_dims: CircleF4E4::from((width, height)) >> 1,
        }
    }

    pub fn span(&self) -> ScalarF4E4 { self.span }
    pub fn ru(&self) -> ScalarF4E4 { self.ru }
    pub fn width(&self) -> usize { self.width }
    pub fn height(&self) -> usize { self.height }
    pub fn half_dims(&self) -> CircleF4E4 { self.half_dims }

    pub fn set_ru(&mut self, ru: ScalarF4E4) {
        self.ru = ru.clamp(0.125, 8);
    }

    pub fn adjust_zoom(&mut self, steps: ScalarF4E4) {
        let steps_i = steps.to_isize();
        let step_count = steps_i.unsigned_abs() as usize;
        let is_zoom_in = steps_i > 0;
        let mut factor = ScalarF4E4::ONE;
        let zoom_in_ratio = ScalarF4E4::from(33) / ScalarF4E4::from(32);
        let zoom_out_ratio = ScalarF4E4::from(32) / ScalarF4E4::from(33);
        for _ in 0..step_count {
            factor = if is_zoom_in { factor * zoom_in_ratio } else { factor * zoom_out_ratio };
        }
        self.set_ru(self.ru * factor);
    }

    #[inline] pub fn ru_to_px_x(&self, x: ScalarF4E4) -> isize { (self.half_dims.r() + x * self.span * self.ru).to_isize() }
    #[inline] pub fn ru_to_px_y(&self, y: ScalarF4E4) -> isize { (self.half_dims.i() + y * self.span * self.ru).to_isize() }
    #[inline] pub fn ru_to_px_w(&self, w: ScalarF4E4) -> isize { (w * self.span * self.ru).to_isize() }
    #[inline] pub fn ru_to_px_h(&self, h: ScalarF4E4) -> isize { (h * self.span * self.ru).to_isize() }
}
