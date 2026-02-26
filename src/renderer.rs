//! Direct VSF ro* to Canvas rendering
//!
//! This module renders VSF renderable object types (rob, roc, row, etc.) directly
//! to the Canvas without any intermediate representation. Transforms are tracked
//! as we traverse the scene graph hierarchy.

use crate::drawing::Canvas;
use spirix::{CircleF4E4, ScalarF4E4};
use vsf::types::{Fill, Transform, VsfType};

/// Rendering context with transform stack
pub struct RenderContext {
    /// Stack of transforms from parent nodes
    transform_stack: Vec<Transform>,
}

impl RenderContext {
    /// Create a new rendering context
    pub fn new() -> Self {
        RenderContext {
            transform_stack: Vec::new(),
        }
    }

    /// Render a VSF renderable object to canvas
    pub fn render(&mut self, vsf: &VsfType, canvas: &mut Canvas) -> Result<(), String> {
        match vsf {
            VsfType::rob(pos, size, fill, stroke, children) => {
                self.render_box(pos, size, fill, stroke, children, canvas)
            }
            VsfType::roc(center, radius, fill, stroke) => {
                self.render_circle(center, radius, fill, stroke, canvas)
            }
            VsfType::row(transform, children) => {
                self.transform_stack.push(transform.clone());
                for child in children {
                    self.render(child, canvas)?;
                }
                self.transform_stack.pop();
                Ok(())
            }
            VsfType::ron(_pos, _size, children) => {
                for child in children {
                    self.render(child, canvas)?;
                }
                Ok(())
            }
            _ => Err(format!("Not a renderable type: {:?}", vsf)),
        }
    }

    /// Render a box (rob)
    fn render_box(
        &mut self,
        pos: &CircleF4E4,
        size: &CircleF4E4,
        fill: &Fill,
        stroke: &Option<vsf::types::Stroke>,
        children: &[VsfType],
        canvas: &mut Canvas,
    ) -> Result<(), String> {
        let world_pos  = self.apply_transforms(*pos);
        let world_size = self.apply_transforms_size(*size);
        let rotation   = self.get_cumulative_rotation();

        #[cfg(target_arch = "wasm32")]
        crate::wasm::js_log(
            &format!("Rotation angle: {} ({} degrees)", rotation, rotation * 180 / ScalarF4E4::PI),
            "info"
        );

        match fill {
            Fill::Solid(colour) => {
                canvas.fill_rotated_rect_ru(world_pos, world_size, rotation, colour)?;
            }
            Fill::Gradient(_) => return Err("Gradients not implemented yet".to_string()),
        }

        if stroke.is_some() {
            return Err("Strokes not implemented yet".to_string());
        }

        for child in children {
            self.render(child, canvas)?;
        }

        Ok(())
    }

    /// Render a circle (roc)
    fn render_circle(
        &mut self,
        center: &CircleF4E4,
        radius: &ScalarF4E4,
        fill: &Fill,
        stroke: &Option<vsf::types::Stroke>,
        canvas: &mut Canvas,
    ) -> Result<(), String> {
        let world_center = self.apply_transforms(*center);
        let world_radius = *radius;

        match fill {
            Fill::Solid(colour) => {
                canvas.fill_circle(world_center, world_radius, colour)?;
            }
            Fill::Gradient(_) => return Err("Gradients not implemented yet".to_string()),
        }

        if stroke.is_some() {
            return Err("Strokes not implemented yet".to_string());
        }

        Ok(())
    }

    fn apply_transforms(&self, pos: CircleF4E4) -> CircleF4E4 {
        let mut result = pos;
        for transform in &self.transform_stack {
            result = self.apply_single_transform(result, transform);
        }
        result
    }

    fn apply_transforms_size(&self, size: CircleF4E4) -> CircleF4E4 {
        let mut result = size;
        for transform in &self.transform_stack {
            if let Some(scale) = transform.scale {
                result = CircleF4E4::from((result.r() * scale.r(), result.i() * scale.i()));
            }
        }
        result
    }

    fn get_cumulative_rotation(&self) -> ScalarF4E4 {
        let mut total = ScalarF4E4::ZERO;
        for transform in &self.transform_stack {
            if let Some(angle) = transform.rotate {
                total = total + angle;
            }
        }
        total
    }

    fn apply_single_transform(&self, pos: CircleF4E4, t: &Transform) -> CircleF4E4 {
        let mut ru_x = pos.r();
        let mut ru_y = pos.i();

        if let Some(origin) = t.origin {
            ru_x = ru_x - origin.r();
            ru_y = ru_y - origin.i();
        }
        if let Some(scale) = t.scale {
            ru_x = ru_x * scale.r();
            ru_y = ru_y * scale.i();
        }
        if let Some(angle) = t.rotate {
            let cos = angle.cos();
            let sin = angle.sin();
            let new_x = ru_x * cos - ru_y * sin;
            let new_y = ru_x * sin + ru_y * cos;
            ru_x = new_x;
            ru_y = new_y;
        }
        if let Some(origin) = t.origin {
            ru_x = ru_x + origin.r();
            ru_y = ru_y + origin.i();
        }
        if let Some(translate) = t.translate {
            ru_x = ru_x + translate.r();
            ru_y = ru_y + translate.i();
        }

        CircleF4E4::from((ru_x, ru_y))
    }
}

/// Extract a VSF colour as packed u32 sRGB for CanvasFast
///
/// Pipeline: VSF colour → linear sRGB → sRGB OETF → u8 → R<<24|G<<16|B<<8|A
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
            (s * ScalarF4E4::from(255)).to_i32() as u8
        }
        _ => 255,
    };

    let packed = ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32);

    #[cfg(target_arch = "wasm32")]
    crate::wasm::js_log(&format!("extract_colour_u32: r={} g={} b={} a={} → {:08X}", r, g, b, a, packed), "info");

    Ok(packed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vsf::types::VsfType;

    #[test]
    fn test_colour_extraction() {
        let black = extract_colour_u32(&VsfType::rck).unwrap();
        println!("rck  → {:08X}  (a={})", black, black & 0xFF);

        let blue = extract_colour_u32(&VsfType::rcb).unwrap();
        println!("rcb  → {:08X}  (a={})", blue, blue & 0xFF);

        let red_half = extract_colour_u32(&VsfType::ra([255, 0, 0, 127])).unwrap();
        println!("ra[255,0,0,127] → {:08X}  (a={})", red_half, red_half & 0xFF);

        assert_eq!(black & 0xFF, 255, "black alpha");
        assert_eq!(blue  & 0xFF, 255, "blue alpha");
        assert_eq!(red_half & 0xFF, 127, "red half alpha");
    }
}

/// Extract a VSF colour as linear S44 RGBA for CanvasQuality
///
/// Pipeline: VSF colour → linear RGBA S44 [R, G, B, A]
/// Gamma-2 OETF is applied later at to_rgba_bytes(), not here.
pub fn extract_colour_linear(vsf: &VsfType) -> Result<crate::drawing::Pixel, String> {
    let rgba = vsf
        .to_rgba_linear_s44()
        .ok_or_else(|| format!("Not a colour type: {:?}", vsf))?;
    Ok([rgba.r, rgba.g, rgba.b, rgba.a])
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
