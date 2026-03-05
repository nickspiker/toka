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

use spirix::{sf, CircleF4E4, ScalarF4E4};

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
        self.ru = ru.clamp(sf!(0.125), ScalarF4E4::from(8));
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

/// Text layout settings — mirrors VSF TextStyle tags.
///
/// Tags (single lowercase ASCII):
///   `l` — align left (flag)
///   `r` — align right (flag)
///   `e` + s44 — leading (line height multiplier)
///   `k` + s44 — kerning (letter spacing in RU)
///   `w` + s44 — weight (100–900, variable font axis)
///   `i` + s44 — tilt (italic angle in degrees, variable font axis)
///   `x` + s44 — wrap width (box width in RU)
///   `f` + 32  — font hash (handled separately via font_key)
///
/// Defaults: center-aligned, no wrap, default weight/leading.
pub struct TextSettings {
    /// Horizontal alignment: 0=center, 1=left (`l`), 2=right (`r`)
    pub align: u8,
    /// `e` — Line height multiplier (1.0 = default)
    pub leading: ScalarF4E4,
    /// `k` — Letter spacing in RU (0.0 = default, added to advance width)
    pub kerning: ScalarF4E4,
    /// `w` — Font weight 100–900 (stub: variable font axis)
    pub weight: Option<ScalarF4E4>,
    /// `i` — Italic tilt angle in degrees (stub: variable font axis)
    pub tilt: Option<ScalarF4E4>,
    /// `x` — Wrap box width in RU. None = no wrapping.
    pub wrap: Option<ScalarF4E4>,
}

impl Default for TextSettings {
    fn default() -> Self {
        TextSettings {
            align: 0,
            leading: ScalarF4E4::ONE,
            kerning: ScalarF4E4::ZERO,
            weight: None,
            tilt: None,
            wrap: None,
        }
    }
}

impl TextSettings {
    /// Build from a VSF TextStyle, if present.
    #[cfg(feature = "spirix")]
    pub fn from_vsf_style(style: &Option<vsf::types::toka_tree::TextStyle>) -> Self {
        let mut s = Self::default();
        if let Some(ts) = style {
            if let Some(a) = ts.align { s.align = a; }
            if let Some(e) = ts.leading { s.leading = e; }
            if let Some(k) = ts.kerning { s.kerning = k; }
            s.weight = ts.weight;
            s.tilt = ts.tilt;
            s.wrap = ts.wrap;
        }
        s
    }

    /// Compute a font cache key that incorporates variable font axes (weight, tilt).
    /// If no variation axes are set, returns the plain font_key unchanged.
    pub fn font_cache_key(&self, font_key: [u8; 32]) -> [u8; 32] {
        if self.weight.is_none() && self.tilt.is_none() {
            return font_key;
        }
        let mut hasher = blake3::Hasher::new();
        hasher.update(&font_key);
        if let Some(w) = self.weight {
            hasher.update(b"wght");
            hasher.update(&w.fraction.to_le_bytes());
            hasher.update(&w.exponent.to_le_bytes());
        }
        if let Some(i) = self.tilt {
            hasher.update(b"ital");
            hasher.update(&i.fraction.to_le_bytes());
            hasher.update(&i.exponent.to_le_bytes());
        }
        *hasher.finalize().as_bytes()
    }
}
