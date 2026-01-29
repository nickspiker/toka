//! Test program - draw a single white rectangle

use toka::builder::Program;

fn main() {
    let bytecode = Program::new()
        // Draw full-screen white rectangle
        .ps_s44(1.0) // r = 1.0
        .ps_s44(1.0) // g = 1.0
        .ps_s44(1.0) // b = 1.0
        .cb() // rgb
        .ps_s44(0.0) // x = 0.0
        .ps_s44(0.0) // y = 0.0
        .ps_s44(1.0) // w = 1.0
        .ps_s44(1.0) // h = 1.0
        .fr() // fill_rect
        .hl() // halt
        .build();

    // Output as JavaScript
    print!("const TEST_BYTECODE = new Uint8Array([");
    for (i, byte) in bytecode.iter().enumerate() {
        if i % 16 == 0 {
            print!("\n    ");
        }
        print!("0x{:02x}", byte);
        if i < bytecode.len() - 1 {
            print!(", ");
        }
    }
    println!("\n]);");
}
