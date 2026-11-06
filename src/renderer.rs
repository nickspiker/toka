//! Direct VSF ro* to Canvas rendering
//!
//! This module renders VSF renderable objects (`rob`, `roe`, `ron`) directly to the Canvas without any intermediate representation. Each `ron` node is a frame: its translate, rotation and scale map its children into its parent's units, and the outermost frame is toka's RU. Angles arrive in turns.

use crate::drawing::Canvas;
use spirix::{CircleF4E4, ScalarF4E4};
use vsf::types::{Ellipse, Node, Paint, Rect, VsfType};

/// One node's frame: a point p in it is at translate + R(rotation)·(scale ⊙ p) in its parent's.
struct Frame {
    translate: [f64; 2],
    rotation: f64,
    scale: [f64; 2],
}

impl Frame {
    fn of(node: &Node) -> Frame {
        let translate = node.translate.as_ref().map_or([0., 0.], |t| {
            let v = t.to_f64();
            [v[0], v[1]]
        });
        let rotation = node.rotation.as_ref().map_or(0., |r| r.to_f64());
        let scale = node.scale.as_ref().map_or([1., 1.], |s| s.to_f64());
        Frame { translate, rotation, scale }
    }
}

/// Rendering context with the stack of frames from the root down.
pub struct RenderContext {
    frames: Vec<Frame>,
}

impl RenderContext {
    /// Create a new rendering context
    pub fn new() -> Self {
        RenderContext { frames: Vec::new() }
    }

    /// Render a VSF renderable object to canvas
    pub fn render(&mut self, vsf: &VsfType, canvas: &mut Canvas) -> Result<(), String> {
        match vsf {
            VsfType::rob(rect) => self.render_box(rect, canvas),
            VsfType::roe(ellipse) => self.render_ellipse(ellipse, canvas),
            VsfType::ron(node) => {
                if node.mask.is_some() {
                    return Err("Masks not implemented yet".to_string());
                }
                self.frames.push(Frame::of(node));
                // Children are in painter's order (later on top); fluor composites front-to-back, so the last child paints first.
                for child in node.children.iter().rev() {
                    self.render(child, canvas)?;
                }
                self.frames.pop();
                Ok(())
            }
            _ => Err(format!("Not a renderable type: {:?}", vsf)),
        }
    }

    /// Render a box (rob)
    fn render_box(&mut self, rect: &Rect, canvas: &mut Canvas) -> Result<(), String> {
        if rect.corners.is_some() {
            return Err("Box corners not implemented yet".to_string());
        }
        if rect.stroke.is_some() {
            return Err("Strokes not implemented yet".to_string());
        }
        let g = rect.geometry.to_f64();
        let centre = self.to_root([g[0], g[1]]);
        let half = self.size_to_root([g[2], g[3]]);
        let turns = self.rotation_to_root(rect.rotation.as_ref().map_or(0., |r| r.to_f64()));
        let colour = match &rect.fill {
            None => return Ok(()),
            Some(Paint::Colour(c)) => c,
            Some(Paint::Gradient(_)) => return Err("Gradients not implemented yet".to_string()),
        };
        let size = c44(2. * half[0], 2. * half[1]);
        canvas.fill_rotated_rect_ru(c44(centre[0], centre[1]), size, ScalarF4E4::from(turns * core::f64::consts::TAU), colour)
    }

    /// Render an ellipse (roe): a whole circle or ellipse; rings and sectors are not drawn yet.
    fn render_ellipse(&mut self, ellipse: &Ellipse, canvas: &mut Canvas) -> Result<(), String> {
        if ellipse.is_ring() || ellipse.sweep.is_some() {
            return Err("Rings and sectors not implemented yet".to_string());
        }
        if ellipse.stroke.is_some() {
            return Err("Strokes not implemented yet".to_string());
        }
        let g = ellipse.geometry.to_f64();
        let centre = self.to_root([g[0], g[1]]);
        let radii = self.size_to_root([g[2], g[3]]);
        let colour = match &ellipse.fill {
            None => return Ok(()),
            Some(Paint::Colour(c)) => c,
            Some(Paint::Gradient(_)) => return Err("Gradients not implemented yet".to_string()),
        };
        if radii[0] == radii[1] {
            return canvas.fill_circle(c44(centre[0], centre[1]), ScalarF4E4::from(radii[0]), colour);
        }
        let turns = self.rotation_to_root(ellipse.rotation.as_ref().map_or(0., |r| r.to_f64()));
        if turns != 0. {
            return Err("Rotated ellipses not implemented yet".to_string());
        }
        canvas.fill_ellipse(c44(centre[0], centre[1]), c44(radii[0], radii[1]), colour)
    }

    /// A point in the innermost frame, in root units.
    fn to_root(&self, p: [f64; 2]) -> [f64; 2] {
        let mut p = p;
        for f in self.frames.iter().rev() {
            let (x, y) = (p[0] * f.scale[0], p[1] * f.scale[1]);
            let (sin, cos) = (f.rotation * core::f64::consts::TAU).sin_cos();
            p = [f.translate[0] + x * cos - y * sin, f.translate[1] + x * sin + y * cos];
        }
        p
    }

    /// A size (half-extents, radii) in the innermost frame, in root units: scaled, never rotated.
    fn size_to_root(&self, s: [f64; 2]) -> [f64; 2] {
        self.frames.iter().fold(s, |s, f| [s[0] * f.scale[0].abs(), s[1] * f.scale[1].abs()])
    }

    /// A rotation in the innermost frame, in turns, as seen from the root.
    fn rotation_to_root(&self, turns: f64) -> f64 {
        self.frames.iter().fold(turns, |t, f| t + f.rotation)
    }
}

fn c44(x: f64, y: f64) -> CircleF4E4 {
    CircleF4E4::from((x, y))
}

/// Extract a VSF colour as a fluor-native α+darkness pixel (`0xααRRGGBB`, RGB = 255−visible).
///
/// Pipeline: VSF colour → linear sRGB → sRGB OETF → u8 → `pack_argb` (flips to darkness).
/// This is the single colour funnel for the whole renderer; every primitive draws with the value it returns, so the format lives in exactly one place.
pub fn extract_colour_u32(vsf: &VsfType) -> Result<u32, String> {
    let (r, g, b) = vsf
        .to_srgb_u8_s44()
        .ok_or_else(|| format!("Not a colour type: {:?}", vsf))?;
    // Alpha: only ra/rt/rh/rw carry real alpha — everything else is opaque
    let a: u8 = match vsf {
        VsfType::ra([_, _, _, a]) => *a,
        VsfType::rt([_, _, _, a]) => (*a >> 8) as u8,
        VsfType::rh([_, _, _, a]) => {
            let s = ScalarF4E4::from_f32(*a);
            (s * 255i32).to_i32() as u8
        }
        _ => 255,
    };

    // fluor α+darkness: α in the high byte, RGB stored as darkness (255 − channel).
    Ok(fluor::paint::pack_argb(r, g, b, a))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vsf::types::VsfType;

    #[test]
    fn test_colour_extraction() {
        // α+darkness: α in high byte; RGB bytes are darkness (255 − visible).
        let black = extract_colour_u32(&VsfType::rck).unwrap();
        assert_eq!(black >> 24, 255, "black alpha (high byte)");

        let blue = extract_colour_u32(&VsfType::rcb).unwrap();
        assert_eq!(blue >> 24, 255, "blue alpha (high byte)");
        // sRGB OETF rounds 255→254, so a "fully bright" channel reads darkness ≤1, not exactly 0.
        assert!(blue & 0xFF <= 1, "blue: B nearly zero darkness, got {}", blue & 0xFF);

        let red_half = extract_colour_u32(&VsfType::ra([255, 0, 0, 127])).unwrap();
        assert_eq!(red_half >> 24, 127, "red half alpha (high byte)");
        assert!((red_half >> 16) & 0xFF <= 1, "red: R nearly zero darkness, got {}", (red_half >> 16) & 0xFF);
    }
}

#[cfg(test)]
mod roundtrip_tests {
    use vsf::types::VsfType;
    use vsf::decoding::parse::parse as vsf_parse;

    #[test]
    fn test_ra_roundtrip() {
        let ra = VsfType::ra([255, 0, 0, 127]);
        let bytes = ra.flatten();
        println!("ra flattened: {:02X?}", bytes);

        let mut offset = 0usize;
        let parsed = vsf_parse(&bytes, &mut offset).unwrap();
        println!("ra parsed back: {:?}", parsed);
        assert!(matches!(parsed, VsfType::ra([255, 0, 0, 127])), "roundtrip failed: got {:?}", parsed);
    }
}
