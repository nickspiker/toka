//! Table rendering test
//!
//! Exercises the {tb} draw_table opcode with header, borders, alt rows.

use toka::builder::Program;
use toka::capsule::CapsuleBuilder;
use vsf::handle;
use vsf::types::VsfType;

const FONT: &[u8] = include_bytes!("/usr/share/fonts/adwaita-mono-fonts/AdwaitaMono-Regular.ttf");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let white = VsfType::rcw.flatten();
    let header_bg = VsfType::ra([40, 50, 80, 255]).flatten();
    let border = VsfType::ra([80, 80, 80, 255]).flatten();
    let alt_bg = VsfType::ra([35, 35, 40, 255]).flatten();

    let p = Program::new()
        .la(1)
        .ps_bytes(FONT).ls(0)
        // Clear to dark
        .ps(&VsfType::ra([20, 20, 25, 255]).flatten()).cr()
        // Title
        .lg(0)
        .ps_c44(0.0, -0.45)
        .ps_s44(0.05)
        .ps_str("Table Test")
        .ps(&VsfType::rcw.flatten())
        .dt_center()
        // Table
        .lg(0)
        .ps_c44(0.0, -0.30)
        .ps_s44(0.03)
        .draw_table(
            &["Name", "Role", "Status"],
            &[
                &["Alice", "Engineer", "Active"],
                &["Bob", "The quick brown fox jumps over the lazy dog", "Away"],
                &["Carol", "PM", "Active"],
                &["Dave", "QA", "Offline"],
                &["Eve", "Security", "Active"],
            ],
            &white,
            Some(&[0.15, 0.4, 0.15]),  // explicit widths, center wraps
            Some(&header_bg),
            Some(&border),
            Some(&alt_bg),
            Some("lcr"),
            None,
        )
        .hl();

    let bytecode = p.build();

    let handle_name = "table test";
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
