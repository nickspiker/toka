//! # Toka
//!
//! Capability-bounded stack VM for secure distributed computing.
//!
//! **Status:** v0 - Early Development (Design Phase)
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
//! - **Viewport Graphics** - Resolution-independent rendering (0-1 coords)
//!
//! ## Current Status
//!
//! This is an early release to claim the crate name. The architecture is fully documented but implementation is in progress.
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
//! All arithmetic uses Spirix (two's complement floating point) instead of IEEE-754, providing fast and deterministic results across all platforms.
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

/// Opcode definitions and parsing
pub mod opcode;

/// VM execution engine
#[cfg(feature = "runtime")]
pub mod vm;

/// Bytecode builder with chainable opcode methods
pub mod builder;

/// Direct VSF ro* to Canvas renderer
#[cfg(feature = "runtime")]
pub mod renderer;

/// Capsule: signed executable bundle
pub mod capsule;

/// Drawing primitives (line, path, etc.)
#[cfg(feature = "runtime")]
pub mod drawing;

/// Placeholder module for future capability system
pub mod capability {
    //! Capability-based security (not yet implemented)
}

/// WASM bindings for browser integration
#[cfg(all(target_arch = "wasm32", feature = "runtime"))]
pub mod wasm {
    //! WebAssembly bindings for running Toka in the browser
    //!
    //! Provides a thin wrapper around the VM that exposes a JavaScript-friendly API:
    //! - `new(bytecode, width, height)` - Create VM with canvas
    //! - `run(steps)` - Execute N instructions
    //! - `get_canvas_rgba()` - Get RGBA bytes for ImageData
    //! - `width()`, `height()` - Canvas dimensions

    use crate::vm::VM;
    use spirix::ScalarF4E4;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        /// Platform-independent logging to HTML console
        ///
        /// Routes log messages to the app.js log() function which displays them in the
        /// HTML console element. This allows logging to work on any platform, not just browsers.
        ///
        /// # Parameters
        /// - `message` - The log message
        /// - `log_type` - Message type: "info", "error", "warn"
        #[wasm_bindgen(js_name = log)]
        pub fn js_log(message: &str, log_type: &str);
    }

    /// WASM-friendly VM wrapper for browser execution
    #[wasm_bindgen]
    pub struct TokaVM {
        vm: VM,
    }

    #[wasm_bindgen]
    impl TokaVM {
        /// Create a new Toka VM with canvas
        ///
        /// # Parameters
        /// - `bytecode` - VSF bytecode to execute
        /// - `width` - Canvas width in pixels
        /// - `height` - Canvas height in pixels
        #[wasm_bindgen(constructor)]
        pub fn new(bytecode: Vec<u8>, width: usize, height: usize) -> Self {
            // Set panic hook for better error messages in browser console
            #[cfg(feature = "console_error_panic_hook")]
            console_error_panic_hook::set_once();

            Self {
                vm: VM::with_canvas(bytecode, width, height),
            }
        }

        /// Execute up to N instructions
        ///
        /// Returns `true` if VM is still running, `false` if halted.
        ///
        /// # Errors
        /// Returns error string if VM encounters invalid opcode or runtime error.
        pub fn run(&mut self, steps: usize) -> Result<bool, String> {
            for _ in 0..steps {
                if self.vm.is_halted() {
                    return Ok(false);
                }
                self.vm.step().map_err(|e| e.to_string())?;
            }
            Ok(!self.vm.is_halted())
        }

        /// Reset VM to re-execute bytecode from beginning
        ///
        /// Clears stack, resets instruction pointer, clears halt flag.
        /// Preserves context variables (scroll, mouse, time) for reactive scenes.
        pub fn reset(&mut self) {
            self.vm.reset();
        }

        /// Get canvas pixels as RGBA byte array for ImageData
        ///
        /// Returns Vec<u8> with format [R, G, B, A, R, G, B, A, ...]
        /// suitable for `new ImageData(new Uint8ClampedArray(bytes), width, height)`
        pub fn get_canvas_rgba(&mut self) -> Vec<u8> {
            // &mut: the output pass under-composites the photon noise backdrop into the buffer
            // (idempotent — opaque pixels early-out) before flipping to visible RGBA.
            self.vm.canvas_mut().to_rgba_bytes()
        }

        /// Drain the resource keys the last render couldn't resolve, as a JS string array. For each,
        /// the host builds a VSF request (`build_resource_request`), POSTs it, and returns the bytes
        /// via `provide_resource`, then re-renders. Empty array = nothing to fetch (converged).
        pub fn take_pending_requests(&mut self) -> Vec<String> {
            self.vm.take_pending_requests()
        }

        /// Build the VSF-framed request the host POSTs to the worker for one resource `key`. VSF
        /// construction stays in wasm so the host only shuttles opaque bytes over the wire.
        pub fn build_resource_request(&self, key: &str) -> Vec<u8> {
            vsf::vsf_builder::VsfBuilder::new()
                .provenance_only()
                .add_section(
                    "resource_get",
                    vec![("key".to_string(), vsf::types::VsfType::a(key.to_string()))],
                )
                .build()
                .unwrap_or_default()
        }

        /// Hand back a fetched resource: `key` is the request key, `vsf_bytes` the worker's VSF
        /// response (a VSF image file). Stored raw; decoded on the next draw_image that needs it.
        pub fn provide_resource(&mut self, key: String, vsf_bytes: Vec<u8>) {
            self.vm.provide_resource(key, vsf_bytes);
        }

        /// Get canvas width in pixels
        pub fn width(&self) -> usize {
            self.vm.canvas().width()
        }

        /// Get canvas height in pixels
        pub fn height(&self) -> usize {
            self.vm.canvas().height()
        }

        /// Get and clear execution trace (list of opcodes executed)
        pub fn get_trace(&mut self) -> Vec<String> {
            self.vm.take_trace()
        }

        /// Check if VM has halted
        pub fn is_halted(&self) -> bool {
            self.vm.is_halted()
        }

        /// Peek at the top value on the stack without popping
        ///
        /// Returns None if stack is empty, NaN for non-scalar types.
        pub fn peek_stack(&self) -> Option<f64> {
            use vsf::types::VsfType;
            self.vm.peek().map(|v| match v {
                VsfType::s44(s) => s.to_f64(),
                _ => f64::NAN, // Non-scalars return NaN (not representable as f64)
            })
        }

        // ==================== ZOOM CONTROLS ====================

        /// Adjust zoom by steps (positive = zoom in, negative = zoom out)
        ///
        /// Uses logarithmic scaling: each step multiplies by 33/32 (in) or 32/33 (out)
        pub fn adjust_zoom(&mut self, steps: f64) {
            self.vm.canvas_mut().adjust_zoom(ScalarF4E4::from_f64(steps));
        }

        /// Set RU multiplier directly
        ///
        /// Clamped to [0.125, 8] for sanity.
        pub fn set_ru(&mut self, ru: f64) {
            self.vm.canvas_mut().set_ru(ScalarF4E4::from_f64(ru));
        }

        /// Get current RU zoom multiplier
        pub fn get_ru(&self) -> f64 {
            self.vm.canvas().ru().to_f64()
        }

        /// Get canvas span (harmonic mean of width/height)
        ///
        /// This is the base unit for RU calculations.
        pub fn get_span(&self) -> f64 {
            self.vm.canvas().span().to_f64()
        }

        /// Set scroll offset (in RU)
        ///
        /// Programs can read scroll via {sx} and {sy} opcodes.
        /// Call `rerun()` after changing scroll to re-execute bytecode with new values.
        pub fn set_scroll(&mut self, scroll_x: f64, scroll_y: f64) {
            self.vm.set_scroll(ScalarF4E4::from_f64(scroll_x), ScalarF4E4::from_f64(scroll_y));
        }

        /// Get scroll offset X (in RU)
        pub fn get_scroll_x(&self) -> f64 {
            self.vm.scroll_x().to_f64()
        }

        /// Get scroll offset Y (in RU)
        pub fn get_scroll_y(&self) -> f64 {
            self.vm.scroll_y().to_f64()
        }

        /// Scroll by pixel delta: shift pixel buffer, update scroll_y offset.
        /// Sets clip to the exposed strip so subsequent rerun only draws there.
        /// Returns the new scroll_y in RU. Positive delta = scroll down.
        pub fn scroll_by(&mut self, delta_pixels: i32) -> f64 {
            let span_ru = self.vm.canvas().span() * self.vm.canvas().ru();
            let delta_ru = ScalarF4E4::from(delta_pixels) / span_ru;
            let new_y = self.vm.scroll_y() + delta_ru;
            self.vm.set_scroll_y(new_y);
            let h = self.vm.canvas().height();
            // Shift pixels — use black as background (rerun will repaint strip)
            self.vm.canvas_mut().scroll_pixels(delta_pixels as isize, 0x000000FF);
            // Set clip to the newly exposed strip
            let d = (delta_pixels.unsigned_abs() as usize).min(h);
            if delta_pixels > 0 {
                // Scrolled down: new strip at bottom
                self.vm.canvas_mut().set_clip_y(h - d, h);
            } else {
                // Scrolled up: new strip at top
                self.vm.canvas_mut().set_clip_y(0, d);
            }
            new_y.to_f64()
        }

        /// Clear the Y clip — restores full-canvas drawing.
        pub fn clear_clip(&mut self) {
            self.vm.canvas_mut().clear_clip_y();
        }

        /// Re-run the bytecode (re-execute from beginning)
        ///
        /// Use after adjusting zoom or scroll to re-render with new values.
        /// Preserves widget state (text inputs, focus) for interactive continuity.
        pub fn rerun(&mut self, bytecode: Vec<u8>) -> Result<bool, String> {
            self.vm.set_bytecode(bytecode);
            self.vm.reset();

            // Run to completion
            self.vm.run().map_err(|e| e.to_string())?;
            Ok(!self.vm.is_halted())
        }

        /// Differential rerender — update only widget visuals without re-executing VM.
        ///
        /// Much faster than rerun() for keystrokes and cursor updates.
        /// Use rerun() for button clicks that need post-table action processing.
        pub fn rerun_widgets(&mut self) -> Result<bool, String> {
            self.vm.rerun_widgets()
        }

        /// Check if differential rerender is available (widgets were rendered)
        pub fn has_widget_snapshots(&self) -> bool {
            self.vm.has_widget_snapshots()
        }

        /// Flip blinkey cursor animation — call from JS timer (~200ms interval)
        pub fn flip_blinkey(&mut self) {
            self.vm.flip_blinkey()
        }

        /// Resize canvas — preserves widget state (text inputs, focus)
        pub fn resize(&mut self, width: u32, height: u32) {
            self.vm.resize(width as usize, height as usize);
        }

        /// Switch rendering pipeline ("fast" or "quality")
        ///
        /// Caller is responsible for re-running bytecode after switching.
        #[wasm_bindgen]
        pub fn set_pipeline(&mut self, name: &str) -> Result<(), String> {
            use crate::drawing::Canvas;

            let w = self.vm.canvas().width();
            let h = self.vm.canvas().height();
            let ru = self.vm.canvas().ru();

            let mut new_canvas = match name {
                "fast" => Canvas::new_fast(w, h),
                "quality" => Canvas::new_quality(w, h),
                _ => return Err(format!("Unknown pipeline: {}", name)),
            };
            new_canvas.set_ru(ru);
            self.vm.set_canvas(new_canvas);
            Ok(())
        }

        /// Return the active rendering pipeline name ("fast" or "quality")
        #[wasm_bindgen]
        pub fn pipeline_name(&self) -> String {
            self.vm.canvas().pipeline_name().to_string()
        }

        // ── Interactive widget API ──────────────────────────

        /// Push a mouse down event (RU coordinates)
        pub fn push_mouse_down(&mut self, x: f64, y: f64) {
            self.vm.push_event(crate::vm::InputEvent::MouseDown {
                x: ScalarF4E4::from_f64(x),
                y: ScalarF4E4::from_f64(y),
            });
        }

        /// Push a mouse up event (RU coordinates)
        pub fn push_mouse_up(&mut self, x: f64, y: f64) {
            self.vm.push_event(crate::vm::InputEvent::MouseUp {
                x: ScalarF4E4::from_f64(x),
                y: ScalarF4E4::from_f64(y),
            });
        }

        /// Push a text input event (printable characters)
        pub fn push_key_press(&mut self, text: &str) {
            self.vm.push_event(crate::vm::InputEvent::KeyPress {
                text: text.to_string(),
            });
        }

        /// Push a key down event (non-character keys like Backspace, ArrowLeft)
        pub fn push_key_down(&mut self, key: &str) {
            self.vm.push_event(crate::vm::InputEvent::KeyDown {
                key: key.to_string(),
            });
        }

        /// Drain events after frame execution
        pub fn drain_events(&mut self) {
            self.vm.drain_events();
        }

        /// Get the cursor type for the current mouse position
        /// Returns: "default", "pointer", or "text"
        pub fn cursor_at(&self, x: f64, y: f64) -> String {
            let sx = ScalarF4E4::from_f64(x);
            let sy = ScalarF4E4::from_f64(y);
            match self.vm.cursor_kind_at(sx, sy) {
                crate::vm::CursorKind::Default => "default".to_string(),
                crate::vm::CursorKind::Pointer => "pointer".to_string(),
                crate::vm::CursorKind::Text => "text".to_string(),
            }
        }

        /// Get the focused widget ID (or -1 if none)
        pub fn focused_widget(&self) -> i32 {
            self.vm.focused_widget().map(|id| id as i32).unwrap_or(-1)
        }

        /// Get the hovered widget ID (or -1 if none). The host polls this after `set_mouse` to
        /// decide whether a hover repaint (differential) is needed.
        pub fn hovered_widget(&self) -> i32 {
            self.vm.hovered_widget().map(|id| id as i32).unwrap_or(-1)
        }

        /// Set mouse position (in RU) — for cursor tracking
        pub fn set_mouse(&mut self, x: f64, y: f64) {
            self.vm.set_mouse(ScalarF4E4::from_f64(x), ScalarF4E4::from_f64(y));
        }

        /// Drain next action as raw bytes (Uint8Array in JS).
        /// Returns empty vec when no more actions. Call in a loop until empty.
        pub fn drain_action(&mut self) -> Vec<u8> {
            self.vm.drain_action().unwrap_or_default()
        }
    }

    /// Load and verify a Toka capsule, returning bytecode if valid
    ///
    /// # Parameters
    /// - `capsule_data` - Raw VSF capsule bytes
    ///
    /// # Returns
    /// - Ok(Vec<u8>) - Bytecode extracted from capsule
    /// - Err(String) - Error message if loading or verification failed
    #[wasm_bindgen]
    pub fn load_capsule(capsule_data: Vec<u8>) -> Result<Vec<u8>, String> {
        use crate::capsule::Capsule;

        // Load and parse capsule
        let capsule = Capsule::load(&capsule_data)?;

        // Verify authenticity and integrity (signature if signed, hb if unsigned)
        capsule.verify()?;

        let bytecode = capsule.bytecode().to_vec();

        // Return bytecode
        Ok(bytecode)
    }

    /// Get provenance hash from a capsule without loading full VM
    ///
    /// Returns hex-encoded provenance hash (String for JavaScript interop).
    #[wasm_bindgen]
    pub fn get_capsule_provenance(capsule_data: Vec<u8>) -> Result<String, String> {
        use crate::capsule::Capsule;

        let capsule = Capsule::load(&capsule_data)?;
        Ok(capsule.provenance_hex())
    }

    /// Inspect a VSF capsule and return formatted output (vsfinfo style, no ANSI colours)
    ///
    /// Returns the same inspector view as vsfinfo, but without ANSI colour codes
    /// so it displays properly in browser console.
    #[wasm_bindgen]
    pub fn inspect_capsule(capsule_data: Vec<u8>) -> Result<String, String> {
        use vsf::inspect::inspect_vsf_plain;

        // Format the VSF file in literal format (vsfinfo style) without ANSI codes
        inspect_vsf_plain(&capsule_data)
    }

    /// Generate example bytecode programs
    ///
    /// Returns properly formatted VSF bytecode for testing.
    #[wasm_bindgen]
    pub fn generate_fill_red_bytecode() -> Vec<u8> {
        use crate::builder::Program;
        use vsf::types::VsfType;
        // Simple test bytecode - immediate mode drawing removed
        Program::new()
            .ps(&VsfType::rcr.flatten()) // Push red colour constant
            .cr() // Clear canvas to red
            .hl() // halt
            .build()
    }

    /// Resolve a plaintext handle to its base64url capsule filename.
    ///
    /// Runs the memory-hard proof (~5s) and returns the filename (e.g. "F0ju6...nvA.vsf").
    /// The caller fetches `capsules/{filename}` to load the capsule.
    #[wasm_bindgen]
    pub fn resolve_handle(handle: &str) -> String {
        let (_id, filename) = vsf::handle::resolve_handle(handle);
        filename
    }

    /// Resolve a plaintext handle and return both the hex public ID and filename.
    ///
    /// Returns a two-element array: [hex_id, filename].
    #[wasm_bindgen]
    pub fn resolve_handle_full(handle: &str) -> Vec<String> {
        let (id, filename) = vsf::handle::resolve_handle(handle);
        vec![id.to_hex().to_string(), filename]
    }

    /// Generate arithmetic test bytecode (2 + 3 = 5)
    #[wasm_bindgen]
    pub fn generate_arithmetic_bytecode() -> Vec<u8> {
        use crate::builder::Program;
        Program::new()
            .ps_s44(1)
            .dp()
            .ad() // 2
            .ps_s44(1)
            .ps_s44(1)
            .ad() // 2
            .ps_s44(1)
            .ad() // 3
            .ad() // 5
            .hl()
            .build()
    }
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
