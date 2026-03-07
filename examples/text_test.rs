//! Comprehensive text rendering test
//!
//! Tests: sizes, colours, alpha, alignment, multiline, word wrapping, leading
//!
//! Layout (top to bottom):
//!   Row 1: Three sizes — small, medium, large (all centered, white)
//!   Row 2: Colours — red, green, blue, cyan, yellow
//!   Row 3: Alpha blending — white text at 100%, 75%, 50%, 25% over blue box
//!   Row 4: Alignment — left, center, right with guide lines
//!   Row 5: Multiline via \n (centered, should stack neatly)
//!   Row 6: Word wrap — left-aligned, center-aligned, right-aligned wrap boxes
//!   Row 7: Leading test — tight (0.8), normal (1.0), loose (1.5)

use spirix::{CircleF4E4, ScalarF4E4};
use toka::builder::Program;
use toka::capsule::CapsuleBuilder;
use vsf::handle;
use vsf::types::{Fill, VsfType};

const FONT: &[u8] = include_bytes!("/usr/share/fonts/adwaita-mono-fonts/AdwaitaMono-Regular.ttf");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut p = Program::new()
        .la(1)
        .ps_bytes(FONT).ls(0);

    // Clear to dark grey
    p = p.ps(&VsfType::ra([30, 30, 30, 255]).flatten()).cr();

    // --- Row labels (left edge) ---
    let label_x = -0.48;
    let label_size = ScalarF4E4::from_f32(0.02);
    let dim = VsfType::ra([100, 100, 100, 255]);

    // === Row 1: Sizes (y = -0.42) ===
    p = p.lg(0).ps_c44(label_x, -0.42).ps_s44(label_size).ps_str("SIZES")
        .ps(&dim.flatten()).dt_left();

    // Baseline-align: larger text needs higher y (anchor = top of text)
    p = p.lg(0).ps_c44(-0.25, -0.38).ps_s44(0.025).ps_str("Small")
        .ps(&VsfType::rcw.flatten()).dt_center();
    p = p.lg(0).ps_c44(0.0, -0.40).ps_s44(0.05).ps_str("Medium")
        .ps(&VsfType::rcw.flatten()).dt_center();
    p = p.lg(0).ps_c44(0.30, -0.44).ps_s44(0.09).ps_str("Large")
        .ps(&VsfType::rcw.flatten()).dt_center();

    // === Row 2: Colours (y = -0.30) ===
    p = p.lg(0).ps_c44(label_x, -0.30).ps_s44(label_size).ps_str("COLOUR")
        .ps(&dim.flatten()).dt_left();

    p = p.lg(0).ps_c44(-0.30, -0.28).ps_s44(0.035).ps_str("Red")
        .ps(&VsfType::rcr.flatten()).dt_center();
    p = p.lg(0).ps_c44(-0.12, -0.28).ps_s44(0.035).ps_str("Green")
        .ps(&VsfType::rcn.flatten()).dt_center();
    p = p.lg(0).ps_c44(0.06, -0.28).ps_s44(0.035).ps_str("Blue")
        .ps(&VsfType::rcb.flatten()).dt_center();
    p = p.lg(0).ps_c44(0.22, -0.28).ps_s44(0.035).ps_str("Cyan")
        .ps(&VsfType::rcc.flatten()).dt_center();
    p = p.lg(0).ps_c44(0.38, -0.28).ps_s44(0.035).ps_str("Yellow")
        .ps(&VsfType::rcy.flatten()).dt_center();

    // === Row 3: Alpha over blue box (y = -0.18) ===
    p = p.lg(0).ps_c44(label_x, -0.18).ps_s44(label_size).ps_str("ALPHA")
        .ps(&dim.flatten()).dt_left();

    let blue_box = VsfType::rob(
        CircleF4E4::from((ScalarF4E4::from_f32(0.05), ScalarF4E4::from_f32(-0.16))),
        CircleF4E4::from((ScalarF4E4::from_f32(0.85), ScalarF4E4::from_f32(0.06))),
        Fill::Solid(Box::new(VsfType::rcb)),
        None, vec![],
    );
    p = p.ps(&blue_box.flatten()).rl();

    p = p.lg(0).ps_c44(-0.25, -0.19).ps_s44(0.04).ps_str("100%")
        .ps(&VsfType::ra([255, 255, 255, 255]).flatten()).dt_center();
    p = p.lg(0).ps_c44(-0.05, -0.19).ps_s44(0.04).ps_str("75%")
        .ps(&VsfType::ra([255, 255, 255, 191]).flatten()).dt_center();
    p = p.lg(0).ps_c44(0.15, -0.19).ps_s44(0.04).ps_str("50%")
        .ps(&VsfType::ra([255, 255, 255, 127]).flatten()).dt_center();
    p = p.lg(0).ps_c44(0.35, -0.19).ps_s44(0.04).ps_str("25%")
        .ps(&VsfType::ra([255, 255, 255, 63]).flatten()).dt_center();

    // === Row 4: Alignment with guide lines (y = -0.04) ===
    p = p.lg(0).ps_c44(label_x, -0.06).ps_s44(label_size).ps_str("ALIGN")
        .ps(&dim.flatten()).dt_left();

    let guide = |x: f32| VsfType::rob(
        CircleF4E4::from((ScalarF4E4::from_f32(x), ScalarF4E4::from_f32(-0.04))),
        CircleF4E4::from((ScalarF4E4::from_f32(0.002), ScalarF4E4::from_f32(0.06))),
        Fill::Solid(Box::new(VsfType::ra([255, 80, 80, 120]))),
        None, vec![],
    );
    p = p.ps(&guide(-0.30).flatten()).rl();
    p = p.ps(&guide(0.0).flatten()).rl();
    p = p.ps(&guide(0.30).flatten()).rl();

    p = p.lg(0).ps_c44(-0.30, -0.04).ps_s44(0.04).ps_str("Left here")
        .ps(&VsfType::rcw.flatten()).dt_left();
    p = p.lg(0).ps_c44(0.0, -0.04).ps_s44(0.04).ps_str("Centered")
        .ps(&VsfType::rcw.flatten()).dt_center();
    p = p.lg(0).ps_c44(0.30, -0.04).ps_s44(0.04).ps_str("Right here")
        .ps(&VsfType::rcw.flatten()).dt_right();

    // === Row 5: Multiline via \n (y = 0.08) ===
    p = p.lg(0).ps_c44(label_x, 0.06).ps_s44(label_size).ps_str("MULTI")
        .ps(&dim.flatten()).dt_left();

    // Guide at center
    p = p.ps(&guide(0.0).flatten()).rl();

    p = p.lg(0).ps_c44(0.0, 0.06).ps_s44(0.03)
        .ps_str("Line one\nLine two (longer)\nThree")
        .ps(&VsfType::rcw.flatten()).dt_center();

    // === Row 6: Word wrap boxes (y = 0.22) ===
    p = p.lg(0).ps_c44(label_x, 0.20).ps_s44(label_size).ps_str("WRAP")
        .ps(&dim.flatten()).dt_left();

    let wrap_text = "The quick brown fox jumps over the lazy dog near the riverbank.";

    // Wrap box outlines (visual guides for the wrap region)
    let wrap_box = |x: f32| VsfType::rob(
        CircleF4E4::from((ScalarF4E4::from_f32(x), ScalarF4E4::from_f32(0.26))),
        CircleF4E4::from((ScalarF4E4::from_f32(0.28), ScalarF4E4::from_f32(0.12))),
        Fill::Solid(Box::new(VsfType::ra([50, 50, 50, 255]))),
        None, vec![],
    );
    p = p.ps(&wrap_box(-0.30).flatten()).rl();
    p = p.ps(&wrap_box(0.0).flatten()).rl();
    p = p.ps(&wrap_box(0.30).flatten()).rl();

    // Left-aligned wrap
    p = p.lg(0).ps_c44(-0.30, 0.21).ps_s44(0.025).ps_str(wrap_text)
        .ps(&VsfType::ra([200, 200, 200, 255]).flatten())
        .dt_wrap_align(0.28, 1);

    // Center-aligned wrap
    p = p.lg(0).ps_c44(0.0, 0.21).ps_s44(0.025).ps_str(wrap_text)
        .ps(&VsfType::ra([200, 200, 200, 255]).flatten())
        .dt_wrap_align(0.28, 0);

    // Right-aligned wrap
    p = p.lg(0).ps_c44(0.30, 0.21).ps_s44(0.025).ps_str(wrap_text)
        .ps(&VsfType::ra([200, 200, 200, 255]).flatten())
        .dt_wrap_align(0.28, 2);

    // Labels under wrap boxes
    p = p.lg(0).ps_c44(-0.30, 0.34).ps_s44(0.018).ps_str("left")
        .ps(&dim.flatten()).dt_center();
    p = p.lg(0).ps_c44(0.0, 0.34).ps_s44(0.018).ps_str("center")
        .ps(&dim.flatten()).dt_center();
    p = p.lg(0).ps_c44(0.30, 0.34).ps_s44(0.018).ps_str("right")
        .ps(&dim.flatten()).dt_center();

    // === Row 7: Leading test (y = 0.42) ===
    p = p.lg(0).ps_c44(label_x, 0.40).ps_s44(label_size).ps_str("LEAD")
        .ps(&dim.flatten()).dt_left();

    let lead_text = "Line A\nLine B\nLine C";

    // Tight leading (0.8)
    p = p.lg(0).ps_c44(-0.25, 0.40).ps_s44(0.025).ps_str(lead_text)
        .ps(&VsfType::rcw.flatten()).dt_leading(0.8);
    p = p.lg(0).ps_c44(-0.25, 0.48).ps_s44(0.015).ps_str("0.8")
        .ps(&dim.flatten()).dt_center();

    // Normal leading (1.0)
    p = p.lg(0).ps_c44(0.0, 0.40).ps_s44(0.025).ps_str(lead_text)
        .ps(&VsfType::rcw.flatten()).dt_leading(1.0);
    p = p.lg(0).ps_c44(0.0, 0.48).ps_s44(0.015).ps_str("1.0")
        .ps(&dim.flatten()).dt_center();

    // Loose leading (1.5)
    p = p.lg(0).ps_c44(0.25, 0.40).ps_s44(0.025).ps_str(lead_text)
        .ps(&VsfType::rcw.flatten()).dt_leading(1.5);
    p = p.lg(0).ps_c44(0.25, 0.48).ps_s44(0.015).ps_str("1.5")
        .ps(&dim.flatten()).dt_center();

    p = p.hl();

    let bytecode = p.build();

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
