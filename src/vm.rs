//! Toka VM execution engine
//!
//! Stack-based VM with:
//! - VsfType stack (no lossy conversion or promotions/demotions - types are preserved)
//! - Local variables (function-scoped)
//! - Instruction pointer
//! - Capability-checked handle system
//!
//! # Type Safety
//!
//! **No implicit type conversion.** If you push an S44 and an S43, you cannot
//! add them - you get a runtime error. This mirrors Rust's compile-time type
//! safety at runtime. Spirix handles all arithmetic with proper type checking.
//!
//! # Bytecode Format
//! The bytecode is a valid VSF stream where:
//! - `{xx}` = Toka opcodes (two lowercase letters)
//! - Other VSF types = data (pushed by {ps} opcode)
//!
//! # Type Safety
//! Type checking happens at build time via Rust's type system in the builder API.
//! Runtime trusts the bytecode and relies on Rust panics/bounds checks for safety.

use crate::drawing::Canvas;
use crate::opcode::Opcode;
use spirix::{CircleF4E4, ScalarF4E4};
use std::collections::HashMap;
// Note: We use VSF RGB directly, NOT sRGB conversion
// WASM wrapper handles sRGB conversion on Chrome/browser side
use vsf::decoding::parse::parse as vsf_parse;
use vsf::types::VsfType;

/// Macro to generate arithmetic operations for all Spirix types (Scalars + Circles)
/// Handles 25 Scalar types (s33-s77) + 25 Circle types (c33-c77) = 50 types
/// Optimized for F4E4 (ScalarF4E4/CircleF4E4) - faster than IEEE, deterministic!
macro_rules! spirix_binop {
    ($lhs:expr, $rhs:expr, $op:tt, $op_name:expr) => {
        match (&$lhs, &$rhs) {
            // ========== SCALARS (25 types) ==========
            (VsfType::s33(a), VsfType::s33(b)) => Ok(VsfType::s33(a $op b)),
            (VsfType::s34(a), VsfType::s34(b)) => Ok(VsfType::s34(a $op b)),
            (VsfType::s35(a), VsfType::s35(b)) => Ok(VsfType::s35(a $op b)),
            (VsfType::s36(a), VsfType::s36(b)) => Ok(VsfType::s36(a $op b)),
            (VsfType::s37(a), VsfType::s37(b)) => Ok(VsfType::s37(a $op b)),
            (VsfType::s43(a), VsfType::s43(b)) => Ok(VsfType::s43(a $op b)),
            (VsfType::s44(a), VsfType::s44(b)) => Ok(VsfType::s44(a $op b)), // ← F4E4 (optimized!)
            (VsfType::s45(a), VsfType::s45(b)) => Ok(VsfType::s45(a $op b)),
            (VsfType::s46(a), VsfType::s46(b)) => Ok(VsfType::s46(a $op b)),
            (VsfType::s47(a), VsfType::s47(b)) => Ok(VsfType::s47(a $op b)),
            (VsfType::s53(a), VsfType::s53(b)) => Ok(VsfType::s53(a $op b)),
            (VsfType::s54(a), VsfType::s54(b)) => Ok(VsfType::s54(a $op b)),
            (VsfType::s55(a), VsfType::s55(b)) => Ok(VsfType::s55(a $op b)),
            (VsfType::s56(a), VsfType::s56(b)) => Ok(VsfType::s56(a $op b)),
            (VsfType::s57(a), VsfType::s57(b)) => Ok(VsfType::s57(a $op b)),
            (VsfType::s63(a), VsfType::s63(b)) => Ok(VsfType::s63(a $op b)),
            (VsfType::s64(a), VsfType::s64(b)) => Ok(VsfType::s64(a $op b)),
            (VsfType::s65(a), VsfType::s65(b)) => Ok(VsfType::s65(a $op b)),
            (VsfType::s66(a), VsfType::s66(b)) => Ok(VsfType::s66(a $op b)),
            (VsfType::s67(a), VsfType::s67(b)) => Ok(VsfType::s67(a $op b)),
            (VsfType::s73(a), VsfType::s73(b)) => Ok(VsfType::s73(a $op b)),
            (VsfType::s74(a), VsfType::s74(b)) => Ok(VsfType::s74(a $op b)),
            (VsfType::s75(a), VsfType::s75(b)) => Ok(VsfType::s75(a $op b)),
            (VsfType::s76(a), VsfType::s76(b)) => Ok(VsfType::s76(a $op b)),
            (VsfType::s77(a), VsfType::s77(b)) => Ok(VsfType::s77(a $op b)),

            // ========== CIRCLES (25 types) - for (x,y) coordinates! ==========
            (VsfType::c33(a), VsfType::c33(b)) => Ok(VsfType::c33(a $op b)),
            (VsfType::c34(a), VsfType::c34(b)) => Ok(VsfType::c34(a $op b)),
            (VsfType::c35(a), VsfType::c35(b)) => Ok(VsfType::c35(a $op b)),
            (VsfType::c36(a), VsfType::c36(b)) => Ok(VsfType::c36(a $op b)),
            (VsfType::c37(a), VsfType::c37(b)) => Ok(VsfType::c37(a $op b)),
            (VsfType::c43(a), VsfType::c43(b)) => Ok(VsfType::c43(a $op b)),
            (VsfType::c44(a), VsfType::c44(b)) => Ok(VsfType::c44(a $op b)), // ← F4E4 (optimized!)
            (VsfType::c45(a), VsfType::c45(b)) => Ok(VsfType::c45(a $op b)),
            (VsfType::c46(a), VsfType::c46(b)) => Ok(VsfType::c46(a $op b)),
            (VsfType::c47(a), VsfType::c47(b)) => Ok(VsfType::c47(a $op b)),
            (VsfType::c53(a), VsfType::c53(b)) => Ok(VsfType::c53(a $op b)),
            (VsfType::c54(a), VsfType::c54(b)) => Ok(VsfType::c54(a $op b)),
            (VsfType::c55(a), VsfType::c55(b)) => Ok(VsfType::c55(a $op b)),
            (VsfType::c56(a), VsfType::c56(b)) => Ok(VsfType::c56(a $op b)),
            (VsfType::c57(a), VsfType::c57(b)) => Ok(VsfType::c57(a $op b)),
            (VsfType::c63(a), VsfType::c63(b)) => Ok(VsfType::c63(a $op b)),
            (VsfType::c64(a), VsfType::c64(b)) => Ok(VsfType::c64(a $op b)),
            (VsfType::c65(a), VsfType::c65(b)) => Ok(VsfType::c65(a $op b)),
            (VsfType::c66(a), VsfType::c66(b)) => Ok(VsfType::c66(a $op b)),
            (VsfType::c67(a), VsfType::c67(b)) => Ok(VsfType::c67(a $op b)),
            (VsfType::c73(a), VsfType::c73(b)) => Ok(VsfType::c73(a $op b)),
            (VsfType::c74(a), VsfType::c74(b)) => Ok(VsfType::c74(a $op b)),
            (VsfType::c75(a), VsfType::c75(b)) => Ok(VsfType::c75(a $op b)),
            (VsfType::c76(a), VsfType::c76(b)) => Ok(VsfType::c76(a $op b)),
            (VsfType::c77(a), VsfType::c77(b)) => Ok(VsfType::c77(a $op b)),

            // Type mismatch
            _ => Err(format!(
                "Type mismatch in {}: {:?} {} {:?}",
                $op_name,
                type_name(&$lhs),
                stringify!($op),
                type_name(&$rhs)
            )),
        }
    };
}

/// Call frame for function calls
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// Return address (IP to resume after function returns)
    pub return_ip: usize,
    /// Number of local variable frames to preserve
    pub local_count: usize,
}

/// VM execution state
pub struct VM {
    /// Value stack (VsfType values - no lossy conversion)
    stack: Vec<VsfType>,

    /// Bytecode being executed
    bytecode: Vec<u8>,

    /// Instruction pointer (offset into bytecode)
    ip: usize,

    /// Local variables (function-scoped)
    /// Outer vec is call stack frames, inner vec is locals within frame
    locals: Vec<Vec<VsfType>>,

    /// Call stack for function calls (return addresses)
    call_stack: Vec<CallFrame>,

    /// Content-addressed function map: BLAKE3 hash → instruction pointer
    /// "If you know the hash, you can call it" - capability by knowledge
    function_map: HashMap<[u8; 32], usize>,

    /// Whether execution has halted
    halted: bool,

    /// Canvas for drawing operations
    canvas: Canvas,

    /// Execution trace for debugging (opcode names)
    trace: Vec<String>,

    /// Scene VSF storage (ro* type rendered by render_loom)
    /// Enables efficient resize without re-executing bytecode
    scene_vsf: Option<VsfType>,

    /// Whether scene graph has been modified since last render
    scene_dirty: bool,

    /// Scroll offset X in RU (resolution-independent)
    scroll_x: ScalarF4E4,

    /// Scroll offset Y in RU (resolution-independent)
    scroll_y: ScalarF4E4,

    /// Mouse/pointer X position in RU (resolution-independent)
    mouse_x: ScalarF4E4,

    /// Mouse/pointer Y position in RU (resolution-independent)
    mouse_y: ScalarF4E4,

    /// Current time in seconds (Unix timestamp as ScalarF4E4)
    time: ScalarF4E4,
}

impl VM {
    /// Create a new VM with the given bytecode and canvas size
    ///
    /// Note: Canvas dimensions are just the pixel buffer size.
    /// RU (Relative Units) handles all coordinate mapping automatically,
    /// so the same bytecode renders correctly at ANY resolution.
    ///
    /// For testing only - use with_canvas() in production.
    #[cfg(test)]
    pub fn new(bytecode: Vec<u8>) -> Self {
        Self::with_canvas(bytecode, 800, 600)
    }

    /// Create a new VM with the given bytecode and custom canvas size
    pub fn with_canvas(bytecode: Vec<u8>, width: usize, height: usize) -> Self {
        Self {
            stack: Vec::new(),
            bytecode,
            ip: 0,
            locals: vec![Vec::new()], // Start with one frame
            call_stack: Vec::new(),
            function_map: HashMap::new(),
            halted: false,
            canvas: Canvas::new_fast(width, height),
            trace: Vec::new(),
            scene_vsf: None,
            scene_dirty: false,
            scroll_x: ScalarF4E4::ZERO,
            scroll_y: ScalarF4E4::ZERO,
            mouse_x: ScalarF4E4::ZERO,
            mouse_y: ScalarF4E4::ZERO,
            time: ScalarF4E4::ZERO,
        }
    }

    /// Reset VM state to re-execute bytecode from the beginning
    ///
    /// Clears stack, resets instruction pointer, and clears halt flag.
    /// Preserves context variables (scroll, mouse, time) for reactive re-execution.
    pub fn reset(&mut self) {
        self.ip = 0;
        self.halted = false;
        self.stack.clear();
    }

    /// Register a function by its BLAKE3 hash
    ///
    /// Content-addressed functions: "If you know the hash, you can call it"
    /// Hash is BLAKE3 of the function bytecode body
    pub fn register_function(&mut self, hash: [u8; 32], ip: usize) {
        self.function_map.insert(hash, ip);
    }

    /// Look up function IP by hash
    fn resolve_function(&self, hash: &[u8; 32]) -> Result<usize, String> {
        self.function_map
            .get(hash)
            .copied()
            .ok_or_else(|| format!("Unknown function hash: {:?}", hash))
    }

    /// Execute until halt or error
    pub fn run(&mut self) -> Result<(), String> {
        while !self.halted && self.ip < self.bytecode.len() {
            self.step()?;
        }
        Ok(())
    }

    /// Execute one instruction
    pub fn step(&mut self) -> Result<(), String> {
        let ip_before = self.ip;
        if self.ip >= self.bytecode.len() {
            return Err(format!("[IP:{}] Unexpected end of bytecode", ip_before));
        }

        let vsf_value = vsf_parse(&self.bytecode, &mut self.ip)
            .map_err(|e| format!("[IP:{}] VSF parse error: {}", ip_before, e))?;

        match vsf_value {
            VsfType::op(a, b) => {
                let opcode = Opcode::from_bytes(&[a, b]).ok_or_else(|| {
                    format!(
                        "[IP:{}] Unknown opcode: {}{}",
                        ip_before, a as char, b as char
                    )
                })?;
                // Add to execution trace
                self.trace.push(format!("{:?}", opcode));
                self.execute(opcode)
                    .map_err(|e| format!("[IP:{}] {}", ip_before, e))?;
            }
            _ => {
                return Err(format!(
                    "[IP:{}] Expected opcode, got VSF type: {:?}",
                    ip_before, vsf_value
                ));
            }
        }

        Ok(())
    }

    fn pop(&mut self) -> Result<VsfType, String> {
        self.stack
            .pop()
            .ok_or_else(|| "Stack underflow".to_string())
    }

    fn execute(&mut self, opcode: Opcode) -> Result<(), String> {
        match opcode {
            Opcode::push => {
                if self.ip >= self.bytecode.len() {
                    return Err("Bytecode truncated in push".to_string());
                }
                let vsf_value = vsf_parse(&self.bytecode, &mut self.ip)
                    .map_err(|e| format!("push: failed to parse VSF value: {}", e))?;

                self.stack.push(vsf_value);
            }

            Opcode::pop => {
                self.pop()?;
            }

            Opcode::dup => {
                let val = self
                    .stack
                    .last()
                    .ok_or_else(|| "Stack underflow on dup".to_string())?
                    .clone();
                self.stack.push(val);
            }

            Opcode::swap => {
                if self.stack.len() < 2 {
                    return Err("Stack underflow on swap".to_string());
                }
                let len = self.stack.len();
                self.stack.swap(len - 1, len - 2);
            }

            Opcode::add => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = self.execute_add(lhs, rhs)?;
                self.stack.push(result);
            }

            Opcode::sub => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = self.execute_sub(lhs, rhs)?;
                self.stack.push(result);
            }

            Opcode::mul => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = self.execute_mul(lhs, rhs)?;
                self.stack.push(result);
            }

            Opcode::div => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = self.execute_div(lhs, rhs)?;
                self.stack.push(result);
            }

            Opcode::mod_ => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = self.execute_mod(lhs, rhs)?;
                self.stack.push(result);
            }

            Opcode::neg => {
                let val = self.pop()?;
                let result = self.execute_neg(val)?;
                self.stack.push(result);
            }

            Opcode::eq => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = self.execute_eq(lhs, rhs)?;
                self.stack.push(result);
            }

            Opcode::lt => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = self.execute_lt(lhs, rhs)?;
                self.stack.push(result);
            }

            Opcode::ne => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = self.execute_ne(lhs, rhs)?;
                self.stack.push(result);
            }

            Opcode::le => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = self.execute_le(lhs, rhs)?;
                self.stack.push(result);
            }

            Opcode::gt => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = self.execute_gt(lhs, rhs)?;
                self.stack.push(result);
            }

            Opcode::ge => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = self.execute_ge(lhs, rhs)?;
                self.stack.push(result);
            }

            // ==================== CONTROL FLOW (Content-Addressed) ====================
            Opcode::jump => {
                // Pop hash target and jump to it
                let target = self.pop()?;
                match target {
                    VsfType::hb(hash_vec) => {
                        let hash: [u8; 32] = hash_vec
                            .try_into()
                            .map_err(|_| "Jump hash must be 32 bytes (BLAKE3)")?;
                        let target_ip = self.resolve_function(&hash)?;
                        self.ip = target_ip;
                    }
                    _ => return Err(format!("Jump requires hb (BLAKE3 hash), got {:?}", target)),
                }
            }

            Opcode::jump_if => {
                // Pop target hash, then condition (strict u0 only)
                let target = self.pop()?;
                let condition = self.pop()?;

                let should_jump = match condition {
                    VsfType::u0(v) => v,
                    other => {
                        return Err(format!(
                            "jump_if requires u0 (bool), got {}",
                            type_name(&other)
                        ))
                    }
                };

                if should_jump {
                    match target {
                        VsfType::hb(hash_vec) => {
                            let hash: [u8; 32] = hash_vec
                                .try_into()
                                .map_err(|_| "Jump hash must be 32 bytes")?;
                            let target_ip = self.resolve_function(&hash)?;
                            self.ip = target_ip;
                        }
                        _ => return Err("Jump requires hb (BLAKE3 hash)".to_string()),
                    }
                }
            }

            Opcode::call => {
                // Pop function hash
                let target = self.pop()?;
                match target {
                    VsfType::hb(hash_vec) => {
                        let hash: [u8; 32] = hash_vec
                            .try_into()
                            .map_err(|_| "Call hash must be 32 bytes")?;
                        let target_ip = self.resolve_function(&hash)?;

                        // Push call frame
                        self.call_stack.push(CallFrame {
                            return_ip: self.ip,
                            local_count: self.locals.len(),
                        });

                        // Allocate new local frame for function
                        self.locals.push(Vec::new());

                        // Jump to function
                        self.ip = target_ip;
                    }
                    _ => return Err("Call requires hb (BLAKE3 hash)".to_string()),
                }
            }

            Opcode::return_ => {
                // Pop call frame and return
                let frame = self
                    .call_stack
                    .pop()
                    .ok_or("Return without matching call")?;

                // Restore locals to before call
                self.locals.truncate(frame.local_count);

                // Jump back to return address
                self.ip = frame.return_ip;
            }

            Opcode::return_value => {
                // Pop return value, then return
                let return_val = self.pop()?;

                let frame = self
                    .call_stack
                    .pop()
                    .ok_or("Return without matching call")?;

                // Restore locals
                self.locals.truncate(frame.local_count);

                // Push return value back
                self.stack.push(return_val);

                // Jump back
                self.ip = frame.return_ip;
            }

            Opcode::halt => {
                self.halted = true;
            }

            // Bitwise operators (&, |, ^, ~) - work on all numeric types
            Opcode::and => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = self.execute_bitwise_and(lhs, rhs)?;
                self.stack.push(result);
            }

            Opcode::or => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = self.execute_bitwise_or(lhs, rhs)?;
                self.stack.push(result);
            }

            Opcode::xor => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = self.execute_bitwise_xor(lhs, rhs)?;
                self.stack.push(result);
            }

            Opcode::not => {
                let a = self.pop()?;
                let result = self.execute_bitwise_not(a)?;
                self.stack.push(result);
            }

            // ==================== SCENE GRAPH CONSTRUCTION ====================

            Opcode::build_row => {
                // Build row: pop children (ron), rotate (s44), translate (c44)
                // Stack: [..., translate_c44, rotate_s44, children_ron]
                let children_vsf = self.pop()?;
                let rotate_vsf = self.pop()?;
                let translate_vsf = self.pop()?;

                let translate = Self::extract_c44(&translate_vsf)?;
                let rotate = Self::extract_s44(&rotate_vsf)?;

                // Extract children from ron node
                let children = match children_vsf {
                    VsfType::ron(_, _, children_vec) => children_vec,
                    _ => return Err(format!("build_row: expected ron for children, got {:?}", type_name(&children_vsf))),
                };

                let transform = vsf::types::Transform {
                    translate: Some(translate),
                    rotate: Some(rotate),
                    scale: None,
                    origin: None,
                };

                self.stack.push(VsfType::row(transform, children));
            }

            Opcode::build_rob => {
                // Build rob: pop children (ron), fill (colour), size (c44), pos (c44)
                // Stack: [..., pos_c44, size_c44, fill_colour, children_ron]
                let children_vsf = self.pop()?;
                let fill_vsf = self.pop()?;
                let size_vsf = self.pop()?;
                let pos_vsf = self.pop()?;

                let pos = Self::extract_c44(&pos_vsf)?;
                let size = Self::extract_c44(&size_vsf)?;

                // Extract children from ron node
                let children = match children_vsf {
                    VsfType::ron(_, _, children_vec) => children_vec,
                    _ => return Err(format!("build_rob: expected ron for children, got {:?}", type_name(&children_vsf))),
                };

                // Build simple solid fill from colour
                let fill = vsf::types::Fill::Solid(Box::new(fill_vsf));

                self.stack.push(VsfType::rob(pos, size, fill, None, children));
            }

            Opcode::build_roc => {
                // Build roc: pop fill (colour), radius (s44), center (c44)
                // Stack: [..., center_c44, radius_s44, fill_colour]
                let fill_vsf = self.pop()?;
                let radius_vsf = self.pop()?;
                let center_vsf = self.pop()?;

                let center = Self::extract_c44(&center_vsf)?;
                let radius = Self::extract_s44(&radius_vsf)?;

                // Build simple solid fill from colour
                let fill = vsf::types::Fill::Solid(Box::new(fill_vsf));

                self.stack.push(VsfType::roc(center, radius, fill, None));
            }

            Opcode::build_transform => {
                // Not needed - use build_row directly
                return Err("build_transform: use build_row instead".to_string());
            }

            // ==================== LOOM LAYOUT ====================

            Opcode::clear_canvas => {
                // Pop VSF colour type (rc*, ra, or rw)
                let colour = self.pop()?;
                self.canvas.clear(&colour)?;
            }

            Opcode::render_loom => {
                // Pop scene graph from stack (ro* type)
                let vsf = self.stack.pop()
                    .ok_or_else(|| "render_loom: stack underflow".to_string())?;

                // Render directly from ro* type
                let mut renderer = crate::renderer::RenderContext::new();
                renderer.render(&vsf, &mut self.canvas)?;

                // Store VSF for resize handling (not LayoutNode!)
                self.scene_vsf = Some(vsf);
                self.scene_dirty = true;
            }

            Opcode::scroll_x => {
                // Push current scroll X offset (in RU)
                self.stack.push(VsfType::s44(self.scroll_x));
            }

            Opcode::scroll_y => {
                // Push current scroll Y offset (in RU)
                self.stack.push(VsfType::s44(self.scroll_y));
            }

            Opcode::mouse_x => {
                // Push current mouse/pointer X position (in RU)
                self.stack.push(VsfType::s44(self.mouse_x));
            }

            Opcode::mouse_y => {
                // Push current mouse/pointer Y position (in RU)
                self.stack.push(VsfType::s44(self.mouse_y));
            }

            Opcode::timestamp => {
                // Push current time (Unix timestamp in seconds)
                self.stack.push(VsfType::s44(self.time));
            }

            Opcode::debug_print => {
                // Pop value and log it for debugging
                let value = self.pop()?;
                let debug_str = format!("DEBUG: {:?}", value);

                #[cfg(target_arch = "wasm32")]
                {
                    use wasm_bindgen::JsValue;
                    web_sys::console::log_1(&JsValue::from_str(&debug_str));
                }

                #[cfg(not(target_arch = "wasm32"))]
                {
                    println!("{}", debug_str);
                }
            }

            _ => {
                return Err(format!(
                    "[IP:{}] Opcode not yet implemented: {:?}",
                    self.ip, opcode
                ));
            }
        }

        Ok(())
    }

    // Type extraction helpers

    fn extract_s44(vsf: &VsfType) -> Result<ScalarF4E4, String> {
        match vsf {
            VsfType::s44(s) => Ok(*s),
            _ => Err(format!("Expected s44, got {:?}", type_name(vsf))),
        }
    }

    fn extract_c44(vsf: &VsfType) -> Result<CircleF4E4, String> {
        match vsf {
            VsfType::c44(c) => Ok(*c),
            _ => Err(format!("Expected c44, got {:?}", type_name(vsf))),
        }
    }

    // Type-safe arithmetic dispatch - uses fully qualified VsfType:: to avoid naming conflicts

    fn execute_add(&self, lhs: VsfType, rhs: VsfType) -> Result<VsfType, String> {
        spirix_binop!(lhs, rhs, +, "add")
    }

    fn execute_sub(&self, lhs: VsfType, rhs: VsfType) -> Result<VsfType, String> {
        spirix_binop!(lhs, rhs, -, "sub")
    }

    fn execute_mul(&self, lhs: VsfType, rhs: VsfType) -> Result<VsfType, String> {
        spirix_binop!(lhs, rhs, *, "mul")
    }

    fn execute_div(&self, lhs: VsfType, rhs: VsfType) -> Result<VsfType, String> {
        // Use macro for Spirix types (handles division by undefined)
        spirix_binop!(lhs, rhs, /, "div")
    }

    fn execute_mod(&self, lhs: VsfType, rhs: VsfType) -> Result<VsfType, String> {
        spirix_binop!(lhs, rhs, %, "mod")
    }

    fn execute_neg(&self, val: VsfType) -> Result<VsfType, String> {
        match val {
            VsfType::s33(v) => Ok(VsfType::s33(-v)),
            VsfType::s34(v) => Ok(VsfType::s34(-v)),
            VsfType::s35(v) => Ok(VsfType::s35(-v)),
            VsfType::s36(v) => Ok(VsfType::s36(-v)),
            VsfType::s37(v) => Ok(VsfType::s37(-v)),
            VsfType::s43(v) => Ok(VsfType::s43(-v)),
            VsfType::s44(v) => Ok(VsfType::s44(-v)),
            VsfType::s45(v) => Ok(VsfType::s45(-v)),
            VsfType::s46(v) => Ok(VsfType::s46(-v)),
            VsfType::s47(v) => Ok(VsfType::s47(-v)),
            VsfType::s53(v) => Ok(VsfType::s53(-v)),
            VsfType::s54(v) => Ok(VsfType::s54(-v)),
            VsfType::s55(v) => Ok(VsfType::s55(-v)),
            VsfType::s56(v) => Ok(VsfType::s56(-v)),
            VsfType::s57(v) => Ok(VsfType::s57(-v)),
            VsfType::s63(v) => Ok(VsfType::s63(-v)),
            VsfType::s64(v) => Ok(VsfType::s64(-v)),
            VsfType::s65(v) => Ok(VsfType::s65(-v)),
            VsfType::s66(v) => Ok(VsfType::s66(-v)),
            VsfType::s67(v) => Ok(VsfType::s67(-v)),
            VsfType::s73(v) => Ok(VsfType::s73(-v)),
            VsfType::s74(v) => Ok(VsfType::s74(-v)),
            VsfType::s75(v) => Ok(VsfType::s75(-v)),
            VsfType::s76(v) => Ok(VsfType::s76(-v)),
            VsfType::s77(v) => Ok(VsfType::s77(-v)),
            VsfType::i3(v) => Ok(VsfType::i3(v.wrapping_neg())),
            VsfType::i4(v) => Ok(VsfType::i4(v.wrapping_neg())),
            VsfType::i5(v) => Ok(VsfType::i5(v.wrapping_neg())),
            VsfType::i6(v) => Ok(VsfType::i6(v.wrapping_neg())),
            VsfType::i7(v) => Ok(VsfType::i7(v.wrapping_neg())),
            other => Err(format!("Cannot negate type: {}", type_name(&other))),
        }
    }

    fn execute_eq(&self, lhs: VsfType, rhs: VsfType) -> Result<VsfType, String> {
        let result = match (&lhs, &rhs) {
            (VsfType::s33(a), VsfType::s33(b)) => a == b,
            (VsfType::s34(a), VsfType::s34(b)) => a == b,
            (VsfType::s35(a), VsfType::s35(b)) => a == b,
            (VsfType::s36(a), VsfType::s36(b)) => a == b,
            (VsfType::s37(a), VsfType::s37(b)) => a == b,
            (VsfType::s43(a), VsfType::s43(b)) => a == b,
            (VsfType::s44(a), VsfType::s44(b)) => a == b,
            (VsfType::s45(a), VsfType::s45(b)) => a == b,
            (VsfType::s46(a), VsfType::s46(b)) => a == b,
            (VsfType::s47(a), VsfType::s47(b)) => a == b,
            (VsfType::s53(a), VsfType::s53(b)) => a == b,
            (VsfType::s54(a), VsfType::s54(b)) => a == b,
            (VsfType::s55(a), VsfType::s55(b)) => a == b,
            (VsfType::s56(a), VsfType::s56(b)) => a == b,
            (VsfType::s57(a), VsfType::s57(b)) => a == b,
            (VsfType::s63(a), VsfType::s63(b)) => a == b,
            (VsfType::s64(a), VsfType::s64(b)) => a == b,
            (VsfType::s65(a), VsfType::s65(b)) => a == b,
            (VsfType::s66(a), VsfType::s66(b)) => a == b,
            (VsfType::s67(a), VsfType::s67(b)) => a == b,
            (VsfType::s73(a), VsfType::s73(b)) => a == b,
            (VsfType::s74(a), VsfType::s74(b)) => a == b,
            (VsfType::s75(a), VsfType::s75(b)) => a == b,
            (VsfType::s76(a), VsfType::s76(b)) => a == b,
            (VsfType::s77(a), VsfType::s77(b)) => a == b,
            (VsfType::u3(a), VsfType::u3(b)) => a == b,
            (VsfType::u4(a), VsfType::u4(b)) => a == b,
            (VsfType::u5(a), VsfType::u5(b)) => a == b,
            (VsfType::u6(a), VsfType::u6(b)) => a == b,
            (VsfType::u7(a), VsfType::u7(b)) => a == b,
            (VsfType::i3(a), VsfType::i3(b)) => a == b,
            (VsfType::i4(a), VsfType::i4(b)) => a == b,
            (VsfType::i5(a), VsfType::i5(b)) => a == b,
            (VsfType::i6(a), VsfType::i6(b)) => a == b,
            (VsfType::i7(a), VsfType::i7(b)) => a == b,
            (VsfType::x(a), VsfType::x(b)) => a == b,
            (VsfType::l(a), VsfType::l(b)) => a == b,
            (VsfType::d(a), VsfType::d(b)) => a == b,
            (VsfType::u0(a), VsfType::u0(b)) => a == b,
            (a, b) => {
                return Err(format!(
                    "Type mismatch in eq: {} == {}",
                    type_name(a),
                    type_name(b)
                ))
            }
        };
        Ok(VsfType::u0(result))
    }

    fn execute_lt(&self, lhs: VsfType, rhs: VsfType) -> Result<VsfType, String> {
        let result = match (lhs, rhs) {
            (VsfType::s33(a), VsfType::s33(b)) => a < b,
            (VsfType::s34(a), VsfType::s34(b)) => a < b,
            (VsfType::s35(a), VsfType::s35(b)) => a < b,
            (VsfType::s36(a), VsfType::s36(b)) => a < b,
            (VsfType::s37(a), VsfType::s37(b)) => a < b,
            (VsfType::s43(a), VsfType::s43(b)) => a < b,
            (VsfType::s44(a), VsfType::s44(b)) => a < b,
            (VsfType::s45(a), VsfType::s45(b)) => a < b,
            (VsfType::s46(a), VsfType::s46(b)) => a < b,
            (VsfType::s47(a), VsfType::s47(b)) => a < b,
            (VsfType::s53(a), VsfType::s53(b)) => a < b,
            (VsfType::s54(a), VsfType::s54(b)) => a < b,
            (VsfType::s55(a), VsfType::s55(b)) => a < b,
            (VsfType::s56(a), VsfType::s56(b)) => a < b,
            (VsfType::s57(a), VsfType::s57(b)) => a < b,
            (VsfType::s63(a), VsfType::s63(b)) => a < b,
            (VsfType::s64(a), VsfType::s64(b)) => a < b,
            (VsfType::s65(a), VsfType::s65(b)) => a < b,
            (VsfType::s66(a), VsfType::s66(b)) => a < b,
            (VsfType::s67(a), VsfType::s67(b)) => a < b,
            (VsfType::s73(a), VsfType::s73(b)) => a < b,
            (VsfType::s74(a), VsfType::s74(b)) => a < b,
            (VsfType::s75(a), VsfType::s75(b)) => a < b,
            (VsfType::s76(a), VsfType::s76(b)) => a < b,
            (VsfType::s77(a), VsfType::s77(b)) => a < b,
            (VsfType::u3(a), VsfType::u3(b)) => a < b,
            (VsfType::u4(a), VsfType::u4(b)) => a < b,
            (VsfType::u5(a), VsfType::u5(b)) => a < b,
            (VsfType::u6(a), VsfType::u6(b)) => a < b,
            (VsfType::u7(a), VsfType::u7(b)) => a < b,
            (VsfType::i3(a), VsfType::i3(b)) => a < b,
            (VsfType::i4(a), VsfType::i4(b)) => a < b,
            (VsfType::i5(a), VsfType::i5(b)) => a < b,
            (VsfType::i6(a), VsfType::i6(b)) => a < b,
            (VsfType::i7(a), VsfType::i7(b)) => a < b,
            (a, b) => {
                return Err(format!(
                    "Type mismatch in lt: {} < {}",
                    type_name(&a),
                    type_name(&b)
                ))
            }
        };
        Ok(VsfType::u0(result))
    }

    fn execute_ne(&self, lhs: VsfType, rhs: VsfType) -> Result<VsfType, String> {
        let result = match (&lhs, &rhs) {
            (VsfType::s33(a), VsfType::s33(b)) => a != b,
            (VsfType::s34(a), VsfType::s34(b)) => a != b,
            (VsfType::s35(a), VsfType::s35(b)) => a != b,
            (VsfType::s36(a), VsfType::s36(b)) => a != b,
            (VsfType::s37(a), VsfType::s37(b)) => a != b,
            (VsfType::s43(a), VsfType::s43(b)) => a != b,
            (VsfType::s44(a), VsfType::s44(b)) => a != b,
            (VsfType::s45(a), VsfType::s45(b)) => a != b,
            (VsfType::s46(a), VsfType::s46(b)) => a != b,
            (VsfType::s47(a), VsfType::s47(b)) => a != b,
            (VsfType::s53(a), VsfType::s53(b)) => a != b,
            (VsfType::s54(a), VsfType::s54(b)) => a != b,
            (VsfType::s55(a), VsfType::s55(b)) => a != b,
            (VsfType::s56(a), VsfType::s56(b)) => a != b,
            (VsfType::s57(a), VsfType::s57(b)) => a != b,
            (VsfType::s63(a), VsfType::s63(b)) => a != b,
            (VsfType::s64(a), VsfType::s64(b)) => a != b,
            (VsfType::s65(a), VsfType::s65(b)) => a != b,
            (VsfType::s66(a), VsfType::s66(b)) => a != b,
            (VsfType::s67(a), VsfType::s67(b)) => a != b,
            (VsfType::s73(a), VsfType::s73(b)) => a != b,
            (VsfType::s74(a), VsfType::s74(b)) => a != b,
            (VsfType::s75(a), VsfType::s75(b)) => a != b,
            (VsfType::s76(a), VsfType::s76(b)) => a != b,
            (VsfType::s77(a), VsfType::s77(b)) => a != b,
            (VsfType::u3(a), VsfType::u3(b)) => a != b,
            (VsfType::u4(a), VsfType::u4(b)) => a != b,
            (VsfType::u5(a), VsfType::u5(b)) => a != b,
            (VsfType::u6(a), VsfType::u6(b)) => a != b,
            (VsfType::u7(a), VsfType::u7(b)) => a != b,
            (VsfType::i3(a), VsfType::i3(b)) => a != b,
            (VsfType::i4(a), VsfType::i4(b)) => a != b,
            (VsfType::i5(a), VsfType::i5(b)) => a != b,
            (VsfType::i6(a), VsfType::i6(b)) => a != b,
            (VsfType::i7(a), VsfType::i7(b)) => a != b,
            (VsfType::x(a), VsfType::x(b)) => a != b,
            (VsfType::l(a), VsfType::l(b)) => a != b,
            (VsfType::d(a), VsfType::d(b)) => a != b,
            (VsfType::u0(a), VsfType::u0(b)) => a != b,
            (a, b) => {
                return Err(format!(
                    "Type mismatch in ne: {} != {}",
                    type_name(a),
                    type_name(b)
                ))
            }
        };
        Ok(VsfType::u0(result))
    }

    fn execute_le(&self, lhs: VsfType, rhs: VsfType) -> Result<VsfType, String> {
        let result = match (lhs, rhs) {
            (VsfType::s33(a), VsfType::s33(b)) => a <= b,
            (VsfType::s34(a), VsfType::s34(b)) => a <= b,
            (VsfType::s35(a), VsfType::s35(b)) => a <= b,
            (VsfType::s36(a), VsfType::s36(b)) => a <= b,
            (VsfType::s37(a), VsfType::s37(b)) => a <= b,
            (VsfType::s43(a), VsfType::s43(b)) => a <= b,
            (VsfType::s44(a), VsfType::s44(b)) => a <= b,
            (VsfType::s45(a), VsfType::s45(b)) => a <= b,
            (VsfType::s46(a), VsfType::s46(b)) => a <= b,
            (VsfType::s47(a), VsfType::s47(b)) => a <= b,
            (VsfType::s53(a), VsfType::s53(b)) => a <= b,
            (VsfType::s54(a), VsfType::s54(b)) => a <= b,
            (VsfType::s55(a), VsfType::s55(b)) => a <= b,
            (VsfType::s56(a), VsfType::s56(b)) => a <= b,
            (VsfType::s57(a), VsfType::s57(b)) => a <= b,
            (VsfType::s63(a), VsfType::s63(b)) => a <= b,
            (VsfType::s64(a), VsfType::s64(b)) => a <= b,
            (VsfType::s65(a), VsfType::s65(b)) => a <= b,
            (VsfType::s66(a), VsfType::s66(b)) => a <= b,
            (VsfType::s67(a), VsfType::s67(b)) => a <= b,
            (VsfType::s73(a), VsfType::s73(b)) => a <= b,
            (VsfType::s74(a), VsfType::s74(b)) => a <= b,
            (VsfType::s75(a), VsfType::s75(b)) => a <= b,
            (VsfType::s76(a), VsfType::s76(b)) => a <= b,
            (VsfType::s77(a), VsfType::s77(b)) => a <= b,
            (VsfType::u3(a), VsfType::u3(b)) => a <= b,
            (VsfType::u4(a), VsfType::u4(b)) => a <= b,
            (VsfType::u5(a), VsfType::u5(b)) => a <= b,
            (VsfType::u6(a), VsfType::u6(b)) => a <= b,
            (VsfType::u7(a), VsfType::u7(b)) => a <= b,
            (VsfType::i3(a), VsfType::i3(b)) => a <= b,
            (VsfType::i4(a), VsfType::i4(b)) => a <= b,
            (VsfType::i5(a), VsfType::i5(b)) => a <= b,
            (VsfType::i6(a), VsfType::i6(b)) => a <= b,
            (VsfType::i7(a), VsfType::i7(b)) => a <= b,
            (a, b) => {
                return Err(format!(
                    "Type mismatch in le: {} <= {}",
                    type_name(&a),
                    type_name(&b)
                ))
            }
        };
        Ok(VsfType::u0(result))
    }

    fn execute_gt(&self, lhs: VsfType, rhs: VsfType) -> Result<VsfType, String> {
        let result = match (lhs, rhs) {
            (VsfType::s33(a), VsfType::s33(b)) => a > b,
            (VsfType::s34(a), VsfType::s34(b)) => a > b,
            (VsfType::s35(a), VsfType::s35(b)) => a > b,
            (VsfType::s36(a), VsfType::s36(b)) => a > b,
            (VsfType::s37(a), VsfType::s37(b)) => a > b,
            (VsfType::s43(a), VsfType::s43(b)) => a > b,
            (VsfType::s44(a), VsfType::s44(b)) => a > b,
            (VsfType::s45(a), VsfType::s45(b)) => a > b,
            (VsfType::s46(a), VsfType::s46(b)) => a > b,
            (VsfType::s47(a), VsfType::s47(b)) => a > b,
            (VsfType::s53(a), VsfType::s53(b)) => a > b,
            (VsfType::s54(a), VsfType::s54(b)) => a > b,
            (VsfType::s55(a), VsfType::s55(b)) => a > b,
            (VsfType::s56(a), VsfType::s56(b)) => a > b,
            (VsfType::s57(a), VsfType::s57(b)) => a > b,
            (VsfType::s63(a), VsfType::s63(b)) => a > b,
            (VsfType::s64(a), VsfType::s64(b)) => a > b,
            (VsfType::s65(a), VsfType::s65(b)) => a > b,
            (VsfType::s66(a), VsfType::s66(b)) => a > b,
            (VsfType::s67(a), VsfType::s67(b)) => a > b,
            (VsfType::s73(a), VsfType::s73(b)) => a > b,
            (VsfType::s74(a), VsfType::s74(b)) => a > b,
            (VsfType::s75(a), VsfType::s75(b)) => a > b,
            (VsfType::s76(a), VsfType::s76(b)) => a > b,
            (VsfType::s77(a), VsfType::s77(b)) => a > b,
            (VsfType::u3(a), VsfType::u3(b)) => a > b,
            (VsfType::u4(a), VsfType::u4(b)) => a > b,
            (VsfType::u5(a), VsfType::u5(b)) => a > b,
            (VsfType::u6(a), VsfType::u6(b)) => a > b,
            (VsfType::u7(a), VsfType::u7(b)) => a > b,
            (VsfType::i3(a), VsfType::i3(b)) => a > b,
            (VsfType::i4(a), VsfType::i4(b)) => a > b,
            (VsfType::i5(a), VsfType::i5(b)) => a > b,
            (VsfType::i6(a), VsfType::i6(b)) => a > b,
            (VsfType::i7(a), VsfType::i7(b)) => a > b,
            (a, b) => {
                return Err(format!(
                    "Type mismatch in gt: {} > {}",
                    type_name(&a),
                    type_name(&b)
                ))
            }
        };
        Ok(VsfType::u0(result))
    }

    fn execute_ge(&self, lhs: VsfType, rhs: VsfType) -> Result<VsfType, String> {
        let result = match (lhs, rhs) {
            (VsfType::s33(a), VsfType::s33(b)) => a >= b,
            (VsfType::s34(a), VsfType::s34(b)) => a >= b,
            (VsfType::s35(a), VsfType::s35(b)) => a >= b,
            (VsfType::s36(a), VsfType::s36(b)) => a >= b,
            (VsfType::s37(a), VsfType::s37(b)) => a >= b,
            (VsfType::s43(a), VsfType::s43(b)) => a >= b,
            (VsfType::s44(a), VsfType::s44(b)) => a >= b,
            (VsfType::s45(a), VsfType::s45(b)) => a >= b,
            (VsfType::s46(a), VsfType::s46(b)) => a >= b,
            (VsfType::s47(a), VsfType::s47(b)) => a >= b,
            (VsfType::s53(a), VsfType::s53(b)) => a >= b,
            (VsfType::s54(a), VsfType::s54(b)) => a >= b,
            (VsfType::s55(a), VsfType::s55(b)) => a >= b,
            (VsfType::s56(a), VsfType::s56(b)) => a >= b,
            (VsfType::s57(a), VsfType::s57(b)) => a >= b,
            (VsfType::s63(a), VsfType::s63(b)) => a >= b,
            (VsfType::s64(a), VsfType::s64(b)) => a >= b,
            (VsfType::s65(a), VsfType::s65(b)) => a >= b,
            (VsfType::s66(a), VsfType::s66(b)) => a >= b,
            (VsfType::s67(a), VsfType::s67(b)) => a >= b,
            (VsfType::s73(a), VsfType::s73(b)) => a >= b,
            (VsfType::s74(a), VsfType::s74(b)) => a >= b,
            (VsfType::s75(a), VsfType::s75(b)) => a >= b,
            (VsfType::s76(a), VsfType::s76(b)) => a >= b,
            (VsfType::s77(a), VsfType::s77(b)) => a >= b,
            (VsfType::u3(a), VsfType::u3(b)) => a >= b,
            (VsfType::u4(a), VsfType::u4(b)) => a >= b,
            (VsfType::u5(a), VsfType::u5(b)) => a >= b,
            (VsfType::u6(a), VsfType::u6(b)) => a >= b,
            (VsfType::u7(a), VsfType::u7(b)) => a >= b,
            (VsfType::i3(a), VsfType::i3(b)) => a >= b,
            (VsfType::i4(a), VsfType::i4(b)) => a >= b,
            (VsfType::i5(a), VsfType::i5(b)) => a >= b,
            (VsfType::i6(a), VsfType::i6(b)) => a >= b,
            (VsfType::i7(a), VsfType::i7(b)) => a >= b,
            (a, b) => {
                return Err(format!(
                    "Type mismatch in ge: {} >= {}",
                    type_name(&a),
                    type_name(&b)
                ))
            }
        };
        Ok(VsfType::u0(result))
    }

    fn execute_bitwise_and(&self, lhs: VsfType, rhs: VsfType) -> Result<VsfType, String> {
        spirix_binop!(lhs, rhs, &, "bitwise AND")
    }

    fn execute_bitwise_or(&self, lhs: VsfType, rhs: VsfType) -> Result<VsfType, String> {
        spirix_binop!(lhs, rhs, |, "bitwise OR")
    }

    fn execute_bitwise_xor(&self, lhs: VsfType, rhs: VsfType) -> Result<VsfType, String> {
        spirix_binop!(lhs, rhs, ^, "bitwise XOR")
    }

    fn execute_bitwise_not(&self, val: VsfType) -> Result<VsfType, String> {
        match val {
            // ========== SCALARS (25 types) ==========
            VsfType::s33(a) => Ok(VsfType::s33(!a)),
            VsfType::s34(a) => Ok(VsfType::s34(!a)),
            VsfType::s35(a) => Ok(VsfType::s35(!a)),
            VsfType::s36(a) => Ok(VsfType::s36(!a)),
            VsfType::s37(a) => Ok(VsfType::s37(!a)),
            VsfType::s43(a) => Ok(VsfType::s43(!a)),
            VsfType::s44(a) => Ok(VsfType::s44(!a)),
            VsfType::s45(a) => Ok(VsfType::s45(!a)),
            VsfType::s46(a) => Ok(VsfType::s46(!a)),
            VsfType::s47(a) => Ok(VsfType::s47(!a)),
            VsfType::s53(a) => Ok(VsfType::s53(!a)),
            VsfType::s54(a) => Ok(VsfType::s54(!a)),
            VsfType::s55(a) => Ok(VsfType::s55(!a)),
            VsfType::s56(a) => Ok(VsfType::s56(!a)),
            VsfType::s57(a) => Ok(VsfType::s57(!a)),
            VsfType::s63(a) => Ok(VsfType::s63(!a)),
            VsfType::s64(a) => Ok(VsfType::s64(!a)),
            VsfType::s65(a) => Ok(VsfType::s65(!a)),
            VsfType::s66(a) => Ok(VsfType::s66(!a)),
            VsfType::s67(a) => Ok(VsfType::s67(!a)),
            VsfType::s73(a) => Ok(VsfType::s73(!a)),
            VsfType::s74(a) => Ok(VsfType::s74(!a)),
            VsfType::s75(a) => Ok(VsfType::s75(!a)),
            VsfType::s76(a) => Ok(VsfType::s76(!a)),
            VsfType::s77(a) => Ok(VsfType::s77(!a)),

            // ========== CIRCLES (25 types) ==========
            VsfType::c33(a) => Ok(VsfType::c33(!a)),
            VsfType::c34(a) => Ok(VsfType::c34(!a)),
            VsfType::c35(a) => Ok(VsfType::c35(!a)),
            VsfType::c36(a) => Ok(VsfType::c36(!a)),
            VsfType::c37(a) => Ok(VsfType::c37(!a)),
            VsfType::c43(a) => Ok(VsfType::c43(!a)),
            VsfType::c44(a) => Ok(VsfType::c44(!a)),
            VsfType::c45(a) => Ok(VsfType::c45(!a)),
            VsfType::c46(a) => Ok(VsfType::c46(!a)),
            VsfType::c47(a) => Ok(VsfType::c47(!a)),
            VsfType::c53(a) => Ok(VsfType::c53(!a)),
            VsfType::c54(a) => Ok(VsfType::c54(!a)),
            VsfType::c55(a) => Ok(VsfType::c55(!a)),
            VsfType::c56(a) => Ok(VsfType::c56(!a)),
            VsfType::c57(a) => Ok(VsfType::c57(!a)),
            VsfType::c63(a) => Ok(VsfType::c63(!a)),
            VsfType::c64(a) => Ok(VsfType::c64(!a)),
            VsfType::c65(a) => Ok(VsfType::c65(!a)),
            VsfType::c66(a) => Ok(VsfType::c66(!a)),
            VsfType::c67(a) => Ok(VsfType::c67(!a)),
            VsfType::c73(a) => Ok(VsfType::c73(!a)),
            VsfType::c74(a) => Ok(VsfType::c74(!a)),
            VsfType::c75(a) => Ok(VsfType::c75(!a)),
            VsfType::c76(a) => Ok(VsfType::c76(!a)),
            VsfType::c77(a) => Ok(VsfType::c77(!a)),

            other => Err(format!("Cannot bitwise NOT type: {}", type_name(&other))),
        }
    }


    /// Peek at top of stack without popping
    pub fn peek(&self) -> Option<&VsfType> {
        self.stack.last()
    }
    /// Get stack depth
    pub fn stack_depth(&self) -> usize {
        self.stack.len()
    }
    /// Check if halted
    pub fn is_halted(&self) -> bool {
        self.halted
    }
    /// Get reference to canvas
    pub fn canvas(&self) -> &Canvas {
        &self.canvas
    }
    /// Get mutable reference to canvas (for zoom control)
    pub fn canvas_mut(&mut self) -> &mut Canvas {
        &mut self.canvas
    }

    /// Check if a scene has been rendered
    pub fn has_scene(&self) -> bool {
        self.scene_vsf.is_some()
    }

    /// Get reference to the stored scene VSF (if any)
    pub fn scene_vsf(&self) -> Option<&VsfType> {
        self.scene_vsf.as_ref()
    }

    /// Set the stored scene VSF (for resize handling)
    pub fn set_scene_vsf(&mut self, vsf: VsfType) {
        self.scene_vsf = Some(vsf);
        self.scene_dirty = true;
    }

    /// Replace the canvas (for resize handling)
    pub fn set_canvas(&mut self, canvas: Canvas) {
        self.canvas = canvas;
    }

    /// Set scroll offset X (in RU)
    pub fn set_scroll_x(&mut self, scroll_x: ScalarF4E4) {
        self.scroll_x = scroll_x;
    }

    /// Set scroll offset Y (in RU)
    pub fn set_scroll_y(&mut self, scroll_y: ScalarF4E4) {
        self.scroll_y = scroll_y;
    }

    /// Set scroll offset (in RU)
    pub fn set_scroll(&mut self, scroll_x: ScalarF4E4, scroll_y: ScalarF4E4) {
        self.scroll_x = scroll_x;
        self.scroll_y = scroll_y;
    }

    /// Get scroll offset X (in RU)
    pub fn scroll_x(&self) -> ScalarF4E4 {
        self.scroll_x
    }

    /// Get scroll offset Y (in RU)
    pub fn scroll_y(&self) -> ScalarF4E4 {
        self.scroll_y
    }

    /// Set mouse/pointer X position (in RU)
    pub fn set_mouse_x(&mut self, mouse_x: ScalarF4E4) {
        self.mouse_x = mouse_x;
    }

    /// Set mouse/pointer Y position (in RU)
    pub fn set_mouse_y(&mut self, mouse_y: ScalarF4E4) {
        self.mouse_y = mouse_y;
    }

    /// Set mouse/pointer position (in RU)
    pub fn set_mouse(&mut self, mouse_x: ScalarF4E4, mouse_y: ScalarF4E4) {
        self.mouse_x = mouse_x;
        self.mouse_y = mouse_y;
    }

    /// Get mouse/pointer X position (in RU)
    pub fn mouse_x(&self) -> ScalarF4E4 {
        self.mouse_x
    }

    /// Get mouse/pointer Y position (in RU)
    pub fn mouse_y(&self) -> ScalarF4E4 {
        self.mouse_y
    }

    /// Set current time (Unix timestamp in seconds)
    pub fn set_time(&mut self, time: ScalarF4E4) {
        self.time = time;
    }

    /// Get current time (Unix timestamp in seconds)
    pub fn time(&self) -> ScalarF4E4 {
        self.time
    }

    /// Re-render stored scene VSF to canvas (for resize handling)
    ///
    /// This enables efficient window resize without re-executing bytecode.
    /// The scene VSF is preserved from render_loom execution and can be
    /// re-rasterized at any resolution.
    pub fn rerender_scene(&mut self) -> Result<(), String> {
        let scene_vsf = self
            .scene_vsf
            .as_ref()
            .ok_or("No scene to render")?;

        // Clear canvas to black
        self.canvas.clear(&VsfType::rck)?;

        // Re-render scene using renderer
        let mut renderer = crate::renderer::RenderContext::new();
        renderer.render(scene_vsf, &mut self.canvas)?;
        self.scene_dirty = false;

        Ok(())
    }

    /// Get and clear execution trace
    pub fn take_trace(&mut self) -> Vec<String> {
        std::mem::take(&mut self.trace)
    }

    /// Get stack slice (for testing)
    #[cfg(test)]
    pub fn stack(&self) -> &[VsfType] {
        &self.stack
    }
}

fn type_name(v: &VsfType) -> &'static str {
    match v {
        VsfType::s33(_) => "s33",
        VsfType::s34(_) => "s34",
        VsfType::s35(_) => "s35",
        VsfType::s36(_) => "s36",
        VsfType::s37(_) => "s37",
        VsfType::s43(_) => "s43",
        VsfType::s44(_) => "s44",
        VsfType::s45(_) => "s45",
        VsfType::s46(_) => "s46",
        VsfType::s47(_) => "s47",
        VsfType::s53(_) => "s53",
        VsfType::s54(_) => "s54",
        VsfType::s55(_) => "s55",
        VsfType::s56(_) => "s56",
        VsfType::s57(_) => "s57",
        VsfType::s63(_) => "s63",
        VsfType::s64(_) => "s64",
        VsfType::s65(_) => "s65",
        VsfType::s66(_) => "s66",
        VsfType::s67(_) => "s67",
        VsfType::s73(_) => "s73",
        VsfType::s74(_) => "s74",
        VsfType::s75(_) => "s75",
        VsfType::s76(_) => "s76",
        VsfType::s77(_) => "s77",
        VsfType::c33(_) => "c33",
        VsfType::c34(_) => "c34",
        VsfType::c35(_) => "c35",
        VsfType::c36(_) => "c36",
        VsfType::c37(_) => "c37",
        VsfType::c43(_) => "c43",
        VsfType::c44(_) => "c44",
        VsfType::c45(_) => "c45",
        VsfType::c46(_) => "c46",
        VsfType::c47(_) => "c47",
        VsfType::c53(_) => "c53",
        VsfType::c54(_) => "c54",
        VsfType::c55(_) => "c55",
        VsfType::c56(_) => "c56",
        VsfType::c57(_) => "c57",
        VsfType::c63(_) => "c63",
        VsfType::c64(_) => "c64",
        VsfType::c65(_) => "c65",
        VsfType::c66(_) => "c66",
        VsfType::c67(_) => "c67",
        VsfType::c73(_) => "c73",
        VsfType::c74(_) => "c74",
        VsfType::c75(_) => "c75",
        VsfType::c76(_) => "c76",
        VsfType::c77(_) => "c77",
        VsfType::u0(_) => "u0",
        VsfType::u3(_) => "u3",
        VsfType::u4(_) => "u4",
        VsfType::u5(_) => "u5",
        VsfType::u6(_) => "u6",
        VsfType::u7(_) => "u7",
        VsfType::i3(_) => "i3",
        VsfType::i4(_) => "i4",
        VsfType::i5(_) => "i5",
        VsfType::i6(_) => "i6",
        VsfType::i7(_) => "i7",
        VsfType::u(_, _) => "u",
        VsfType::i(_) => "i",
        VsfType::f5(_) => "f5",
        VsfType::f6(_) => "f6",
        VsfType::j5(_) => "j5",
        VsfType::j6(_) => "j6",
        VsfType::x(_) => "x",
        VsfType::l(_) => "l",
        VsfType::d(_) => "d",
        VsfType::rck
        | VsfType::rcw
        | VsfType::rcr
        | VsfType::rcn
        | VsfType::rcb
        | VsfType::rcc
        | VsfType::rcj
        | VsfType::rcy
        | VsfType::rcg
        | VsfType::rco
        | VsfType::rcv
        | VsfType::rcl
        | VsfType::rcq => "colour",
        // Catch-all for unhandled VSF types (useful for debugging)
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arithmetic() {
        // Test 1 + 1 + 1 + 1 + 1 = 5 using builder API
        use crate::builder::Program;

        let bytecode = Program::new()
            .ps_s44(1)
            .dp()
            .ad() // 2
            .ps_s44(1)
            .ps_s44(1)
            .ad() // 2, 2
            .ps_s44(1)
            .ad() // 2, 3
            .ad() // 5
            .hl()
            .build();

        let mut vm = VM::new(bytecode);
        vm.run().unwrap();
        assert_eq!(vm.stack_depth(), 1);
        match vm.peek().unwrap() {
            VsfType::s44(s) => assert_eq!(*s, ScalarF4E4::from(5)),
            _ => panic!("Expected s44"),
        }
    }

    #[test]
    fn test_comparison() {
        // Test 2 < 3 = true
        use crate::builder::Program;

        let bytecode = Program::new().ps_s44(2).ps_s44(3).lo().hl().build();

        let mut vm = VM::new(bytecode);
        vm.run().unwrap();
        match vm.peek().unwrap() {
            VsfType::s44(s) => assert_eq!(*s, ScalarF4E4::ONE),
            _ => panic!("Expected s44"),
        }
    }
}
