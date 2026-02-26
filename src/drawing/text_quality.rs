//! Text rendering for CanvasQuality using fontdue-spirix

use crate::drawing::canvas_quality::{CanvasQuality, Pixel};
use crate::vm::FontCache;
use fontdue::Font as FontdueFont;
use spirix::{CircleF4E4, ScalarF4E4};

impl CanvasQuality {
    /// Draw text onto the canvas.
    ///
    /// Stack: font_bytes, pos (c44), size (s44), text, colour
    /// Glyphs are alpha-blended in linear light using the coverage bitmap.
    /// Measure text width in pixels without rasterizing bitmaps.
    fn measure_width(font: &FontdueFont, text: &str, px: ScalarF4E4) -> isize {
        text.chars()
            .map(|ch| font.metrics(ch, px).advance_width.ceil().to_isize())
            .sum()
    }

    /// Draw text onto the canvas.
    ///
    /// `align`: 0=center (default), 1=left, 2=right
    pub fn draw_text(
        &mut self,
        font_cache: &mut FontCache,
        font_key: [u8; 32],
        font_bytes: &[u8],
        pos: CircleF4E4,
        size: ScalarF4E4,
        text: &str,
        colour: Pixel,
        align: u8,
    ) {
        let font = font_cache.entry(font_key).or_insert_with(|| {
            FontdueFont::from_bytes(font_bytes, fontdue::FontSettings::default())
                .expect("draw_text: invalid font bytes")
        });

        let px = size * self.span() * self.ru();
        if !px.is_positive() { return; }

        let anchor_x = self.ru_to_px_x(pos.r());
        let start_y = self.ru_to_px_y(pos.i());
        let canvas_w = self.width() as isize;
        let canvas_h = self.height() as isize;

        let text_width = Self::measure_width(font, text, px);
        let start_x = match align {
            1 => anchor_x,                      // left
            2 => anchor_x - text_width,         // right
            _ => anchor_x - text_width / 2,     // center (default)
        };

        let mut cursor_x = start_x;

        for ch in text.chars() {
            let (metrics, bitmap) = font.rasterize(ch, px);
            let glyph_w = metrics.width as isize;
            let glyph_h = metrics.height as isize;
            let offset_x = metrics.xmin as isize;
            let offset_y = metrics.ymin as isize;

            for row in 0..glyph_h {
                let py = start_y - offset_y - glyph_h + row;
                if py < 0 || py >= canvas_h { continue; }
                for col in 0..glyph_w {
                    let px_x = cursor_x + offset_x + col;
                    if px_x < 0 || px_x >= canvas_w { continue; }
                    let coverage = bitmap[(row * glyph_w + col) as usize];
                    if coverage == 0 { continue; }
                    let alpha = ScalarF4E4::from(coverage as i32) >> 8usize;
                    let inv = ScalarF4E4::ONE - alpha;
                    let idx = (py * canvas_w + px_x) as usize;
                    let bg = self.pixels_mut()[idx];
                    self.pixels_mut()[idx] = [
                        colour[0] * alpha + bg[0] * inv,
                        colour[1] * alpha + bg[1] * inv,
                        colour[2] * alpha + bg[2] * inv,
                        colour[3] * alpha + bg[3] * inv,
                    ];
                }
            }

            cursor_x += metrics.advance_width.ceil().to_isize();
        }
    }
}
