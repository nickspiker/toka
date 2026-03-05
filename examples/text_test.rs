//! Comprehensive text rendering test
//!
//! Tests: sizes, colours, alpha, alignment, multiline wrap (via \n)
//!
//! Layout (top to bottom):
//!   Row 1: Three sizes — small, medium, large (all centered, white)
//!   Row 2: Colours — red, green, blue, cyan, yellow, magenta
//!   Row 3: Alpha blending — white text at 100%, 75%, 50%, 25% over blue box
//!   Row 4: Alignment — left, center, right with guide lines
//!   Row 5: Multiline — hard breaks via \n

use spirix::{CircleF4E4, ScalarF4E4};
use toka::builder::Program;
use toka::capsule::CapsuleBuilder;
use vsf::handle;
use vsf::types::{Fill, VsfType};

const FONT: &[u8] = include_bytes!("/usr/share/fonts/adwaita-mono-fonts/AdwaitaMono-Regular.ttf");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Store font in local 0 so it's only embedded once in bytecode
    let mut p = Program::new()
        .la(1)                   // allocate 1 local slot
        .ps_bytes(FONT).ls(0);  // store font bytes in local[0]

    // Clear to dark grey
    p = p.ps(&VsfType::ra([30, 30, 30, 255]).flatten()).cr();

    // === Row 1: Sizes ===
    p = p.lg(0).ps_c44(-0.35, -0.35).ps_s44(0.03).ps_str("Small")
        .ps(&VsfType::rcw.flatten()).dt();
    p = p.lg(0).ps_c44(0.0, -0.35).ps_s44(0.06).ps_str("Medium")
        .ps(&VsfType::rcw.flatten()).dt();
    p = p.lg(0).ps_c44(0.35, -0.35).ps_s44(0.10).ps_str("Large")
        .ps(&VsfType::rcw.flatten()).dt();

    // === Row 2: Colours ===
    p = p.lg(0).ps_c44(-0.40, -0.20).ps_s44(0.04).ps_str("Red")
        .ps(&VsfType::rcr.flatten()).dt();
    p = p.lg(0).ps_c44(-0.20, -0.20).ps_s44(0.04).ps_str("Green")
        .ps(&VsfType::rcn.flatten()).dt();
    p = p.lg(0).ps_c44(0.0, -0.20).ps_s44(0.04).ps_str("Blue")
        .ps(&VsfType::rcb.flatten()).dt();
    p = p.lg(0).ps_c44(0.20, -0.20).ps_s44(0.04).ps_str("Cyan")
        .ps(&VsfType::rcc.flatten()).dt();
    p = p.lg(0).ps_c44(0.40, -0.20).ps_s44(0.04).ps_str("Yellow")
        .ps(&VsfType::rcy.flatten()).dt();

    // === Row 3: Alpha over blue box ===
    let blue_box = VsfType::rob(
        CircleF4E4::from((ScalarF4E4::ZERO, ScalarF4E4::from_f32(-0.05))),
        CircleF4E4::from((ScalarF4E4::ONE, ScalarF4E4::from_f32(0.08))),
        Fill::Solid(Box::new(VsfType::rcb)),
        None, vec![],
    );
    p = p.ps(&blue_box.flatten()).rl();

    p = p.lg(0).ps_c44(-0.35, -0.05).ps_s44(0.05).ps_str("100%")
        .ps(&VsfType::ra([255, 255, 255, 255]).flatten()).dt();
    p = p.lg(0).ps_c44(-0.12, -0.05).ps_s44(0.05).ps_str("75%")
        .ps(&VsfType::ra([255, 255, 255, 191]).flatten()).dt();
    p = p.lg(0).ps_c44(0.12, -0.05).ps_s44(0.05).ps_str("50%")
        .ps(&VsfType::ra([255, 255, 255, 127]).flatten()).dt();
    p = p.lg(0).ps_c44(0.35, -0.05).ps_s44(0.05).ps_str("25%")
        .ps(&VsfType::ra([255, 255, 255, 63]).flatten()).dt();

    // === Row 4: Alignment with guide lines ===
    let guide = |x: f32| VsfType::rob(
        CircleF4E4::from((ScalarF4E4::from_f32(x), ScalarF4E4::from_f32(0.12))),
        CircleF4E4::from((ScalarF4E4::from_f32(0.003), ScalarF4E4::from_f32(0.08))),
        Fill::Solid(Box::new(VsfType::ra([255, 255, 255, 60]))),
        None, vec![],
    );
    p = p.ps(&guide(-0.35).flatten()).rl();
    p = p.ps(&guide(0.0).flatten()).rl();
    p = p.ps(&guide(0.35).flatten()).rl();

    p = p.lg(0).ps_c44(-0.35, 0.12).ps_s44(0.05).ps_str("Left")
        .ps(&VsfType::rcw.flatten()).dt_left();
    p = p.lg(0).ps_c44(0.0, 0.12).ps_s44(0.05).ps_str("Center")
        .ps(&VsfType::rcw.flatten()).dt_center();
    p = p.lg(0).ps_c44(0.35, 0.12).ps_s44(0.05).ps_str("Right")
        .ps(&VsfType::rcw.flatten()).dt_right();

    // === Row 5: Multiline ===
    p = p.lg(0).ps_c44(0.0, 0.30).ps_s44(0.04)
        .ps_str("Line one\nLine two\nLine three")
        .ps(&VsfType::rcw.flatten()).dt();

    p = p.hl();

    let bytecode = p.build();

    // Resolve handle to get provenance hash and filename
    let handle_name = "text test";
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
