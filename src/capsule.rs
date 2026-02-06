//! Capsule: signed executable bundle for Toka VM
//!
//! A Capsule is a VSF file containing:
//! - Header with provenance hash (content identity)
//! - Optional Ed25519 signature for authenticity
//! - Bytecode section containing Toka opcodes
//!
//! # Creating a Capsule
//!
//! ```ignore
//! use toka::capsule::CapsuleBuilder;
//! use toka::builder::Program;
//!
//! let bytecode = Program::new()
//!     .fill_rect(0.0, 0.0, 0.5, 0.5, VsfType::rw)
//!     .hl()
//!     .build();
//!
//! let capsule = CapsuleBuilder::new(bytecode)
//!     .build()?;
//!
//! std::fs::write("app.vsf", &capsule)?;
//! ```
//!
//! # Loading a Capsule
//!
//! ```ignore
//! use toka::capsule::Capsule;
//!
//! let data = std::fs::read("app.vsf")?;
//! let capsule = Capsule::load(&data)?;
//!
//! // Verify integrity
//! capsule.verify()?;
//!
//! // Get bytecode for VM execution
//! let bytecode = capsule.bytecode();
//! ```

use vsf::VsfBuilder;

/// Builder for creating Capsule files
pub struct CapsuleBuilder {
    bytecode: Vec<u8>,
    signer_pubkey: Option<[u8; 32]>,
    signature: Option<[u8; 64]>,
}

impl CapsuleBuilder {
    /// Create a new capsule builder with bytecode
    pub fn new(bytecode: Vec<u8>) -> Self {
        Self {
            bytecode,
            signer_pubkey: None,
            signature: None,
        }
    }

    /// Add Ed25519 signature for authenticity
    pub fn sign(mut self, pubkey: [u8; 32], signature: [u8; 64]) -> Self {
        self.signer_pubkey = Some(pubkey);
        self.signature = Some(signature);
        self
    }

    /// Build the capsule as VSF bytes
    pub fn build(self) -> Result<Vec<u8>, String> {
        use vsf::decoding::parse;
        use vsf::file_format::VsfSection;

        // Parse bytecode into VsfTypes (opcodes and scalars)
        let mut values = Vec::new();
        let mut ptr = 0;
        while ptr < self.bytecode.len() {
            let val = parse(&self.bytecode, &mut ptr)
                .map_err(|e| format!("Failed to parse bytecode at offset {}: {}", ptr, e))?;
            values.push(val);
        }

        // Create toka section with multi-value field
        let mut section = VsfSection::new("toka");
        section.add_field_multi("main", values);
        let mut builder = VsfBuilder::new().add_section_direct(section);

        // Add signature if provided
        if let (Some(pubkey), Some(sig)) = (self.signer_pubkey, self.signature) {
            builder = builder.signature_ed25519(pubkey, sig);
        }

        builder.build()
    }
}

/// A loaded and parsed Capsule
pub struct Capsule {
    /// Raw VSF file bytes (for verification)
    raw: Vec<u8>,
    /// Extracted executable bytecode (raw VSF-encoded opcodes and values)
    bytecode: Vec<u8>,
    /// Provenance hash (hp: content-addressed identity)
    provenance: Vec<u8>,
    /// Whether this capsule has an Ed25519 signature
    is_signed: bool,
}

impl Capsule {
    /// Load a capsule from VSF bytes
    pub fn load(data: &[u8]) -> Result<Self, String> {
        use vsf::file_format::{VsfHeader, VsfSection};
        use vsf::types::VsfType;

        // Parse header
        let (header, _header_end) =
            VsfHeader::decode(data).map_err(|e| format!("Invalid capsule header: {}", e))?;

        // Get provenance hash (hp: content identity)
        let provenance = match &header.provenance_hash {
            VsfType::hp(bytes) => bytes.clone(),
            _ => return Err("Capsule missing hp (provenance hash)".to_string()),
        };

        // Find toka section in header TOC
        let section_toc = header
            .fields
            .iter()
            .find(|f| f.name == "toka")
            .ok_or("Capsule missing toka section")?;

        // Parse the section body manually (since we don't include section name for <1MB files)
        let mut ptr = section_toc.offset_bytes;

        // Skip optional section markers (> and [)
        if ptr < data.len() && data[ptr] == b')' {
            ptr += 1;  // Skip TOC closing paren
        }
        if ptr < data.len() && data[ptr] == b'>' {
            ptr += 1;  // Skip > marker
        }
        if ptr >= data.len() || data[ptr] != b'[' {
            return Err(format!(
                "Expected '[' at offset {} (found {:02x})",
                ptr,
                data.get(ptr).copied().unwrap_or(0)
            ));
        }
        ptr += 1;  // Skip [

        // Parse the field (which contains our bytecode values)
        use vsf::file_format::VsfField;
        let field = VsfField::parse(data, &mut ptr)
            .map_err(|e| format!("Failed to parse main field: {}", e))?;

        // Verify it's the "main" field
        if field.name != "main" {
            return Err(format!("Expected 'main' field, found '{}'", field.name));
        }

        // Re-encode just the field values as raw bytecode (with commas between values)
        let mut bytecode = Vec::new();
        for (i, value) in field.values.iter().enumerate() {
            if i > 0 {
                bytecode.push(b',');  // VSF parser expects commas between values
            }
            bytecode.extend_from_slice(&value.flatten());
        }

        // Check if signed (has Ed25519 signature)
        let is_signed = header.signature.is_some();

        Ok(Self {
            raw: data.to_vec(),
            bytecode,
            provenance,
            is_signed,
        })
    }

    /// Verify capsule authenticity and integrity
    ///
    /// - Signed capsules: Verifies Ed25519 signature against hp (proves authenticity + integrity)
    /// - Unsigned capsules: Verifies hb integrity hash (tamper detection only)
    ///
    /// Provenance (hp) = content identity (always present)
    /// Signature (ge + ke) = cryptographic proof of authenticity (optional, proves integrity too)
    /// Integrity (hb) = tamper detection hash (only for unsigned files)
    pub fn verify(&self) -> Result<(), String> {
        if self.is_signed {
            // Verify Ed25519 signature (proves both authenticity and integrity)
            vsf::verification::verify_file_signature(&self.raw)
                .and_then(|valid| {
                    if valid {
                        Ok(())
                    } else {
                        Err("Signature verification failed".to_string())
                    }
                })
                .map_err(|e| format!("Capsule signature verification failed: {}", e))
        } else {
            // Fall back to hb integrity hash for unsigned capsules
            vsf::verification::is_original(&self.raw)
                .map_err(|e| format!("Capsule integrity check (hb) failed: {}", e))
        }
    }

    /// Get bytecode for VM execution
    pub fn bytecode(&self) -> &[u8] {
        &self.bytecode
    }

    /// Get provenance hash (hp: content-addressed identity)
    pub fn provenance(&self) -> &[u8] {
        &self.provenance
    }

    /// Get provenance hash as hex string (for JavaScript/display)
    pub fn provenance_hex(&self) -> String {
        self.provenance
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::Program;
    use vsf::types::VsfType;

    #[test]
    fn test_capsule_roundtrip() {
        // Build bytecode
        let bytecode = Program::new()
            .clear(VsfType::rck)
            .fill_rect(0.0, 0.0, 0.5, 0.5, VsfType::rcw)
            .hl()
            .build();

        // Create capsule
        let capsule_bytes = CapsuleBuilder::new(bytecode)
            .build()
            .expect("Failed to build capsule");

        // Load capsule
        let capsule = Capsule::load(&capsule_bytes).expect("Failed to load capsule");

        // Verify capsule integrity
        capsule.verify().expect("Verification failed");

        // Verify capsule has bytecode
        assert!(!capsule.bytecode().is_empty(), "Capsule bytecode is empty");
    }
}
