//! FGTW Landing Page — rendered entirely by Toka VM
//!
//! Replicates the fgtw.org landing page (previously CSS/HTML) as a toka capsule.
//! Exercises: text alignment, word wrapping, rectangles, circles, lines, multiple sizes.

use spirix::{CircleF4E4, ScalarF4E4};
use toka::builder::Program;
use toka::capsule::CapsuleBuilder;
use vsf::handle;
use vsf::types::{Fill, VsfType};

const FONT: &[u8] = include_bytes!("/usr/share/fonts/adwaita-mono-fonts/AdwaitaMono-Regular.ttf");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Colours
    let cyan = VsfType::ra([79, 195, 247, 255]);       // #4FC3F7 — title + peer count
    let _white = VsfType::ra([255, 255, 255, 255]);
    let grey = VsfType::ra([136, 136, 136, 255]);       // #888 — subtitle
    let green = VsfType::ra([76, 175, 80, 255]);         // #4CAF50 — status dot/border
    let dim_green = VsfType::ra([76, 175, 80, 40]);      // status bg

    // Status indicator box
    let status_bg = VsfType::rob(
        CircleF4E4::from((ScalarF4E4::ZERO, ScalarF4E4::from_f32(0.15))),
        CircleF4E4::from((ScalarF4E4::from_f32(0.6), ScalarF4E4::from_f32(0.08))),
        Fill::Solid(Box::new(dim_green.clone())),
        None,
        vec![],
    );

    // Status dot (small green circle)
    let status_dot = VsfType::roc(
        CircleF4E4::from((ScalarF4E4::from_f32(-0.2), ScalarF4E4::from_f32(0.15))),
        ScalarF4E4::from_f32(0.015),
        Fill::Solid(Box::new(green.clone())),
        None,
    );

    let bytecode = Program::new()
        // Store font in local 0
        .la(1)
        .ps_bytes(FONT).ls(0)
        // Clear to black
        .ps(&VsfType::rck.flatten())
        .cr()
        // --- Title: "FGTW" — large, cyan, centered ---
        .lg(0)
        .ps_c44(0.0, -0.25)
        .ps_s44(ScalarF4E4::from_f32(0.18))
        .ps_str("FGTW")
        .ps(&cyan.flatten())
        .dt_center()
        // --- Subtitle: "Fractal Gradient Trust Web" — grey, centered ---
        .lg(0)
        .ps_c44(0.0, -0.05)
        .ps_s44(ScalarF4E4::from_f32(0.055))
        .ps_str("Fractal Gradient Trust Web")
        .ps(&grey.flatten())
        .dt_center()
        // --- Status box background ---
        .ps(&status_bg.flatten())
        .rl()
        // --- Status dot ---
        .ps(&status_dot.flatten())
        .rl()
        // --- "operational" text in green ---
        .lg(0)
        .ps_c44(0.0, 0.15)
        .ps_s44(ScalarF4E4::from_f32(0.04))
        .ps_str("operational")
        .ps(&green.flatten())
        .dt_center()
        // --- Peer count line ---
        .lg(0)
        .ps_c44(0.0, 0.32)
        .ps_s44(ScalarF4E4::from_f32(0.055))
        .ps_str("0 peers")
        .ps(&cyan.flatten())
        .dt_center()
        // --- Decorative line ---
        .ps_c44(-0.4, 0.5)
        .ps_c44(0.4, 0.5)
        .ps(&VsfType::ra([50, 50, 50, 255]).flatten())
        .dl()
        // --- Wrap test: description text below the line ---
        .lg(0)
        .ps_c44(-0.4, 0.6)
        .ps_s44(ScalarF4E4::from_f32(0.03))
        .ps_str("A decentralised trust network built on cryptographic identity, \
peer-to-peer connectivity, and fractal reputation propagation. \
No servers. No accounts. Just math.")
        .ps(&grey.flatten())
        .dt_wrap(0.8)
        .hl()
        .build();

    // Build for both "fgtw" and "octopus" handles
    for handle_name in &["fgtw", "octopus"] {
        let (public_id, filename) = handle::resolve_handle(handle_name);
        let provenance_bytes: [u8; 32] = *public_id.as_bytes();

        let capsule = CapsuleBuilder::new(bytecode.clone())
            .provenance(provenance_bytes)
            .build()?;

        let path = format!("www/capsules/{}", filename);
        std::fs::write(&path, &capsule)?;

        println!("Handle: {}", handle_name);
        println!("Public ID: {}", public_id.to_hex());
        println!("Created {} ({} bytes)", path, capsule.len());
    }

    Ok(())
}
