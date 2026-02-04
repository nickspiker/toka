//! Loom: Hierarchical Layout System for Toka
//!
//! Loom provides a declarative UI construction system with parent-relative
//! positioning, vector and raster graphics, and built-in primitives for
//! buttons, text, and shapes.
//!
//! # VSF Encoding
//!
//! Loom layouts serialize to VSF `vt` wrapped types (Toka Tree):
//! - Format: `v(b't', inner_data)` where inner_data contains:
//!   - `b` = Box container
//!   - `c` = Circle shape
//!   - `x` = Text label
//!   - `u` = Button UI element
//!   - `g` = Group (logical container)
//!   - `l` = Line stroke
//!   - `p` = Path (vector)
//!   - `i` = Image (raster)
//!   - `s` = Surface (raw pixels)
//!
//! # Coordinate System
//!
//! All coordinates are parent-relative (0.0 = parent edge, 1.0 = opposite edge):
//! - Values > 1.0 extend beyond parent (explicit overflow)
//! - Negative values position outside parent bounds
//! - Root node uses viewport coords (0,0 = top-left, 1,1 = bottom-right)
//! - Harmonic mean (span) maintains aspect ratio for circles/photos

use spirix::{CircleF4E4, ScalarF4E4};

use crate::canvas::Canvas;

/// Loom layout node
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutNode {
    /// Box container (can have children)
    Box {
        /// Parent-relative position (x, y)
        pos: CircleF4E4,
        /// Size (w, h) in parent coords
        size: CircleF4E4,
        /// RGBA fill colour
        colour: [ScalarF4E4; 4],
        /// Child nodes
        children: Vec<LayoutNode>,
    },

    /// Group container (logical only, no visual)
    Group {
        /// Parent-relative position (x, y)
        pos: CircleF4E4,
        /// Size (w, h) in parent coords
        size: CircleF4E4,
        /// Child nodes
        children: Vec<LayoutNode>,
    },

    /// Circle shape
    Circle {
        /// Parent-relative center (x, y)
        center: CircleF4E4,
        /// Radius in parent coords
        radius: ScalarF4E4,
        /// RGBA fill colour
        colour: [ScalarF4E4; 4],
    },

    /// Line stroke
    Line {
        /// Parent-relative start point
        start: CircleF4E4,
        /// Parent-relative end point
        end: CircleF4E4,
        /// Line width in parent coords
        width: ScalarF4E4,
        /// RGBA stroke colour
        colour: [ScalarF4E4; 4],
    },

    /// Text label
    Text {
        /// Parent-relative position (x, y)
        pos: CircleF4E4,
        /// Font size in parent coords
        size: ScalarF4E4,
        /// Text content
        content: String,
        /// RGBA text colour
        colour: [ScalarF4E4; 4],
    },

    /// Button UI element
    Button {
        /// Parent-relative position (x, y)
        pos: CircleF4E4,
        /// Size (w, h) in parent coords
        size: CircleF4E4,
        /// Button label text
        label: String,
        /// Visual style variant
        variant: ButtonVariant,
        /// RGBA background colour override
        colour: [ScalarF4E4; 4],
    },

    /// Vector path (stub - reference Photon rasterizer when implementing)
    /// See: photon/src/ui/compositing.rs for Bézier curve rendering
    Path {
        /// Path drawing commands
        commands: Vec<PathCommand>,
        /// Stroke line width
        stroke_width: ScalarF4E4,
        /// RGBA stroke colour
        colour: [ScalarF4E4; 4],
    },

    /// Image (raster, capability handle)
    Image {
        /// Parent-relative position (x, y)
        pos: CircleF4E4,
        /// Size (w, h) in parent coords
        size: CircleF4E4,
        /// Capability handle for image data
        handle: u64,
        /// RGBA tint colour (multiply blend)
        tint: [ScalarF4E4; 4],
    },

    /// Surface (raw pixel buffer, capability handle)
    Surface {
        /// Parent-relative position (x, y)
        pos: CircleF4E4,
        /// Size (w, h) in parent coords
        size: CircleF4E4,
        /// Capability handle for pixel buffer
        handle: u64,
    },
}

/// Button visual variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ButtonVariant {
    /// Filled button with solid background
    Filled = 0,
    /// Outlined button with border only
    Outlined = 1,
    /// Text-only button with no background or border
    Text = 2,
}

/// Vector path command
#[derive(Debug, Clone, PartialEq)]
pub enum PathCommand {
    /// Move to position without drawing
    MoveTo(CircleF4E4),
    /// Draw line to position
    LineTo(CircleF4E4),
    /// Draw quadratic Bézier curve
    QuadraticTo {
        /// Control point
        ctrl: CircleF4E4,
        /// End point
        end: CircleF4E4,
    },
    /// Draw cubic Bézier curve
    CubicTo {
        /// First control point
        ctrl1: CircleF4E4,
        /// Second control point
        ctrl2: CircleF4E4,
        /// End point
        end: CircleF4E4,
    },
    /// Close path to starting point
    Close,
}

/// Computed absolute layout bounds
#[derive(Debug, Clone)]
pub struct LayoutBounds {
    /// Absolute viewport coordinates
    pub pos: CircleF4E4,
    /// Absolute size in viewport units
    pub size: CircleF4E4,
}

impl LayoutNode {
    /// Compute absolute bounds given parent bounds
    pub fn compute_bounds(&self, parent: &LayoutBounds) -> LayoutBounds {
        match self {
            LayoutNode::Box { pos, size, .. }
            | LayoutNode::Group { pos, size, .. }
            | LayoutNode::Button { pos, size, .. }
            | LayoutNode::Image { pos, size, .. }
            | LayoutNode::Surface { pos, size, .. } => {
                // Absolute = parent_pos + (relative_pos * parent_size)
                let abs_pos = CircleF4E4::from((
                    parent.pos.r() + pos.r() * parent.size.r(),
                    parent.pos.i() + pos.i() * parent.size.i(),
                ));
                let abs_size = CircleF4E4::from((
                    size.r() * parent.size.r(),
                    size.i() * parent.size.i(),
                ));
                LayoutBounds {
                    pos: abs_pos,
                    size: abs_size,
                }
            }

            LayoutNode::Circle { center, radius, .. } => {
                let abs_pos = CircleF4E4::from((
                    parent.pos.r() + center.r() * parent.size.r(),
                    parent.pos.i() + center.i() * parent.size.i(),
                ));
                // Radius scales with parent width (use harmonic mean for aspect ratio)
                let abs_size = CircleF4E4::from((
                    *radius * parent.size.r(),
                    *radius * parent.size.r(), // Square for circle
                ));
                LayoutBounds {
                    pos: abs_pos,
                    size: abs_size,
                }
            }

            LayoutNode::Line { start, end, width: _, .. } => {
                let abs_start = CircleF4E4::from((
                    parent.pos.r() + start.r() * parent.size.r(),
                    parent.pos.i() + start.i() * parent.size.i(),
                ));
                let abs_end = CircleF4E4::from((
                    parent.pos.r() + end.r() * parent.size.r(),
                    parent.pos.i() + end.i() * parent.size.i(),
                ));
                // Size represents bounding box of line
                // Use Spirix's magnitude() for absolute value (not IEEE abs())
                let size_x = (abs_end.r() - abs_start.r()).magnitude();
                let size_y = (abs_end.i() - abs_start.i()).magnitude();
                LayoutBounds {
                    pos: abs_start,
                    size: CircleF4E4::from((size_x, size_y)),
                }
            }

            LayoutNode::Text { pos, size, .. } => {
                let abs_pos = CircleF4E4::from((
                    parent.pos.r() + pos.r() * parent.size.r(),
                    parent.pos.i() + pos.i() * parent.size.i(),
                ));
                // Text size scales with parent height
                let abs_height = *size * parent.size.i();
                // Width is approximate (based on character count in render)
                LayoutBounds {
                    pos: abs_pos,
                    size: CircleF4E4::from((abs_height, abs_height)),
                }
            }

            LayoutNode::Path { .. } => {
                // TODO: Compute bounding box from path commands
                LayoutBounds {
                    pos: parent.pos,
                    size: parent.size,
                }
            }
        }
    }

    /// Render node and children to canvas
    pub fn render(&self, canvas: &mut Canvas, bounds: &LayoutBounds) {
        match self {
            LayoutNode::Box { colour, children, .. } => {
                // Fill rectangle with colour
                canvas.fill_rect_vp(bounds.pos, bounds.size, *colour);

                // Render children
                for child in children {
                    let child_bounds = child.compute_bounds(bounds);
                    child.render(canvas, &child_bounds);
                }
            }

            LayoutNode::Group { children, .. } => {
                // Group is logical only - just render children
                for child in children {
                    let child_bounds = child.compute_bounds(bounds);
                    child.render(canvas, &child_bounds);
                }
            }

            LayoutNode::Circle { radius, colour, .. } => {
                // Convert radius from parent-relative to absolute
                // Use parent width for radius scaling
                let abs_radius = *radius * bounds.size.r();

                // Circle needs to be rendered in RU coordinates, but bounds are in viewport
                // For now, use viewport rendering (future: convert to RU for aspect-correct circles)
                let _center_vp = CircleF4E4::from((
                    bounds.pos.r(),
                    bounds.pos.i(),
                ));

                // Convert viewport circle to RU circle for aspect-correct rendering
                // This is a simplified conversion - future: use Canvas::vp_to_ru()
                let _canvas_width = ScalarF4E4::from(canvas.dimensions().0);
                let _canvas_height = ScalarF4E4::from(canvas.dimensions().1);

                // Convert viewport position to RU (center-origin)
                let ru_x = (bounds.pos.r() - ScalarF4E4::from(1) / ScalarF4E4::from(2)) * ScalarF4E4::from(2);
                let ru_y = (bounds.pos.i() - ScalarF4E4::from(1) / ScalarF4E4::from(2)) * ScalarF4E4::from(2);
                let ru_center = CircleF4E4::from((ru_x, ru_y));

                // Radius in RU (scale by 2 since viewport is 0-1, RU is -1 to 1)
                let ru_radius = abs_radius * ScalarF4E4::from(2);

                canvas.fill_circle(ru_center, ru_radius, *colour);
            }

            LayoutNode::Line { start, end, width: _, colour } => {
                // Convert viewport coords to pixel coords
                let width_s = ScalarF4E4::from(canvas.width());
                let height_s = ScalarF4E4::from(canvas.height());

                let start_px = CircleF4E4::from((
                    start.r() * width_s,
                    start.i() * height_s,
                ));
                let end_px = CircleF4E4::from((
                    end.r() * width_s,
                    end.i() * height_s,
                ));

                // Draw anti-aliased line
                canvas.draw_line(start_px, end_px, *colour, *colour);
            }

            LayoutNode::Text { size: _, content, colour, .. } => {
                // Text size is already computed as absolute
                let abs_size = bounds.size.i();

                // Convert viewport position to RU for text rendering
                let ru_x = (bounds.pos.r() - ScalarF4E4::from(1) / ScalarF4E4::from(2)) * ScalarF4E4::from(2);
                let ru_y = (bounds.pos.i() - ScalarF4E4::from(1) / ScalarF4E4::from(2)) * ScalarF4E4::from(2);
                let ru_pos = CircleF4E4::from((ru_x, ru_y));

                // Radius in RU (scale by 2)
                let ru_size = abs_size * ScalarF4E4::from(2);

                canvas.draw_text(ru_pos, ru_size, content, *colour);
            }

            LayoutNode::Button { label, variant: _, colour, .. } => {
                // TODO: Reference photon/src/ui/compositing.rs for button rendering
                // For now, render as coloured box with text
                canvas.fill_rect_vp(bounds.pos, bounds.size, *colour);

                // Draw label in center
                let text_size = bounds.size.i() * ScalarF4E4::from(5) / ScalarF4E4::from(10); // 50% of button height

                // Convert to RU coordinates
                let ru_x = (bounds.pos.r() - ScalarF4E4::from(1) / ScalarF4E4::from(2)) * ScalarF4E4::from(2);
                let ru_y = (bounds.pos.i() - ScalarF4E4::from(1) / ScalarF4E4::from(2)) * ScalarF4E4::from(2);
                let ru_pos = CircleF4E4::from((ru_x, ru_y));
                let ru_text_size = text_size * ScalarF4E4::from(2);

                // Use inverted colour for text (simple contrast)
                let text_colour = [
                    ScalarF4E4::ONE - colour[0],
                    ScalarF4E4::ONE - colour[1],
                    ScalarF4E4::ONE - colour[2],
                    colour[3],
                ];

                canvas.draw_text(ru_pos, ru_text_size, label, text_colour);
            }

            LayoutNode::Path { .. } => {
                // TODO: Stub - reference Photon's path rasterizer
                // photon/src/ui/compositing.rs has Bézier curve rendering
            }

            LayoutNode::Image { handle: _, tint, .. } => {
                // TODO: Image rendering requires capability system
                // Placeholder: draw coloured rectangle indicating image
                canvas.fill_rect_vp(bounds.pos, bounds.size, *tint);
            }

            LayoutNode::Surface { handle: _, .. } => {
                // TODO: Surface rendering requires capability system
                // Placeholder: draw gray rectangle indicating surface
                let gray = [
                    ScalarF4E4::from(5) / ScalarF4E4::from(10),
                    ScalarF4E4::from(5) / ScalarF4E4::from(10),
                    ScalarF4E4::from(5) / ScalarF4E4::from(10),
                    ScalarF4E4::ONE,
                ];
                canvas.fill_rect_vp(bounds.pos, bounds.size, gray);
            }
        }
    }
}

/// Conversion functions from VSF TokaNode types to Toka LayoutNode
impl LayoutNode {
    /// Convert from VSF TokaBox to Toka LayoutNode::Box
    pub fn from_vsf_box(vsf_box: &vsf::types::TokaBox) -> Self {
        LayoutNode::Box {
            pos: vsf_box.pos,
            size: vsf_box.size,
            colour: circle_to_rgba(&vsf_box.colour),
            children: vec![],
        }
    }

    /// Convert from VSF TokaGroup to Toka LayoutNode::Group
    pub fn from_vsf_group(vsf_group: &vsf::types::TokaGroup) -> Self {
        let children = vsf_group
            .children
            .iter()
            .map(|child| LayoutNode::from_vsf_node(child))
            .collect();

        LayoutNode::Group {
            pos: vsf_group.pos,
            size: vsf_group.size,
            children,
        }
    }

    /// Convert from VSF TokaCircle to Toka LayoutNode::Circle
    pub fn from_vsf_circle(vsf_circle: &vsf::types::TokaCircle) -> Self {
        LayoutNode::Circle {
            center: vsf_circle.pos,
            radius: vsf_circle.span,
            colour: circle_to_rgba(&vsf_circle.colour),
        }
    }

    /// Convert from VSF TokaLine to Toka LayoutNode::Line
    pub fn from_vsf_line(vsf_line: &vsf::types::TokaLine) -> Self {
        LayoutNode::Line {
            start: vsf_line.start,
            end: vsf_line.end,
            width: vsf_line.width,
            colour: circle_to_rgba(&vsf_line.colour),
        }
    }

    /// Convert from VSF TokaText to Toka LayoutNode::Text
    pub fn from_vsf_text(vsf_text: &vsf::types::TokaText) -> Self {
        LayoutNode::Text {
            pos: vsf_text.pos,
            size: vsf_text.size.r(), // Use real component for font size
            content: vsf_text.content.clone(),
            colour: circle_to_rgba(&vsf_text.colour),
        }
    }

    /// Convert from VSF TokaButton to Toka LayoutNode::Button
    pub fn from_vsf_button(vsf_button: &vsf::types::TokaButton) -> Self {
        let variant = match vsf_button.variant {
            vsf::types::ButtonVariant::Filled => ButtonVariant::Filled,
            vsf::types::ButtonVariant::Outlined => ButtonVariant::Outlined,
            vsf::types::ButtonVariant::Text => ButtonVariant::Text,
        };

        LayoutNode::Button {
            pos: vsf_button.pos,
            size: vsf_button.size,
            label: vsf_button.label.clone(),
            variant,
            colour: circle_to_rgba(&vsf_button.colour),
        }
    }

    /// Convert from VSF TokaPath to Toka LayoutNode::Path
    pub fn from_vsf_path(vsf_path: &vsf::types::TokaPath) -> Self {
        let commands = vsf_path
            .commands
            .iter()
            .map(|cmd| match cmd {
                vsf::types::PathCommand::MoveTo(pos) => PathCommand::MoveTo(*pos),
                vsf::types::PathCommand::LineTo(pos) => PathCommand::LineTo(*pos),
                vsf::types::PathCommand::QuadraticTo { ctrl, end } => {
                    PathCommand::QuadraticTo {
                        ctrl: *ctrl,
                        end: *end,
                    }
                }
                vsf::types::PathCommand::CubicTo { ctrl1, ctrl2, end } => PathCommand::CubicTo {
                    ctrl1: *ctrl1,
                    ctrl2: *ctrl2,
                    end: *end,
                },
                vsf::types::PathCommand::Close => PathCommand::Close,
            })
            .collect();

        LayoutNode::Path {
            colour: circle_to_rgba(&vsf_path.colour),
            stroke_width: vsf_path.width,
            commands,
        }
    }

    /// Convert from VSF TokaImage to Toka LayoutNode::Image
    pub fn from_vsf_image(vsf_image: &vsf::types::TokaImage) -> Self {
        LayoutNode::Image {
            pos: vsf_image.pos,
            size: vsf_image.size,
            handle: vsf_image.handle,
            tint: circle_to_rgba(&vsf_image.tint),
        }
    }

    /// Convert from VSF TokaSurface to Toka LayoutNode::Surface
    pub fn from_vsf_surface(vsf_surface: &vsf::types::TokaSurface) -> Self {
        LayoutNode::Surface {
            pos: vsf_surface.pos,
            size: vsf_surface.size,
            handle: vsf_surface.handle,
        }
    }

    /// Convert from VSF TokaNode to Toka LayoutNode
    pub fn from_vsf_node(vsf_node: &vsf::types::TokaNode) -> Self {
        match vsf_node {
            vsf::types::TokaNode::Box(b) => Self::from_vsf_box(b),
            vsf::types::TokaNode::Group(g) => Self::from_vsf_group(g),
            vsf::types::TokaNode::Circle(c) => Self::from_vsf_circle(c),
            vsf::types::TokaNode::Line(l) => Self::from_vsf_line(l),
            vsf::types::TokaNode::Text(t) => Self::from_vsf_text(t),
            vsf::types::TokaNode::Button(b) => Self::from_vsf_button(b),
            vsf::types::TokaNode::Path(p) => Self::from_vsf_path(p),
            vsf::types::TokaNode::Image(i) => Self::from_vsf_image(i),
            vsf::types::TokaNode::Surface(s) => Self::from_vsf_surface(s),
        }
    }
}

/// Convert CircleF4E4 colour to RGBA array
///
/// VSF uses CircleF4E4 for sRGBA colours where:
/// - real = Red channel
/// - imaginary = Green channel
/// - magnitude/phase encode Blue and Alpha
///
/// For now, we'll do a simple mapping (may need refinement):
/// - r() -> R
/// - i() -> G
/// - magnitude() -> B
/// - phase()/magnitude() -> A (normalized)
fn circle_to_rgba(colour: &CircleF4E4) -> [ScalarF4E4; 4] {
    // Simple extraction - may need better colour space conversion
    let r = colour.r();
    let g = colour.i();
    let mag = colour.magnitude();

    // For now, use magnitude for B and full opacity for A
    // TODO: Proper sRGBA encoding from Spirix Circle
    [r, g, mag, ScalarF4E4::ONE]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parent_relative_coords() {
        let parent = LayoutBounds {
            pos: CircleF4E4::from((ScalarF4E4::ZERO, ScalarF4E4::ZERO)),
            size: CircleF4E4::from((ScalarF4E4::ONE, ScalarF4E4::ONE)),
        };

        let child = LayoutNode::Box {
            pos: CircleF4E4::from((
                ScalarF4E4::from(1) / ScalarF4E4::from(4),  // 0.25
                ScalarF4E4::from(1) / ScalarF4E4::from(4),
            )),
            size: CircleF4E4::from((
                ScalarF4E4::from(1) / ScalarF4E4::from(2),  // 0.5
                ScalarF4E4::from(1) / ScalarF4E4::from(2),
            )),
            colour: [ScalarF4E4::ONE; 4],
            children: vec![],
        };

        let bounds = child.compute_bounds(&parent);

        // Child at 0.25 relative = 0.25 absolute in full viewport
        assert_eq!(bounds.pos.r(), ScalarF4E4::from(1) / ScalarF4E4::from(4));
        assert_eq!(bounds.size.r(), ScalarF4E4::from(1) / ScalarF4E4::from(2));
    }

    #[test]
    fn test_circle_bounds() {
        let parent = LayoutBounds {
            pos: CircleF4E4::from((ScalarF4E4::ZERO, ScalarF4E4::ZERO)),
            size: CircleF4E4::from((ScalarF4E4::ONE, ScalarF4E4::ONE)),
        };

        let circle = LayoutNode::Circle {
            center: CircleF4E4::from((
                ScalarF4E4::ONE / ScalarF4E4::from(2),  // 0.5 (centered)
                ScalarF4E4::ONE / ScalarF4E4::from(2),
            )),
            radius: ScalarF4E4::from(3) / ScalarF4E4::from(10),  // 0.3
            colour: [ScalarF4E4::ONE, ScalarF4E4::ZERO, ScalarF4E4::ZERO, ScalarF4E4::ONE],
        };

        let bounds = circle.compute_bounds(&parent);

        // Circle centered at 0.5 relative = 0.5 absolute
        assert_eq!(bounds.pos.r(), ScalarF4E4::ONE / ScalarF4E4::from(2));
        assert_eq!(bounds.pos.i(), ScalarF4E4::ONE / ScalarF4E4::from(2));
    }

    #[test]
    fn test_nested_layout() {
        let viewport = LayoutBounds {
            pos: CircleF4E4::from((ScalarF4E4::ZERO, ScalarF4E4::ZERO)),
            size: CircleF4E4::from((ScalarF4E4::ONE, ScalarF4E4::ONE)),
        };

        let inner_circle = LayoutNode::Circle {
            center: CircleF4E4::from((
                ScalarF4E4::ONE / ScalarF4E4::from(2),  // Centered in parent
                ScalarF4E4::ONE / ScalarF4E4::from(2),
            )),
            radius: ScalarF4E4::from(2) / ScalarF4E4::from(10),  // 0.2 radius
            colour: [ScalarF4E4::ONE, ScalarF4E4::ZERO, ScalarF4E4::ZERO, ScalarF4E4::ONE],
        };

        let outer_box = LayoutNode::Box {
            pos: CircleF4E4::from((
                ScalarF4E4::from(1) / ScalarF4E4::from(4),  // 0.25, 0.25
                ScalarF4E4::from(1) / ScalarF4E4::from(4),
            )),
            size: CircleF4E4::from((
                ScalarF4E4::ONE / ScalarF4E4::from(2),  // 0.5 x 0.5
                ScalarF4E4::ONE / ScalarF4E4::from(2),
            )),
            colour: [ScalarF4E4::ZERO; 4],
            children: vec![inner_circle.clone()],
        };

        // Compute outer box bounds
        let box_bounds = outer_box.compute_bounds(&viewport);
        assert_eq!(box_bounds.pos.r(), ScalarF4E4::from(1) / ScalarF4E4::from(4));
        assert_eq!(box_bounds.size.r(), ScalarF4E4::ONE / ScalarF4E4::from(2));

        // Compute inner circle bounds relative to box
        let circle_bounds = inner_circle.compute_bounds(&box_bounds);

        // Circle at 0.5 in box that starts at 0.25 with size 0.5
        // = 0.25 + 0.5 * 0.5 = 0.25 + 0.25 = 0.5 (viewport center)
        assert_eq!(circle_bounds.pos.r(), ScalarF4E4::ONE / ScalarF4E4::from(2));
    }
}
