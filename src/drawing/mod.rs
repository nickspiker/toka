//! Drawing layer — single fluor-backed pipeline, front-to-back.
//!
//! `Canvas` owns an α+darkness pixel buffer (`0xααRRGGBB`, RGB = 255−visible) and draws into it
//! with fluor's paint primitives + `TextRenderer`, in fluor's native order: **front-to-back**.
//! The buffer starts EMPTY (`0x00000000`); content paints frontmost-first (cell text → grid lines
//! → row fills — see `VM::render_table`), each `under`-blending BEHIND what's already there with
//! the opaque early-out skipping occluded pixels. The photon "liquid stone" noise backdrop
//! ([`fluor::paint::background_noise`]) lands last of all in [`Canvas::to_rgba_bytes`], under-
//! compositing behind every remaining empty/translucent pixel. This mirrors photon's fluor-native
//! GUI (`PhotonApp::render`): widgets and watermarks first, background noise as the final pass.
//!
//! Coordinates: RU math stays in [`shared::RuCoords`] (spirix, center-origin, harmonic-mean span);
//! each primitive converts to f32 pixel space at the fluor boundary via the `ru_to_px_*f` helpers.
//! Colour funnels through [`crate::renderer::extract_colour_u32`] → fluor α+darkness. Output flips
//! back to visible RGBA in [`Canvas::to_rgba_bytes`].

pub mod shared;

pub use shared::RuCoords;
pub use shared::{BlendMode, LineSettings, TableSettings, TextSettings};

use crate::vm::FontCache;
use fluor::text::TextStyle;
use shared::RuCoords as Coords;
use spirix::{CircleF4E4, ScalarF4E4};
use std::collections::HashMap;

use fluor::canvas::{Canvas as FCanvas, Damage};
use fluor::paint::{self, Clip, HitId};
use fluor::pixel::Blend;
use fluor::text::TextRenderer;
use fluor::widgets::button::Button as FButton;
use fluor::widgets::textbox::Textbox as FTextbox;

/// Empty pixel — no opacity, no darkness. The canvas resets to this; the noise backdrop
/// under-fills whatever content leaves empty.
const EMPTY: u32 = 0x0000_0000;

/// Single fluor-backed canvas.
pub struct Canvas {
    coords: Coords,
    /// α+darkness pixel buffer, composited front-to-back.
    pixels: Vec<u32>,
    /// fluor text engine (owns its cosmic-text font system + glyph cache).
    text: TextRenderer,
    /// Capsule-shipped fonts: `font_key` → registered family name in fluor's font DB.
    fonts: HashMap<[u8; 32], String>,
    /// fluor widgets keyed by the VM's widget id — persist across frames so their pill/text
    /// caches survive and (for textboxes) blinkey/scroll state carries over.
    buttons: HashMap<u32, FButton>,
    textboxes: HashMap<u32, FTextbox>,
    /// Dense fluor hit-id allocator for the widgets above (unused for routing now — see `hit_map`).
    hit_counter: HitId,
    /// Per-pixel widget silhouette map, parallel to `pixels`. Each interactive widget stamps its
    /// VM widget id (cast to `HitId`) here as fluor paints its true pill/textbox silhouette, so
    /// `hit_map[y*w + x]` is the source of truth for "what's under this pixel" — the same model the
    /// desktop/Photon fluor host uses. `0` = nothing. Cleared within the clip band each frame and
    /// shifted alongside `pixels` on scroll, so a scrolled widget's hit region rides with its pixels.
    hit_map: Vec<HitId>,
    /// Saved cursor-strip pixels per focused textbox: (pixels, x, y, w, h). Captured after the
    /// textbox content renders but before the blinkey wave, so a blink flip can restore the
    /// strip and repaint the alternate wave without a full frame rerun.
    cursor_snaps: HashMap<u32, (Vec<u32>, usize, usize, usize, usize)>,
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
            pixels: vec![EMPTY; width * height],
            text: TextRenderer::new(),
            fonts: HashMap::new(),
            buttons: HashMap::new(),
            textboxes: HashMap::new(),
            hit_counter: 1,
            hit_map: vec![0; width * height],
            cursor_snaps: HashMap::new(),
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

    /// Reset the (clip band of the) canvas to EMPTY for a fresh front-to-back pass. The capsule's
    /// requested clear colour is superseded by the photon noise backdrop painted at output time —
    /// one look across the whole fluor stack.
    pub fn clear(&mut self, _colour: &vsf::VsfType) -> Result<(), String> {
        let w = self.coords.width;
        let y0 = self.coords.clip_y_min;
        let y1 = self.coords.clip_y_max;
        self.pixels[y0 * w..y1 * w].fill(EMPTY);
        // Widget silhouettes are re-stamped as widgets repaint this frame — clear the same band.
        self.hit_map[y0 * w..y1 * w].fill(0);
        // Frame reset: cursor snapshots reference the old frame's pixels.
        self.cursor_snaps.clear();
        Ok(())
    }

    /// Final composite + output: under-paint the photon "liquid stone" noise backdrop behind all
    /// content (idempotent — pixels already opaque early-out), then flip α+darkness → visible RGBA
    /// for the browser's ImageData.
    ///
    /// The noise scrolls with content (photon behaviour): `scroll_offset` shifts which logical row
    /// seeds each screen row, so the exposed strip after a scroll regenerates pattern-continuous
    /// rows rather than restarting at the screen edge.
    pub fn to_rgba_bytes(&mut self) -> Vec<u8> {
        let (w, h) = (self.coords.width, self.coords.height);
        let scroll_px = self.coords.ru_to_px_h(self.coords.scroll_y);
        {
            let mut dmg = Damage::new();
            let mut fc = FCanvas::new(&mut self.pixels, w, h, &mut dmg);
            paint::background_noise(&mut fc, 0, true, scroll_px, None, None);
        }
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

    // ── Geometry primitives (fluor paint, direct under-blend, f32 pixel space) ──

    pub fn fill_rect_ru(&mut self, pos: CircleF4E4, size: CircleF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        let c = crate::renderer::extract_colour_u32(colour)?;
        let cx = self.coords.ru_to_px_xf(pos.r());
        let cy = self.coords.ru_to_px_yf(pos.i());
        let w = self.coords.ru_to_px_wf(size.r());
        let h = self.coords.ru_to_px_hf(size.i());
        let clip = self.clip();
        let (bw, bh) = (self.coords.width, self.coords.height);
        let mut dmg = Damage::new();
        let mut fc = FCanvas::new(&mut self.pixels, bw, bh, &mut dmg);
        paint::draw_rect(&mut fc, cx, cy, w, h, c, clip);
        Ok(())
    }

    /// Blit a decoded image (α + darkness pixels, row-major `src_w × src_h`) scaled into the RU
    /// rect at `pos` with `size`. Mirrors `fill_rect_ru`'s RU→px + FCanvas plumbing; the scale +
    /// UNDER-composite live in `paint::draw_image`.
    pub fn blit_image_ru(&mut self, pos: CircleF4E4, size: CircleF4E4, src: &[u32], src_w: usize, src_h: usize) -> Result<(), String> {
        let cx = self.coords.ru_to_px_xf(pos.r());
        let cy = self.coords.ru_to_px_yf(pos.i());
        let w = self.coords.ru_to_px_wf(size.r());
        let h = self.coords.ru_to_px_hf(size.i());
        let clip = self.clip();
        let (bw, bh) = (self.coords.width, self.coords.height);
        let mut dmg = Damage::new();
        let mut fc = FCanvas::new(&mut self.pixels, bw, bh, &mut dmg);
        paint::draw_image(&mut fc, src, src_w, src_h, cx, cy, w, h, clip);
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
        let (bw, bh) = (self.coords.width, self.coords.height);
        let mut dmg = Damage::new();
        let mut fc = FCanvas::new(&mut self.pixels, bw, bh, &mut dmg);
        paint::draw_rect(&mut fc, cx, py, w, 0.0, c, clip);
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
        let (bw, bh) = (self.coords.width, self.coords.height);
        let mut dmg = Damage::new();
        let mut fc = FCanvas::new(&mut self.pixels, bw, bh, &mut dmg);
        paint::draw_rect(&mut fc, px, cy, 0.0, h, c, clip);
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
        let clip = self.clip();
        let (bw, bh) = (self.coords.width, self.coords.height);
        let mut dmg = Damage::new();
        let mut fc = FCanvas::new(&mut self.pixels, bw, bh, &mut dmg);
        paint::stroke_rect(&mut fc, x, yy, w.round() as isize, h.round() as isize, 1, c, clip, None);
        Ok(())
    }

    pub fn fill_rotated_rect_ru(&mut self, pos: CircleF4E4, size: CircleF4E4, angle: ScalarF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        let c = crate::renderer::extract_colour_u32(colour)?;
        let cx = self.coords.ru_to_px_xf(pos.r());
        let cy = self.coords.ru_to_px_yf(pos.i());
        let w = self.coords.ru_to_px_wf(size.r());
        let h = self.coords.ru_to_px_hf(size.i());
        let ang = angle.to_f32();
        let clip = self.clip();
        let (bw, bh) = (self.coords.width, self.coords.height);
        let mut dmg = Damage::new();
        let mut fc = FCanvas::new(&mut self.pixels, bw, bh, &mut dmg);
        paint::draw_rect_rotated(&mut fc, cx, cy, w, h, ang, c, clip);
        Ok(())
    }

    pub fn fill_circle(&mut self, center: CircleF4E4, radius: ScalarF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        let c = crate::renderer::extract_colour_u32(colour)?;
        let cx = self.coords.ru_to_px_xf(center.r());
        let cy = self.coords.ru_to_px_yf(center.i());
        let r = self.coords.ru_to_px_wf(radius);
        let clip = self.clip();
        let (bw, bh) = (self.coords.width, self.coords.height);
        let mut dmg = Damage::new();
        let mut fc = FCanvas::new(&mut self.pixels, bw, bh, &mut dmg);
        paint::draw_circle(&mut fc, cx, cy, r, c, clip);
        Ok(())
    }

    pub fn fill_ellipse(&mut self, center: CircleF4E4, radii: CircleF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        let c = crate::renderer::extract_colour_u32(colour)?;
        let cx = self.coords.ru_to_px_xf(center.r());
        let cy = self.coords.ru_to_px_yf(center.i());
        let rx = self.coords.ru_to_px_wf(radii.r());
        let ry = self.coords.ru_to_px_hf(radii.i());
        let clip = self.clip();
        let (bw, bh) = (self.coords.width, self.coords.height);
        let mut dmg = Damage::new();
        let mut fc = FCanvas::new(&mut self.pixels, bw, bh, &mut dmg);
        paint::draw_ellipse(&mut fc, cx, cy, rx, ry, c, clip);
        Ok(())
    }

    /// Ellipse outline. fluor has no stroke-ellipse primitive; approximate as a filled ellipse
    /// (the VSF renderer currently rejects strokes upstream, so this path is effectively unused).
    pub fn stroke_ellipse(&mut self, center: CircleF4E4, radii: CircleF4E4, _stroke_width: ScalarF4E4, colour: &vsf::VsfType) -> Result<(), String> {
        self.fill_ellipse(center, radii, colour)
    }

    /// General line via a thin rotated rect (fluor's line = degenerate rect).
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
        let (bw, bh) = (self.coords.width, self.coords.height);
        let mut dmg = Damage::new();
        let mut fc = FCanvas::new(&mut self.pixels, bw, bh, &mut dmg);
        paint::draw_rect_rotated(&mut fc, cx, cy, len, weight, ang, c, clip);
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
        // Split borrows: `text` and `pixels` are disjoint fields.
        let Canvas { text: engine, pixels, .. } = self;
        let mut dmg = Damage::new();
        let mut fc = FCanvas::new(pixels, w, h, &mut dmg);
        // fluor's draw_text_* centre a SINGLE line on `y` (its height measure is the first
        // layout run only), so multi-line blocks must be split here and block-centred: line i
        // of n draws at cy − (n−1−2i)·line_h/2. line_h matches fluor's Metrics::relative ratio.
        let lines: Vec<&str> = text.split('\n').collect();
        let n = lines.len();
        let line_h = px * 1.2;
        for (i, line) in lines.into_iter().enumerate() {
            if line.is_empty() { continue; }
            let ly = cy - (n as f32 - 1.0 - 2.0 * i as f32) * line_h * 0.5;
            match settings.align {
                1 => { engine.draw_text_left(&mut fc, line, cx, ly, &TextStyle::new(px, c).weight(weight).font(&family), clip, None); }
                2 => { engine.draw_text_right(&mut fc, line, cx, ly, &TextStyle::new(px, c).weight(weight).font(&family), clip, None); }
                _ => { engine.draw_text_center(&mut fc, line, cx, ly, &TextStyle::new(px, c).weight(weight).font(&family), clip, None); }
            }
        }
        Ok(())
    }

    // ── Scroll / layers / regions ────────────────────────────────────────

    /// Shift the buffer by `delta_y` rows; the exposed strip resets to EMPTY (`bg` is ignored —
    /// the noise backdrop under-fills the strip at output, pattern-continuous via scroll offset).
    pub fn scroll_pixels(&mut self, delta_y: isize, _bg: u32) {
        let h = self.coords.height;
        let w = self.coords.width;
        let d = delta_y.unsigned_abs().min(h);
        if d == 0 { return; }
        if d >= h {
            self.pixels.fill(EMPTY);
            self.hit_map.fill(0);
            return;
        }
        // Shift the hit_map in lockstep with the pixels so a widget's hit region rides along with
        // its silhouette; the exposed strip is re-stamped by the clipped rerun that follows.
        if delta_y > 0 {
            self.pixels.copy_within(d * w..h * w, 0);
            self.pixels[((h - d) * w)..].fill(EMPTY);
            self.hit_map.copy_within(d * w..h * w, 0);
            self.hit_map[((h - d) * w)..].fill(0);
        } else {
            self.pixels.copy_within(0..(h - d) * w, d * w);
            self.pixels[..d * w].fill(EMPTY);
            self.hit_map.copy_within(0..(h - d) * w, d * w);
            self.hit_map[..d * w].fill(0);
        }
    }

    /// Transparent layer matching this canvas's dimensions + RU state (for opacity groups).
    pub fn new_layer(&self) -> Canvas {
        let mut coords = Coords::new(self.coords.width, self.coords.height);
        coords.set_ru(self.coords.ru());
        coords.set_scroll_y(self.coords.scroll_y);
        Canvas {
            coords,
            pixels: vec![EMPTY; self.coords.width * self.coords.height],
            text: TextRenderer::new(),
            fonts: HashMap::new(),
            buttons: HashMap::new(),
            textboxes: HashMap::new(),
            hit_counter: 1,
            hit_map: vec![0; self.coords.width * self.coords.height],
            cursor_snaps: HashMap::new(),
        }
    }

    /// Composite a finished layer into the front-to-back stream: the layer goes BEHIND existing
    /// content and in front of anything drawn after (`layer.under(main)`), scaled by opacity.
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

    // ── Hit-testing (per-pixel silhouette map) ──────────────────────────

    /// Sample the widget silhouette map at a pixel. `0` = nothing interactive under that pixel.
    pub fn hit_at_px(&self, px_x: usize, px_y: usize) -> HitId {
        if px_x >= self.coords.width || px_y >= self.coords.height {
            return 0;
        }
        self.hit_map[px_y * self.coords.width + px_x]
    }

    /// Resolve a screen-space RU point (center-origin, scroll NOT baked in — exactly what the host
    /// delivers from `pageToRU`) to the widget id stamped under it, or `None`. Goes straight to
    /// screen pixel space and samples `hit_map`, so there's no scroll/rect arithmetic to drift and
    /// the hit matches the pixels actually on screen.
    pub fn hit_at_screen_ru(&self, x: ScalarF4E4, y: ScalarF4E4) -> Option<u32> {
        let px = self.coords.ru_to_px_x(x); // x axis carries no scroll term
        let py = self.coords.ru_to_px_y_screen(y);
        if px < 0 || py < 0 {
            return None;
        }
        match self.hit_at_px(px as usize, py as usize) {
            0 => None,
            id => Some(id as u32),
        }
    }

    // ── fluor widgets (buttons / textboxes) ─────────────────────────────

    /// Paint a fluor `Button` for VM widget `id` at the RU rect. Widget state (pill/text caches)
    /// persists across frames keyed by id. As fluor paints the pill it stamps `id` into `hit_map`
    /// at every silhouette pixel — that map is the hit-test source of truth (see [`Self::hit_at_px`]).
    /// `fill` overrides the resting pill colour (`None` = fluor's default theme fill); `hovered` /
    /// `pressed` fold the state colour into the baked fill (headless mode — no host overlay pass).
    pub fn draw_widget_button(&mut self, id: u32, pos: CircleF4E4, size: CircleF4E4, label: &str, fill: Option<u32>, hovered: bool, pressed: bool) -> Result<(), String> {
        let cx = self.coords.ru_to_px_xf(pos.r());
        let cy = self.coords.ru_to_px_yf(pos.i());
        let w = self.coords.ru_to_px_wf(size.r());
        let h = self.coords.ru_to_px_hf(size.i());
        let fs = h * (2.0 / 3.0);
        let clip = self.clip();
        let (bw, bh) = (self.coords.width, self.coords.height);
        let Canvas { pixels, text: engine, buttons, hit_counter, hit_map, .. } = self;
        let btn = buttons
            .entry(id)
            .or_insert_with(|| FButton::new(hit_counter, cx, cy, w, h, fs, label));
        btn.set_rect(cx, cy, w, h);
        btn.set_font_size(fs);
        // Headless: bake hover/pressed into the fill (fluor's host overlay pass isn't running here).
        btn.set_bake_states(true);
        btn.set_fill(fill);
        btn.set_hovered(hovered);
        btn.set_pressed(pressed);
        if btn.label() != label {
            btn.set_label(label);
        }
        let mut dmg = Damage::new();
        let mut fc = FCanvas::new(pixels, bw, bh, &mut dmg);
        btn.render_content_into(&mut fc, 0.0, 0.0, engine, clip, Some(hit_map.as_mut_slice()), id as HitId);
        Ok(())
    }

    /// Paint a fluor `Textbox` for VM widget `id`, syncing content/cursor/focus from the VM's
    /// input state (the VM stays the source of truth; the widget is the renderer). When focused,
    /// snapshots the cursor strip (post-content, pre-wave) so [`Self::flip_textbox_blinkey`] can
    /// animate the cursor without a frame rerun, then paints the wave.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_widget_textbox(
        &mut self,
        id: u32,
        pos: CircleF4E4,
        size: CircleF4E4,
        font_size: ScalarF4E4,
        content: &str,
        cursor: usize,
        is_focused: bool,
        placeholder: &str,
    ) -> Result<(), String> {
        let cx = self.coords.ru_to_px_xf(pos.r());
        let cy = self.coords.ru_to_px_yf(pos.i());
        let w = self.coords.ru_to_px_wf(size.r());
        let h = self.coords.ru_to_px_hf(size.i());
        let fs = self.coords.ru_to_px_hf(font_size);
        let clip = self.clip();
        let (bw, bh) = (self.coords.width, self.coords.height);
        let Canvas { pixels, text: engine, textboxes, hit_counter, hit_map, cursor_snaps, .. } = self;
        let tb = textboxes
            .entry(id)
            .or_insert_with(|| FTextbox::new(hit_counter, cx, cy, w, h, fs));
        tb.set_rect(cx, cy, w, h);
        tb.set_font_size(fs, engine);
        let current: String = tb.chars.iter().collect();
        if current != content {
            tb.clear();
            tb.insert_str(content, engine);
        }
        tb.cursor = cursor.min(tb.chars.len());
        tb.set_focused(is_focused);
        if is_focused {
            tb.blinkey_visible = true;
        }

        let mut dmg = Damage::new();
        let mut fc = FCanvas::new(pixels, bw, bh, &mut dmg);
        // Placeholder: faint text drawn FIRST (frontmost) when empty + unfocused; the pill
        // interior paints under it. fluor Textbox has no placeholder concept of its own.
        if content.is_empty() && !is_focused && !placeholder.is_empty() {
            let ph = paint::pack_argb(160, 160, 160, 140);
            let tl = tb.text_left();
            engine.draw_text_left(&mut fc, placeholder, tl, cy, &TextStyle::new(fs, ph), clip, None);
        }
        // Stamp `id` into hit_map at the textbox silhouette as fluor paints the pill.
        tb.render_content_into(&mut fc, 0.0, 0.0, engine, clip, None, Some(hit_map.as_mut_slice()), id as HitId);

        if is_focused {
            // Snapshot the cursor strip AFTER content, BEFORE the wave — the blink flip
            // restores this and repaints the alternate wave.
            let bb = tb.cursor_bbox();
            let sx = (bb.x.floor().max(0.0) as usize).min(bw);
            let sy = (bb.y.floor().max(0.0) as usize).min(bh);
            let sw = (bb.w.ceil() as usize).min(bw - sx);
            let sh = (bb.h.ceil() as usize).min(bh - sy);
            if sw > 0 && sh > 0 {
                let mut snap = Vec::with_capacity(sw * sh);
                for row in sy..sy + sh {
                    snap.extend_from_slice(&fc.pixels[row * bw + sx..row * bw + sx + sw]);
                }
                cursor_snaps.insert(id, (snap, sx, sy, sw, sh));
            }
            tb.render_blinkey_into(&mut fc, 0.0, 0.0);
        } else {
            cursor_snaps.remove(&id);
        }
        Ok(())
    }

    /// Blink tick for the focused textbox: restore the saved cursor strip, flip fluor's wave
    /// state (alternating top/bottom-bright), repaint. No-op without a focused textbox + snapshot.
    pub fn flip_textbox_blinkey(&mut self, id: u32) {
        let (bw, bh) = (self.coords.width, self.coords.height);
        let Canvas { pixels, textboxes, cursor_snaps, .. } = self;
        let Some(tb) = textboxes.get_mut(&id) else { return };
        if !tb.is_focused() {
            return;
        }
        let Some((snap, sx, sy, sw, sh)) = cursor_snaps.get(&id) else { return };
        for row in 0..*sh {
            let dst = (sy + row) * bw + sx;
            pixels[dst..dst + sw].copy_from_slice(&snap[row * sw..(row + 1) * sw]);
        }
        tb.flip_blinkey();
        let mut dmg = Damage::new();
        let mut fc = FCanvas::new(pixels, bw, bh, &mut dmg);
        tb.render_blinkey_into(&mut fc, 0.0, 0.0);
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

    // NOTE: on spirix 0.0.12 (toka's pinned crates.io version) `from_f32(0.0)` does NOT yield a
    // clean zero — downstream `to_f32()` produces NaN. The local /mnt/Octopus/Code/spirix tree
    // fixed this, but toka is pinned to 0.0.12. Route exact zeros through ZERO.
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
    /// The noise backdrop is dark (photon BG_BASE + masked walk) — every channel stays low.
    fn is_backdrop((r, g, b): (u8, u8, u8)) -> bool {
        r < 0x40 && g < 0x40 && b < 0x40
    }

    /// A filled rect on the empty canvas is opaque at its centre; untouched corners get the photon
    /// noise backdrop at output. Proves the front-to-back chain end to end.
    #[test]
    fn fill_rect_center_and_backdrop() {
        let mut c = Canvas::new_fast(128, 128);
        c.fill_rect_ru(c44(0.0, 0.0), c44(0.4, 0.4), &VsfType::ra([255, 0, 0, 255])).unwrap();
        let rgba = c.to_rgba_bytes();
        let (r, g, b) = px(&rgba, 128, 64, 64);
        assert!(r >= 254 && g <= 1 && b <= 1, "centre is red, got ({r},{g},{b})");
        assert!(is_backdrop(px(&rgba, 128, 2, 2)), "corner is dark noise backdrop, got {:?}", px(&rgba, 128, 2, 2));
    }

    /// Front-to-back: the FIRST draw is frontmost — a later overlapping draw lands behind it.
    #[test]
    fn earlier_draw_wins_overlap() {
        let mut c = Canvas::new_fast(128, 128);
        c.fill_rect_ru(c44(0.0, 0.0), c44(0.3, 0.3), &VsfType::ra([0, 0, 255, 255])).unwrap();
        c.fill_rect_ru(c44(0.0, 0.0), c44(0.5, 0.5), &VsfType::ra([255, 0, 0, 255])).unwrap();
        let rgba = c.to_rgba_bytes();
        let (_, _, b) = px(&rgba, 128, 64, 64);
        assert!(b >= 254, "earlier blue rect stays in front, got b={b}");
        // span = 64 px/RU: red 0.5 RU → x∈[48,80); blue 0.3 RU → x∈[54.4,73.6). x=50 is red-only.
        let (r_edge, _, _) = px(&rgba, 128, 50, 64);
        assert!(r_edge >= 254, "later red shows where blue doesn't cover, got r={r_edge}");
    }

    /// A circle inks its centre; corners stay backdrop.
    #[test]
    fn fill_circle_center() {
        let mut c = Canvas::new_fast(128, 128);
        c.fill_circle(c44(0.0, 0.0), ScalarF4E4::from_f32(0.3), &VsfType::ra([0, 255, 0, 255])).unwrap();
        let rgba = c.to_rgba_bytes();
        let (r, g, b) = px(&rgba, 128, 64, 64);
        assert!(g >= 254 && r <= 1 && b <= 1, "centre is green, got ({r},{g},{b})");
        assert!(is_backdrop(px(&rgba, 128, 2, 2)), "corner untouched → backdrop");
    }

    /// A horizontal hairline inks its row; rows a few pixels away are backdrop.
    #[test]
    fn hline_inks_one_row() {
        let mut c = Canvas::new_fast(128, 128);
        c.hline_ru(ScalarF4E4::ZERO, s(-0.4), s(0.4), &VsfType::ra([0, 0, 255, 255])).unwrap();
        let rgba = c.to_rgba_bytes();
        let (_, _, b_on) = px(&rgba, 128, 64, 64);
        assert!(b_on >= 200, "hairline row is blue-inked, got b={b_on}");
        assert!(is_backdrop(px(&rgba, 128, 64, 60)), "four rows away is backdrop");
    }

    /// clear() resets to empty — output becomes pure backdrop everywhere.
    #[test]
    fn clear_resets_to_backdrop() {
        let mut c = Canvas::new_fast(128, 128);
        c.fill_rect_ru(c44(0.0, 0.0), c44(0.5, 0.5), &VsfType::ra([255, 0, 0, 255])).unwrap();
        c.clear(&VsfType::rck).unwrap();
        let rgba = c.to_rgba_bytes();
        assert!(is_backdrop(px(&rgba, 128, 64, 64)), "cleared centre is backdrop");
    }
}
