//! Test the exact same bytecode that's in fgtw.html
//!
//! Uses RU coordinates with center-origin:
//! - (0.0, 0.0) = center of canvas
//! - Rectangle is centered at (x, y)

use toka::builder::Program;
use toka::vm::VM;
use vsf::types::VsfType;

fn main() {
    println!("Testing white square with RU coordinates...");

    // Draw white square centered at (0,0), 0.5 RU wide/tall
    let bytecode = Program::new()
        .clear(VsfType::rk)
        .fill_rect(0.0, 0.0, 0.5, 0.5, VsfType::rw)
        .hl()
        .build();

    println!("Bytecode length: {} bytes", bytecode.len());
    println!("Bytecode: {:02x?}", bytecode);

    let mut vm = VM::with_canvas(bytecode, 100, 100);
    vm.run().unwrap();

    let rgba = vm.canvas().to_rgba_bytes();

    // Check corners (should be black)
    println!("\nCorner pixels (should be black):");
    println!(
        "  Top-left (0,0): R={} G={} B={} A={}",
        rgba[0], rgba[1], rgba[2], rgba[3]
    );

    let tr_idx = (0 * 100 + 99) * 4;
    println!(
        "  Top-right (99,0): R={} G={} B={} A={}",
        rgba[tr_idx],
        rgba[tr_idx + 1],
        rgba[tr_idx + 2],
        rgba[tr_idx + 3]
    );

    // Check center (should be white)
    println!("\nCenter pixels (should be white):");
    let center_idx = (50 * 100 + 50) * 4;
    println!(
        "  Center (50,50): R={} G={} B={} A={}",
        rgba[center_idx],
        rgba[center_idx + 1],
        rgba[center_idx + 2],
        rgba[center_idx + 3]
    );

    let edge_idx = (25 * 100 + 25) * 4;
    println!(
        "  Edge of white (25,25): R={} G={} B={} A={}",
        rgba[edge_idx],
        rgba[edge_idx + 1],
        rgba[edge_idx + 2],
        rgba[edge_idx + 3]
    );

    if rgba[center_idx] == 255 && rgba[center_idx + 1] == 255 && rgba[center_idx + 2] == 255 {
        println!("\n✓ SUCCESS: White square rendered correctly!");
    } else {
        println!("\n✗ FAIL: Center is not white!");
    }
}
