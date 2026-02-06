//! Generate test VSF capsules with executable Toka bytecode

use std::fs;
use toka::builder::Program;
use toka::capsule::CapsuleBuilder;
use vsf::types::VsfType;

fn main() -> std::io::Result<()> {
    fs::create_dir_all("www/capsules")?;

    // Red box - fullscreen background (0,0 to 1,1 in RU coordinates)
    let bytecode = Program::new()
        .fill_rect(
            0.0,  // x: center
            0.0,  // y: center
            1.0,  // width: 1 RU (fullscreen)
            1.0,  // height: 1 RU (fullscreen)
            VsfType::rcr,  // VSF pure red
        )
        .hl()  // halt
        .build();

    let bytes = CapsuleBuilder::new(bytecode)
        .build()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write("www/capsules/redbox.vsf", bytes)?;
    println!("✓ Created redbox.vsf");

    // Green box - centered quarter (0.25,0.25 to 0.75,0.75)
    let bytecode = Program::new()
        .fill_rect(
            0.0,   // x: center
            0.0,   // y: center
            0.5,   // width: half screen
            0.5,   // height: half screen
            VsfType::rcn,  // VSF pure green
        )
        .hl()
        .build();

    let bytes = CapsuleBuilder::new(bytecode)
        .build()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write("www/capsules/greenbox.vsf", bytes)?;
    println!("✓ Created greenbox.vsf");

    // Blue circle - centered with 0.3 RU radius
    let bytecode = Program::new()
        .fill_circle(
            0.0,   // cx: center
            0.0,   // cy: center
            0.3,   // radius: 0.3 RU
            VsfType::rcb,  // VSF pure blue
        )
        .hl()
        .build();

    let bytes = CapsuleBuilder::new(bytecode)
        .build()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write("www/capsules/bluecircle.vsf", bytes)?;
    println!("✓ Created bluecircle.vsf");

    println!("\n✓ All test capsules generated with executable bytecode");
    Ok(())
}
