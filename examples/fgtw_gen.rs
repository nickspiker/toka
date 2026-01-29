//! Generate FGTW capsule for browser rendering
//!
//! This program creates a proper VSF capsule file containing the
//! white square bytecode, ready for loading and verification.
//!
//! Usage:
//!   cargo run --example fgtw_gen
//!
//! Output:
//!   www/fgtw.vsf - The capsule file

use std::fs;
use toka::builder::Program;
use toka::capsule::CapsuleBuilder;
use vsf::types::VsfType;

fn main() -> Result<(), String> {
    // Build bytecode using the builder DSL
    let bytecode = Program::new()
        // Clear canvas to black
        .clear(VsfType::rk)
        // Draw a white square, 0.5 span wide/tall, centered
        .fill_rect(0.0, 0.0, 0.5, 0.5, VsfType::rw)
        .hl()
        .build();

    println!("Bytecode: {} bytes", bytecode.len());

    // Wrap in a capsule
    let capsule = CapsuleBuilder::new(bytecode).build()?;

    println!("Capsule: {} bytes", capsule.len());

    // Verify it loads correctly
    let loaded = toka::capsule::Capsule::load(&capsule)?;
    loaded.verify()?;
    println!("Provenance: {}", loaded.provenance_hex());
    println!("Bytecode section: {} bytes", loaded.bytecode().len());

    // Write to www directory
    let output_path = "www/fgtw.vsf";
    fs::write(output_path, &capsule)
        .map_err(|e| format!("Failed to write {}: {}", output_path, e))?;

    println!("\nWrote: {}", output_path);
    println!("\nCapsule ready! Update fgtw.html to fetch this file.");

    Ok(())
}
