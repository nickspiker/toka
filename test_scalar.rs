use spirix::ScalarF4E4;

fn main() {
    let one = ScalarF4E4::ONE;
    let zero = ScalarF4E4::ZERO;

    println!("ONE: fraction={} (0x{:04x}), exponent={} (0x{:04x})",
        one.fraction, one.fraction as u16, one.exponent, one.exponent as u16);
    println!("ZERO: fraction={} (0x{:04x}), exponent={} (0x{:04x})",
        zero.fraction, zero.fraction as u16, zero.exponent, zero.exponent as u16);

    // Test color values
    let color = [ScalarF4E4::ONE, ScalarF4E4::ZERO, ScalarF4E4::ZERO, ScalarF4E4::ONE];
    println!("\nRed color [R, G, B, A]:");
    for (i, c) in color.iter().enumerate() {
        println!("  [{}]: fraction={} (0x{:04x}), exponent={} (0x{:04x})",
            i, c.fraction, c.fraction as u16, c.exponent, c.exponent as u16);
    }
}
