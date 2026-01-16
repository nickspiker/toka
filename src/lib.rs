//! # Toka
//!
//! Capability-bounded stack VM for secure distributed computing.
//!
//! **Status:** v0.0 - Early Development (Design Phase)
//!
//! ## Overview
//!
//! Toka is a stack-based virtual machine designed for executing signed,
//! capability-bounded bytecode in distributed systems. It provides:
//!
//! - **Spirix-Native Arithmetic** - Two's complement floating point (no IEEE-754)
//! - **Capability-Based Security** - Fine-grained permission system
//! - **Cryptographic Verification** - BLAKE3 hashes, ed25519 signatures
//! - **Deterministic Execution** - Same bytecode, same results, everywhere
//! - **VSF Bytecode** - Compact, self-describing binary format
//! - **Handle-Only Memory** - No pointers, no buffer overflows
//! - **Viewport Graphics** - Resolution-independent rendering (0.0-1.0 coords)
//!
//! ## Current Status
//!
//! This is an early release to claim the crate name. The architecture is fully
//! documented but implementation is in progress.
//!
//! **What works:**
//! - Architecture documentation (see README.md)
//! - Instruction set specification (see OPCODES.md)
//! - Design documents (see SCAFFOLD.md)
//!
//! **What doesn't work yet:**
//! - Everything else (VM, bytecode parser, canvas backend)
//!
//! See the [GitHub repository](https://github.com/nickspiker/toka) for:
//! - Full documentation
//! - Roadmap and milestones
//! - Architecture design
//! - Contributing guidelines
//!
//! ## Example (Future API)
//!
//! ```rust,ignore
//! use toka::VM;
//!
//! // Load a signed capsule
//! let bytecode = include_bytes!("app.vsf");
//! let mut vm = VM::new(bytecode)?;
//!
//! // Execute with capability bounds
//! vm.grant_capability("canvas_draw")?;
//! vm.run()?;
//! ```
//!
//! ## Architecture
//!
//! Toka uses a stack-based execution model with:
//! - **Value Stack** - All operands pushed/popped from stack
//! - **Local Variables** - Function-local storage slots
//! - **Handles** - Capability-checked references to resources
//! - **No Linear Memory** - Eliminates buffer overflows
//!
//! All arithmetic uses Spirix (two's complement floating point) instead of
//! IEEE-754, providing deterministic results across all platforms.
//!
//! Drawing operations use viewport-relative coordinates (0.0-1.0) for
//! resolution-independent rendering.
//!
//! ## License
//!
//! Dual-licensed under MIT or Apache-2.0, at your option.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

/// Toka version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Placeholder module for future VM implementation
pub mod vm {
    //! VM execution engine (not yet implemented)
}

/// Placeholder module for future bytecode parser
pub mod bytecode {
    //! VSF bytecode parser (not yet implemented)
}

/// Placeholder module for future value types
pub mod value {
    //! Stack value types (not yet implemented)
}

/// Placeholder module for future capability system
pub mod capability {
    //! Capability-based security (not yet implemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!VERSION.is_empty());
        assert_eq!(VERSION, "0.0.0");
    }
}
