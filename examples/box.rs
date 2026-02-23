//! Test example for ro* scene graph types
//!
//! Creates a red box with a blue circle child, wrapped in a transform group.
//! This demonstrates the proper ro* architecture (rob, roc, row) instead of the vt hack.

use spirix::{CircleF4E4, ScalarF4E4};
use toka::builder::Program;
use toka::capsule::CapsuleBuilder;
use vsf::types::{Fill, VsfType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Blue circle (standalone, renders first = behind)
    let blue_circle = VsfType::roc(
        CircleF4E4::from((0, 0)),            // center
        ScalarF4E4::from(0.3),               // radius
        Fill::Solid(Box::new(VsfType::rcb)), // blue fill
        None,                                // no stroke
    );

    // Red box (no children, renders second = on top)
    let red_box = VsfType::rob(
        CircleF4E4::from((0, 0)), // pos (center)
        CircleF4E4::from((1, 1)), // size (2x2 in RU)
        Fill::Solid(Box::new(VsfType::ra([255, 0, 0, 127]))),
        None,   // no stroke
        vec![], // no children
    );

    // Both as siblings - circle first = renders behind red box
    let children_node = VsfType::ron(
        CircleF4E4::from((0, 0)),   // pos (unused for container)
        CircleF4E4::from((0, 0)),   // size (unused for container)
        vec![blue_circle, red_box], // circle behind, box on top
    );

    // Reactive bytecode: builds transform from scroll_y at runtime
    let bytecode = Program::new()
        // Clear canvas to black (VSF RGB colour space)
        .ps(&VsfType::rck.flatten()) // Black (VSF RGB shortcut)
        .cr() // Clear canvas
        // Push translate (static)
        .ps_c44(0.1, 0.1)
        // Push rotate (from scroll wheel)
        .sy() // scroll_y context variable
        .ps_s44(0.001) // Scale factor
        .ml() // rotation = scroll_y * 0.001
        // Push children (ron node)
        .ps(&children_node.flatten())
        // Build row from stack and render
        .kw() // Build row from stack
        .rl() // Render
        .hl() // Halt
        .build();

    // Wrap in VSF capsule format
    let capsule = CapsuleBuilder::new(bytecode).build()?;

    // Write to box.vsf
    let output_path = "www/capsules/box.vsf";
    std::fs::write(output_path, &capsule)?;

    println!("✓ Created {} ({} bytes)", output_path, capsule.len());
    println!("  Scene graph: row → ron → [roc, rob]");
    println!("  Transform: translate + rotate");
    println!("  Shapes: blue circle (behind) + red box (on top)");

    Ok(())
}
