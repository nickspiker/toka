use spirix::ScalarF4E4;

fn main() {
    let one = ScalarF4E4::ONE;
    let zero = ScalarF4E4::ZERO;

    println!("ONE: fraction={} (0x{:04x}), exponent={} (0x{:04x})",
        one.fraction, one.fraction as u16, one.exponent, one.exponent as u16);
    println!("ZERO: fraction={} (0x{:04x}), exponent={} (0x{:04x})",
        zero.fraction, zero.fraction as u16, zero.exponent, zero.exponent as u16);

    // Test colour values
    let colour = [ScalarF4E4::ONE, ScalarF4E4::ZERO, ScalarF4E4::ZERO, ScalarF4E4::ONE];
    println!("\nRed colour [R, G, B, A]:");
    for (i, c) in colour.iter().enumerate() {
        println!("  [{}]: fraction={} (0x{:04x}), exponent={} (0x{:04x})",
            i, c.fraction, c.fraction as u16, c.exponent, c.exponent as u16);
    }
}

#[cfg(test)]
mod colour_tests {
    use vsf::types::VsfType;
    use crate::renderer::extract_colour_u32;

    #[test]
    fn test_extract_colour_black() {
        let packed = extract_colour_u32(&VsfType::rck).unwrap();
        println!("rck → {:08X}", packed);
        assert_eq!(packed & 0xFF, 0xFF, "alpha should be 255");
        assert_eq!(packed >> 8, 0, "RGB should be 0");
    }

    #[test]
    fn test_extract_colour_blue() {
        let packed = extract_colour_u32(&VsfType::rcb).unwrap();
        println!("rcb → {:08X}", packed);
        assert_eq!(packed & 0xFF, 0xFF, "alpha should be 255");
    }

    #[test]
    fn test_extract_colour_red_half_alpha() {
        let packed = extract_colour_u32(&VsfType::ra([255, 0, 0, 127])).unwrap();
        println!("ra[255,0,0,127] → {:08X}", packed);
        assert_eq!(packed & 0xFF, 127, "alpha should be 127");
    }
}
