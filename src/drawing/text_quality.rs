//! Text rendering for CanvasQuality using fontdue-spirix Layout engine
//!
//! Same alignment semantics as text_fast — see that module for details.

use crate::drawing::canvas_quality::{CanvasQuality, Pixel};
use crate::drawing::shared::TextSettings;
use crate::vm::FontCache;
use fontdue::layout::{CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle};
use fontdue::Font as FontdueFont;
use spirix::{CircleF4E4, ScalarF4E4};

impl CanvasQuality {
    /// Draw text onto the canvas using fontdue's Layout engine.
    pub fn draw_text(
        &mut self,
        font_cache: &mut FontCache,
        font_key: [u8; 32],
        font_bytes: &[u8],
        pos: CircleF4E4,
        size: ScalarF4E4,
        text: &str,
        colour: Pixel,
        settings: &TextSettings,
    ) {
        let cache_key = settings.font_cache_key(font_key);
        let font = font_cache.entry(cache_key).or_insert_with(|| {
            FontdueFont::from_bytes(font_bytes, fontdue::FontSettings::default())
                .expect("draw_text: invalid font bytes")
        });

        let px = size * self.span() * self.ru();
        if !px.is_positive() { return; }

        let anchor_x = self.ru_to_px_x(pos.r());
        let anchor_y = self.ru_to_px_y(pos.i());
        let canvas_w = self.width() as isize;
        let canvas_h = self.height() as isize;

        // Layout starts at anchor_y; shift_y (computed after layout) will
        // correct vertical position to center the full text block on anchor_y.
        let baseline_y = ScalarF4E4::from(anchor_y);

        let h_align = match settings.align {
            1 => HorizontalAlign::Left,
            2 => HorizontalAlign::Right,
            _ => HorizontalAlign::Center,
        };

        let wrap_px = settings.wrap.map(|w| w * self.span() * self.ru());

        // Compute layout origin x based on alignment + wrap
        let anchor_x_s = ScalarF4E4::from(anchor_x);
        let (layout_x, use_fontdue_align) = if let Some(w) = wrap_px {
            let x = match settings.align {
                1 => anchor_x_s,
                2 => anchor_x_s - w,
                _ => anchor_x_s - (w >> 1usize),
            };
            (x, true)
        } else {
            (anchor_x_s, false)
        };

        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        let layout_settings = LayoutSettings {
            x: layout_x,
            y: baseline_y,
            max_width: wrap_px,
            horizontal_align: if use_fontdue_align { h_align } else { HorizontalAlign::Left },
            line_height: settings.leading,
            ..LayoutSettings::default()
        };
        layout.reset(&layout_settings);
        layout.append(&[font as &FontdueFont], &TextStyle::new(text, px, 0));

        // Compute vertical shift to center entire text block on anchor_y.
        let glyphs = layout.glyphs();
        let shift_y = if glyphs.is_empty() {
            ScalarF4E4::ZERO
        } else {
            let mut min_y = glyphs[0].y;
            let mut max_y = glyphs[0].y + (glyphs[0].height);
            for g in glyphs.iter().skip(1) {
                if g.y < min_y { min_y = g.y; }
                let bottom = g.y + (g.height);
                if bottom > max_y { max_y = bottom; }
            }
            let actual_h = max_y - min_y;
            ScalarF4E4::from(anchor_y) - (actual_h >> 1usize) - min_y
        };

        // Global shift for no-wrap center/right alignment
        let shift_x = if use_fontdue_align || settings.align == 1 {
            ScalarF4E4::ZERO
        } else {
            if glyphs.is_empty() {
                ScalarF4E4::ZERO
            } else {
                let mut min_x = glyphs[0].x;
                let mut max_x = glyphs[0].x + glyphs[0].width;
                for g in glyphs.iter().skip(1) {
                    if g.x < min_x { min_x = g.x; }
                    let end = g.x + g.width;
                    if end > max_x { max_x = end; }
                }
                let w = max_x - min_x;
                match settings.align {
                    2 => -w,
                    _ => -(w >> 1usize),
                }
            }
        };

        let fonts = [font as &FontdueFont];
        for glyph in layout.glyphs() {
            if glyph.width == 0 || glyph.height == 0 { continue; }
            if !glyph.char_data.rasterize() { continue; }

            let (metrics, bitmap) = fonts[glyph.font_index].rasterize_config(glyph.key);
            let glyph_w = metrics.width as isize;
            let glyph_h = metrics.height as isize;
            let gx = (glyph.x + shift_x).floor().to_isize();
            let gy = (glyph.y + shift_y).floor().to_isize();

            let row_start = ((-gy).max(0)) as isize;
            let row_end = ((canvas_h - gy).min(glyph_h)) as isize;
            let col_start = ((-gx).max(0)) as isize;
            let col_end = ((canvas_w - gx).min(glyph_w)) as isize;

            for row in row_start..row_end {
                let py = (gy + row) as usize;
                let row_offset = row * glyph_w;
                for col in col_start..col_end {
                    let coverage = bitmap[(row_offset + col) as usize];
                    if coverage == 0 { continue; }
                    let alpha = ScalarF4E4::from(coverage) >> 8usize;
                    let inv = ScalarF4E4::ONE - alpha;
                    let idx = py * canvas_w as usize + (gx + col) as usize;
                    let bg = self.pixels_mut()[idx];
                    self.pixels_mut()[idx] = [
                        colour[0] * alpha + bg[0] * inv,
                        colour[1] * alpha + bg[1] * inv,
                        colour[2] * alpha + bg[2] * inv,
                        colour[3] * alpha + bg[3] * inv,
                    ];
                }
            }
        }
    }
}

