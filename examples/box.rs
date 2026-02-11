//! Test example for ro* scene graph types
//!
//! Creates a red box with a blue circle child, wrapped in a transform group.
//! This demonstrates the proper ro* architecture (rob, roc, row) instead of the vt hack.

use spirix::{CircleF4E4, ScalarF4E4};
use toka::builder::Program;
use toka::capsule::CapsuleBuilder;
use vsf::types::{Fill, Transform, VsfType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Blue circle child (centered at origin, radius 0.3)
    let blue_circle = VsfType::roc(
        CircleF4E4::from((0, 0)),              // center
        ScalarF4E4::from(0.3),                     // radius
        Fill::Solid(Box::new(VsfType::rcb)),       // blue fill
        None,                                       // no stroke
    );

    // Red box parent (centered, fullscreen)
    let red_box = VsfType::rob(
        CircleF4E4::from((0, 0)),              // pos (center)
        CircleF4E4::from((1, 1)),              // size (2x2 in RU)
        Fill::Solid(Box::new(VsfType::rcr)),       // red fill
        None,                                       // no stroke
        vec![blue_circle],                          // child circle
    );

    // Transform group: translate (0.1, 0.1) and rotate 0.5 radians
    let transformed = VsfType::row(
        Transform {
            translate: Some(CircleF4E4::from((0.1, 0.1))),
            rotate: Some(ScalarF4E4::from(0.5)),
            scale: None,
            origin: None,
        },
        vec![red_box],
    );

    // Build Toka bytecode that pushes the scene graph and renders it
    let bytecode = Program::new()
        .ps(&transformed.flatten())  // Push ro* scene graph
        .rl()                         // render_loom
        .hl()                         // halt
        .build();

    // Wrap in VSF capsule format
    let capsule = CapsuleBuilder::new(bytecode).build()?;

    // Write to box.vsf
    let output_path = "www/capsules/box.vsf";
    std::fs::write(output_path, &capsule)?;

    println!("✓ Created {} ({} bytes)", output_path, capsule.len());
    println!("  Scene graph: row → rob → roc");
    println!("  Transform: translate + rotate");
    println!("  Shapes: red box (1 blue circle child)");

    Ok(())
}
