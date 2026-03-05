//! Text alignment test
//!
//! Renders a grey rectangle with three text labels demonstrating alignment:
//!   - "centered"         — center-aligned at box center (0, 0)      — white
//!   - "left at right"    — left-aligned  at right edge (+0.5, -0.15) — cyan
//!   - "right at left"    — right-aligned at left edge  (-0.5,  0.15) — yellow
//!
//! Box is 1.0 RU wide × 0.5 RU tall (full extents), centered at origin.
//! A thin vertical guide line of rects marks the three anchor X positions.

use spirix::{CircleF4E4, ScalarF4E4};
use toka::builder::Program;
use toka::capsule::CapsuleBuilder;
use vsf::handle;
use vsf::types::{Fill, VsfType};

const FONT: &[u8] = include_bytes!("/usr/share/fonts/adwaita-mono-fonts/AdwaitaMono-Regular.ttf");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let half = ScalarF4E4::from_f32(0.5);

    // Grey box: 1.0 RU wide × 0.5 RU tall, centered at origin
    let grey_box = VsfType::rob(
        CircleF4E4::from((ScalarF4E4::ZERO, ScalarF4E4::ZERO)),
        CircleF4E4::from((ScalarF4E4::ONE, half)),
        Fill::Solid(Box::new(VsfType::ra([60, 60, 60, 255]))),
        None,
        vec![],
    );

    // Thin vertical guide at x=0 (center anchor)
    let guide_center = VsfType::rob(
        CircleF4E4::from((ScalarF4E4::ZERO, ScalarF4E4::ZERO)),
        CircleF4E4::from((ScalarF4E4::from_f32(0.005), half)),
        Fill::Solid(Box::new(VsfType::ra([255, 255, 255, 80]))),
        None, vec![],
    );

    // Guide at x=+0.5 (right edge)
    let guide_right = VsfType::rob(
        CircleF4E4::from((half, ScalarF4E4::ZERO)),
        CircleF4E4::from((ScalarF4E4::from_f32(0.005), half)),
        Fill::Solid(Box::new(VsfType::ra([0, 220, 220, 80]))),
        None, vec![],
    );

    // Guide at x=-0.5 (left edge)
    let guide_left = VsfType::rob(
        CircleF4E4::from((-half, ScalarF4E4::ZERO)),
        CircleF4E4::from((ScalarF4E4::from_f32(0.005), half)),
        Fill::Solid(Box::new(VsfType::ra([220, 220, 0, 80]))),
        None, vec![],
    );

    let bytecode = Program::new()
        // Store font in local 0
        .la(1)
        .ps_bytes(FONT).ls(0)
        // Clear to near-black
        .ps(&VsfType::rck.flatten())
        .cr()
        // Grey box + guide lines
        .ps(&grey_box.flatten())
        .rl()
        .ps(&guide_center.flatten())
        .rl()
        .ps(&guide_right.flatten())
        .rl()
        .ps(&guide_left.flatten())
        .rl()
        // Center-aligned at (0, 0) — white
        .lg(0)
        .ps_c44(0.0, 0.0)
        .ps_s44(0.06)
        .ps_str("centered")
        .ps(&VsfType::rcw.flatten())
        .dt_center()
        // Left-aligned at right edge (+0.5, -0.1) — cyan
        .lg(0)
        .ps_c44(0.5, -0.1)
        .ps_s44(0.06)
        .ps_str("left at right edge")
        .ps(&VsfType::ra([0, 220, 220, 255]).flatten())
        .dt_left()
        // Right-aligned at left edge (-0.5, +0.1) — yellow
        .lg(0)
        .ps_c44(-0.5, 0.1)
        .ps_s44(0.06)
        .ps_str("right at left edge")
        .ps(&VsfType::ra([220, 220, 0, 255]).flatten())
        .dt_right()
        .hl()
        .build();

    let handle_name = "text align";
    let (public_id, filename) = handle::resolve_handle(handle_name);
    let provenance_bytes: [u8; 32] = *public_id.as_bytes();

    let capsule = CapsuleBuilder::new(bytecode)
        .provenance(provenance_bytes)
        .build()?;

    let path = format!("www/capsules/{}", filename);
    std::fs::write(&path, &capsule)?;
    println!("Handle: \"{}\"", handle_name);
    println!("Public ID: {}", public_id.to_hex());
    println!("Created {} ({} bytes)", path, capsule.len());
    Ok(())
}
