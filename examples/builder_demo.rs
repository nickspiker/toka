//! Demo of the Toka bytecode builder
//!
//! Shows how to use the Rust DSL to construct Toka programs with
//! type safety and compile-time checking.

use toka::builder::Program;
use toka::vm::VM;

fn main() -> Result<(), String> {
    println!("=== Toka Builder Demo ===\n");

    // Example 1: Simple arithmetic (2 + 3 = 5)
    println!("Example 1: Computing 2 + 3");
    let bytecode = Program::new()
        .po()   // push 1
        .dp()   // dup → [1, 1]
        .ad()   // add → [2]
        .po()   // push 1 → [2, 1]
        .po()   // push 1 → [2, 1, 1]
        .ad()   // add → [2, 2]
        .po()   // push 1 → [2, 2, 1]
        .ad()   // add → [2, 3]
        .ad()   // add → [5]
        .hl()   // halt
        .build();

    let mut vm = VM::new(bytecode);
    vm.run()?;

    if let Some(result) = vm.peek() {
        println!("Result: {}\n", result);
    }

    // Example 2: Drawing would use ps_s44, but push opcode not yet implemented
    // Skipping for now - will work once VM implements VSF value decoding
    println!("Example 2: Drawing (skipped - requires ps_s44 implementation)\n");


    // Example 3: Comparison (2 < 3 = true)
    println!("Example 3: Testing 2 < 3");
    let bytecode = Program::new()
        .po()   // push 1
        .po()   // push 1
        .ad()   // add → [2]
        .po()   // push 1
        .po()   // push 1
        .ad()   // add → [2, 2]
        .po()   // push 1
        .ad()   // add → [2, 3]
        .lo()   // less-than: 2 < 3 → [1.0]
        .hl()   // halt
        .build();

    let mut vm = VM::new(bytecode);
    vm.run()?;

    if let Some(result) = vm.peek() {
        println!("2 < 3 = {} (1.0 = true)\n", result);
    }

    // Example 4: ps_s44 would be useful but requires push implementation
    println!("Example 4: ps_s44 usage (skipped - requires VSF decoder)\n");

    println!("\n=== All examples completed successfully! ===");
    Ok(())
}
