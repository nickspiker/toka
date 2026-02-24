//! Pixel operations and alpha blending
//!
//! All blending in linear S44 light — no gamma-space math.
//! Porter-Duff "src over dst" compositing.

use crate::drawing::canvas_quality::{CanvasQuality, Pixel};
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
}
