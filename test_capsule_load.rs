//! Test capsule loading and bytecode extraction

use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let capsule_data = fs::read("www/capsules/redbox.vsf")?;
    println!("Loaded {} bytes from redbox.vsf", capsule_data.len());

    // Use the Capsule loader
    let capsule = toka::capsule::Capsule::load(&capsule_data)?;

    println!("\n=== EXTRACTED BYTECODE ===");
    let bytecode = capsule.bytecode();
    println!("Length: {} bytes\n", bytecode.len());

    // Hex dump
    println!("Hex dump:");
    for (i, chunk) in bytecode.chunks(16).enumerate() {
        print!("{:08x}  ", i * 16);
        for byte in chunk {
            print!("{:02x} ", byte);
        }
        print!(" |");
        for byte in chunk {
            let c = *byte as char;
            if c.is_ascii_graphic() || c == ' ' {
                print!("{}", c);
            } else {
                print!(".");
            }
        }
        println!("|");
    }

    println!("\n=== EXPECTED START ===");
    println!("Should start with: {{ps}} = 0x7b 0x70 0x73 0x7d");
    println!("Actually starts with: 0x{:02x} 0x{:02x} 0x{:02x} 0x{:02x}",
        bytecode[0], bytecode[1], bytecode[2], bytecode[3]);

    Ok(())
}
