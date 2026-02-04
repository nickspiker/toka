//! Generate Loom bytecode for web testing
//! Outputs JavaScript arrays with correct vt capsule encoding

use spirix::{CircleF4E4, ScalarF4E4};
use vsf::types::{TokaBox, TokaCircle};

fn to_js_array(bytes: &[u8]) -> String {
    let hex_strs: Vec<String> = bytes.iter().map(|b| format!("0x{:02x}", b)).collect();
    hex_strs.join(", ")
}

fn main() {
    println!("// Generated Loom bytecode for JavaScript\n");

    // Red box
    let red_box = TokaBox {
        pos: CircleF4E4::from((ScalarF4E4::ZERO, ScalarF4E4::ZERO)),
        size: CircleF4E4::from((ScalarF4E4::ONE, ScalarF4E4::ONE)),
        colour: CircleF4E4::from((ScalarF4E4::ONE, ScalarF4E4::ZERO)),
    };
    let vt_capsule = red_box.to_vsf_type();
    let capsule_bytes = vt_capsule.flatten();

    let mut bytecode = vec![];
    bytecode.extend_from_slice(b"{ps}");
    bytecode.extend_from_slice(&capsule_bytes);
    bytecode.extend_from_slice(b"{rl}");
    bytecode.extend_from_slice(b"{hl}");

    println!("// Red box (full viewport)");
    println!("const redBoxBytecode = [{}];", to_js_array(&bytecode));
    println!();

    // Green box at quarter offset
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
    bytecode.extend_from_slice(b"{ps}");
    bytecode.extend_from_slice(&capsule_bytes);
    bytecode.extend_from_slice(b"{rl}");
    bytecode.extend_from_slice(b"{hl}");

    println!("// Green box at 0.25, 0.25 with size 0.5, 0.5");
    println!("const greenBoxBytecode = [{}];", to_js_array(&bytecode));
    println!();

    // Blue circle
    let blue_circle = TokaCircle {
        pos: CircleF4E4::from((
            ScalarF4E4::ONE / ScalarF4E4::from(2),
            ScalarF4E4::ONE / ScalarF4E4::from(2),
        )),
        span: ScalarF4E4::from(3) / ScalarF4E4::from(10),
        colour: CircleF4E4::from((ScalarF4E4::ZERO, ScalarF4E4::ONE)),
    };
    let vt_capsule = blue_circle.to_vsf_type();
    let capsule_bytes = vt_capsule.flatten();

    let mut bytecode = vec![];
    bytecode.extend_from_slice(b"{ps}");
    bytecode.extend_from_slice(&capsule_bytes);
    bytecode.extend_from_slice(b"{rl}");
    bytecode.extend_from_slice(b"{hl}");

    println!("// Blue circle at center with 0.3 radius");
    println!("const blueCircleBytecode = [{}];", to_js_array(&bytecode));
}
