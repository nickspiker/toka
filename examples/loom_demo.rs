//! Demo of Loom hierarchical layout system with vt capsules
//!
//! This example demonstrates rendering Toka Tree (Loom) layout nodes
//! serialized as VSF vt wrapped capsules.

use spirix::{CircleF4E4, ScalarF4E4};
use toka::vm::VM;
use vsf::types::{TokaBox, TokaCircle};

fn main() -> Result<(), String> {
    println!("=== Loom Layout Demo ===\n");

    // Example 1: Simple red box using vt capsule
    println!("Example 1: Red box (vt capsule)");
    let red_box = TokaBox {
        pos: CircleF4E4::from((ScalarF4E4::from(0), ScalarF4E4::from(0))),
        size: CircleF4E4::from((ScalarF4E4::ONE, ScalarF4E4::ONE)),
        colour: CircleF4E4::from((ScalarF4E4::ONE, ScalarF4E4::ZERO)),
    };

    // Convert to vt capsule and flatten to bytes
    let vt_capsule = red_box.to_vsf_type();
    let capsule_bytes = vt_capsule.flatten();

    // Build bytecode: push opcode + vt capsule, then render_loom
    let mut bytecode = vec![];
    bytecode.extend_from_slice(b"{ps}"); // push opcode
    bytecode.extend_from_slice(&capsule_bytes); // vt capsule data
    bytecode.extend_from_slice(b"{rl}"); // render_loom opcode
    bytecode.extend_from_slice(b"{hl}"); // halt

    let mut vm = VM::with_canvas(bytecode, 800, 600);
    vm.run()?;
    println!("✓ Red box rendered via vt capsule\n");

    // Example 2: Blue circle using vt capsule
    println!("Example 2: Blue circle (vt capsule)");
    let blue_circle = TokaCircle {
        pos: CircleF4E4::from((
            ScalarF4E4::ONE / ScalarF4E4::from(2),
            ScalarF4E4::ONE / ScalarF4E4::from(2),
        )),
        span: ScalarF4E4::from(3) / ScalarF4E4::from(10), // 0.3 radius
        colour: CircleF4E4::from((ScalarF4E4::ZERO, ScalarF4E4::ONE)),
    };

    let vt_capsule = blue_circle.to_vsf_type();
    let capsule_bytes = vt_capsule.flatten();

    let mut bytecode = vec![];
    bytecode.extend_from_slice(b"{ps}"); // push opcode
    bytecode.extend_from_slice(&capsule_bytes);
    bytecode.extend_from_slice(b"{rl}");
    bytecode.extend_from_slice(b"{hl}");

    let mut vm = VM::with_canvas(bytecode, 800, 600);
    vm.run()?;
    println!("✓ Blue circle rendered via vt capsule\n");

    // Example 3: Green box at different position
    println!("Example 3: Green box at quarter offset");
    let green_box = TokaBox {
        pos: CircleF4E4::from((
            ScalarF4E4::from(1) / ScalarF4E4::from(4),
            ScalarF4E4::from(1) / ScalarF4E4::from(4),
        )),
        size: CircleF4E4::from((
            ScalarF4E4::ONE / ScalarF4E4::from(2),
            ScalarF4E4::ONE / ScalarF4E4::from(2),
        )),
        colour: CircleF4E4::from((ScalarF4E4::ZERO, ScalarF4E4::ONE)),
    };

    let vt_capsule = green_box.to_vsf_type();
    let capsule_bytes = vt_capsule.flatten();

    let mut bytecode = vec![];
    bytecode.extend_from_slice(b"{ps}"); // push opcode
    bytecode.extend_from_slice(&capsule_bytes);
    bytecode.extend_from_slice(b"{rl}");
    bytecode.extend_from_slice(b"{hl}");

    let mut vm = VM::with_canvas(bytecode, 800, 600);
    vm.run()?;
    println!("✓ Green box rendered\n");

    println!("All Loom demos completed successfully!");
    Ok(())
}
