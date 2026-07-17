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
    pub scroll_y: ScalarF4E4,
    /// Clip Y bounds (pixel rows). Drawing is restricted to clip_y_min..clip_y_max.
    /// Default: 0..height (full canvas). Set to exposed strip during scroll.
    pub clip_y_min: usize,
    pub clip_y_max: usize,
}

impl RuCoords {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            span: ScalarF4E4::from(width * height) / (width + height),
            ru: ScalarF4E4::ONE,
            half_dims: CircleF4E4::from((width, height)) >> 1,
            scroll_y: ScalarF4E4::ZERO,
            clip_y_min: 0,
            clip_y_max: height,
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

    pub fn set_scroll_y(&mut self, scroll_y: ScalarF4E4) {
        self.scroll_y = scroll_y;
    }

    /// Restrict drawing to pixel rows clip_y_min..clip_y_max
    pub fn set_clip_y(&mut self, min: usize, max: usize) {
        self.clip_y_min = min;
        self.clip_y_max = max.min(self.height);
    }

    /// Remove clip — draw to full canvas
    pub fn clear_clip_y(&mut self) {
        self.clip_y_min = 0;
        self.clip_y_max = self.height;
    }

    pub fn adjust_zoom(&mut self, steps: ScalarF4E4) {
        let steps_i = steps.to_isize();
        let step_count = steps_i.unsigned_abs() as usize;
        let is_zoom_in = steps_i > 0;
        let mut factor = ScalarF4E4::ONE;
        let zoom_in_ratio = ScalarF4E4::from(33) / 32;
        let zoom_out_ratio = ScalarF4E4::from(32) / 33;
        for _ in 0..step_count {
            factor = if is_zoom_in { factor * zoom_in_ratio } else { factor * zoom_out_ratio };
        }
        self.set_ru(self.ru * factor);
    }

    #[inline] pub fn ru_to_px_x(&self, x: ScalarF4E4) -> isize { (self.half_dims.r() + x * self.span * self.ru).to_isize() }
    #[inline] pub fn ru_to_px_y(&self, y: ScalarF4E4) -> isize { (self.half_dims.i() + (y - self.scroll_y) * self.span * self.ru).to_isize() }
    /// Screen-space Y: no scroll term. For mapping a host pointer (already scroll-free, center-origin
    /// RU) straight to the on-screen pixel row, e.g. to sample the hit_map. The inverse of the host's
    /// `pageY → (offsetY - h/2)/(span·ru)` conversion.
    #[inline] pub fn ru_to_px_y_screen(&self, y: ScalarF4E4) -> isize { (self.half_dims.i() + y * self.span * self.ru).to_isize() }
    #[inline] pub fn ru_to_px_w(&self, w: ScalarF4E4) -> isize { (w * self.span * self.ru).to_isize() }
    #[inline] pub fn ru_to_px_h(&self, h: ScalarF4E4) -> isize { (h * self.span * self.ru).to_isize() }

    // f32 pixel-space variants — same formula, but keep the sub-pixel fraction so fluor's
    // AA primitives can antialias against the true edge instead of a pre-floored integer.
    // Direct `to_f32()` is exact here (F4E4's fraction is narrower than f32's 24-bit mantissa),
    // so there's no precision reason to detour through f64.
    #[inline] pub fn ru_to_px_xf(&self, x: ScalarF4E4) -> f32 { (self.half_dims.r() + x * self.span * self.ru).to_f32() }
    #[inline] pub fn ru_to_px_yf(&self, y: ScalarF4E4) -> f32 { (self.half_dims.i() + (y - self.scroll_y) * self.span * self.ru).to_f32() }
    #[inline] pub fn ru_to_px_wf(&self, w: ScalarF4E4) -> f32 { (w * self.span * self.ru).to_f32() }
    #[inline] pub fn ru_to_px_hf(&self, h: ScalarF4E4) -> f32 { (h * self.span * self.ru).to_f32() }
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

/// Table layout settings — parsed from VM stack tags.
///
/// Grid structure:
///   `c` + u    — column count
///   `r` + u    — row count
///   `d`        — cell data marker (pops cols×rows strings)
///
/// Column widths (shared concept with text `x` wrap width):
///   `x` + cols×s44 — per-column widths in RU. 0 = hidden. Omit = all auto-fit.
///
/// Row height:
///   `y` + s44 — fixed row height in RU. Omit = auto from font metrics / wrapped content.
///
/// Grid visuals:
///   `h` + colour — header row background colour
///   `b` + colour — border/grid colour
///   `a` + colour — alternating row background colour
///   `p` + s44   — cell padding in RU (default: 0)
///   `g` + bytes — bitpacked grid mask (per-segment border control)
///     Byte 0: flags (bit 0 = horizontal mask, bit 1 = vertical mask)
///     Then horizontal mask: (rows+1)×cols bits, MSB-first, row-major
///     Then vertical mask: rows×(cols+1) bits, MSB-first, row-major
///     Bit=1 draws segment, bit=0 skips. Default (no `g` tag) = all segments.
///     Masks are applied with last-wins — push multiple `g` tags to layer.
///
/// Per-column alignment:
///   `j` + string — horizontal justify per column (`l`/`c`/`r`, default: center)
///   `v` + string — vertical alignment per column (`t`/`m`/`b`, default: middle)
#[derive(Clone)]
pub struct TableSettings {
    /// `x` — Per-column widths in RU. 0 = hidden. None = all auto-fit.
    pub col_widths: Option<Vec<ScalarF4E4>>,
    /// `y` — Fixed row height in RU. None = derive from font metrics.
    pub row_height: Option<ScalarF4E4>,
    /// `h` — Header row background (None = no special header bg)
    pub header_bg: Option<vsf::VsfType>,
    /// `b` — Border/grid line colour (None = no grid)
    pub border_colour: Option<vsf::VsfType>,
    /// `a` — Alternating row background (None = transparent)
    pub alt_row_bg: Option<vsf::VsfType>,
    /// `p` — Cell padding in RU
    pub padding: ScalarF4E4,
    /// `g` — Bitpacked grid mask for per-segment border control
    pub grid_mask: Option<GridMask>,
    /// `j` — Per-column horizontal justify (one char per column: l/c/r)
    pub h_align: Option<Vec<u8>>,
    /// `v` — Per-column vertical alignment (one char per column: t/m/b)
    pub v_align: Option<Vec<u8>>,
}

/// Bitpacked per-segment border mask.
///
/// Each bit controls one border segment. Horizontal segments are
/// the lines between rows (including top/bottom edges), spanning
/// each column. Vertical segments are the lines between columns
/// (including left/right edges), spanning each row.
///
/// Bits are packed MSB-first, row-major within each mask.
#[derive(Clone)]
pub struct GridMask {
    /// Horizontal segment bits: (rows+1) × cols, one bit per segment
    pub h_bits: Vec<u8>,
    /// Vertical segment bits: rows × (cols+1), one bit per segment
    pub v_bits: Vec<u8>,
    /// Whether horizontal mask is present (if false, no horizontal lines)
    pub has_h: bool,
    /// Whether vertical mask is present (if false, no vertical lines)
    pub has_v: bool,
}

impl GridMask {
    /// Check if a horizontal segment should be drawn.
    /// `row_gap` = 0..=rows, `col` = 0..cols
    pub fn h_segment(&self, row_gap: usize, col: usize, cols: usize) -> bool {
        if !self.has_h { return false; }
        let bit_idx = row_gap * cols + col;
        let byte_idx = bit_idx / 8;
        let bit_pos = 7 - (bit_idx % 8); // MSB-first
        if byte_idx >= self.h_bits.len() { return false; }
        (self.h_bits[byte_idx] >> bit_pos) & 1 == 1
    }

    /// Check if a vertical segment should be drawn.
    /// `row` = 0..rows, `col_gap` = 0..=cols
    pub fn v_segment(&self, row: usize, col_gap: usize, cols_plus_1: usize) -> bool {
        if !self.has_v { return false; }
        let bit_idx = row * cols_plus_1 + col_gap;
        let byte_idx = bit_idx / 8;
        let bit_pos = 7 - (bit_idx % 8);
        if byte_idx >= self.v_bits.len() { return false; }
        (self.v_bits[byte_idx] >> bit_pos) & 1 == 1
    }
}

impl Default for TableSettings {
    fn default() -> Self {
        TableSettings {
            col_widths: None,
            row_height: None,
            header_bg: None,
            border_colour: None,
            alt_row_bg: None,
            padding: ScalarF4E4::ZERO,
            grid_mask: None,
            h_align: None,
            v_align: None,
        }
    }
}

/// Blend mode for layer compositing and per-pixel operations.
///
/// All modes operate per-channel on RGB; alpha composited separately.
/// Quality pipeline: no clamping — values can go negative or above 1.0.
/// Fast pipeline: saturates to 0..255 (inherent to u8 representation).
///
/// Tag shortcodes (single lowercase ASCII, matches VSF convention):
///   `n` — normal (default, src-over)
///   `m` — multiply
///   `s` — screen
///   `o` — overlay
///   `d` — darken
///   `l` — lighten
///   `g` — color dodge
///   `b` — color burn
///   `h` — hard light
///   `f` — soft light
///   `i` — difference
///   `e` — exclusion
///   `a` — add
///   `t` — subtract
///   `v` — divide
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// `n` — Porter-Duff src-over (default)
    Normal,
    /// `m` — src × dst
    Multiply,
    /// `s` — src + dst − src×dst
    Screen,
    /// `o` — Multiply if dst dark, Screen if dst light
    Overlay,
    /// `d` — min(src, dst)
    Darken,
    /// `l` — max(src, dst)
    Lighten,
    /// `g` — dst / (1 − src)
    ColorDodge,
    /// `b` — 1 − (1 − dst) / src
    ColorBurn,
    /// `h` — Overlay with src/dst roles swapped
    HardLight,
    /// `f` — Pegtop: (1−2s)·d² + 2s·d
    SoftLight,
    /// `i` — |src − dst|
    Difference,
    /// `e` — src + dst − 2·src·dst
    Exclusion,
    /// `a` — src + dst
    Add,
    /// `t` — dst − src
    Subtract,
    /// `v` — dst / src
    Divide,
}

impl BlendMode {
    /// Parse from ASCII tag byte (single lowercase letter)
    pub fn from_tag(v: u8) -> Self {
        match v {
            b'm' => BlendMode::Multiply,
            b's' => BlendMode::Screen,
            b'o' => BlendMode::Overlay,
            b'd' => BlendMode::Darken,
            b'l' => BlendMode::Lighten,
            b'g' => BlendMode::ColorDodge,
            b'b' => BlendMode::ColorBurn,
            b'h' => BlendMode::HardLight,
            b'f' => BlendMode::SoftLight,
            b'i' => BlendMode::Difference,
            b'e' => BlendMode::Exclusion,
            b'a' => BlendMode::Add,
            b't' => BlendMode::Subtract,
            b'v' => BlendMode::Divide,
            _ => BlendMode::Normal,
        }
    }

    /// Convert to ASCII tag byte
    pub fn to_tag(self) -> u8 {
        match self {
            BlendMode::Normal => b'n',
            BlendMode::Multiply => b'm',
            BlendMode::Screen => b's',
            BlendMode::Overlay => b'o',
            BlendMode::Darken => b'd',
            BlendMode::Lighten => b'l',
            BlendMode::ColorDodge => b'g',
            BlendMode::ColorBurn => b'b',
            BlendMode::HardLight => b'h',
            BlendMode::SoftLight => b'f',
            BlendMode::Difference => b'i',
            BlendMode::Exclusion => b'e',
            BlendMode::Add => b'a',
            BlendMode::Subtract => b't',
            BlendMode::Divide => b'v',
        }
    }

    /// True if this mode is a no-op passthrough (Normal blend)
    pub fn is_passthrough(&self) -> bool {
        matches!(self, BlendMode::Normal)
    }
}

/// Cap style for line endpoints
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cap {
    /// Flat end at the endpoint (default)
    Butt = 0,
    /// Semicircle extending past the endpoint by half the weight
    Round = 1,
    /// Rectangle extending past the endpoint by half the weight
    Square = 2,
}

impl Cap {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Cap::Round,
            2 => Cap::Square,
            _ => Cap::Butt,
        }
    }
}

/// Line drawing settings — mirrors the tag pattern from TextSettings.
///
/// Tags (single lowercase ASCII):
///   `w` + s44 — stroke weight in RU (None = 1px hairline)
///   `c` + u3  — cap style for both endpoints: 0=butt, 1=round, 2=square
///   `s` + u3  — start cap override (if different from end)
///   `e` + u3  — end cap override (if different from start)
///   `p`       — pixel mode flag: always 1 device pixel (ignores zoom/resolution)
///
/// Defaults: 1px hairline, butt caps, pixel mode off.
/// Dashes are user-side (bake separate segments in bytecode).
pub struct LineSettings {
    /// `w` — Stroke weight in RU. None = 1px hairline (Wu's algorithm).
    pub weight: Option<ScalarF4E4>,
    /// `s` — Cap style at start endpoint
    pub cap_start: Cap,
    /// `e` — Cap style at end endpoint
    pub cap_end: Cap,
    /// `p` — Pixel mode: always exactly 1 device pixel regardless of zoom
    pub pixel: bool,
}

impl Default for LineSettings {
    fn default() -> Self {
        LineSettings {
            weight: None,
            cap_start: Cap::Butt,
            cap_end: Cap::Butt,
            pixel: false,
        }
    }
}
