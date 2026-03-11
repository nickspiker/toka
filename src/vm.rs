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
use fontdue::Font as FontdueFont;
use spirix::{CircleF4E4, ScalarF4E4};
use std::collections::HashMap;

/// Cache of parsed fonts keyed by BLAKE3 hash of the font file bytes
pub type FontCache = HashMap<[u8; 32], FontdueFont>;
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
/// Cursor type for hit regions — tells the host what CSS cursor to show
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorKind {
    /// Default arrow cursor
    Default,
    /// Pointer (hand) cursor — for clickable elements
    Pointer,
    /// Text (I-beam) cursor — for editable text fields
    Text,
}

/// Hit region registered by an interactive widget during drawing
#[derive(Debug, Clone)]
pub struct HitRegion {
    /// Left edge in RU
    pub x: ScalarF4E4,
    /// Top edge in RU
    pub y: ScalarF4E4,
    /// Width in RU
    pub w: ScalarF4E4,
    /// Height in RU
    pub h: ScalarF4E4,
    /// Widget ID for event routing
    pub widget_id: u32,
    /// Cursor to show when hovering this region
    pub cursor: CursorKind,
}

/// Input event from the host (JS runtime)
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Mouse button pressed at position (RU coordinates)
    MouseDown {
        /// X position in RU
        x: ScalarF4E4,
        /// Y position in RU
        y: ScalarF4E4,
    },
    /// Mouse button released at position (RU coordinates)
    MouseUp {
        /// X position in RU
        x: ScalarF4E4,
        /// Y position in RU
        y: ScalarF4E4,
    },
    /// Text input (printable characters)
    KeyPress {
        /// Characters typed
        text: String,
    },
    /// Non-character key (Backspace, Delete, Arrow keys, etc.)
    KeyDown {
        /// Key name (e.g. "Backspace", "ArrowLeft")
        key: String,
    },
}

/// Persistent state for a text input widget (survives across frames)
#[derive(Debug, Clone)]
pub struct TextInputState {
    /// Character buffer
    pub chars: Vec<char>,
    /// Cursor position (character index)
    pub cursor_pos: usize,
    /// Selection start (if any)
    pub selection_anchor: Option<usize>,
    /// Horizontal scroll offset in RU
    pub scroll_offset: ScalarF4E4,
}

impl TextInputState {
    fn new() -> Self {
        Self {
            chars: Vec::new(),
            cursor_pos: 0,
            selection_anchor: None,
            scroll_offset: ScalarF4E4::ZERO,
        }
    }

    /// Get current text content as a String
    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }
}

/// Toka virtual machine — stack-based bytecode executor with canvas rendering
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

    /// Cache of parsed fonts keyed by BLAKE3 hash of font file bytes
    font_cache: FontCache,

    /// Execution trace for debugging (opcode names)
    trace: Vec<String>,

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

    // ── Interactive widget state (persists across reset) ──

    /// Input event queue from host — consumed during execution, cleared after
    events: Vec<InputEvent>,

    /// Hit regions registered by widgets this frame — cleared on reset
    hit_regions: Vec<HitRegion>,

    /// Persistent text input state keyed by widget ID
    text_inputs: HashMap<u32, TextInputState>,

    /// Which widget has focus (receives keyboard events)
    focused_widget: Option<u32>,

    /// Mouse button state (true = currently pressed)
    mouse_down: bool,

    /// Action URLs triggered by button clicks this frame
    actions: Vec<String>,
}

/// Cell content types for table rendering (text, buttons, text inputs, sub-tables)
enum CellContent {
    Text(String),
    Styled(String, VsfType, Option<ScalarF4E4>),
    Button {
        label: String,
        colour: VsfType,
        id: u32,
        action_url: Option<String>,
    },
    TextInput {
        placeholder: String,
        colour: VsfType,
        id: u32,
    },
    SubTable {
        cols: usize,
        rows: usize,
        cells: Vec<CellContent>,
        settings: crate::drawing::shared::TableSettings,
    },
}

/// Result from render_table: total height + widget results to push on stack
struct RenderTableResult {
    total_height: ScalarF4E4,
    widget_results: Vec<VsfType>,
    row_heights: Vec<ScalarF4E4>,
    col_widths: Vec<ScalarF4E4>,
    col_lefts: Vec<ScalarF4E4>,
    row_tops: Vec<ScalarF4E4>,
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
            font_cache: HashMap::new(),
            trace: Vec::new(),
            scroll_x: ScalarF4E4::ZERO,
            scroll_y: ScalarF4E4::ZERO,
            mouse_x: ScalarF4E4::ZERO,
            mouse_y: ScalarF4E4::ZERO,
            time: ScalarF4E4::ZERO,
            events: Vec::new(),
            hit_regions: Vec::new(),
            text_inputs: HashMap::new(),
            focused_widget: None,
            mouse_down: false,
            actions: Vec::new(),
        }
    }

    /// Replace bytecode (for rerun with new code)
    pub fn set_bytecode(&mut self, bytecode: Vec<u8>) {
        self.bytecode = bytecode;
    }

    /// Reset VM state to re-execute bytecode from the beginning
    ///
    /// Clears stack, resets instruction pointer, and clears halt flag.
    /// Preserves context variables (scroll, mouse, time) for reactive re-execution.
    /// Preserves widget state (text inputs, focus) and events for interactive continuity.
    pub fn reset(&mut self) {
        self.ip = 0;
        self.halted = false;
        self.stack.clear();
        self.hit_regions.clear(); // Rebuilt each frame
        self.actions.clear(); // Rebuilt each frame
        // events, text_inputs, focused_widget, mouse_down persist
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
                    _ => {
                        return Err(format!(
                            "build_row: expected ron for children, got {:?}",
                            type_name(&children_vsf)
                        ))
                    }
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
                    _ => {
                        return Err(format!(
                            "build_rob: expected ron for children, got {:?}",
                            type_name(&children_vsf)
                        ))
                    }
                };

                // Build simple solid fill from colour
                let fill = vsf::types::Fill::Solid(Box::new(fill_vsf));

                self.stack
                    .push(VsfType::rob(pos, size, fill, None, children));
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
            Opcode::draw_text => {
                // Stack (bottom→top): font_bytes, pos (c44), size (s44), text, colour [, settings tags]
                // Settings tags (parsed from top of stack, same pattern as draw_line):
                //   l("l")       = left-align (flag)
                //   l("r")       = right-align (flag)
                //   l("e") + s44 = leading (line height multiplier)
                //   l("k") + s44 = kerning (letter spacing in RU)
                //   l("w") + s44 = weight (variable font axis, 100-900)
                //   l("i") + s44 = tilt (italic angle in degrees)
                //   l("x") + s44 = wrap width in RU
                // Legacy: u3 align value (0=center, 1=left, 2=right) still accepted
                use crate::drawing::TextSettings;

                let mut settings = TextSettings::default();
                let mut next = self.pop()?;

                // Parse optional modifier tags from top of stack
                loop {
                    match &next {
                        VsfType::l(tag) => match tag.as_str() {
                            "l" => {
                                settings.align = 1;
                                next = self.pop()?;
                            }
                            "r" => {
                                settings.align = 2;
                                next = self.pop()?;
                            }
                            "e" => {
                                match self.pop()? {
                                    VsfType::s44(v) => settings.leading = v,
                                    other => {
                                        return Err(format!(
                                            "draw_text: 'e' tag expected s44, got {:?}",
                                            other
                                        ))
                                    }
                                }
                                next = self.pop()?;
                            }
                            "k" => {
                                match self.pop()? {
                                    VsfType::s44(v) => settings.kerning = v,
                                    other => {
                                        return Err(format!(
                                            "draw_text: 'k' tag expected s44, got {:?}",
                                            other
                                        ))
                                    }
                                }
                                next = self.pop()?;
                            }
                            "w" => {
                                match self.pop()? {
                                    VsfType::s44(v) => settings.weight = Some(v),
                                    other => {
                                        return Err(format!(
                                            "draw_text: 'w' tag expected s44, got {:?}",
                                            other
                                        ))
                                    }
                                }
                                next = self.pop()?;
                            }
                            "i" => {
                                match self.pop()? {
                                    VsfType::s44(v) => settings.tilt = Some(v),
                                    other => {
                                        return Err(format!(
                                            "draw_text: 'i' tag expected s44, got {:?}",
                                            other
                                        ))
                                    }
                                }
                                next = self.pop()?;
                            }
                            "x" => {
                                match self.pop()? {
                                    VsfType::s44(v) => settings.wrap = Some(v),
                                    other => {
                                        return Err(format!(
                                            "draw_text: 'x' tag expected s44, got {:?}",
                                            other
                                        ))
                                    }
                                }
                                next = self.pop()?;
                            }
                            _ => break,
                        },
                        VsfType::u3(a) => {
                            // Legacy: u3 alignment value
                            settings.align = *a;
                            next = self.pop()?;
                        }
                        _ => break,
                    }
                }
                let colour = next;

                let text = match self.pop()? {
                    VsfType::x(s) | VsfType::l(s) => s,
                    other => {
                        return Err(format!(
                            "draw_text: expected string for text, got {:?}",
                            other
                        ))
                    }
                };
                let size = match self.pop()? {
                    VsfType::s44(s) => s,
                    other => {
                        return Err(format!("draw_text: expected s44 for size, got {:?}", other))
                    }
                };
                let pos = match self.pop()? {
                    VsfType::c44(c) => c,
                    other => {
                        return Err(format!("draw_text: expected c44 for pos, got {:?}", other))
                    }
                };
                let font_bytes = match self.pop()? {
                    VsfType::v(b'b', bytes) => bytes,
                    other => {
                        return Err(format!(
                            "draw_text: expected binary blob for font, got {:?}",
                            other
                        ))
                    }
                };
                let font_key = *blake3::hash(&font_bytes).as_bytes();
                self.canvas.draw_text(
                    &mut self.font_cache,
                    font_key,
                    &font_bytes,
                    pos,
                    size,
                    &text,
                    &colour,
                    &settings,
                )?;
            }

            Opcode::draw_line => {
                // Stack (bottom→top): start (c44), end (c44), colour [, settings tags]
                // Settings tags are parsed from top of stack before colour:
                //   l("w") + s44 = weight
                //   l("c") + u3  = cap (both endpoints)
                //   l("s") + u3  = start cap override
                //   l("e") + u3  = end cap override
                //   l("p")       = pixel mode
                use crate::drawing::shared::Cap;
                use crate::drawing::LineSettings;

                let mut settings = LineSettings::default();
                let mut next = self.pop()?;

                // Parse optional modifier tags from top of stack
                loop {
                    match &next {
                        VsfType::l(tag) => match tag.as_str() {
                            "w" => {
                                match self.pop()? {
                                    VsfType::s44(w) => settings.weight = Some(w),
                                    other => {
                                        return Err(format!(
                                            "draw_line: 'w' tag expected s44, got {:?}",
                                            other
                                        ))
                                    }
                                }
                                next = self.pop()?;
                            }
                            "c" => {
                                // Set both caps at once
                                match self.pop()? {
                                    VsfType::u3(c) => {
                                        let cap = Cap::from_u8(c);
                                        settings.cap_start = cap;
                                        settings.cap_end = cap;
                                    }
                                    other => {
                                        return Err(format!(
                                            "draw_line: 'c' tag expected u3, got {:?}",
                                            other
                                        ))
                                    }
                                }
                                next = self.pop()?;
                            }
                            "s" => {
                                match self.pop()? {
                                    VsfType::u3(c) => settings.cap_start = Cap::from_u8(c),
                                    other => {
                                        return Err(format!(
                                            "draw_line: 's' tag expected u3, got {:?}",
                                            other
                                        ))
                                    }
                                }
                                next = self.pop()?;
                            }
                            "e" => {
                                match self.pop()? {
                                    VsfType::u3(c) => settings.cap_end = Cap::from_u8(c),
                                    other => {
                                        return Err(format!(
                                            "draw_line: 'e' tag expected u3, got {:?}",
                                            other
                                        ))
                                    }
                                }
                                next = self.pop()?;
                            }
                            "p" => {
                                settings.pixel = true;
                                next = self.pop()?;
                            }
                            _ => break,
                        },
                        _ => break,
                    }
                }

                let colour = next;
                let end = match self.pop()? {
                    VsfType::c44(c) => c,
                    other => {
                        return Err(format!(
                            "draw_line: expected c44 for end point, got {:?}",
                            other
                        ))
                    }
                };
                let start = match self.pop()? {
                    VsfType::c44(c) => c,
                    other => {
                        return Err(format!(
                            "draw_line: expected c44 for start point, got {:?}",
                            other
                        ))
                    }
                };

                self.canvas.draw_line(start, end, &colour, &settings)?;
            }

            Opcode::draw_table => {
                // Same base stack as draw_text: font_bytes, pos(c44), size(s44), colour
                // All table-specific data via tags:
                //   l("c") + u   = column count
                //   l("r") + u   = row count
                //   l("d")       = cell data marker (next cols*rows stack values are cell strings)
                //   l("w") + s44 = table width in RU
                //   l("h") + col = header row background colour
                //   l("b") + col = border/grid colour
                //   l("a") + col = alternating row background colour
                //   l("p") + s44 = cell padding in RU
                use crate::drawing::TableSettings;

                let mut settings = TableSettings::default();
                let mut cols: usize = 0;
                let mut rows: usize = 0;
                let mut cells: Vec<CellContent> = Vec::new();
                let mut query_cells: Vec<(usize, usize)> = Vec::new(); // (row, col) pairs for geometry push
                let mut next = self.pop()?;

                loop {
                    match &next {
                        VsfType::l(tag) => match tag.as_str() {
                            "c" => {
                                cols = match self.pop()? {
                                    VsfType::u(n, _) => n,
                                    VsfType::u3(n) => n as usize,
                                    other => {
                                        return Err(format!(
                                            "draw_table: 'c' tag expected u, got {:?}",
                                            other
                                        ))
                                    }
                                };
                                next = self.pop()?;
                            }
                            "r" => {
                                rows = match self.pop()? {
                                    VsfType::u(n, _) => n,
                                    VsfType::u3(n) => n as usize,
                                    other => {
                                        return Err(format!(
                                            "draw_table: 'r' tag expected u, got {:?}",
                                            other
                                        ))
                                    }
                                };
                                next = self.pop()?;
                            }
                            "d" => {
                                // Pop cols*rows cell values from stack
                                // String → Text cell
                                // rou(_, _, label, variant, colour) → Button: pop action_url, id
                                // roq(_, _, placeholder, colour) → TextInput: pop id
                                let count = cols * rows;
                                if count == 0 {
                                    return Err(
                                        "draw_table: 'd' tag requires 'c' and 'r' tags first"
                                            .to_string(),
                                    );
                                }
                                cells = Vec::with_capacity(count);
                                for _ in 0..count {
                                    let cell = match self.pop()? {
                                        VsfType::x(s) | VsfType::l(s) => CellContent::Text(s),
                                        colour @ VsfType::ra(_) => {
                                            // Styled text: colour on top, then optional s44 size, then text
                                            let next = self.pop()?;
                                            let (size_override, text) = match next {
                                                VsfType::s44(s) => {
                                                    let t = match self.pop()? {
                                                        VsfType::x(s) | VsfType::l(s) => s,
                                                        other => return Err(format!(
                                                            "draw_table: styled cell expected string after size, got {:?}", other
                                                        )),
                                                    };
                                                    (Some(s), t)
                                                }
                                                VsfType::x(s) | VsfType::l(s) => (None, s),
                                                other => return Err(format!(
                                                    "draw_table: styled cell expected string or s44, got {:?}", other
                                                )),
                                            };
                                            CellContent::Styled(text, colour, size_override)
                                        }
                                        VsfType::rou(_pos, _size, label, _variant, colour) => {
                                            // Button cell: drawable carries label + colour
                                            // Pop action_url, then id from stack
                                            let action_url = match self.pop()? {
                                                VsfType::x(s) | VsfType::l(s) => {
                                                    if s.is_empty() { None } else { Some(s) }
                                                }
                                                other => return Err(format!(
                                                    "draw_table: button cell expected string for action_url, got {:?}", other
                                                )),
                                            };
                                            let id = match self.pop()? {
                                                VsfType::u(n, _) => n as u32,
                                                VsfType::u3(n) => n as u32,
                                                other => return Err(format!(
                                                    "draw_table: button cell expected u for id, got {:?}", other
                                                )),
                                            };
                                            CellContent::Button { label, colour: *colour, id, action_url }
                                        }
                                        VsfType::roq(_pos, _size, placeholder, colour) => {
                                            // TextInput cell: drawable carries placeholder + colour
                                            // Pop id from stack
                                            let id = match self.pop()? {
                                                VsfType::u(n, _) => n as u32,
                                                VsfType::u3(n) => n as u32,
                                                other => return Err(format!(
                                                    "draw_table: text_input cell expected u for id, got {:?}", other
                                                )),
                                            };
                                            CellContent::TextInput { placeholder, colour: *colour, id }
                                        }
                                        VsfType::roa(sub_cols, sub_rows, children) => {
                                            // SubTable cell: one child per cell, then settings tags
                                            let cell_count = sub_cols * sub_rows;
                                            let mut sub_cells = Vec::with_capacity(cell_count);
                                            let mut sub_settings = TableSettings::default();
                                            let mut ci = 0;
                                            // First cell_count children are cell data (1 entry per cell)
                                            while ci < children.len() && sub_cells.len() < cell_count {
                                                match &children[ci] {
                                                    VsfType::x(s) | VsfType::l(s) => {
                                                        sub_cells.push(CellContent::Text(s.clone()));
                                                    }
                                                    VsfType::rou(_, _, label, _variant, colour) => {
                                                        // Display-only button (no id/action in sub-tables)
                                                        sub_cells.push(CellContent::Button {
                                                            label: label.clone(),
                                                            colour: *colour.clone(),
                                                            id: 0,
                                                            action_url: None,
                                                        });
                                                    }
                                                    VsfType::roq(_, _, placeholder, colour) => {
                                                        // Display-only text input (no id in sub-tables)
                                                        sub_cells.push(CellContent::TextInput {
                                                            placeholder: placeholder.clone(),
                                                            colour: *colour.clone(),
                                                            id: 0,
                                                        });
                                                    }
                                                    VsfType::roa(..) => {
                                                        // Nested sub-table placeholder (depth limit TODO)
                                                        sub_cells.push(CellContent::Text("[sub-table]".to_string()));
                                                    }
                                                    other => {
                                                        sub_cells.push(CellContent::Text(format!("{}", other)));
                                                    }
                                                }
                                                ci += 1;
                                            }
                                            // Remaining children are settings tags
                                            while ci < children.len() {
                                                if let VsfType::l(tag) = &children[ci] {
                                                    ci += 1;
                                                    match tag.as_str() {
                                                        "x" => {
                                                            let mut widths = Vec::new();
                                                            while ci < children.len() {
                                                                if let VsfType::s44(v) = &children[ci] {
                                                                    widths.push(*v);
                                                                    ci += 1;
                                                                } else { break; }
                                                            }
                                                            if !widths.is_empty() {
                                                                sub_settings.col_widths = Some(widths);
                                                            }
                                                        }
                                                        "j" => {
                                                            if ci < children.len() {
                                                                if let VsfType::x(s) | VsfType::l(s) = &children[ci] {
                                                                    sub_settings.h_align = Some(s.bytes().collect());
                                                                    ci += 1;
                                                                }
                                                            }
                                                        }
                                                        "b" => {
                                                            if ci < children.len() {
                                                                sub_settings.border_colour = Some(children[ci].clone());
                                                                ci += 1;
                                                            }
                                                            if ci < children.len() {
                                                                if let VsfType::v(_, mask_data) = &children[ci] {
                                                                    // Parse grid mask inline
                                                                    if !mask_data.is_empty() {
                                                                        let flags = mask_data[0];
                                                                        let has_h = flags & 1 != 0;
                                                                        let has_v = flags & 2 != 0;
                                                                        let h_bits_count = if has_h { (sub_rows + 1) * sub_cols } else { 0 };
                                                                        let h_bytes = (h_bits_count + 7) / 8;
                                                                        let v_bits_count = if has_v { sub_rows * (sub_cols + 1) } else { 0 };
                                                                        let v_bytes = (v_bits_count + 7) / 8;
                                                                        let h_start = 1;
                                                                        let v_start = h_start + h_bytes;
                                                                        let h_bits = if has_h && h_start + h_bytes <= mask_data.len() {
                                                                            mask_data[h_start..h_start + h_bytes].to_vec()
                                                                        } else { vec![] };
                                                                        let v_bits = if has_v && v_start + v_bytes <= mask_data.len() {
                                                                            mask_data[v_start..v_start + v_bytes].to_vec()
                                                                        } else { vec![] };
                                                                        sub_settings.grid_mask = Some(crate::drawing::shared::GridMask {
                                                                            h_bits, v_bits, has_h, has_v,
                                                                        });
                                                                    }
                                                                    ci += 1;
                                                                }
                                                            }
                                                        }
                                                        "h" => {
                                                            if ci < children.len() {
                                                                sub_settings.header_bg = Some(children[ci].clone());
                                                                ci += 1;
                                                            }
                                                        }
                                                        "a" => {
                                                            if ci < children.len() {
                                                                sub_settings.alt_row_bg = Some(children[ci].clone());
                                                                ci += 1;
                                                            }
                                                        }
                                                        "p" => {
                                                            if ci < children.len() {
                                                                if let VsfType::s44(v) = &children[ci] {
                                                                    sub_settings.padding = *v;
                                                                    ci += 1;
                                                                }
                                                            }
                                                        }
                                                        _ => {}
                                                    }
                                                } else {
                                                    ci += 1;
                                                }
                                            }
                                            CellContent::SubTable {
                                                cols: sub_cols,
                                                rows: sub_rows,
                                                cells: sub_cells,
                                                settings: sub_settings,
                                            }
                                        }
                                        other => return Err(format!(
                                            "draw_table: expected string, rou, roq, or roa for cell, got {:?}",
                                            other
                                        )),
                                    };
                                    cells.push(cell);
                                }
                                cells.reverse(); // LIFO → row-major order
                                next = self.pop()?;
                            }
                            "x" => {
                                if cols == 0 {
                                    return Err("draw_table: 'x' tag requires 'c' tag first".to_string());
                                }
                                let mut widths = Vec::with_capacity(cols);
                                for _ in 0..cols {
                                    match self.pop()? {
                                        VsfType::s44(v) => widths.push(v),
                                        other => {
                                            return Err(format!(
                                                "draw_table: 'x' tag expected s44, got {:?}",
                                                other
                                            ))
                                        }
                                    }
                                }
                                widths.reverse(); // LIFO → column order
                                settings.col_widths = Some(widths);
                                next = self.pop()?;
                            }
                            "y" => {
                                match self.pop()? {
                                    VsfType::s44(v) => settings.row_height = Some(v),
                                    other => {
                                        return Err(format!(
                                            "draw_table: 'y' tag expected s44, got {:?}",
                                            other
                                        ))
                                    }
                                }
                                next = self.pop()?;
                            }
                            "h" => {
                                settings.header_bg = Some(self.pop()?);
                                next = self.pop()?;
                            }
                            "b" => {
                                settings.border_colour = Some(self.pop()?);
                                next = self.pop()?;
                            }
                            "a" => {
                                settings.alt_row_bg = Some(self.pop()?);
                                next = self.pop()?;
                            }
                            "p" => {
                                match self.pop()? {
                                    VsfType::s44(v) => settings.padding = v,
                                    other => {
                                        return Err(format!(
                                            "draw_table: 'p' tag expected s44, got {:?}",
                                            other
                                        ))
                                    }
                                }
                                next = self.pop()?;
                            }
                            "g" => {
                                // Bitpacked grid mask: byte 0 = flags, then h_bits, then v_bits
                                let raw = match self.pop()? {
                                    VsfType::v(_, data) => data,
                                    other => {
                                        return Err(format!(
                                            "draw_table: 'g' tag expected bytes, got {:?}",
                                            other
                                        ))
                                    }
                                };
                                if !raw.is_empty() {
                                    let flags = raw[0];
                                    let has_h = flags & 1 != 0;
                                    let has_v = flags & 2 != 0;
                                    let h_bits_count = if has_h { (rows + 1) * cols } else { 0 };
                                    let h_bytes = (h_bits_count + 7) / 8;
                                    let v_bits_count = if has_v { rows * (cols + 1) } else { 0 };
                                    let v_bytes = (v_bits_count + 7) / 8;
                                    let h_start = 1;
                                    let v_start = h_start + h_bytes;
                                    let h_bits = if has_h && h_start + h_bytes <= raw.len() {
                                        raw[h_start..h_start + h_bytes].to_vec()
                                    } else {
                                        vec![]
                                    };
                                    let v_bits = if has_v && v_start + v_bytes <= raw.len() {
                                        raw[v_start..v_start + v_bytes].to_vec()
                                    } else {
                                        vec![]
                                    };
                                    settings.grid_mask = Some(crate::drawing::shared::GridMask {
                                        h_bits,
                                        v_bits,
                                        has_h,
                                        has_v,
                                    });
                                }
                                next = self.pop()?;
                            }
                            "j" => {
                                match self.pop()? {
                                    VsfType::x(s) | VsfType::l(s) => settings.h_align = Some(s.into_bytes()),
                                    other => {
                                        return Err(format!(
                                            "draw_table: 'j' tag expected string, got {:?}",
                                            other
                                        ))
                                    }
                                }
                                next = self.pop()?;
                            }
                            "v" => {
                                match self.pop()? {
                                    VsfType::x(s) | VsfType::l(s) => settings.v_align = Some(s.into_bytes()),
                                    other => {
                                        return Err(format!(
                                            "draw_table: 'v' tag expected string, got {:?}",
                                            other
                                        ))
                                    }
                                }
                                next = self.pop()?;
                            }
                            "q" => {
                                // Query cells: pop count, then (row, col) pairs
                                // After table draws, pushes geometry for these cells
                                let count = match self.pop()? {
                                    VsfType::u(n, _) => n,
                                    VsfType::u3(n) => n as usize,
                                    other => {
                                        return Err(format!(
                                            "draw_table: 'q' tag expected u for count, got {:?}",
                                            other
                                        ))
                                    }
                                };
                                for _ in 0..count {
                                    // Builder pushes row then col, so col is on top (LIFO)
                                    let col = match self.pop()? {
                                        VsfType::u(n, _) => n,
                                        VsfType::u3(n) => n as usize,
                                        other => {
                                            return Err(format!(
                                                "draw_table: 'q' cell expected u for col, got {:?}",
                                                other
                                            ))
                                        }
                                    };
                                    let row = match self.pop()? {
                                        VsfType::u(n, _) => n,
                                        VsfType::u3(n) => n as usize,
                                        other => {
                                            return Err(format!(
                                                "draw_table: 'q' cell expected u for row, got {:?}",
                                                other
                                            ))
                                        }
                                    };
                                    query_cells.push((row, col));
                                }
                                next = self.pop()?;
                            }
                            _ => break,
                        },
                        _ => break,
                    }
                }

                if cols == 0 || rows == 0 || cells.is_empty() {
                    return Err("draw_table: requires 'c', 'r', and 'd' tags".to_string());
                }

                let text_colour = next;
                let size = match self.pop()? {
                    VsfType::s44(s) => s,
                    other => {
                        return Err(format!(
                            "draw_table: expected s44 for size, got {:?}",
                            other
                        ))
                    }
                };
                let pos = match self.pop()? {
                    VsfType::c44(c) => c,
                    other => {
                        return Err(format!("draw_table: expected c44 for pos, got {:?}", other))
                    }
                };
                let font_bytes = match self.pop()? {
                    VsfType::v(b'b', bytes) => bytes,
                    other => {
                        return Err(format!(
                            "draw_table: expected binary blob for font, got {:?}",
                            other
                        ))
                    }
                };
                let font_key = *blake3::hash(&font_bytes).as_bytes();

                let result = self.render_table(
                    &cells, cols, rows, &settings, pos,
                    font_key, &font_bytes, size, &text_colour,
                    &query_cells, 0, false,
                )?;

                // Push widget results onto stack (in order of encounter)
                for r in result.widget_results {
                    self.stack.push(r);
                }

                // Push geometry for queried cells (reverse order → first cell on top)
                for &(row, col) in query_cells.iter().rev() {
                    if row >= rows || col >= cols { continue; }
                    let rh = result.row_heights[row];
                    let cell_center = CircleF4E4::from((
                        result.col_lefts[col] + (result.col_widths[col] >> 1usize),
                        result.row_tops[row] + (rh >> 1usize),
                    ));
                    let cell_size = CircleF4E4::from((
                        result.col_widths[col],
                        rh,
                    ));
                    self.stack.push(VsfType::v(b'b', font_bytes.clone()));
                    self.stack.push(VsfType::c44(cell_center));
                    self.stack.push(VsfType::c44(cell_size));
                    self.stack.push(text_colour.clone());
                }

                // Always push total table height last (top of stack)
                self.stack.push(VsfType::s44(result.total_height));
            }

            Opcode::clear_canvas => {
                // Pop VSF colour type (rc*, ra, or rw)
                let colour = self.pop()?;
                self.canvas.clear(&colour)?;
            }

            Opcode::render_loom => {
                // Pop scene graph from stack (ro* type)
                let vsf = self
                    .stack
                    .pop()
                    .ok_or_else(|| "render_loom: stack underflow".to_string())?;

                // Render directly from ro* type
                let mut renderer = crate::renderer::RenderContext::new();
                renderer.render(&vsf, &mut self.canvas)?;
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

            Opcode::canvas_w => {
                // Push canvas width in RU: pixels / (span * ru)
                let span_ru = self.canvas.span() * self.canvas.ru();
                let w = ScalarF4E4::from(self.canvas.width()) / span_ru;
                self.stack.push(VsfType::s44(w));
            }

            Opcode::canvas_h => {
                // Push canvas height in RU: pixels / (span * ru)
                let span_ru = self.canvas.span() * self.canvas.ru();
                let h = ScalarF4E4::from(self.canvas.height()) / span_ru;
                self.stack.push(VsfType::s44(h));
            }

            Opcode::aspect_ratio => {
                // Push width / height (dimensionless)
                let ar = ScalarF4E4::from(self.canvas.width()) / ScalarF4E4::from(self.canvas.height());
                self.stack.push(VsfType::s44(ar));
            }

            Opcode::button => {
                // Stack (bottom→top): font, pos(c44), size(c44), label(string), colour, id(u)
                // Draws 1px hairline rect with centered label
                // Pushes: s44(1.0) if clicked this frame, s44(0.0) otherwise

                let widget_id = match self.pop()? {
                    VsfType::u(n, _) => n as u32,
                    VsfType::u3(n) => n as u32,
                    other => return Err(format!("button: expected u for id, got {:?}", other)),
                };
                let colour = self.pop()?;
                let label = match self.pop()? {
                    VsfType::x(s) | VsfType::l(s) => s,
                    other => return Err(format!("button: expected string for label, got {:?}", other)),
                };
                let size = match self.pop()? {
                    VsfType::c44(c) => c,
                    other => return Err(format!("button: expected c44 for size, got {:?}", other)),
                };
                let pos = match self.pop()? {
                    VsfType::c44(c) => c,
                    other => return Err(format!("button: expected c44 for pos, got {:?}", other)),
                };
                let font_bytes = match self.pop()? {
                    VsfType::v(b'b', bytes) => bytes,
                    other => return Err(format!("button: expected binary blob for font, got {:?}", other)),
                };

                // Register hit region (pos is center, convert to top-left)
                let half_w = size.r() >> 1usize;
                let half_h = size.i() >> 1usize;
                self.hit_regions.push(HitRegion {
                    x: pos.r() - half_w,
                    y: pos.i() - half_h,
                    w: size.r(),
                    h: size.i(),
                    widget_id,
                    cursor: CursorKind::Pointer,
                });

                // Draw 1px hairline border (no AA — axis-aligned fast path)
                let left = pos.r() - half_w;
                let right = pos.r() + half_w;
                let top = pos.i() - half_h;
                let bottom = pos.i() + half_h;
                self.canvas.stroke_rect_ru(pos, size, &colour)?;

                // Draw centered label text
                let font_key = *blake3::hash(&font_bytes).as_bytes();
                let text_size = size.i() * ScalarF4E4::from(2) / ScalarF4E4::from(3); // 2/3 of button height
                let text_settings = crate::drawing::TextSettings {
                    align: 0, // center
                    ..Default::default()
                };
                self.canvas.draw_text(
                    &mut self.font_cache,
                    font_key,
                    &font_bytes,
                    pos,
                    text_size,
                    &label,
                    &colour,
                    &text_settings,
                )?;

                // Check if clicked this frame
                let mut clicked = false;
                for event in &self.events {
                    if let InputEvent::MouseDown { x, y } = event {
                        if *x >= left && *x <= right && *y >= top && *y <= bottom {
                            clicked = true;
                        }
                    }
                }
                self.stack.push(VsfType::s44(if clicked { ScalarF4E4::ONE } else { ScalarF4E4::ZERO }));
            }

            Opcode::text_input => {
                // Stack (bottom→top): font, pos(c44), size(c44), placeholder(string), colour, id(u)
                // Draws 1px hairline rect with editable text content
                // Pushes: current text content (string)

                let widget_id = match self.pop()? {
                    VsfType::u(n, _) => n as u32,
                    VsfType::u3(n) => n as u32,
                    other => return Err(format!("text_input: expected u for id, got {:?}", other)),
                };
                let colour = self.pop()?;
                let placeholder = match self.pop()? {
                    VsfType::x(s) | VsfType::l(s) => s,
                    other => return Err(format!("text_input: expected string for placeholder, got {:?}", other)),
                };
                let size = match self.pop()? {
                    VsfType::c44(c) => c,
                    other => return Err(format!("text_input: expected c44 for size, got {:?}", other)),
                };
                let pos = match self.pop()? {
                    VsfType::c44(c) => c,
                    other => return Err(format!("text_input: expected c44 for pos, got {:?}", other)),
                };
                let font_bytes = match self.pop()? {
                    VsfType::v(b'b', bytes) => bytes,
                    other => return Err(format!("text_input: expected binary blob for font, got {:?}", other)),
                };

                // Ensure widget state exists
                if !self.text_inputs.contains_key(&widget_id) {
                    self.text_inputs.insert(widget_id, TextInputState::new());
                }

                // Register hit region
                let half_w = size.r() >> 1usize;
                let half_h = size.i() >> 1usize;
                let left = pos.r() - half_w;
                let right = pos.r() + half_w;
                let top = pos.i() - half_h;
                let bottom = pos.i() + half_h;
                self.hit_regions.push(HitRegion {
                    x: left,
                    y: top,
                    w: size.r(),
                    h: size.i(),
                    widget_id,
                    cursor: CursorKind::Text,
                });

                let is_focused = self.focused_widget == Some(widget_id);

                // Process events for this widget if focused
                if is_focused {
                    let state = self.text_inputs.get_mut(&widget_id).unwrap();
                    for event in &self.events {
                        match event {
                            InputEvent::KeyPress { text } => {
                                // Delete selection first
                                if let Some(anchor) = state.selection_anchor.take() {
                                    let (start, end) = if anchor < state.cursor_pos {
                                        (anchor, state.cursor_pos)
                                    } else {
                                        (state.cursor_pos, anchor)
                                    };
                                    state.chars.drain(start..end);
                                    state.cursor_pos = start;
                                }
                                // Insert characters
                                for ch in text.chars() {
                                    state.chars.insert(state.cursor_pos, ch);
                                    state.cursor_pos += 1;
                                }
                            }
                            InputEvent::KeyDown { key } => {
                                match key.as_str() {
                                    "Backspace" => {
                                        if let Some(anchor) = state.selection_anchor.take() {
                                            let (start, end) = if anchor < state.cursor_pos {
                                                (anchor, state.cursor_pos)
                                            } else {
                                                (state.cursor_pos, anchor)
                                            };
                                            state.chars.drain(start..end);
                                            state.cursor_pos = start;
                                        } else if state.cursor_pos > 0 {
                                            state.cursor_pos -= 1;
                                            state.chars.remove(state.cursor_pos);
                                        }
                                    }
                                    "Delete" => {
                                        if let Some(anchor) = state.selection_anchor.take() {
                                            let (start, end) = if anchor < state.cursor_pos {
                                                (anchor, state.cursor_pos)
                                            } else {
                                                (state.cursor_pos, anchor)
                                            };
                                            state.chars.drain(start..end);
                                            state.cursor_pos = start;
                                        } else if state.cursor_pos < state.chars.len() {
                                            state.chars.remove(state.cursor_pos);
                                        }
                                    }
                                    "ArrowLeft" => {
                                        if state.cursor_pos > 0 {
                                            state.cursor_pos -= 1;
                                        }
                                        state.selection_anchor = None;
                                    }
                                    "ArrowRight" => {
                                        if state.cursor_pos < state.chars.len() {
                                            state.cursor_pos += 1;
                                        }
                                        state.selection_anchor = None;
                                    }
                                    "Home" => {
                                        state.cursor_pos = 0;
                                        state.selection_anchor = None;
                                    }
                                    "End" => {
                                        state.cursor_pos = state.chars.len();
                                        state.selection_anchor = None;
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                }

                // Handle click-to-focus and click-to-position
                for event in &self.events {
                    if let InputEvent::MouseDown { x, y } = event {
                        if *x >= left && *x <= right && *y >= top && *y <= bottom {
                            self.focused_widget = Some(widget_id);
                            // TODO: compute cursor position from click x using font metrics
                            let state = self.text_inputs.get_mut(&widget_id).unwrap();
                            state.selection_anchor = None;
                        }
                    }
                }

                // Draw 1px hairline border (no AA — axis-aligned fast path)
                self.canvas.stroke_rect_ru(pos, size, &colour)?;

                // Draw text content (or placeholder if empty)
                let state = self.text_inputs.get(&widget_id).unwrap();
                let display_text = if state.chars.is_empty() {
                    placeholder.clone()
                } else {
                    state.text()
                };
                let font_key = *blake3::hash(&font_bytes).as_bytes();
                let text_size = size.i() * ScalarF4E4::from(2) / ScalarF4E4::from(3);
                let padding = size.r() / ScalarF4E4::from(40); // small left padding
                let text_pos = CircleF4E4::from((left + padding, pos.i()));

                // Use dimmer colour for placeholder
                let text_colour = if state.chars.is_empty() {
                    // Dim the colour — halve RGB channels
                    // For simplicity, use the colour as-is but with alpha hint
                    VsfType::ra([128, 128, 128, 255])
                } else {
                    colour.clone()
                };

                let text_settings = crate::drawing::TextSettings {
                    align: 1, // left-align
                    ..Default::default()
                };
                self.canvas.draw_text(
                    &mut self.font_cache,
                    font_key,
                    &font_bytes,
                    text_pos,
                    text_size,
                    &display_text,
                    &text_colour,
                    &text_settings,
                )?;

                // Draw cursor if focused
                if is_focused {
                    let state = self.text_inputs.get(&widget_id).unwrap();
                    // Measure text width up to cursor position
                    let cursor_text: String = state.chars[..state.cursor_pos].iter().collect();
                    let font = self.font_cache.get(&font_key);
                    if let Some(font) = font {
                        use fontdue::{Font as FontdueFont, layout::*};
                        let span_ru = self.canvas.span() * self.canvas.ru();
                        let px = text_size * span_ru;
                        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
                        layout.reset(&LayoutSettings::default());
                        layout.append(&[font as &FontdueFont], &TextStyle::new(&cursor_text, px, 0));
                        let cursor_width_px = layout.glyphs().iter()
                            .map(|g| g.x + ScalarF4E4::from(g.width as i32))
                            .fold(ScalarF4E4::ZERO, |a, b| if b > a { b } else { a });
                        let cursor_x = left + padding + cursor_width_px / span_ru;

                        // Draw cursor line (1px vertical, no AA)
                        let cursor_y0 = top + (size.i() / ScalarF4E4::from(6));
                        let cursor_y1 = bottom - (size.i() / ScalarF4E4::from(6));
                        self.canvas.vline_ru(cursor_x, cursor_y0, cursor_y1, &colour)?;
                    }
                }

                // Push current text content
                let state = self.text_inputs.get(&widget_id).unwrap();
                self.stack.push(VsfType::x(state.text()));
            }

            Opcode::action => {
                // Pop URL (string), pop condition (s44/u)
                // If condition is non-zero, queue the URL for JS to execute as POST
                let url = match self.pop()? {
                    VsfType::x(s) | VsfType::l(s) => s,
                    other => return Err(format!("action: expected string for URL, got {:?}", other)),
                };
                let condition = self.pop()?;
                let is_truthy = match &condition {
                    VsfType::s44(s) => *s != ScalarF4E4::ZERO,
                    VsfType::u(n, _) => *n != 0,
                    VsfType::u3(n) => *n != 0,
                    _ => false,
                };
                if is_truthy {
                    self.actions.push(url);
                }
            }

            Opcode::guard => {
                // Pop condition; halt if zero
                let cond = self.pop()?;
                let is_zero = match &cond {
                    VsfType::s44(s) => *s == ScalarF4E4::ZERO,
                    VsfType::u(n, _) => *n == 0,
                    VsfType::l(s) => s.is_empty(),
                    _ => false,
                };
                if is_zero {
                    self.halted = true;
                    return Err("guard failed: condition was zero".to_string());
                }
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

            Opcode::local_alloc => {
                // Immediate: u count — extend current frame with N default slots
                let n = match vsf_parse(&self.bytecode, &mut self.ip)
                    .map_err(|e| format!("local_alloc: {}", e))?
                {
                    VsfType::u3(n) => n as usize,
                    VsfType::u(n, _) => n,
                    other => return Err(format!("local_alloc: expected u, got {:?}", other)),
                };
                let frame = self.locals.last_mut().unwrap();
                frame.resize(frame.len() + n, VsfType::u3(0));
            }

            Opcode::local_get => {
                // Immediate: u index — push locals[index] (clone)
                let idx = match vsf_parse(&self.bytecode, &mut self.ip)
                    .map_err(|e| format!("local_get: {}", e))?
                {
                    VsfType::u3(i) => i as usize,
                    VsfType::u(i, _) => i,
                    other => return Err(format!("local_get: expected u, got {:?}", other)),
                };
                let frame = self.locals.last().unwrap();
                if idx >= frame.len() {
                    return Err(format!(
                        "local_get: index {} out of bounds ({})",
                        idx,
                        frame.len()
                    ));
                }
                self.stack.push(frame[idx].clone());
            }

            Opcode::local_set => {
                // Immediate: u index — pop value from stack, store in locals[index]
                let idx = match vsf_parse(&self.bytecode, &mut self.ip)
                    .map_err(|e| format!("local_set: {}", e))?
                {
                    VsfType::u3(i) => i as usize,
                    VsfType::u(i, _) => i,
                    other => return Err(format!("local_set: expected u, got {:?}", other)),
                };
                let val = self.pop()?;
                let frame = self.locals.last_mut().unwrap();
                if idx >= frame.len() {
                    return Err(format!(
                        "local_set: index {} out of bounds ({})",
                        idx,
                        frame.len()
                    ));
                }
                frame[idx] = val;
            }

            Opcode::local_tee => {
                // Immediate: u index — copy top of stack to locals[index] without popping
                let idx = match vsf_parse(&self.bytecode, &mut self.ip)
                    .map_err(|e| format!("local_tee: {}", e))?
                {
                    VsfType::u3(i) => i as usize,
                    VsfType::u(i, _) => i,
                    other => return Err(format!("local_tee: expected u, got {:?}", other)),
                };
                let val = self.stack.last().ok_or("local_tee: stack empty")?.clone();
                let frame = self.locals.last_mut().unwrap();
                if idx >= frame.len() {
                    return Err(format!(
                        "local_tee: index {} out of bounds ({})",
                        idx,
                        frame.len()
                    ));
                }
                frame[idx] = val;
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

    /// Render a table (or sub-table) with full layout, backgrounds, grid, and cell content.
    /// Returns total height and widget results. Recursive for SubTable cells (depth-limited).
    /// When `measure_only` is true, computes dimensions without drawing anything.
    fn render_table(
        &mut self,
        cells: &[CellContent],
        cols: usize,
        rows: usize,
        settings: &crate::drawing::TableSettings,
        pos: CircleF4E4,
        font_key: [u8; 32],
        font_bytes: &[u8],
        size: ScalarF4E4,
        text_colour: &VsfType,
        query_cells: &[(usize, usize)],
        depth: usize,
        measure_only: bool,
    ) -> Result<RenderTableResult, String> {
        use fontdue::layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle};

        if depth > 4 {
            return Err("render_table: recursion depth limit exceeded".to_string());
        }

        let font = self.font_cache.entry(font_key).or_insert_with(|| {
            FontdueFont::from_bytes(font_bytes, fontdue::FontSettings::default())
                .expect("render_table: invalid font bytes")
        });
        let px = size * self.canvas.span() * self.canvas.ru();
        let metrics = font.horizontal_line_metrics(px);
        let ascent = metrics.map(|m| m.ascent).unwrap_or(px);
        let descent = metrics.map(|m| m.descent).unwrap_or(ScalarF4E4::ZERO);
        let span_ru = self.canvas.span() * self.canvas.ru();

        // Column widths: explicit or auto-fit
        let col_widths: Vec<ScalarF4E4> = if let Some(ref ws) = settings.col_widths {
            ws.clone()
        } else {
            let mut max_widths = vec![ScalarF4E4::ZERO; cols];
            let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
            for col in 0..cols {
                for row in 0..rows {
                    let measure_text = match &cells[row * cols + col] {
                        CellContent::Text(s) | CellContent::Styled(s, _, _) => s.as_str(),
                        CellContent::Button { label, .. } => label.as_str(),
                        CellContent::TextInput { placeholder, .. } => placeholder.as_str(),
                        CellContent::SubTable { .. } => "",
                    };
                    let font = self.font_cache.get(&font_key).unwrap();
                    layout.reset(&LayoutSettings::default());
                    layout.append(&[font as &FontdueFont], &TextStyle::new(measure_text, px, 0));
                    let glyphs = layout.glyphs();
                    if !glyphs.is_empty() {
                        let last = &glyphs[glyphs.len() - 1];
                        let text_w = last.x - glyphs[0].x + last.width;
                        if text_w > max_widths[col] { max_widths[col] = text_w; }
                    }
                }
            }
            let pad2 = settings.padding << 1usize;
            max_widths.iter().map(|w| *w / span_ru + pad2).collect()
        };
        let table_width: ScalarF4E4 = col_widths.iter().copied()
            .fold(ScalarF4E4::ZERO, |a, b| a + b);
        let table_left = pos.r() - (table_width >> 1usize);

        let mut col_lefts = Vec::with_capacity(cols + 1);
        col_lefts.push(table_left);
        for col in 0..cols {
            col_lefts.push(col_lefts[col] + col_widths[col]);
        }

        // Row heights
        let single_line_text_h = (ascent - descent) / span_ru;
        let single_line_ru = {
            let line_size = metrics.map(|m| m.new_line_size).unwrap_or(px);
            line_size / span_ru + (settings.padding << 1usize)
        };
        let canvas_h_px = ScalarF4E4::from(self.canvas.height());
        let mut cell_text_heights = vec![single_line_text_h; rows * cols];
        let row_heights: Vec<ScalarF4E4> = if let Some(rh) = settings.row_height {
            vec![rh; rows]
        } else if settings.col_widths.is_some() {
            let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
            let mut heights = Vec::with_capacity(rows);
            for row in 0..rows {
                let mut max_h = single_line_ru;
                for col in 0..cols {
                    let cw = col_widths[col];
                    if !cw.is_positive() { continue; }
                    match &cells[row * cols + col] {
                        CellContent::Text(cell_text) | CellContent::Styled(cell_text, _, _) => {
                            let cell_px = match &cells[row * cols + col] {
                                CellContent::Styled(_, _, Some(sz)) => *sz * span_ru,
                                _ => px,
                            };
                            let wrap_px = cw * span_ru - (settings.padding << 1usize) * span_ru;
                            if !wrap_px.is_positive() { continue; }
                            let font = self.font_cache.get(&font_key).unwrap();
                            layout.reset(&LayoutSettings {
                                max_width: Some(wrap_px),
                                line_height: ScalarF4E4::ONE,
                                ..LayoutSettings::default()
                            });
                            layout.append(&[font as &FontdueFont], &TextStyle::new(cell_text, cell_px, 0));
                            let glyphs = layout.glyphs();
                            if !glyphs.is_empty() {
                                let last = &glyphs[glyphs.len() - 1];
                                let text_bottom = last.y + last.height;
                                if text_bottom > canvas_h_px { continue; }
                                cell_text_heights[row * cols + col] = text_bottom / span_ru;
                                let cell_h = text_bottom / span_ru + (settings.padding << 1usize);
                                if cell_h > max_h { max_h = cell_h; }
                            }
                        }
                        CellContent::SubTable { cols: sub_cols, rows: sub_rows, cells: sub_cells, settings: sub_settings } => {
                            // Measure sub-table height recursively
                            let padded_w = cw - (settings.padding << 1usize);
                            let mut sub_s = sub_settings.clone();
                            if let Some(ref ws) = sub_s.col_widths {
                                // Scale explicit widths to fill parent cell
                                let total: ScalarF4E4 = ws.iter().copied()
                                    .fold(ScalarF4E4::ZERO, |a, b| a + b);
                                if total.is_positive() {
                                    sub_s.col_widths = Some(ws.iter()
                                        .map(|w| *w * padded_w / total)
                                        .collect());
                                }
                            } else {
                                let sub_col_w = padded_w / *sub_cols as isize;
                                sub_s.col_widths = Some(vec![sub_col_w; *sub_cols]);
                            }
                            let dummy_pos = CircleF4E4::ZERO;
                            let sub_result = self.render_table(
                                sub_cells, *sub_cols, *sub_rows, &sub_s, dummy_pos,
                                font_key, font_bytes, size, text_colour,
                                &[], depth + 1, true,
                            )?;
                            let cell_h = sub_result.total_height + (settings.padding << 1usize);
                            if cell_h > max_h { max_h = cell_h; }
                        }
                        _ => {} // Button/TextInput: single-line height is fine
                    }
                }
                heights.push(max_h);
            }
            heights
        } else {
            vec![single_line_ru; rows]
        };

        let mut row_tops = Vec::with_capacity(rows + 1);
        row_tops.push(pos.i());
        for row in 0..rows {
            row_tops.push(row_tops[row] + row_heights[row]);
        }

        // Draw backgrounds
        if !measure_only { for row in 0..rows {
            let rh = row_heights[row];
            let row_pos = CircleF4E4::from((pos.r(), row_tops[row] + (rh >> 1usize)));
            let row_size = CircleF4E4::from((table_width, rh));
            if row == 0 {
                if let Some(ref bg) = settings.header_bg {
                    self.canvas.fill_rect_ru(row_pos, row_size, bg)?;
                }
            } else if row % 2 == 0 {
                if let Some(ref bg) = settings.alt_row_bg {
                    self.canvas.fill_rect_ru(row_pos, row_size, bg)?;
                }
            }
        } }

        // Draw grid lines
        if !measure_only {
        if let (Some(ref border), Some(ref mask)) = (&settings.border_colour, &settings.grid_mask) {
            for row_gap in 0..=rows {
                for col in 0..cols {
                    if mask.h_segment(row_gap, col, cols) {
                        self.canvas.hline_ru(
                            row_tops[row_gap], col_lefts[col],
                            col_lefts[col] + col_widths[col], border,
                        )?;
                    }
                }
            }
            for row in 0..rows {
                for col_gap in 0..=cols {
                    if mask.v_segment(row, col_gap, cols + 1) {
                        self.canvas.vline_ru(
                            col_lefts[col_gap], row_tops[row],
                            row_tops[row + 1], border,
                        )?;
                    }
                }
            }
        }
        }

        // Draw cells (skip when measuring only)
        let mut widget_results: Vec<VsfType> = Vec::new();
        if !measure_only { for row in 0..rows {
            let rh = row_heights[row];
            for col in 0..cols {
                if !col_widths[col].is_positive() { continue; }
                let cell_idx = row * cols + col;
                let cell_center = CircleF4E4::from((
                    col_lefts[col] + (col_widths[col] >> 1usize),
                    row_tops[row] + (rh >> 1usize),
                ));
                let padded_w = col_widths[col] - (settings.padding << 1usize);
                let padded_h = rh - (settings.padding << 1usize);

                match &cells[cell_idx] {
                    CellContent::Text(cell_text) | CellContent::Styled(cell_text, _, _) => {
                        if query_cells.contains(&(row, col)) { continue; }
                        let text_h = cell_text_heights[cell_idx];
                        let (cell_colour, cell_size) = match &cells[cell_idx] {
                            CellContent::Styled(_, c, sz) => (c, sz.unwrap_or(size)),
                            _ => (text_colour, size),
                        };

                        let h = settings.h_align.as_ref()
                            .and_then(|a| a.get(col).copied())
                            .unwrap_or(b'c');
                        let align = match h { b'l' => 1, b'r' => 2, _ => 0 };
                        let cell_x = match h {
                            b'l' => col_lefts[col] + settings.padding,
                            b'r' => col_lefts[col + 1] - settings.padding,
                            _ => col_lefts[col] + (col_widths[col] >> 1usize),
                        };
                        let v = settings.v_align.as_ref()
                            .and_then(|a| a.get(col).copied())
                            .unwrap_or(b'm');
                        let rt = row_tops[row];
                        let cell_y = match v {
                            b't' => rt + settings.padding + (text_h >> 1usize),
                            b'b' => rt + rh - settings.padding - (text_h >> 1usize),
                            _ => rt + (rh >> 1usize),
                        };
                        let cell_pos = CircleF4E4::from((cell_x, cell_y));
                        let wrap = if settings.col_widths.is_some() {
                            let w = col_widths[col] - (settings.padding << 1usize);
                            if w.is_positive() { Some(w) } else { None }
                        } else { None };
                        let text_settings = crate::drawing::TextSettings {
                            align, wrap, ..Default::default()
                        };
                        self.canvas.draw_text(
                            &mut self.font_cache, font_key, font_bytes,
                            cell_pos, cell_size, cell_text, cell_colour, &text_settings,
                        )?;
                    }

                    CellContent::Button { label, colour, id, action_url } => {
                        let widget_id = *id;
                        let btn_size = CircleF4E4::from((padded_w, padded_h));
                        let half_w = padded_w >> 1usize;
                        let half_h = padded_h >> 1usize;
                        let left = cell_center.r() - half_w;
                        let right = cell_center.r() + half_w;
                        let top = cell_center.i() - half_h;
                        let bottom = cell_center.i() + half_h;

                        self.hit_regions.push(HitRegion {
                            x: left, y: top, w: padded_w, h: padded_h,
                            widget_id, cursor: CursorKind::Pointer,
                        });
                        self.canvas.stroke_rect_ru(cell_center, btn_size, colour)?;

                        let text_size = size;
                        let text_settings = crate::drawing::TextSettings {
                            align: 0, wrap: Some(padded_w), ..Default::default()
                        };
                        self.canvas.draw_text(
                            &mut self.font_cache, font_key, font_bytes,
                            cell_center, text_size, label, colour, &text_settings,
                        )?;

                        let mut clicked = false;
                        for event in &self.events {
                            if let InputEvent::MouseDown { x, y } = event {
                                if *x >= left && *x <= right && *y >= top && *y <= bottom {
                                    clicked = true;
                                }
                            }
                        }
                        if clicked {
                            if let Some(url) = action_url {
                                self.actions.push(url.clone());
                            }
                        }
                        widget_results.push(VsfType::s44(
                            if clicked { ScalarF4E4::ONE } else { ScalarF4E4::ZERO }
                        ));
                    }

                    CellContent::TextInput { placeholder, colour, id } => {
                        let widget_id = *id;
                        let input_size = CircleF4E4::from((padded_w, padded_h));
                        if !self.text_inputs.contains_key(&widget_id) {
                            self.text_inputs.insert(widget_id, TextInputState::new());
                        }
                        let half_w = padded_w >> 1usize;
                        let half_h = padded_h >> 1usize;
                        let left = cell_center.r() - half_w;
                        let right = cell_center.r() + half_w;
                        let top = cell_center.i() - half_h;
                        let bottom = cell_center.i() + half_h;

                        self.hit_regions.push(HitRegion {
                            x: left, y: top, w: padded_w, h: padded_h,
                            widget_id, cursor: CursorKind::Text,
                        });

                        let is_focused = self.focused_widget == Some(widget_id);
                        if is_focused {
                            let state = self.text_inputs.get_mut(&widget_id).unwrap();
                            for event in &self.events {
                                match event {
                                    InputEvent::KeyPress { text } => {
                                        if let Some(anchor) = state.selection_anchor.take() {
                                            let (start, end) = if anchor < state.cursor_pos { (anchor, state.cursor_pos) } else { (state.cursor_pos, anchor) };
                                            state.chars.drain(start..end);
                                            state.cursor_pos = start;
                                        }
                                        for ch in text.chars() {
                                            state.chars.insert(state.cursor_pos, ch);
                                            state.cursor_pos += 1;
                                        }
                                    }
                                    InputEvent::KeyDown { key } => {
                                        match key.as_str() {
                                            "Backspace" => {
                                                if let Some(anchor) = state.selection_anchor.take() {
                                                    let (start, end) = if anchor < state.cursor_pos { (anchor, state.cursor_pos) } else { (state.cursor_pos, anchor) };
                                                    state.chars.drain(start..end);
                                                    state.cursor_pos = start;
                                                } else if state.cursor_pos > 0 {
                                                    state.cursor_pos -= 1;
                                                    state.chars.remove(state.cursor_pos);
                                                }
                                            }
                                            "Delete" => {
                                                if let Some(anchor) = state.selection_anchor.take() {
                                                    let (start, end) = if anchor < state.cursor_pos { (anchor, state.cursor_pos) } else { (state.cursor_pos, anchor) };
                                                    state.chars.drain(start..end);
                                                    state.cursor_pos = start;
                                                } else if state.cursor_pos < state.chars.len() {
                                                    state.chars.remove(state.cursor_pos);
                                                }
                                            }
                                            "ArrowLeft" => { if state.cursor_pos > 0 { state.cursor_pos -= 1; } state.selection_anchor = None; }
                                            "ArrowRight" => { if state.cursor_pos < state.chars.len() { state.cursor_pos += 1; } state.selection_anchor = None; }
                                            "Home" => { state.cursor_pos = 0; state.selection_anchor = None; }
                                            "End" => { state.cursor_pos = state.chars.len(); state.selection_anchor = None; }
                                            _ => {}
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }

                        for event in &self.events {
                            if let InputEvent::MouseDown { x, y } = event {
                                if *x >= left && *x <= right && *y >= top && *y <= bottom {
                                    self.focused_widget = Some(widget_id);
                                    let state = self.text_inputs.get_mut(&widget_id).unwrap();
                                    state.selection_anchor = None;
                                }
                            }
                        }

                        self.canvas.stroke_rect_ru(cell_center, input_size, colour)?;
                        let state = self.text_inputs.get(&widget_id).unwrap();
                        let display_text = if state.chars.is_empty() {
                            placeholder.clone()
                        } else { state.text() };
                        let text_size = padded_h * ScalarF4E4::from(2) / ScalarF4E4::from(3);
                        let text_padding = padded_w / ScalarF4E4::from(40);
                        let text_pos = CircleF4E4::from((left + text_padding, cell_center.i()));
                        let display_colour = if state.chars.is_empty() {
                            VsfType::ra([128, 128, 128, 255])
                        } else { colour.clone() };
                        let text_settings = crate::drawing::TextSettings {
                            align: 1, ..Default::default()
                        };
                        self.canvas.draw_text(
                            &mut self.font_cache, font_key, font_bytes,
                            text_pos, text_size, &display_text, &display_colour, &text_settings,
                        )?;

                        if is_focused {
                            let state = self.text_inputs.get(&widget_id).unwrap();
                            let cursor_text: String = state.chars[..state.cursor_pos].iter().collect();
                            let font = self.font_cache.get(&font_key);
                            if let Some(font) = font {
                                let cursor_px = text_size * span_ru;
                                let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
                                layout.reset(&LayoutSettings::default());
                                layout.append(&[font as &FontdueFont], &TextStyle::new(&cursor_text, cursor_px, 0));
                                let cursor_w_px = layout.glyphs().iter()
                                    .map(|g| g.x + g.width)
                                    .fold(ScalarF4E4::ZERO, |a, b| if b > a { b } else { a });
                                let cursor_x = left + text_padding + cursor_w_px / span_ru;
                                self.canvas.vline_ru(cursor_x, top, bottom, colour)?;
                            }
                        }

                        let result_text = self.text_inputs.get(&widget_id).unwrap().text();
                        widget_results.push(VsfType::x(result_text));
                    }

                    CellContent::SubTable { cols: sub_cols, rows: sub_rows, cells: sub_cells, settings: sub_settings } => {
                        // Recursive sub-table rendering — fill parent cell width
                        let sub_pos = CircleF4E4::from((
                            cell_center.r(),
                            cell_center.i() - (padded_h >> 1usize),
                        ));
                        let mut sub_s = sub_settings.clone();
                        if let Some(ref ws) = sub_s.col_widths {
                            // Scale explicit widths to fill parent cell
                            let total: ScalarF4E4 = ws.iter().copied()
                                .fold(ScalarF4E4::ZERO, |a, b| a + b);
                            if total.is_positive() {
                                sub_s.col_widths = Some(ws.iter()
                                    .map(|w| *w * padded_w / total)
                                    .collect());
                            }
                        } else {
                            // Auto-distribute columns across available width
                            let sub_col_w = padded_w / *sub_cols as isize;
                            sub_s.col_widths = Some(vec![sub_col_w; *sub_cols]);
                        }
                        let sub_result = self.render_table(
                            sub_cells, *sub_cols, *sub_rows, &sub_s, sub_pos,
                            font_key, font_bytes, size, text_colour,
                            &[], depth + 1, measure_only,
                        )?;
                        // Sub-table widget results bubble up
                        widget_results.extend(sub_result.widget_results);
                    }
                }
            }
        } }

        let total_height = row_heights.iter().copied().fold(ScalarF4E4::ZERO, |a, b| a + b);
        Ok(RenderTableResult {
            total_height,
            widget_results,
            row_heights,
            col_widths,
            col_lefts,
            row_tops,
        })
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
    /// Replace the canvas (for pipeline switching)
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

    /// Get and clear execution trace
    pub fn take_trace(&mut self) -> Vec<String> {
        std::mem::take(&mut self.trace)
    }

    /// Get stack slice (for testing)
    #[cfg(test)]
    pub fn stack(&self) -> &[VsfType] {
        &self.stack
    }

    // ── Interactive widget API ──────────────────────────────

    /// Push an input event (from JS host)
    pub fn push_event(&mut self, event: InputEvent) {
        self.events.push(event);
    }

    /// Drain events after frame execution
    pub fn drain_events(&mut self) {
        self.events.clear();
    }

    /// Drain triggered action URLs (called by JS after frame execution)
    pub fn drain_actions(&mut self) -> Vec<String> {
        std::mem::take(&mut self.actions)
    }

    /// Get hit regions registered this frame (for cursor management)
    pub fn hit_regions(&self) -> &[HitRegion] {
        &self.hit_regions
    }

    /// Get the currently focused widget ID
    pub fn focused_widget(&self) -> Option<u32> {
        self.focused_widget
    }

    /// Get text input state for a widget
    pub fn text_input_state(&self, id: u32) -> Option<&TextInputState> {
        self.text_inputs.get(&id)
    }

    /// Check if a point (in RU) hits any registered widget
    pub fn hit_test(&self, x: ScalarF4E4, y: ScalarF4E4) -> Option<&HitRegion> {
        // Last registered wins (top-most in draw order)
        self.hit_regions.iter().rev().find(|r| {
            x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h
        })
    }

    /// Process a click event — update focus based on hit testing
    fn process_click(&mut self, x: ScalarF4E4, y: ScalarF4E4) -> Option<u32> {
        // Find which widget was clicked
        let clicked_id = self.hit_regions.iter().rev()
            .find(|r| x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h)
            .map(|r| r.widget_id);

        // Update focus
        self.focused_widget = clicked_id;

        // For text inputs, set cursor position from click x
        if let Some(id) = clicked_id {
            if let Some(state) = self.text_inputs.get_mut(&id) {
                state.selection_anchor = None;
                // Cursor positioning is done in the opcode handler where we have font metrics
            }
        }

        clicked_id
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
            VsfType::s44(s) => assert_eq!(*s, (5)),
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
