//! Test VSF colour rendering
//!
//! Verifies that VSF colour types are correctly converted to canvas pixels

use toka::builder::Program;
use toka::vm::VM;
use vsf::types::VsfType;

fn main() {
    // Test black (rk)
    println!("Testing VSF rk (black)...");
    let black_bytecode = Program::new().clear(VsfType::rk).hl().build();

    let mut vm_black = VM::with_canvas(black_bytecode, 10, 10);
    vm_black.run().unwrap();
    let rgba = vm_black.canvas().to_rgba_bytes();
    println!(
        "  Black pixel: R={} G={} B={} A={}",
        rgba[0], rgba[1], rgba[2], rgba[3]
    );
    assert_eq!(rgba[3], 255, "Alpha should be 255");

    // Test white (rw)
    println!("\nTesting VSF rw (white)...");
    let white_bytecode = Program::new().clear(VsfType::rw).hl().build();

    let mut vm_white = VM::with_canvas(white_bytecode, 10, 10);
    vm_white.run().unwrap();
    let rgba = vm_white.canvas().to_rgba_bytes();
    println!(
        "  White pixel: R={} G={} B={} A={}",
        rgba[0], rgba[1], rgba[2], rgba[3]
    );
    assert_eq!(rgba[0], 255, "R should be 255");
    assert_eq!(rgba[1], 255, "G should be 255");
    assert_eq!(rgba[2], 255, "B should be 255");
    assert_eq!(rgba[3], 255, "A should be 255");

    // Test red (rr)
    println!("\nTesting VSF rr (red)...");
    let red_bytecode = Program::new().clear(VsfType::rr).hl().build();

    let mut vm_red = VM::with_canvas(red_bytecode, 10, 10);
    vm_red.run().unwrap();
    let rgba = vm_red.canvas().to_rgba_bytes();
    println!(
        "  Red pixel: R={} G={} B={} A={}",
        rgba[0], rgba[1], rgba[2], rgba[3]
    );
    assert!(rgba[0] > 200, "R should be bright (>200)");
    assert_eq!(rgba[3], 255, "A should be 255");

    // Test green (rn)
    println!("\nTesting VSF rn (green)...");
    let green_bytecode = Program::new().clear(VsfType::rn).hl().build();

    let mut vm_green = VM::with_canvas(green_bytecode, 10, 10);
    vm_green.run().unwrap();
    let rgba = vm_green.canvas().to_rgba_bytes();
    println!(
        "  Green pixel: R={} G={} B={} A={}",
        rgba[0], rgba[1], rgba[2], rgba[3]
    );
    assert!(rgba[1] > 200, "G should be bright (>200)");
    assert_eq!(rgba[3], 255, "A should be 255");

    // Test blue (rb)
    println!("\nTesting VSF rb (blue)...");
    let blue_bytecode = Program::new().clear(VsfType::rb).hl().build();

    let mut vm_blue = VM::with_canvas(blue_bytecode, 10, 10);
    vm_blue.run().unwrap();
    let rgba = vm_blue.canvas().to_rgba_bytes();
    println!(
        "  Blue pixel: R={} G={} B={} A={}",
        rgba[0], rgba[1], rgba[2], rgba[3]
    );
    assert!(rgba[2] > 200, "B should be bright (>200)");
    assert_eq!(rgba[3], 255, "A should be 255");

    // Test cyan (rc)
    println!("\nTesting VSF rc (cyan)...");
    let cyan_bytecode = Program::new().clear(VsfType::rc).hl().build();

    let mut vm_cyan = VM::with_canvas(cyan_bytecode, 10, 10);
    vm_cyan.run().unwrap();
    let rgba = vm_cyan.canvas().to_rgba_bytes();
    println!(
        "  Cyan pixel: R={} G={} B={} A={}",
        rgba[0], rgba[1], rgba[2], rgba[3]
    );
    assert!(rgba[1] > 200 && rgba[2] > 200, "G and B should be bright");
    assert_eq!(rgba[3], 255, "A should be 255");

    // Test grey (rg)
    println!("\nTesting VSF rg (grey)...");
    let grey_bytecode = Program::new().clear(VsfType::rg).hl().build();

    let mut vm_grey = VM::with_canvas(grey_bytecode, 10, 10);
    vm_grey.run().unwrap();
    let rgba = vm_grey.canvas().to_rgba_bytes();
    println!(
        "  Grey pixel: R={} G={} B={} A={}",
        rgba[0], rgba[1], rgba[2], rgba[3]
    );
    assert_eq!(rgba[3], 255, "A should be 255");

    println!("\n✓ All VSF colour tests passed!");
}
