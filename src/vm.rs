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

use crate::canvas::Canvas;
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
            canvas: Canvas::new(width, height),
        }
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
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("[VM] push: Starting at IP {}", self.ip).into());

                if self.ip >= self.bytecode.len() {
                    return Err("Bytecode truncated in push".to_string());
                }
                let vsf_value = vsf_parse(&self.bytecode, &mut self.ip)
                    .map_err(|e| format!("push: failed to parse VSF value: {}", e))?;

                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("[VM] push: Parsed value, new IP {}", self.ip).into());

                self.stack.push(vsf_value);

                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("[VM] push: Stack size now {}", self.stack.len()).into());
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
                // Pop target hash, then condition
                let target = self.pop()?;
                let condition = self.pop()?;

                // Check if condition is truthy (non-zero)
                let is_truthy = match &condition {
                    VsfType::s44(v) => !v.is_zero(),
                    VsfType::u3(v) => *v != 0,
                    VsfType::u4(v) => *v != 0,
                    VsfType::u5(v) => *v != 0,
                    _ => {
                        return Err(format!(
                            "jump_if condition must be numeric, got {:?}",
                            condition
                        ))
                    }
                };

                if is_truthy {
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

            Opcode::jump_zero => {
                // Pop target hash, then condition
                let target = self.pop()?;
                let condition = self.pop()?;

                let is_zero = match &condition {
                    VsfType::s44(v) => v.is_zero(),
                    VsfType::u3(v) => *v == 0,
                    VsfType::u4(v) => *v == 0,
                    VsfType::u5(v) => *v == 0,
                    _ => {
                        return Err(format!(
                            "jump_zero condition must be numeric, got {:?}",
                            condition
                        ))
                    }
                };

                if is_zero {
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

            Opcode::call_indirect => {
                // Pop handle (capability-based indirect call)
                return Err(
                    "call_indirect not yet implemented (requires capability system)".to_string(),
                );
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
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&"[VM] halt: Halting VM".into());
                self.halted = true;
            }

            Opcode::clear => {
                // Pop RGBA as 4 separate s44 values (stack is LIFO, so reverse order)
                let a = self.pop_s44()?;
                let b = self.pop_s44()?;
                let g = self.pop_s44()?;
                let r = self.pop_s44()?;
                self.canvas.clear(r, g, b, a);
            }

            Opcode::fill_rect => {
                let size = self.pop_c44()?;   // (w, h)
                let pos = self.pop_c44()?;    // (x, y)
                let colour = [
                    self.pop_s44()?,  // r
                    self.pop_s44()?,  // g
                    self.pop_s44()?,  // b
                    self.pop_s44()?,  // a
                ];
                self.canvas.fill_rect_ru(pos, size, colour);
            }

            Opcode::draw_text => {
                let size = self.pop_s44()?;
                let pos = self.pop_c44()?;    // (x, y)
                let text = match self.pop()? {
                    VsfType::x(s) => s,
                    VsfType::l(s) => s,
                    other => {
                        return Err(format!(
                            "draw_text requires string, got {}",
                            type_name(&other)
                        ))
                    }
                };
                let colour = [
                    self.pop_s44()?,  // r
                    self.pop_s44()?,  // g
                    self.pop_s44()?,  // b
                    self.pop_s44()?,  // a
                ];
                self.canvas.draw_text(pos, size, &text, colour);
            }

            Opcode::fill_circle => {
                let radius = self.pop_s44()?;
                let center = self.pop_c44()?;  // (x, y)
                let colour = [
                    self.pop_s44()?,  // r
                    self.pop_s44()?,  // g
                    self.pop_s44()?,  // b
                    self.pop_s44()?,  // a
                ];
                self.canvas.fill_circle(center, radius, colour);
            }

            Opcode::stroke_circle => {
                let stroke_width = self.pop_s44()?;
                let radius = self.pop_s44()?;
                let center = self.pop_c44()?;  // (x, y)
                let colour = [
                    self.pop_s44()?,  // r
                    self.pop_s44()?,  // g
                    self.pop_s44()?,  // b
                    self.pop_s44()?,  // a
                ];
                self.canvas.stroke_circle(center, radius, stroke_width, colour);
            }

            Opcode::rgba => {
                // Pop RGBA components and push as 4 separate s44 values
                // Keeps VSF RGB colour space, no conversion until WASM boundary
                let alpha = self.pop_s44()?;
                let blue = self.pop_s44()?;
                let green = self.pop_s44()?;
                let red = self.pop_s44()?;

                self.stack.push(VsfType::s44(red));
                self.stack.push(VsfType::s44(green));
                self.stack.push(VsfType::s44(blue));
                self.stack.push(VsfType::s44(alpha));
            }

            Opcode::rgb => {
                // Pop RGB components, default alpha=1, push as 4 separate s44 values
                let blue = self.pop_s44()?;
                let green = self.pop_s44()?;
                let red = self.pop_s44()?;

                self.stack.push(VsfType::s44(red));
                self.stack.push(VsfType::s44(green));
                self.stack.push(VsfType::s44(blue));
                self.stack.push(VsfType::s44(ScalarF4E4::ONE));
            }

            // Logical operators (&&, ||, !) - return 1 or 0 based on truthiness
            Opcode::and => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = if self.is_truthy(&a) && self.is_truthy(&b) {
                    VsfType::s44(ScalarF4E4::ONE)
                } else {
                    VsfType::s44(ScalarF4E4::ZERO)
                };
                self.stack.push(result);
            }

            Opcode::or => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = if self.is_truthy(&a) || self.is_truthy(&b) {
                    VsfType::s44(ScalarF4E4::ONE)
                } else {
                    VsfType::s44(ScalarF4E4::ZERO)
                };
                self.stack.push(result);
            }

            Opcode::not => {
                let a = self.pop()?;
                let result = if self.is_truthy(&a) {
                    VsfType::s44(ScalarF4E4::ZERO)
                } else {
                    VsfType::s44(ScalarF4E4::ONE)
                };
                self.stack.push(result);
            }

            // Bitwise operators (&, |, ^, ~) - operate on bits via Spirix
            Opcode::bit_and => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = self.execute_bitwise_and(lhs, rhs)?;
                self.stack.push(result);
            }

            Opcode::bit_or => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = self.execute_bitwise_or(lhs, rhs)?;
                self.stack.push(result);
            }

            Opcode::bit_xor => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = self.execute_bitwise_xor(lhs, rhs)?;
                self.stack.push(result);
            }

            Opcode::bit_not => {
                let a = self.pop()?;
                let result = self.execute_bitwise_not(a)?;
                self.stack.push(result);
            }

            // ==================== LOOM LAYOUT ====================

            Opcode::render_loom => {
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&"[VM] render_loom: Starting".into());

                // Pop vt capsule containing Toka Tree layout node
                let vsf_capsule = match self.stack.pop() {
                    Some(vsf @ VsfType::v(encoding, _)) => {
                        if encoding == b't' {
                            #[cfg(target_arch = "wasm32")]
                            web_sys::console::log_1(&"[VM] render_loom: Got vt capsule".into());
                            vsf
                        } else {
                            return Err(format!(
                                "render_loom expects vt capsule (Toka Tree), got encoding: {}",
                                encoding as char
                            ));
                        }
                    }
                    Some(other) => {
                        return Err(format!(
                            "render_loom expects vt capsule, got: {:?}",
                            type_name(&other)
                        ))
                    }
                    None => return Err("render_loom: stack underflow".to_string()),
                };

                // Parse vt wrapped Toka Tree node
                let vsf_node = vsf::decoding::toka_tree::parse_vt_toka_node(&vsf_capsule)
                    .map_err(|e| format!("Failed to parse vt Toka Tree capsule: {}", e))?;

                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("[VM] render_loom: Parsed node type").into());

                // Convert VSF TokaNode to Toka LayoutNode
                use crate::loom::LayoutNode;
                let layout_node = LayoutNode::from_vsf_node(&vsf_node);

                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&"[VM] render_loom: Converted to LayoutNode".into());

                // Render to canvas using root viewport bounds
                let root_bounds = crate::loom::LayoutBounds {
                    pos: spirix::CircleF4E4::from((
                        spirix::ScalarF4E4::ZERO,
                        spirix::ScalarF4E4::ZERO,
                    )),
                    size: spirix::CircleF4E4::from((
                        spirix::ScalarF4E4::ONE,
                        spirix::ScalarF4E4::ONE,
                    )),
                };

                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&"[VM] render_loom: Calling render()".into());

                layout_node.render(&mut self.canvas, &root_bounds);

                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&"[VM] render_loom: Complete".into());
            }

            Opcode::loom_box => {
                // TODO: Create box node and push to stack
                // Needs VSF tTb type variant
                return Err("loom_box not yet implemented (needs VSF tTb type)".to_string());
            }

            Opcode::loom_circle => {
                // TODO: Create circle node and push to stack
                // Needs VSF tTc type variant
                return Err("loom_circle not yet implemented (needs VSF tTc type)".to_string());
            }

            Opcode::loom_text => {
                // TODO: Create text node and push to stack
                // Needs VSF tTt type variant
                return Err("loom_text not yet implemented (needs VSF tTt type)".to_string());
            }

            Opcode::loom_button => {
                // TODO: Create button node and push to stack
                // Needs VSF tTu type variant
                return Err("loom_button not yet implemented (needs VSF tTu type)".to_string());
            }

            Opcode::loom_group => {
                // TODO: Create group node and push to stack
                // Needs VSF tTg type variant
                return Err("loom_group not yet implemented (needs VSF tTg type)".to_string());
            }

            Opcode::loom_line => {
                // TODO: Create line node and push to stack
                // Needs VSF tTl type variant
                return Err("loom_line not yet implemented (needs VSF tTl type)".to_string());
            }

            Opcode::loom_path => {
                // TODO: Create path node and push to stack
                // Needs VSF tTp type variant
                return Err("loom_path not yet implemented (needs VSF tTp type)".to_string());
            }

            Opcode::loom_image => {
                // TODO: Create image node and push to stack
                // Needs VSF tTi type variant
                return Err("loom_image not yet implemented (needs VSF tTi type)".to_string());
            }

            Opcode::loom_surface => {
                // TODO: Create surface node and push to stack
                // Needs VSF tTs type variant
                return Err("loom_surface not yet implemented (needs VSF tTs type)".to_string());
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
        let result = match (lhs, rhs) {
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
            _ => false,
        };
        Ok(VsfType::s44(if result {
            ScalarF4E4::ONE
        } else {
            ScalarF4E4::ZERO
        }))
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
        // Return 1 for true, 0 for false (comparison results are booleans)
        Ok(VsfType::s44(if result {
            ScalarF4E4::ONE
        } else {
            ScalarF4E4::ZERO
        }))
    }

    fn execute_bitwise_and(&self, lhs: VsfType, rhs: VsfType) -> Result<VsfType, String> {
        match (lhs, rhs) {
            (VsfType::s44(a), VsfType::s44(b)) => Ok(VsfType::s44(a & b)),
            (a, b) => Err(format!(
                "Type mismatch in bitwise AND: {} & {}",
                type_name(&a),
                type_name(&b)
            )),
        }
    }

    fn execute_bitwise_or(&self, lhs: VsfType, rhs: VsfType) -> Result<VsfType, String> {
        match (lhs, rhs) {
            (VsfType::s44(a), VsfType::s44(b)) => Ok(VsfType::s44(a | b)),
            (a, b) => Err(format!(
                "Type mismatch in bitwise OR: {} | {}",
                type_name(&a),
                type_name(&b)
            )),
        }
    }

    fn execute_bitwise_xor(&self, lhs: VsfType, rhs: VsfType) -> Result<VsfType, String> {
        match (lhs, rhs) {
            (VsfType::s44(a), VsfType::s44(b)) => Ok(VsfType::s44(a ^ b)),
            (a, b) => Err(format!(
                "Type mismatch in bitwise XOR: {} ^ {}",
                type_name(&a),
                type_name(&b)
            )),
        }
    }

    fn execute_bitwise_not(&self, val: VsfType) -> Result<VsfType, String> {
        match val {
            VsfType::s44(a) => Ok(VsfType::s44(!a)),
            other => Err(format!("Cannot bitwise NOT type: {}", type_name(&other))),
        }
    }

    /// Check if a value is "truthy" for logical operations
    /// Numbers: non-zero = true, zero = false
    fn is_truthy(&self, val: &VsfType) -> bool {
        match val {
            VsfType::s44(v) => !v.is_zero(),
            VsfType::u3(v) => *v != 0,
            VsfType::u4(v) => *v != 0,
            VsfType::u5(v) => *v != 0,
            VsfType::u6(v) => *v != 0,
            VsfType::u7(v) => *v != 0,
            VsfType::i3(v) => *v != 0,
            VsfType::i4(v) => *v != 0,
            VsfType::i5(v) => *v != 0,
            VsfType::i6(v) => *v != 0,
            VsfType::i7(v) => *v != 0,
            _ => true, // Other types (strings, arrays, etc) default to true
        }
    }

    fn pop_s44(&mut self) -> Result<ScalarF4E4, String> {
        match self.pop()? {
            VsfType::s44(s) => Ok(s),
            other => Err(format!("Expected s44, got {}", type_name(&other))),
        }
    }

    fn pop_c44(&mut self) -> Result<CircleF4E4, String> {
        match self.pop()? {
            VsfType::c44(c) => Ok(c),
            other => Err(format!("Expected c44, got {}", type_name(&other))),
        }
    }

    #[allow(dead_code)]
    fn push_c44(&mut self, circle: CircleF4E4) {
        self.stack.push(VsfType::c44(circle));
    }

    /// Convert VsfType to F4E4 RGBA components
    /// Handles u5 packed colours, VSF colour constants, and other integer types
    #[allow(dead_code)]
    fn vsf_to_rgba(
        &self,
        v: VsfType,
    ) -> Result<(ScalarF4E4, ScalarF4E4, ScalarF4E4, ScalarF4E4), String> {
        let u32_val = match v {
            VsfType::u5(val) => val,
            VsfType::u3(val) => val as u32,
            VsfType::u4(val) => val as u32,
            VsfType::rck => 0xFF000000, // Black
            VsfType::rcw => 0xFFFFFFFF, // White
            VsfType::rcr => 0xFFFF0000, // Red
            VsfType::rcn => 0xFF00FF00, // Green
            VsfType::rcb => 0xFF0000FF, // Blue
            VsfType::rcc => 0xFF00FFFF, // Cyan
            VsfType::rcj => 0xFFFF00FF, // Magenta
            VsfType::rcy => 0xFFFFFF00, // Yellow
            VsfType::rcg => 0xFF808080, // Gray
            VsfType::rco => 0xFFFF8000, // Orange
            VsfType::rcv => 0xFF8000FF, // Violet
            VsfType::rcl => 0xFF00FF00, // Lime (duplicate of green)
            VsfType::rcq => 0xFF00FFFF, // Aqua (duplicate of cyan)
            other => {
                return Err(format!(
                    "Cannot convert {} to RGBA colour",
                    type_name(&other)
                ))
            }
        };

        // Unpack AARRGGBB to F4E4 RGBA components (0.0-1.0) using Spirix
        let a = ScalarF4E4::from((u32_val >> 24) as u8) >> 8;
        let r = ScalarF4E4::from((u32_val >> 16) as u8) >> 8;
        let g = ScalarF4E4::from((u32_val >> 8) as u8) >> 8;
        let b = ScalarF4E4::from(u32_val as u8) >> 8;

        Ok((r, g, b, a))
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
