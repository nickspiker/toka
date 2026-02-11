//! Direct VSF ro* to Canvas rendering
//!
//! This module renders VSF renderable object types (rob, roc, row, etc.) directly
//! to the Canvas without any intermediate representation. Transforms are tracked
//! as we traverse the scene graph hierarchy.

use crate::canvas::Canvas;
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
                // Push transform, render children, pop transform
                self.transform_stack.push(transform.clone());
                for child in children {
                    self.render(child, canvas)?;
                }
                self.transform_stack.pop();
                Ok(())
            }
            VsfType::ron(_pos, _size, children) => {
                // Container node - just render children
                // TODO: Apply position/size bounds clipping
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
        // Apply current transforms
        let world_pos = self.apply_transforms(*pos);
        let world_size = self.apply_transforms_size(*size);
        let rotation = self.get_cumulative_rotation();

        // DEBUG: Log rotation angle
        #[cfg(target_arch = "wasm32")]
        crate::wasm::js_log(
            &format!(
                "Rotation angle: {} ({} degrees)",
                rotation,
                rotation * 180 / ScalarF4E4::PI
            ),
            "info"
        );

        // Render fill
        match fill {
            Fill::Solid(colour) => {
                let rgba = extract_colour(colour)?;

                // Always use rotated rectangle (handles zero rotation as axis-aligned)
                canvas.fill_rotated_rect_ru(world_pos, world_size, rotation, rgba);
            }
            Fill::Gradient(_) => {
                return Err("Gradients not implemented yet".to_string());
            }
        }

        // Render stroke if present
        if stroke.is_some() {
            return Err("Strokes not implemented yet".to_string());
        }

        // Render children
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
        // Apply current transforms
        let world_center = self.apply_transforms(*center);
        // TODO: Transform radius with scale
        let world_radius = *radius;

        // Render fill
        match fill {
            Fill::Solid(colour) => {
                let rgba = extract_colour(colour)?;
                canvas.fill_circle(world_center, world_radius, rgba);
            }
            Fill::Gradient(_) => {
                return Err("Gradients not implemented yet".to_string());
            }
        }

        // Render stroke if present
        if stroke.is_some() {
            return Err("Strokes not implemented yet".to_string());
        }

        Ok(())
    }

    /// Apply all transforms in the stack to a position
    fn apply_transforms(&self, pos: CircleF4E4) -> CircleF4E4 {
        let mut result = pos;
        for transform in &self.transform_stack {
            result = self.apply_single_transform(result, transform);
        }
        result
    }

    /// Apply all transforms in the stack to a size
    fn apply_transforms_size(&self, size: CircleF4E4) -> CircleF4E4 {
        // For size, only apply scale transforms (not translation/rotation)
        let mut result = size;
        for transform in &self.transform_stack {
            if let Some(scale) = transform.scale {
                result = CircleF4E4::from((result.r() * scale.r(), result.i() * scale.i()));
            }
        }
        result
    }

    /// Get cumulative rotation angle from all transforms in the stack
    fn get_cumulative_rotation(&self) -> ScalarF4E4 {
        let mut total_rotation = ScalarF4E4::ZERO;
        for transform in &self.transform_stack {
            if let Some(angle) = transform.rotate {
                total_rotation = total_rotation + angle;
            }
        }
        total_rotation
    }

    /// Apply a single transform to a position
    ///
    /// Transform order (matches deleted loom.rs):
    /// 1. Translate to origin
    /// 2. Apply scale
    /// 3. Apply rotation
    /// 4. Translate back from origin
    /// 5. Apply final translation
    fn apply_single_transform(&self, pos: CircleF4E4, t: &Transform) -> CircleF4E4 {
        let mut ru_x = pos.r();
        let mut ru_y = pos.i();

        // 1. Translate to origin
        if let Some(origin) = t.origin {
            ru_x = ru_x - origin.r();
            ru_y = ru_y - origin.i();
        }

        // 2. Apply scale
        if let Some(scale) = t.scale {
            ru_x = ru_x * scale.r();
            ru_y = ru_y * scale.i();
        }

        // 3. Apply rotation
        if let Some(angle) = t.rotate {
            let cos = angle.cos();
            let sin = angle.sin();
            let new_x = ru_x * cos - ru_y * sin;
            let new_y = ru_x * sin + ru_y * cos;
            ru_x = new_x;
            ru_y = new_y;
        }

        // 4. Translate back from origin
        if let Some(origin) = t.origin {
            ru_x = ru_x + origin.r();
            ru_y = ru_y + origin.i();
        }

        // 5. Apply final translation
        if let Some(translate) = t.translate {
            ru_x = ru_x + translate.r();
            ru_y = ru_y + translate.i();
        }

        CircleF4E4::from((ru_x, ru_y))
    }
}

/// Extract and convert VSF colour to packed u32 sRGB
///
/// Pipeline: VSF colour constant → linear S44 RGBA → sRGB u8 → packed u32
fn extract_colour(vsf: &VsfType) -> Result<u32, String> {
    use vsf::colour::convert::{
        apply_matrix_3x3_s44, linearize_gamma2_s44, srgb_oetf_s44, vsf_rgb2srgb_s44,
    };

    let rgba = vsf
        .to_rgba_linear_s44()
        .ok_or_else(|| format!("Not a colour type: {:?}", vsf))?;

    let [r_vsf, g_vsf, b_vsf, a] = [rgba.r, rgba.g, rgba.b, rgba.a];

    // 1. Decode VSF gamma 2: encoded^2 → linear
    let r_lin_vsf = linearize_gamma2_s44(r_vsf);
    let g_lin_vsf = linearize_gamma2_s44(g_vsf);
    let b_lin_vsf = linearize_gamma2_s44(b_vsf);

    // 2. Colour space transform: linear VSF RGB → linear sRGB
    let [r_lin_srgb, g_lin_srgb, b_lin_srgb] =
        apply_matrix_3x3_s44(&vsf_rgb2srgb_s44(), &[r_lin_vsf, g_lin_vsf, b_lin_vsf]);

    // 3. Apply sRGB OETF (gamma encoding for display)
    let r_srgb = srgb_oetf_s44(r_lin_srgb);
    let g_srgb = srgb_oetf_s44(g_lin_srgb);
    let b_srgb = srgb_oetf_s44(b_lin_srgb);

    // 4. Quantize to u8 and pack into u32 (RGBA: R in low byte for little-endian)
    let r = (r_srgb << 8isize).to_u8();
    let g = (g_srgb << 8isize).to_u8();
    let b = (b_srgb << 8isize).to_u8();
    let a = (a << 8isize).to_u8();

    // Pack as R | G<<8 | B<<16 | A<<24 (matches canvas.rs expected format)
    let packed = (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24);

    // DEBUG: Log colour packing
    #[cfg(target_arch = "wasm32")]
    crate::wasm::js_log(&format!("Colour: r={} g={} b={} a={} → {:08X}", r, g, b, a, packed), "info");

    Ok(packed)
}
