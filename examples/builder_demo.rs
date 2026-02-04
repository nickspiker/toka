//! Demo of the Toka bytecode builder with CircleF4E4 coordinates
//!
//! Shows how to use the Rust DSL to construct Toka programs with
//! type safety and compile-time checking, using CircleF4E4 for
//! coordinate pairs.

use toka::builder::Program;
use toka::vm::VM;

fn main() -> Result<(), String> {
    println!("=== Toka Builder Demo (with CircleF4E4) ===\n");

    // Example 1: Draw a red square at center
    println!("Example 1: Drawing red square at center");
    let bytecode = Program::new()
        .ps_s44(1.0)        // r = 1 (red)
        .ps_s44(0.0)        // g = 0
        .ps_s44(0.0)        // b = 0
        .ps_s44(1.0)        // a = 1 (opaque)
        .ps_c44(0.0, 0.0)   // position: center (x=0, y=0)
        .ps_c44(0.5, 0.5)   // size: 0.5 RU wide and tall
        .fr()               // fill_rect
        .hl()               // halt
        .build();

    let mut vm = VM::with_canvas(bytecode, 800, 600);
    vm.run()?;
    println!("✓ Red square rendered\n");

    // Example 2: Draw a green circle
    println!("Example 2: Drawing green circle");
    let bytecode = Program::new()
        .ps_s44(0.0)        // r = 0
        .ps_s44(1.0)        // g = 1 (green)
        .ps_s44(0.0)        // b = 0
        .ps_s44(1.0)        // a = 1 (opaque)
        .ps_c44(0.0, 0.0)   // center: (x=0, y=0)
        .ps_s44(0.3)        // radius: 0.3 RU
        .fc()               // fill_circle
        .hl()               // halt
        .build();

    let mut vm = VM::with_canvas(bytecode, 800, 600);
    vm.run()?;
    println!("✓ Green circle rendered\n");

    // Example 3: Draw a blue circle outline
    println!("Example 3: Drawing blue circle outline");
    let bytecode = Program::new()
        .ps_s44(0.0)        // r = 0
        .ps_s44(0.0)        // g = 0
        .ps_s44(1.0)        // b = 1 (blue)
        .ps_s44(1.0)        // a = 1 (opaque)
        .ps_c44(0.0, 0.0)   // center: (x=0, y=0)
        .ps_s44(0.4)        // radius: 0.4 RU
        .ps_s44(0.05)       // stroke width: 0.05 RU
        .so()               // stroke_circle
        .hl()               // halt
        .build();

    let mut vm = VM::with_canvas(bytecode, 800, 600);
    vm.run()?;
    println!("✓ Blue circle outline rendered\n");

    // Example 4: Arithmetic still works (2 + 3 = 5)
    println!("Example 4: Computing 2 + 3");
    let bytecode = Program::new()
        .ps_s44(1.0) // push 1
        .dp() // dup → [1, 1]
        .ad() // add → [2]
        .ps_s44(1.0) // push 1 → [2, 1]
        .ps_s44(1.0) // push 1 → [2, 1, 1]
        .ad() // add → [2, 2]
        .ps_s44(1.0) // push 1 → [2, 2, 1]
        .ad() // add → [2, 3]
        .ad() // add → [5]
        .hl() // halt
        .build();

    let mut vm = VM::with_canvas(bytecode, 800, 600);
    vm.run()?;

    if let Some(result) = vm.peek() {
        println!("Result: {:?}\n", result);
    }

    // Example 5: Comparison (2 < 3 = true)
    println!("Example 5: Testing 2 < 3");
    let bytecode = Program::new()
        .ps_s44(1.0) // push 1
        .ps_s44(1.0) // push 1
        .ad() // add → [2]
        .ps_s44(1.0) // push 1
        .ps_s44(1.0) // push 1
        .ad() // add → [2, 2]
        .ps_s44(1.0) // push 1
        .ad() // add → [2, 3]
        .lo() // less-than: 2 < 3 → [1.0]
        .hl() // halt
        .build();

    let mut vm = VM::with_canvas(bytecode, 800, 600);
    vm.run()?;

    if let Some(result) = vm.peek() {
        println!("2 < 3 = {:?} (1.0 = true)\n", result);
    }

    println!("\n=== All examples completed successfully! ===");
    println!("Note: CircleF4E4 is now used for all (x,y) coordinate pairs!");
    Ok(())
}
