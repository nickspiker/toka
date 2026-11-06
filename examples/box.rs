//! Test example for ro* scene graph types
//!
//! Creates a blue circle behind a red box, inside a node that moves and turns them.
//! This demonstrates the ro* scene graph: a node (ron) holding an ellipse (roe) and a box (rob).

use spirix::{CircleF4E4, ScalarF4E4};
use toka::builder::{filled_box, filled_circle, Program};
use toka::capsule::CapsuleBuilder;
use vsf::types::{Node, VsfType};

const FONT: &[u8] = include_bytes!("/usr/share/fonts/adwaita-mono-fonts/AdwaitaMono-Regular.ttf");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Blue circle (standalone, renders first = behind)
    let blue_circle = filled_circle(
        CircleF4E4::from((0, 0)),
        ScalarF4E4::from(0.3),
        VsfType::rcb,
    );

    // Red box (no children, renders second = on top)
    let red_box = filled_box(
        CircleF4E4::from((0, 0)), // centre
        CircleF4E4::from((1, 1)), // full width and height (1×1 RU)
        VsfType::ra([255, 0, 0, 127]),
    );

    // Both as siblings, in painter's order: circle first = behind the red box
    let mut children_node = Node::new();
    children_node.children = vec![blue_circle, red_box];
    let children_node = VsfType::ron(Box::new(children_node));

    // Reactive bytecode: builds transform from scroll_y at runtime
    let bytecode = Program::new()
        // Clear canvas to black (VSF RGB colour space)
        .ps(&VsfType::rck.flatten()) // Black (VSF RGB shortcut)
        .cr() // Clear canvas
        // Push translate (static)
        .ps_c44(0.1, 0.1)
        // Push rotate (from scroll wheel)
        .sy() // scroll_y context variable
        .ps_s44(1. / (1 << 12) as f32) // turns per scroll step
        .ml() // rotation = scroll_y / 4096 turns
        // Push children (ron node)
        .ps(&children_node.flatten())
        // Build the moving node from the stack and render
        .kw() // Build node from stack
        .rl() // Render
        // Draw text "Box" over the scene
        .ps_bytes(FONT)                  // font bytes
        .ps_c44(-0.15, -0.05)            // pos (near center)
        .ps_s44(0.08)                    // size (0.08 RU)
        .ps_str("Box")                   // text
        .ps(&VsfType::rcw.flatten()) // white
        .dt_center()
        .hl() // Halt
        .build();

    // Wrap in VSF capsule format
    let capsule = CapsuleBuilder::new(bytecode).build()?;

    // Write to box.vsf
    let output_path = "www/capsules/box.vsf";
    std::fs::write(output_path, &capsule)?;

    println!("✓ Created {} ({} bytes)", output_path, capsule.len());
    println!("  Scene graph: ron (moved) → ron → [roe, rob]");
    println!("  Transform: translate + rotate");
    println!("  Shapes: blue circle (behind) + red box (on top)");

    Ok(())
}
