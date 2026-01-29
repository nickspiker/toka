//! Debug VSF colour conversions in builder
//!
//! Shows what RGB values VSF colours convert to

use vsf::colour::convert::RgbLinear;
use vsf::types::VsfType;

fn main() {
    println!("Testing VSF colour conversions:\n");

    let colours = vec![
        ("rk (black)", VsfType::rk),
        ("rw (white)", VsfType::rw),
        ("rr (red)", VsfType::rr),
        ("rn (green)", VsfType::rn),
        ("rb (blue)", VsfType::rb),
        ("rc (cyan)", VsfType::rc),
        ("rg (grey)", VsfType::rg),
    ];

    for (name, colour) in colours {
        let rgb: RgbLinear = colour.to_rgb_linear().expect("Failed to convert colour");

        println!(
            "{:15} -> r={:.6} g={:.6} b={:.6}",
            name, rgb.r, rgb.g, rgb.b
        );

        // Show what these would be as u8
        let r_u8 = (rgb.r.clamp(0.0, 1.0) * 255.0).round() as u8;
        let g_u8 = (rgb.g.clamp(0.0, 1.0) * 255.0).round() as u8;
        let b_u8 = (rgb.b.clamp(0.0, 1.0) * 255.0).round() as u8;
        println!(
            "                -> R={:3} G={:3} B={:3}\n",
            r_u8, g_u8, b_u8
        );
    }
}
