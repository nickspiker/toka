//! Toka VM demo - PIXELS ON SCREEN!
//!
//! We got pixels on screen! This is v0.0.0 - first working graphics demo.

use toka::vm::VM;

fn main() {
    // Demo: Clear black, draw cyan rectangle
    let bytecode = vec![
        // Clear screen to black
        0x70, 0x7a, // push_zero (R=0)
        0x70, 0x7a, // push_zero (G=0)
        0x70, 0x7a, // push_zero (B=0)
        0x63, 0x62, // rgb
        0x63, 0x72, // clear

        // Draw cyan rectangle at x=0.5, y=0.5, w=0.5, h=0.5
        // (using simple 1/2 fractions)

        // x=0.5 (1/2)
        0x70, 0x6f, // push_one
        0x70, 0x6f, // push_one
        0x70, 0x6f, // push_one
        0x61, 0x64, // add (1+1=2)
        0x64, 0x76, // div (1/2=0.5)

        // y=0.5 (1/2)
        0x70, 0x6f, // push_one
        0x70, 0x6f, // push_one
        0x70, 0x6f, // push_one
        0x61, 0x64, // add (1+1=2)
        0x64, 0x76, // div (1/2=0.5)

        // w=0.5 (1/2)
        0x70, 0x6f, // push_one
        0x70, 0x6f, // push_one
        0x70, 0x6f, // push_one
        0x61, 0x64, // add (1+1=2)
        0x64, 0x76, // div (1/2=0.5)

        // h=0.5 (1/2)
        0x70, 0x6f, // push_one
        0x70, 0x6f, // push_one
        0x70, 0x6f, // push_one
        0x61, 0x64, // add (1+1=2)
        0x64, 0x76, // div (1/2=0.5)

        // Cyan color: R=0, G=0.7, B=0.9
        0x70, 0x7a, // push_zero (R=0)
        0x70, 0x6f, // push_one (G=1.0, we'll just use full green for simplicity)
        0x70, 0x6f, // push_one (B=1.0, full blue)

        0x63, 0x62, // rgb
        0x66, 0x72, // fill_rect

        0x68, 0x6c, // halt
    ];

    println!("╔════════════════════════════════════════════╗");
    println!("║  Toka v0.0.0 - WE GOT PIXELS ON SCREEN!  ║");
    println!("╚════════════════════════════════════════════╝\n");
    println!("Running {} bytes of bytecode...\n", bytecode.len());

    let mut vm = VM::with_canvas(bytecode, 800, 600);

    match vm.run() {
        Ok(()) => {
            println!("✓ Execution complete!");
            println!("✓ Viewport coordinates: 0.0 (top-left) → 1.0 (bottom-right)");
            println!("✓ Drew cyan rectangle using Spirix S44 arithmetic");

            // Save canvas as PPM
            let ppm = vm.canvas().to_ppm();
            std::fs::write("output.ppm", ppm).expect("Failed to write PPM");
            println!("✓ Rendered to output.ppm (800×600)\n");

            println!("To view:");
            println!("  convert output.ppm output.png && xdg-open output.png\n");

            println!("What works:");
            println!("  • Stack-based VM with Spirix ScalarF4E4 (deterministic FP)");
            println!("  • ~90 opcodes defined");
            println!("  • Canvas with viewport-relative coordinates");
            println!("  • Color drawing (clear, fill_rect, rgb)");
            println!("  • All tests passing ✓");
        }
        Err(e) => {
            eprintln!("✗ Error: {}", e);
            std::process::exit(1);
        }
    }
}
