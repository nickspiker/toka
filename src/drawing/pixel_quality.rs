//! Pixel operations and alpha blending
//!
//! All blending in linear S44 light — no gamma-space math.
//! Porter-Duff "src over dst" compositing.
//!
//! No clamping on blend mode results — values can go negative or above 1.0.
//! This is intentional: users may chain operations that depend on full range.

use crate::drawing::canvas_quality::{CanvasQuality, Pixel};
use crate::drawing::shared::BlendMode;
use spirix::ScalarF4E4;

impl CanvasQuality {
    /// Blend src over dst using src alpha (Porter-Duff src-over, linear light)
    ///
    /// out_rgb = src_a * src_rgb + (1 - src_a) * dst_rgb
    /// out_a   = src_a + (1 - src_a) * dst_a
    #[inline]
    pub(crate) fn blend(src: Pixel, dst: Pixel) -> Pixel {
        let src_a = src[3];
        let inv_a = ScalarF4E4::ONE - src_a;
        [
            src_a * src[0] + inv_a * dst[0],
            src_a * src[1] + inv_a * dst[1],
            src_a * src[2] + inv_a * dst[2],
            src_a + inv_a * dst[3],
        ]
    }

    /// Blend src over dst with an explicit coverage weight multiplied into src alpha
    #[inline]
    pub(crate) fn blend_weighted(src: Pixel, dst: Pixel, weight: ScalarF4E4) -> Pixel {
        Self::blend([src[0], src[1], src[2], src[3] * weight], dst)
    }

    /// Apply a blend mode per-channel, then alpha-composite.
    ///
    /// No clamping — results can exceed [0, 1]. The blend mode determines
    /// how src and dst colours combine; alpha handles transparency.
    ///
    /// out_rgb = src_a * blend(src_rgb, dst_rgb) + (1 - src_a) * dst_rgb
    /// out_a   = src_a + (1 - src_a) * dst_a
    #[inline]
    pub(crate) fn blend_with_mode(src: Pixel, dst: Pixel, mode: BlendMode) -> Pixel {
        use BlendMode::*;
        if matches!(mode, Normal) {
            return Self::blend(src, dst);
        }

        let one = ScalarF4E4::ONE;
        let two = ScalarF4E4::from(2);
        let half = one >> 1usize;

        let blend_ch = |s: ScalarF4E4, d: ScalarF4E4| -> ScalarF4E4 {
            match mode {
                Normal => s,
                Multiply => s * d,
                Screen => s + d - s * d,
                Overlay => {
                    if d < half { two * s * d }
                    else { one - two * (one - s) * (one - d) }
                }
                Darken => if s < d { s } else { d },
                Lighten => if s > d { s } else { d },
                ColorDodge => {
                    if s >= one { one } else { d / (one - s) }
                }
                ColorBurn => {
                    if !s.is_positive() { ScalarF4E4::ZERO }
                    else { one - (one - d) / s }
                }
                HardLight => {
                    if s < half { two * s * d }
                    else { one - two * (one - s) * (one - d) }
                }
                SoftLight => {
                    // Pegtop: (1 - 2s)·d² + 2s·d
                    (one - two * s) * d * d + two * s * d
                }
                Difference => (s - d).magnitude(),
                Exclusion => s + d - two * s * d,
                Add => s + d,
                Subtract => d - s,
                Divide => {
                    if !s.is_positive() { one } else { d / s }
                }
            }
        };

        let src_a = src[3];
        let inv_a = one - src_a;

        let br = blend_ch(src[0], dst[0]);
        let bg = blend_ch(src[1], dst[1]);
        let bb = blend_ch(src[2], dst[2]);

        [
            src_a * br + inv_a * dst[0],
            src_a * bg + inv_a * dst[1],
            src_a * bb + inv_a * dst[2],
            src_a + inv_a * dst[3],
        ]
    }

    /// Composite a source layer onto this canvas with opacity and blend mode.
    ///
    /// Skips fully transparent source pixels (alpha <= 0).
    pub fn composite_from(&mut self, src: &CanvasQuality, opacity: ScalarF4E4, mode: BlendMode) {
        let len = self.pixels().len().min(src.pixels().len());
        for i in 0..len {
            let s = src.pixels()[i];
            if !s[3].is_positive() { continue; }
            let weighted = [s[0], s[1], s[2], s[3] * opacity];
            self.pixels_mut()[i] = Self::blend_with_mode(weighted, self.pixels()[i], mode);
        }
    }
}
