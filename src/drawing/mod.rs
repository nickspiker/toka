//! Drawing layer — single fluor-backed pipeline.
//!
//! `Canvas` owns an α+darkness pixel buffer (`0xααRRGGBB`, RGB = 255−visible) and draws into it
//! with fluor's paint primitives + `TextRenderer`. The old dual Fast(sRGB)/Quality(linear) pipelines
//! and their bespoke rasterizers are gone; fluor is the one compositor.
//!
//! **Compositing direction.** fluor paints *front-to-back* (`under` blend: new content goes behind
//! what's already there, opaque pixels early-out). toka emits *back-to-front* (painter's: clear an
//! opaque background, then draw over it). To bridge, every primitive rasterizes into a transparent
//! `scratch` buffer first (fluor AA onto empty), then [`Canvas::blit_scratch`] composites scratch
//! *over* the main buffer (`scratch.under(main)` — new in front), so later draws land on top.
//!
//! Coordinates: RU math stays in [`shared::RuCoords`] (spirix, center-origin, harmonic-mean span);
//! each primitive converts to f32 pixel space at the fluor boundary via the `ru_to_px_*f` helpers.
//! Colour funnels through [`crate::renderer::extract_colour_u32`] → fluor α+darkness. Output flips
//! back to visible RGBA in [`Canvas::to_rgba_bytes`].

pub mod shared;

pub use shared::RuCoords;
pub use shared::{BlendMode, LineSettings, TableSettings, TextSettings};

use crate::vm::FontCache;
use shared::RuCoords as Coords;
use spirix::{CircleF4E4, ScalarF4E4};
use std::collections::HashMap;

use fluor::canvas::{Canvas as FCanvas, Damage, PixelRect};
use fluor::paint::{self, Clip};
use fluor::pixel::Blend;
use fluor::text::TextRenderer;

/// Opaque black in α+darkness (α=255, darkness=255 → visible black). The default surface fill.
const OPAQUE_BLACK: u32 = 0xFFFF_FFFF;

/// Single fluor-backed canvas.
pub struct Canvas {
    coords: Coords,
    /// α+darkness pixel buffer (the composited result).
    pixels: Vec<u32>,
    /// Transparent scratch each primitive rasterizes into before being composited over `pixels`.
    /// Kept at zero between draws (blit resets the touched region), so it never needs a full clear.
    scratch: Vec<u32>,
    /// fluor text engine (owns its cosmic-text font system + glyph cache).
    text: TextRenderer,
    /// Capsule-shipped fonts: `font_key` → registered family name in fluor's font DB.
    fonts: HashMap<[u8; 32], String>,
}

/// Map toka's blend-mode enum onto fluor's smaller set; anything without a fluor equivalent
/// falls back to Normal (source-over).
fn to_fluor_blend(mode: BlendMode) -> fluor::BlendMode {
    use fluor::BlendMode as F;
    match mode {
        BlendMode::Multiply => F::Multiply,
        BlendMode::Screen => F::Screen,
        BlendMode::Overlay => F::Overlay,
        BlendMode::Darken => F::Darken,
        BlendMode::Lighten => F::Lighten,
        BlendMode::Add => F::Add,
        BlendMode::Subtract => F::Subtract,
        _ => F::Normal,
    }
}

#[allow(missing_docs)]
impl Canvas {
    pub fn new_fast(width: usize, height: usize) -> Self {
        Self {
            coords: Coords::new(width, height),
            pixels: vec![OPAQUE_BLACK; width * height],
            scratch: vec![0u32; width * height],
            text: TextRenderer::new(),
            fonts: HashMap::new(),
        }
    }

    /// One pipeline now — kept as an alias so callers that asked for "quality" still work.
    pub fn new_quality(width: usize, height: usize) -> Self {
        Self::new_fast(width, height)
    }

    pub fn pipeline_name(&self) -> &'static str {
        "fluor"
    }

    // ── Coordinate / state accessors (delegate to RuCoords) ──────────────

    pub fn span(&self) -> ScalarF4E4 { self.coords.span() }
    pub fn ru(&self) -> ScalarF4E4 { self.coords.ru() }
    pub fn width(&self) -> usize { self.coords.width() }
    pub fn height(&self) -> usize { self.coords.height() }
    pub fn dimensions(&self) -> (usize, usize) { (self.coords.width(), self.coords.height()) }
    pub fn half_dims(&self) -> CircleF4E4 { self.coords.half_dims() }
    pub fn ru_to_px_x(&self, x: ScalarF4E4) -> usize { self.coords.ru_to_px_x(x).max(0) as usize }
    pub fn ru_to_px_y(&self, y: ScalarF4E4) -> usize { self.coords.ru_to_px_y(y).max(0) as usize }
    pub fn set_ru(&mut self, ru: ScalarF4E4) { self.coords.set_ru(ru); }
    pub fn set_scroll_y(&mut self, scroll_y: ScalarF4E4) { self.coords.set_scroll_y(scroll_y); }
    pub fn set_clip_y(&mut self, min: usize, max: usize) { self.coords.set_clip_y(min, max); }
    pub fn clear_clip_y(&mut self) { self.coords.clear_clip_y(); }
    pub fn adjust_zoom(&mut self, steps: ScalarF4E4) { self.coords.adjust_zoom(steps); }

    /// Clip rect for the current clip_y band, or `None` if unclipped (full canvas).
    fn clip(&self) -> Option<Clip> {
        if self.coords.clip_y_min == 0 && self.coords.clip_y_max >= self.coords.height {
            None
        } else {
            Some(Clip::new(0, self.coords.clip_y_min, self.coords.width, self.coords.clip_y_max))
        }
    }

    /// Composite the scratch buffer *over* `pixels` within `bb` (painter's order — scratch is the
    /// new content, in front), then reset the touched scratch pixels back to transparent.
    fn blit_scratch(&mut self, bb: PixelRect) {
        if bb.is_empty() { return; }
        let w = self.coords.width;
        let h = self.coords.height;
        let x0 = bb.x0.min(w);
        let x1 = bb.x1.min(w);
        let y0 = bb.y0.min(h);
        let y1 = bb.y1.min(h);
        for y in y0..y1 {
            let row = y * w;
            for x in x0..x1 {
                let i = row + x;
                let s = self.scratch[i];
                if s >> 24 != 0 {
                    self.pixels[i] = s.under(self.pixels[i], fluor::BlendMode::Normal);
                    self.scratch[i] = 0;
                }
            }
        }
    }

    /// Rasterize one primitive into the transparent scratch buffer via `f`, then composite it over
    /// the main buffer. `f` receives a fluor canvas backed by `scratch`.
    fn paint_over(&mut self, f: impl FnOnce(&mut FCanvas)) {
        let (w, h) = (self.coords.width, self.coords.height);
        let mut dmg = Damage::new();
        {
            let mut fc = FCanvas::new(&mut self.scratch, w, h, &mut dmg);
            f(&mut fc);
        }
        self.blit_scratch(dmg.bbox());
    }

    pub fn clear(&mut self, colour: &vsf::VsfType) -> Result<(), String> {
        let c = crate::renderer::extract_colour_u32(colour)?;
        let w = self.coords.width;
        let y0 = self.coords.clip_y_min;
        let y1 = self.coords.clip_y_max;
        self.pixels[y0 * w..y1 * w].fill(c);
        Ok(())
    }

    /// Convert α+darkness buffer → visible RGBA bytes for browser ImageData.
    /// `pixel ^ 0x00FFFFFF` flips darkness→visible RGB; the surface is forced opaque.
    pub fn to_rgba_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.pixels.len() * 4);
        for &p in &self.pixels {
            let v = p ^ 0x00FF_FFFF;
            bytes.push((v >> 16) as u8); // R
            bytes.push((v >> 8) as u8); // G
            bytes.push(v as u8); // B
            bytes.push(0xFF); // A — opaque output surface
        }
        bytes
    }

    // ── Geometry primitives (fluor paint into scratch, composited over) ──

    pub fn fill_rect_ru(&mut self, pos: CircleF4E4, size: CircleF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        let c = crate::renderer::extract_colour_u32(colour)?;
        let cx = self.coords.ru_to_px_xf(pos.r());
        let cy = self.coords.ru_to_px_yf(pos.i());
        let w = self.coords.ru_to_px_wf(size.r());
        let h = self.coords.ru_to_px_hf(size.i());
        let clip = self.clip();
        self.paint_over(|fc| paint::draw_rect(fc, cx, cy, w, h, c, clip));
        Ok(())
    }

    /// 1px horizontal line at RU `y` from `x0` to `x1` — a zero-height rect (fluor's line convention).
    /// Centre on the pixel row for a crisp rule (`+0.5` = pixel centre).
    pub fn hline_ru(&mut self, y: ScalarF4E4, x0: ScalarF4E4, x1: ScalarF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        let c = crate::renderer::extract_colour_u32(colour)?;
        let py = self.coords.ru_to_px_yf(y).floor() + 0.5;
        let px0 = self.coords.ru_to_px_xf(x0);
        let px1 = self.coords.ru_to_px_xf(x1);
        let cx = (px0 + px1) * 0.5;
        let w = (px1 - px0).abs();
        let clip = self.clip();
        self.paint_over(|fc| paint::draw_rect(fc, cx, py, w, 0.0, c, clip));
        Ok(())
    }

    /// 1px vertical line at RU `x` from `y0` to `y1` — a zero-width rect.
    pub fn vline_ru(&mut self, x: ScalarF4E4, y0: ScalarF4E4, y1: ScalarF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        let c = crate::renderer::extract_colour_u32(colour)?;
        let px = self.coords.ru_to_px_xf(x).floor() + 0.5;
        let py0 = self.coords.ru_to_px_yf(y0);
        let py1 = self.coords.ru_to_px_yf(y1);
        let cy = (py0 + py1) * 0.5;
        let h = (py1 - py0).abs();
        let clip = self.clip();
        self.paint_over(|fc| paint::draw_rect(fc, px, cy, 0.0, h, c, clip));
        Ok(())
    }

    /// 1px axis-aligned rectangle outline (borders).
    pub fn stroke_rect_ru(&mut self, pos: CircleF4E4, size: CircleF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        let c = crate::renderer::extract_colour_u32(colour)?;
        let cx = self.coords.ru_to_px_xf(pos.r());
        let cy = self.coords.ru_to_px_yf(pos.i());
        let w = self.coords.ru_to_px_wf(size.r());
        let h = self.coords.ru_to_px_hf(size.i());
        let x = (cx - w * 0.5).round() as isize;
        let yy = (cy - h * 0.5).round() as isize;
        let (rw, rh) = (w.round() as isize, h.round() as isize);
        let clip = self.clip();
        self.paint_over(|fc| paint::stroke_rect(fc, x, yy, rw, rh, 1, c, clip, None));
        Ok(())
    }

    pub fn fill_rotated_rect_ru(&mut self, pos: CircleF4E4, size: CircleF4E4, angle: ScalarF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        let c = crate::renderer::extract_colour_u32(colour)?;
        let cx = self.coords.ru_to_px_xf(pos.r());
        let cy = self.coords.ru_to_px_yf(pos.i());
        let w = self.coords.ru_to_px_wf(size.r());
        let h = self.coords.ru_to_px_hf(size.i());
        let ang = angle.to_f64() as f32;
        let clip = self.clip();
        self.paint_over(|fc| paint::draw_rect_rotated(fc, cx, cy, w, h, ang, c, clip));
        Ok(())
    }

    pub fn fill_circle(&mut self, center: CircleF4E4, radius: ScalarF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        let c = crate::renderer::extract_colour_u32(colour)?;
        let cx = self.coords.ru_to_px_xf(center.r());
        let cy = self.coords.ru_to_px_yf(center.i());
        let r = self.coords.ru_to_px_wf(radius);
        let clip = self.clip();
        self.paint_over(|fc| paint::draw_circle(fc, cx, cy, r, c, clip));
        Ok(())
    }

    pub fn fill_ellipse(&mut self, center: CircleF4E4, radii: CircleF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        let c = crate::renderer::extract_colour_u32(colour)?;
        let cx = self.coords.ru_to_px_xf(center.r());
        let cy = self.coords.ru_to_px_yf(center.i());
        let rx = self.coords.ru_to_px_wf(radii.r());
        let ry = self.coords.ru_to_px_hf(radii.i());
        let clip = self.clip();
        self.paint_over(|fc| paint::draw_ellipse(fc, cx, cy, rx, ry, c, clip));
        Ok(())
    }

    /// Ellipse outline. fluor has no stroke-ellipse primitive; approximate the ring as an outer
    /// filled ellipse (the VSF renderer currently rejects strokes upstream, so this path is
    /// effectively unused — kept faithful to the API surface).
    pub fn stroke_ellipse(&mut self, center: CircleF4E4, radii: CircleF4E4, _stroke_width: ScalarF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        let c = crate::renderer::extract_colour_u32(colour)?;
        let cx = self.coords.ru_to_px_xf(center.r());
        let cy = self.coords.ru_to_px_yf(center.i());
        let rx = self.coords.ru_to_px_wf(radii.r());
        let ry = self.coords.ru_to_px_hf(radii.i());
        let clip = self.clip();
        self.paint_over(|fc| paint::draw_ellipse(fc, cx, cy, rx, ry, c, clip));
        Ok(())
    }

    /// General line via a thin rotated rect (fluor has no line primitive; a rotated rect is it).
    pub fn draw_line(&mut self, start: CircleF4E4, end: CircleF4E4, colour: &vsf::VsfType, settings: &LineSettings) -> Result<(), String> {
        let c = crate::renderer::extract_colour_u32(colour)?;
        let x0 = self.coords.ru_to_px_xf(start.r());
        let y0 = self.coords.ru_to_px_yf(start.i());
        let x1 = self.coords.ru_to_px_xf(end.r());
        let y1 = self.coords.ru_to_px_yf(end.i());
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();
        let ang = dy.atan2(dx);
        let weight = settings.weight.map(|w| self.coords.ru_to_px_wf(w)).unwrap_or(1.0).max(1.0);
        let cx = (x0 + x1) * 0.5;
        let cy = (y0 + y1) * 0.5;
        let clip = self.clip();
        self.paint_over(|fc| paint::draw_rect_rotated(fc, cx, cy, len, weight, ang, c, clip));
        Ok(())
    }

    // ── Text (fluor TextRenderer) ────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub fn draw_text(
        &mut self,
        _font_cache: &mut FontCache,
        font_key: [u8; 32],
        font_bytes: &[u8],
        pos: CircleF4E4,
        size: ScalarF4E4,
        text: &str,
        colour: &vsf::VsfType,
        settings: &TextSettings,
    ) -> Result<(), String> {
        let c = crate::renderer::extract_colour_u32(colour)?;
        let cx = self.coords.ru_to_px_xf(pos.r());
        let cy = self.coords.ru_to_px_yf(pos.i());
        let px = self.coords.ru_to_px_hf(size);
        let weight = settings.weight.map(|w| w.to_i32().clamp(1, 1000) as u16).unwrap_or(400);

        // Resolve (and lazily register) the capsule's shipped font family.
        let family = match self.fonts.get(&font_key) {
            Some(f) => f.clone(),
            None => {
                let name = self
                    .text
                    .load_font_data_named(font_bytes.to_vec())
                    .unwrap_or_else(|| "Open Sans".to_string());
                self.fonts.insert(font_key, name.clone());
                name
            }
        };

        let clip = self.clip();
        let (w, h) = (self.coords.width, self.coords.height);
        let mut dmg = Damage::new();
        {
            // Split borrows: `text` and `scratch` are disjoint fields; rasterize into scratch.
            let Canvas { text: engine, scratch, .. } = self;
            let mut fc = FCanvas::new(scratch, w, h, &mut dmg);
            match settings.align {
                1 => { engine.draw_text_left_u32(&mut fc, text, cx, cy, px, weight, c, &family, clip, None, None); }
                2 => { engine.draw_text_right_u32(&mut fc, text, cx, cy, px, weight, c, &family, clip, None, None); }
                _ => { engine.draw_text_center_u32(&mut fc, text, cx, cy, px, weight, c, &family, clip, None, None); }
            }
        }
        self.blit_scratch(dmg.bbox());
        Ok(())
    }

    // ── Scroll / layers / regions ────────────────────────────────────────

    /// Shift the buffer by `delta_y` rows; the exposed strip is filled with `bg` (α+darkness).
    pub fn scroll_pixels(&mut self, delta_y: isize, bg: u32) {
        let h = self.coords.height;
        let w = self.coords.width;
        let d = delta_y.unsigned_abs().min(h);
        if d == 0 { return; }
        if d >= h { self.pixels.fill(bg); return; }
        if delta_y > 0 {
            self.pixels.copy_within(d * w..h * w, 0);
            self.pixels[((h - d) * w)..].fill(bg);
        } else {
            self.pixels.copy_within(0..(h - d) * w, d * w);
            self.pixels[..d * w].fill(bg);
        }
    }

    /// Transparent layer matching this canvas's dimensions + RU state (for opacity groups).
    pub fn new_layer(&self) -> Canvas {
        let mut coords = Coords::new(self.coords.width, self.coords.height);
        coords.set_ru(self.coords.ru());
        coords.set_scroll_y(self.coords.scroll_y);
        Canvas {
            coords,
            pixels: vec![0u32; self.coords.width * self.coords.height], // fully transparent
            scratch: vec![0u32; self.coords.width * self.coords.height],
            text: TextRenderer::new(),
            fonts: HashMap::new(),
        }
    }

    /// Composite `layer` on top of self with opacity + blend mode (α+darkness under-blend).
    pub fn composite_layer(&mut self, layer: &Canvas, opacity: ScalarF4E4, mode: BlendMode) {
        let op = (opacity * 255i32).to_i32().clamp(0, 255) as u32;
        if op == 0 { return; }
        let fmode = to_fluor_blend(mode);
        let n = self.pixels.len().min(layer.pixels.len());
        for i in 0..n {
            let s = layer.pixels[i];
            let sa = s >> 24;
            if sa == 0 { continue; } // transparent source pixel
            let scaled_a = (sa * op / 255) & 0xFF;
            let top = (scaled_a << 24) | (s & 0x00FF_FFFF);
            // layer sits on top; base is underneath → top.under(base)
            self.pixels[i] = top.under(self.pixels[i], fmode);
        }
    }

    pub fn is_layer_passthrough(opacity: ScalarF4E4, mode: BlendMode) -> bool {
        mode.is_passthrough() && opacity >= ScalarF4E4::ONE
    }

    /// Save a rectangular pixel region (differential rerender). Format-agnostic raw copy.
    pub fn save_region_ru(&self, pos: CircleF4E4, size: CircleF4E4) -> (Vec<u32>, usize, usize, usize, usize) {
        let half_w = size.r() >> 1usize;
        let half_h = size.i() >> 1usize;
        let left = self.coords.ru_to_px_x(pos.r() - half_w).max(0) as usize;
        let top = self.coords.ru_to_px_y(pos.i() - half_h).max(0) as usize;
        let right = (self.coords.ru_to_px_x(pos.r() + half_w).max(0) as usize).min(self.coords.width);
        let bottom = (self.coords.ru_to_px_y(pos.i() + half_h).max(0) as usize).min(self.coords.height);
        let w = right.saturating_sub(left);
        let h = bottom.saturating_sub(top);
        let mut pixels = Vec::with_capacity(w * h);
        for row in top..bottom {
            let start = row * self.coords.width + left;
            pixels.extend_from_slice(&self.pixels[start..start + w]);
        }
        (pixels, left, top, w, h)
    }

    pub fn restore_region(&mut self, pixels: &[u32], px_x: usize, px_y: usize, px_w: usize, px_h: usize) {
        let canvas_w = self.coords.width;
        for row in 0..px_h {
            let dst_y = px_y + row;
            if dst_y >= self.coords.height { break; }
            let dst_start = dst_y * canvas_w + px_x;
            let src_start = row * px_w;
            if src_start + px_w <= pixels.len() && dst_start + px_w <= self.pixels.len() {
                self.pixels[dst_start..dst_start + px_w].copy_from_slice(&pixels[src_start..src_start + px_w]);
            }
        }
    }

    // ── Blinkey cursor (α+darkness: brighten = subtract darkness) ─────────

    /// Cursor brightness peak (darkness units subtracted at the wave crest).
    const BLINKEY_BRIGHTNESS: i32 = 100;
    /// Horizontal smear half-width in pixels.
    const BLINKEY_SMEAR: i32 = 7;

    /// Apply the blinkey wave over a textbox column. `top_bright` picks the wave shape;
    /// `brighten` subtracts darkness (show cursor) vs adds it back (erase).
    fn blinkey(&mut self, px_x: usize, px_y: usize, px_h: usize, top_bright: bool, brighten: bool) {
        let w = self.coords.width;
        let h = self.coords.height;
        if px_h < 2 { return; }
        let half = px_h as i32 / 2;
        for row in 0..px_h {
            let y = px_y + row;
            if y >= h { break; }
            let t_1024 = (row as i32 - half) * 1024 / half;
            let one_minus_t2 = 1024 - (t_1024 * t_1024 / 1024);
            let shaped = if top_bright { 1024 - t_1024 } else { 1024 + t_1024 };
            let wave = one_minus_t2 * shaped / 1024 * shaped / 1024;
            let bright = (wave * Self::BLINKEY_BRIGHTNESS / 1024).max(0) as u32;
            if bright == 0 { continue; }
            for dx in -Self::BLINKEY_SMEAR..=Self::BLINKEY_SMEAR {
                let x = px_x as i32 + dx;
                if x < 0 || x as usize >= w { continue; }
                let idx = y * w + x as usize;
                let k = bright >> dx.unsigned_abs();
                if k == 0 { continue; }
                let p = self.pixels[idx];
                let a = p & 0xFF00_0000;
                let (r, g, b) = ((p >> 16) & 0xFF, (p >> 8) & 0xFF, p & 0xFF);
                let (r, g, b) = if brighten {
                    (r.saturating_sub(k), g.saturating_sub(k), b.saturating_sub(k))
                } else {
                    ((r + k).min(255), (g + k).min(255), (b + k).min(255))
                };
                self.pixels[idx] = a | (r << 16) | (g << 8) | b;
            }
        }
    }

    pub fn blinkey_add_top(&mut self, px_x: usize, px_y: usize, px_h: usize) { self.blinkey(px_x, px_y, px_h, true, true); }
    pub fn blinkey_add_bottom(&mut self, px_x: usize, px_y: usize, px_h: usize) { self.blinkey(px_x, px_y, px_h, false, true); }
    pub fn blinkey_sub_top(&mut self, px_x: usize, px_y: usize, px_h: usize) { self.blinkey(px_x, px_y, px_h, true, false); }
    pub fn blinkey_sub_bottom(&mut self, px_x: usize, px_y: usize, px_h: usize) { self.blinkey(px_x, px_y, px_h, false, false); }
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use spirix::ScalarF4E4;
    use vsf::types::VsfType;

    // spirix's Zero is a special ambiguous pattern; `from_f32(0.0)` does NOT produce it (yields an
    // undefined value → NaN downstream), so route exact zeros through the ZERO constant.
    fn s(v: f32) -> ScalarF4E4 {
        if v == 0.0 { ScalarF4E4::ZERO } else { ScalarF4E4::from_f32(v) }
    }
    fn c44(x: f32, y: f32) -> CircleF4E4 {
        CircleF4E4::from((s(x), s(y)))
    }
    fn px(rgba: &[u8], w: usize, x: usize, y: usize) -> (u8, u8, u8) {
        let i = (y * w + x) * 4;
        (rgba[i], rgba[i + 1], rgba[i + 2])
    }

    /// Filled rect over an opaque background lands its fill colour at the centre and leaves the bg
    /// at the corner. Proves the painter-over composite (scratch → main) works against opaque bg.
    #[test]
    fn fill_rect_center_and_bg() {
        let mut c = Canvas::new_fast(128, 128);
        c.clear(&VsfType::rck).unwrap(); // opaque black bg
        c.fill_rect_ru(c44(0.0, 0.0), c44(0.4, 0.4), &VsfType::ra([255, 0, 0, 255])).unwrap();
        let rgba = c.to_rgba_bytes();
        let (r, g, b) = px(&rgba, 128, 64, 64);
        assert!(r >= 254 && g <= 1 && b <= 1, "centre is red, got ({r},{g},{b})");
        assert_eq!(px(&rgba, 128, 2, 2), (0, 0, 0), "corner is black bg");
    }

    /// A circle inks its centre and leaves a far corner clear.
    #[test]
    fn fill_circle_center() {
        let mut c = Canvas::new_fast(128, 128);
        c.clear(&VsfType::rck).unwrap();
        c.fill_circle(c44(0.0, 0.0), ScalarF4E4::from_f32(0.3), &VsfType::ra([0, 255, 0, 255])).unwrap();
        let rgba = c.to_rgba_bytes();
        let (r, g, b) = px(&rgba, 128, 64, 64);
        assert!(g >= 254 && r <= 1 && b <= 1, "centre is green, got ({r},{g},{b})");
        assert_eq!(px(&rgba, 128, 2, 2), (0, 0, 0), "corner untouched");
    }

    /// Later draws land on top (painter's order) — a blue rect drawn after a red one wins the overlap.
    #[test]
    fn later_draw_wins_overlap() {
        let mut c = Canvas::new_fast(128, 128);
        c.clear(&VsfType::rck).unwrap();
        c.fill_rect_ru(c44(0.0, 0.0), c44(0.5, 0.5), &VsfType::ra([255, 0, 0, 255])).unwrap();
        c.fill_rect_ru(c44(0.0, 0.0), c44(0.3, 0.3), &VsfType::ra([0, 0, 255, 255])).unwrap();
        let (_, _, b) = px(&c.to_rgba_bytes(), 128, 64, 64);
        assert!(b >= 254, "later blue rect is on top, got b={b}");
    }

    /// A horizontal hairline inks its row and leaves rows a few pixels away clear.
    #[test]
    fn hline_inks_one_row() {
        let mut c = Canvas::new_fast(128, 128);
        c.clear(&VsfType::rck).unwrap();
        c.hline_ru(ScalarF4E4::ZERO, ScalarF4E4::from_f32(-0.4), ScalarF4E4::from_f32(0.4), &VsfType::rcb).unwrap();
        let rgba = c.to_rgba_bytes();
        let (_, _, b_on) = px(&rgba, 128, 64, 64);
        assert!(b_on >= 200, "hairline row is blue-inked, got b={b_on}");
        assert_eq!(px(&rgba, 128, 64, 60), (0, 0, 0), "four rows away is clean bg");
    }
}
