//! Text rendering for CanvasFast using fontdue-spirix Layout engine

use crate::drawing::canvas_fast::CanvasFast;
use crate::drawing::shared::TextSettings;
use crate::vm::FontCache;
use fontdue::layout::{CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle};
use fontdue::Font as FontdueFont;
use spirix::{CircleF4E4, ScalarF4E4};

impl CanvasFast {
    /// Draw text onto the canvas using fontdue's Layout engine.
    ///
    /// Layout handles kerning, line wrapping, alignment, and line height.
    pub fn draw_text(
        &mut self,
        font_cache: &mut FontCache,
        font_key: [u8; 32],
        font_bytes: &[u8],
        pos: CircleF4E4,
        size: ScalarF4E4,
        text: &str,
        colour: u32,
        settings: &TextSettings,
    ) {
        let cache_key = settings.font_cache_key(font_key);
        let font = font_cache.entry(cache_key).or_insert_with(|| {
            // TODO: when weight/tilt is set, apply set_variation() on the
            // ttf-parser Face before constructing the fontdue Font.
            FontdueFont::from_bytes(font_bytes, fontdue::FontSettings::default())
                .expect("draw_text: invalid font bytes")
        });

        let px = size * self.coords.span * self.coords.ru;
        if !px.is_positive() { return; }

        let anchor_x = self.ru_to_px_x(pos.r());
        let anchor_y = self.ru_to_px_y(pos.i());
        let canvas_w = self.coords.width as isize;
        let canvas_h = self.coords.height as isize;

        let wrap_px = settings.wrap.map(|w| w * self.coords.span * self.coords.ru);

        // Lay out left-to-right from anchor. We'll shift glyphs for center/right after.
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        let layout_settings = LayoutSettings {
            x: ScalarF4E4::from(anchor_x as i32),
            y: ScalarF4E4::from(anchor_y as i32),
            max_width: wrap_px,
            horizontal_align: HorizontalAlign::Left,
            line_height: settings.leading,
            ..LayoutSettings::default()
        };
        layout.reset(&layout_settings);
        layout.append(&[font as &FontdueFont], &TextStyle::new(text, px, 0));

        // For center/right alignment, compute per-line widths and shift glyphs
        let shift_x = if settings.align == 1 {
            // Left-aligned: no shift needed
            ScalarF4E4::ZERO
        } else {
            // Measure total text width from glyph positions
            let glyphs = layout.glyphs();
            if glyphs.is_empty() {
                ScalarF4E4::ZERO
            } else {
                let first_x = glyphs[0].x;
                let last = &glyphs[glyphs.len() - 1];
                let text_w = last.x + ScalarF4E4::from(last.width as i32) - first_x;
                match settings.align {
                    2 => -text_w,                              // Right: shift left by full width
                    _ => -(text_w >> 1usize),                  // Center: shift left by half width
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
            let gy = glyph.y.floor().to_isize();

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
                    let idx = py * canvas_w as usize + (gx + col) as usize;
                    self.pixels[idx] = CanvasFast::blend(colour, self.pixels[idx], coverage);
                }
            }
        }
    }
}
