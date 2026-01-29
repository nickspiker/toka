//! Comparison of low-level vs high-level builder APIs
//!
//! Demonstrates how the high-level helpers prevent stack ordering mistakes
//! and improve code readability.

use toka::builder::Program;
use vsf::types::VsfType;

fn main() {
    println!("=== Low-Level API (manual stack management) ===\n");

    let low_level = Program::new()
        .ps_s44(0.0) // r = 0.0
        .ps_s44(0.7) // g = 0.7
        .ps_s44(0.7) // b = 0.7
        .cb() // rgb
        .ps_s44(0.35) // x
        .ps_s44(0.2) // y
        .ps_s44(0.3) // w
        .ps_s44(0.08) // h
        .fr() // fill_rect
        .hl()
        .build();

    println!("Bytecode size: {} bytes", low_level.len());
    println!("Requires: understanding stack order, opcode mnemonics");
    println!("Risk: easy to get stack order wrong\n");

    println!("=== High-Level API (automatic stack management) ===\n");

    let high_level = Program::new()
        .fill_rect(0.35, 0.2, 0.3, 0.08, VsfType::rc) // VSF cyan with spectral definition
        .hl()
        .build();

    println!("Bytecode size: {} bytes", high_level.len());
    println!("Requires: just normal Rust function calls");
    println!("Risk: none - stack order handled automatically\n");

    println!("=== Comparison ===\n");
    println!("Same bytecode? {}", low_level == high_level);
    println!(
        "Size difference: {} bytes",
        high_level.len() as i32 - low_level.len() as i32
    );
    println!("Low-level lines of code: 10");
    println!("High-level lines of code: 1");
    println!("Readability improvement: 10x");
    println!("\nTrade-off: High-level API is slightly larger but:");
    println!("  ✓ Impossible to get stack order wrong");
    println!("  ✓ Self-documenting parameter names");
    println!("  ✓ Type-safe Rust function signatures");
}
