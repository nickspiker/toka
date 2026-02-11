//! Reactive scene: scroll to rotate
//!
//! Demonstrates runtime scene graph construction from reactive variables.
//! Scroll controls the rotation of a red box with blue circle child.

use spirix::{CircleF4E4, ScalarF4E4};
use toka::builder::Program;
use toka::capsule::CapsuleBuilder;
use vsf::types::{Fill, VsfType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build scene at runtime from reactive variables
    // The bytecode reads scroll_y and constructs the scene graph each frame

    // Pre-build static children (blue circle in red box)
    let blue_circle = VsfType::roc(
        CircleF4E4::from((0, 0)),
        ScalarF4E4::from(0.3),
        Fill::Solid(Box::new(VsfType::rcb)),
        None,
    );

    let red_box = VsfType::rob(
        CircleF4E4::from((0, 0)),
        CircleF4E4::from((1, 1)),
        Fill::Solid(Box::new(VsfType::rcr)),
        None,
        vec![blue_circle],
    );

    // Wrap in ron node for use with {kw}
    let children_node = VsfType::ron(
        CircleF4E4::from((0, 0)),
        CircleF4E4::from((1, 1)),
        vec![red_box],
    );

    // Bytecode builds row from reactive scroll_y
    let bytecode = Program::new()
        // Push translate
        .ps_c44(0.1, 0.1)

        // Push rotate (from scroll)
        .sy()                        // rotation = scroll_y

        // Push children
        .ps(&children_node.flatten())

        // Build row from stack
        .kw()                        // row(translate, rotate, children)

        // Render
        .rl()
        .hl()
        .build();

    let capsule = CapsuleBuilder::new(bytecode).build()?;
    let output_path = "www/capsules/reactive.vsf";
    std::fs::write(output_path, &capsule)?;

    println!("✓ Created {} ({} bytes)", output_path, capsule.len());
    println!("  Reactive: scroll_y → rotation");
    println!("  Scroll the page to rotate the box!");

    Ok(())
}
