//! Text rendering (placeholder implementation)
//!
//! Currently draws a coloured rectangle for text bounds.

use crate::drawing::canvas_quality::{CanvasQuality, Pixel};
use spirix::{CircleF4E4, ScalarF4E4};

impl CanvasQuality {
    /// Draw text placeholder — renders a rectangle representing text bounds
    pub fn draw_text(&mut self, pos: CircleF4E4, size: ScalarF4E4, text: &str, colour: Pixel) {
        let char_width = size * ScalarF4E4::from(6) / ScalarF4E4::from(10);
        let text_width = char_width * ScalarF4E4::from(text.len());
        let text_size = CircleF4E4::from((text_width, size));
        self.fill_rect_ru(pos, text_size, colour);
    }
}
