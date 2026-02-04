//! Generate test VSF capsules for handle resolution testing

use spirix::{CircleF4E4, ScalarF4E4};
use std::fs;
use vsf::types::{TokaBox, TokaCircle};

fn main() -> std::io::Result<()> {
    // Create output directory
    fs::create_dir_all("www/capsules")?;

    // Red box test
    let red_box = TokaBox {
        pos: CircleF4E4::from((ScalarF4E4::ZERO, ScalarF4E4::ZERO)),
        size: CircleF4E4::from((ScalarF4E4::ONE, ScalarF4E4::ONE)),
        colour: CircleF4E4::from((ScalarF4E4::ONE, ScalarF4E4::ZERO)),
    };
    let vt_capsule = red_box.to_vsf_type();
    let bytes = vt_capsule.flatten();
    fs::write("www/capsules/red_box_test.vsf", bytes)?;
    println!("✓ Created red_box_test.vsf");

    // Green box test
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
    let bytes = vt_capsule.flatten();
    fs::write("www/capsules/green_box_test.vsf", bytes)?;
    println!("✓ Created green_box_test.vsf");

    // Blue circle test
    let blue_circle = TokaCircle {
        pos: CircleF4E4::from((
            ScalarF4E4::ONE / ScalarF4E4::from(2),
            ScalarF4E4::ONE / ScalarF4E4::from(2),
        )),
        span: ScalarF4E4::from(3) / ScalarF4E4::from(10),
        colour: CircleF4E4::from((ScalarF4E4::ZERO, ScalarF4E4::ONE)),
    };
    let vt_capsule = blue_circle.to_vsf_type();
    let bytes = vt_capsule.flatten();
    fs::write("www/capsules/blue_circle_test.vsf", bytes)?;
    println!("✓ Created blue_circle_test.vsf");

    println!("\nAll test capsules generated in www/capsules/");
    Ok(())
}
