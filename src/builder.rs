//! Rust DSL for building Toka programs
//!
//! Provides a type-safe, chainable API for constructing Toka bytecode using mnemonic method names that match the opcodes exactly (e.g., `.ps()` for push).
//!
//! # Example
//!
//! ```rust
//! use toka::builder::Program;
//! use vsf::types::VsfType;
//!
//! // Simple arithmetic: 1 + 1 = 2
//! let bytecode = Program::new()
//!     .ps_s44(1)      // push 1
//!     .ps_s44(1)      // push 1
//!     .ad()           // add
//!     .hl()           // halt
//!     .build();
//! ```

use spirix::*;
use vsf::types::VsfType;

/// Emit a VSF-encoded opcode: `{ab}` -> 4 bytes
#[inline]
fn emit_op(bytecode: &mut Vec<u8>, a: u8, b: u8) {
    bytecode.extend_from_slice(&VsfType::op(a, b).flatten());
}

/// Toka builder with chainable opcode methods
///
/// Each method corresponds to a Toka opcode and appends the appropriate bytes to the bytecode vector. The builder pattern allows for readable, type-safe program construction with compile-time checking.
pub struct Program {
    bytecode: Vec<u8>,
}

impl Program {
    /// Create a new empty program
    pub fn new() -> Self {
        Self {
            bytecode: Vec::new(),
        }
    }

    /// Build and return the final bytecode
    pub fn build(self) -> Vec<u8> {
        self.bytecode
    }

    // ==================== STACK MANIPULATION ====================

    /// Push a VSF-encoded value (requires inline VSF data after opcode)
    /// VSF: {ps}[vsf_value]
    pub fn ps(mut self, vsf_bytes: &[u8]) -> Self {
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(vsf_bytes);
        self
    }

    /// Push S44 value (encodes as VSF s44)
    /// VSF: {ps}s44[bytes]
    pub fn ps_s44(mut self, value: impl Into<ScalarF4E4>) -> Self {
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode
            .extend_from_slice(&VsfType::s44(value.into()).flatten());
        self
    }

    /// Push C44 value - Circle with two components (e.g., position, size)
    /// VSF: {ps}c44[bytes]
    pub fn ps_c44(
        mut self,
        re: impl Into<ScalarF4E4>,
        im: impl Into<ScalarF4E4>,
    ) -> Self {
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(
            &VsfType::c44(CircleF4E4::from((re.into(), im.into()))).flatten(),
        );
        self
    }

    /// Push u32 value as unbounded VSF u (variable-length encoding)
    /// VSF: {ps}u[bytes]
    pub fn ps_u32(mut self, value: u32) -> Self {
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode
            .extend_from_slice(&VsfType::u(value as usize, false).flatten());
        self
    }

    /// Push string value (encodes as VSF l - ASCII)
    /// VSF: {ps}l[len][bytes]
    pub fn ps_str(mut self, s: &str) -> Self {
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode
            .extend_from_slice(&VsfType::l(s.to_string()).flatten());
        self
    }

    /// Pop top of stack
    /// VSF: {pp}
    pub fn pp(mut self) -> Self {
        emit_op(&mut self.bytecode, b'p', b'p');
        self
    }

    /// Duplicate top of stack
    /// VSF: {dp}
    pub fn dp(mut self) -> Self {
        emit_op(&mut self.bytecode, b'd', b'p');
        self
    }

    /// Duplicate N items from stack
    /// VSF: {dn}[count:u]
    pub fn dn(mut self, count: u32) -> Self {
        emit_op(&mut self.bytecode, b'd', b'n');
        self.bytecode
            .extend_from_slice(&VsfType::u(count as usize, false).flatten());
        self
    }

    /// Swap top two stack items (runtime swaps whatever is on stack)
    /// VSF: {sw}
    pub fn sw(mut self) -> Self {
        emit_op(&mut self.bytecode, b's', b'w');
        self
    }

    /// Rotate top N stack items (runtime operation)
    /// VSF: {rt}[count:u]
    pub fn rt(mut self, count: u32) -> Self {
        emit_op(&mut self.bytecode, b'r', b't');
        self.bytecode
            .extend_from_slice(&VsfType::u(count as usize, false).flatten());
        self
    }

    // ==================== LOCAL VARIABLES ====================

    /// Allocate N local variables
    /// VSF: {la}[count:u]
    pub fn la(mut self, count: u32) -> Self {
        emit_op(&mut self.bytecode, b'l', b'a');
        self.bytecode
            .extend_from_slice(&VsfType::u(count as usize, false).flatten());
        self
    }

    /// Get local variable at index
    /// VSF: {lg}[index:u]
    pub fn lg(mut self, index: u32) -> Self {
        emit_op(&mut self.bytecode, b'l', b'g');
        self.bytecode
            .extend_from_slice(&VsfType::u(index as usize, false).flatten());
        self
    }

    /// Set local variable at index
    /// VSF: {ls}[index:u]
    pub fn ls(mut self, index: u32) -> Self {
        emit_op(&mut self.bytecode, b'l', b's');
        self.bytecode
            .extend_from_slice(&VsfType::u(index as usize, false).flatten());
        self
    }

    /// Tee local variable (set without popping)
    /// VSF: {lt}[index:u]
    pub fn lt(mut self, index: u32) -> Self {
        emit_op(&mut self.bytecode, b'l', b't');
        self.bytecode
            .extend_from_slice(&VsfType::u(index as usize, false).flatten());
        self
    }

    // ==================== ARITHMETIC ====================

    /// Add: pop b, a; push a+b (Spirix handles type compatibility)
    /// VSF: {ad}
    pub fn ad(mut self) -> Self {
        emit_op(&mut self.bytecode, b'a', b'd');
        self
    }

    /// Subtract: pop b, a; push a-b
    /// VSF: {sb}
    pub fn sb(mut self) -> Self {
        emit_op(&mut self.bytecode, b's', b'b');
        self
    }

    /// Multiply: pop b, a; push a*b
    /// VSF: {ml}
    pub fn ml(mut self) -> Self {
        emit_op(&mut self.bytecode, b'm', b'l');
        self
    }

    /// Divide: pop b, a; push a/b
    /// VSF: {dv}
    pub fn dv(mut self) -> Self {
        emit_op(&mut self.bytecode, b'd', b'v');
        self
    }

    /// Modulo: pop b, a; push a%b
    /// VSF: {md}
    pub fn md(mut self) -> Self {
        emit_op(&mut self.bytecode, b'm', b'd');
        self
    }

    /// Reciprocal: pop a; push 1/a (works for all Spirix numeric types)
    /// VSF: {rc}
    pub fn rc(mut self) -> Self {
        emit_op(&mut self.bytecode, b'r', b'c');
        self
    }

    /// Negate: pop a; push -a
    /// VSF: {ng}
    pub fn ng(mut self) -> Self {
        emit_op(&mut self.bytecode, b'n', b'g');
        self
    }

    /// Absolute value: pop a; push |a|
    /// VSF: {ab}
    pub fn ab(mut self) -> Self {
        emit_op(&mut self.bytecode, b'a', b'b');
        self
    }

    /// Square root: pop a; push sqrt(a)
    /// VSF: {sq}
    pub fn sq(mut self) -> Self {
        emit_op(&mut self.bytecode, b's', b'q');
        self
    }

    /// Power: pop b, a; push a^b
    /// VSF: {pw}
    pub fn pw(mut self) -> Self {
        emit_op(&mut self.bytecode, b'p', b'w');
        self
    }

    /// Minimum: pop b, a; push min(a, b)
    /// VSF: {mn}
    pub fn mn(mut self) -> Self {
        emit_op(&mut self.bytecode, b'm', b'n');
        self
    }

    /// Maximum: pop b, a; push max(a, b)
    /// VSF: {mx}
    pub fn mx(mut self) -> Self {
        emit_op(&mut self.bytecode, b'm', b'x');
        self
    }

    /// Clamp: pop max, min, a; push clamp(a, min, max)
    /// VSF: {cm}
    pub fn cm(mut self) -> Self {
        emit_op(&mut self.bytecode, b'c', b'm');
        self
    }

    /// Floor: pop a; push floor(a)
    /// VSF: {fl}
    pub fn fl(mut self) -> Self {
        emit_op(&mut self.bytecode, b'f', b'l');
        self
    }

    /// Ceiling: pop a; push ceil(a)
    /// VSF: {cl}
    pub fn cl(mut self) -> Self {
        emit_op(&mut self.bytecode, b'c', b'l');
        self
    }

    /// Round: pop a; push round(a)
    /// VSF: {rn}
    pub fn rn(mut self) -> Self {
        emit_op(&mut self.bytecode, b'r', b'n');
        self
    }

    /// Fractional part: pop a; push frac(a)
    /// VSF: {fa}
    pub fn fa(mut self) -> Self {
        emit_op(&mut self.bytecode, b'f', b'a');
        self
    }

    /// Linear interpolation: pop t, b, a; push a + t*(b-a)
    /// VSF: {lp}
    pub fn lp(mut self) -> Self {
        emit_op(&mut self.bytecode, b'l', b'p');
        self
    }

    // ==================== TRIGONOMETRY ====================

    /// Sine: pop a; push sin(a) (Spirix trigonometry)
    /// VSF: {sn}
    pub fn sn(mut self) -> Self {
        emit_op(&mut self.bytecode, b's', b'n');
        self
    }

    /// Cosine: pop a; push cos(a)
    /// VSF: {cs}
    pub fn cs(mut self) -> Self {
        emit_op(&mut self.bytecode, b'c', b's');
        self
    }

    /// Tangent: pop a; push tan(a)
    /// VSF: {tn}
    pub fn tn(mut self) -> Self {
        emit_op(&mut self.bytecode, b't', b'n');
        self
    }

    /// Arcsine: pop a; push asin(a)
    /// VSF: {is}
    pub fn is(mut self) -> Self {
        emit_op(&mut self.bytecode, b'i', b's');
        self
    }

    /// Arccosine: pop a; push acos(a)
    /// VSF: {ic}
    pub fn ic(mut self) -> Self {
        emit_op(&mut self.bytecode, b'i', b'c');
        self
    }

    /// Arctangent: pop a; push atan(a)
    /// VSF: {ia}
    pub fn ia(mut self) -> Self {
        emit_op(&mut self.bytecode, b'i', b'a');
        self
    }

    /// Arctangent2: pop x, y; push atan2(y, x)
    /// VSF: {at}
    pub fn at(mut self) -> Self {
        emit_op(&mut self.bytecode, b'a', b't');
        self
    }

    // ==================== COMPARISON ====================

    /// Equal: pop b, a; push 1 if a==b else 0
    /// VSF: {eq}
    pub fn eq(mut self) -> Self {
        emit_op(&mut self.bytecode, b'e', b'q');
        self
    }

    /// Not equal: pop b, a; push 1 if a!=b else 0
    /// VSF: {ne}
    pub fn ne(mut self) -> Self {
        emit_op(&mut self.bytecode, b'n', b'e');
        self
    }

    /// Less than: pop b, a; push 1 if a<b else 0
    /// VSF: {lo}
    pub fn lo(mut self) -> Self {
        emit_op(&mut self.bytecode, b'l', b'o');
        self
    }

    /// Less than or equal: pop b, a; push 1 if a<=b else 0
    /// VSF: {le}
    pub fn le(mut self) -> Self {
        emit_op(&mut self.bytecode, b'l', b'e');
        self
    }

    /// Greater than: pop b, a; push 1 if a>b else 0
    /// VSF: {gt}
    pub fn gt(mut self) -> Self {
        emit_op(&mut self.bytecode, b'g', b't');
        self
    }

    /// Greater than or equal: pop b, a; push 1 if a>=b else 0
    /// (Returns numeric 1/0, not bool - VSF has no bool type)
    /// VSF: {ge}
    pub fn ge(mut self) -> Self {
        emit_op(&mut self.bytecode, b'g', b'e');
        self
    }

    // ==================== BITWISE (All Numeric Types) ====================

    /// Bitwise AND: pop b, a; push a & b (works on all Spirix numeric types)
    /// VSF: {an}
    pub fn an(mut self) -> Self {
        emit_op(&mut self.bytecode, b'a', b'n');
        self
    }

    /// Bitwise OR: pop b, a; push a | b (works on all Spirix numeric types)
    /// VSF: {or}
    pub fn or(mut self) -> Self {
        emit_op(&mut self.bytecode, b'o', b'r');
        self
    }

    /// Bitwise XOR: pop b, a; push a ^ b (works on all Spirix numeric types)
    /// VSF: {xr}
    pub fn xor(mut self) -> Self {
        emit_op(&mut self.bytecode, b'x', b'r');
        self
    }

    /// Bitwise NOT: pop a; push ~a (works on all Spirix numeric types)
    /// VSF: {nt}
    pub fn nt(mut self) -> Self {
        emit_op(&mut self.bytecode, b'n', b't');
        self
    }

    // ==================== TYPE SYSTEM ====================

    /// Typeof: pop value; push type name as string (e.g., "s44", "u", "string")
    /// VSF: {ty}
    pub fn ty(mut self) -> Self {
        emit_op(&mut self.bytecode, b't', b'y');
        self
    }

    /// Convert to S44: pop value; push s44 scalar
    /// VSF: {ts}
    pub fn ts(mut self) -> Self {
        emit_op(&mut self.bytecode, b't', b's');
        self
    }

    /// Convert to unbounded uint: pop value; push VSF u
    /// VSF: {tu}
    pub fn tu(mut self) -> Self {
        emit_op(&mut self.bytecode, b't', b'u');
        self
    }

    /// To string: pop value; push string representation
    /// VSF: {tx}
    pub fn tx(mut self) -> Self {
        emit_op(&mut self.bytecode, b't', b'x');
        self
    }


    // ==================== CONTROL FLOW ====================

    /// Call function at bytecode offset (low-level - symbolic names TBD)
    /// VSF: {cn}[offset:u]
    pub fn cn(mut self, offset: u32) -> Self {
        emit_op(&mut self.bytecode, b'c', b'n');
        self.bytecode
            .extend_from_slice(&VsfType::u(offset as usize, false).flatten());
        self
    }

    /// Return from function (no value)
    /// VSF: {re}
    pub fn re(mut self) -> Self {
        emit_op(&mut self.bytecode, b'r', b'e');
        self
    }

    /// Return value from function
    /// VSF: {rv}
    pub fn rv(mut self) -> Self {
        emit_op(&mut self.bytecode, b'r', b'v');
        self
    }

    /// Unconditional jump to bytecode offset (low-level - labels TBD)
    /// VSF: {jm}[offset:u]
    pub fn jm(mut self, offset: u32) -> Self {
        emit_op(&mut self.bytecode, b'j', b'm');
        self.bytecode
            .extend_from_slice(&VsfType::u(offset as usize, false).flatten());
        self
    }

    /// Conditional jump: pop value; if truthy (non-zero), jump to offset
    /// VSF: {ji}[offset:u]
    pub fn ji(mut self, offset: u32) -> Self {
        emit_op(&mut self.bytecode, b'j', b'i');
        self.bytecode
            .extend_from_slice(&VsfType::u(offset as usize, false).flatten());
        self
    }

    // ==================== RENDERING ====================

    /// Push raw bytes as a VSF binary blob (vb3 encoding)
    /// VSF: {ps}v'b'[bytes]
    pub fn ps_bytes(mut self, bytes: &[u8]) -> Self {
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::v(b'b', bytes.to_vec()).flatten());
        self
    }

    /// Draw text with no align param (defaults to center).
    /// Stack before call: font_bytes(vb), pos(c44), size(s44), text(x|l), colour
    /// VSF: {dt}
    pub fn dt(mut self) -> Self {
        emit_op(&mut self.bytecode, b'd', b't');
        self
    }

    /// Draw text, center-aligned (default).
    /// Stack before call: font_bytes(vb), pos(c44), size(s44), text(x|l), colour
    /// VSF: {ps}u3[0x00]{dt}
    pub fn dt_center(mut self) -> Self {
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::u3(0).flatten());
        emit_op(&mut self.bytecode, b'd', b't');
        self
    }

    /// Draw text, left-aligned.
    /// Stack before call: font_bytes(vb), pos(c44), size(s44), text(x|l), colour
    /// VSF: {ps}u3[0x01]{dt}
    pub fn dt_left(mut self) -> Self {
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::u3(1).flatten());
        emit_op(&mut self.bytecode, b'd', b't');
        self
    }

    /// Draw text, right-aligned.
    /// Stack before call: font_bytes(vb), pos(c44), size(s44), text(x|l), colour
    /// VSF: {ps}u3[0x02]{dt}
    pub fn dt_right(mut self) -> Self {
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::u3(2).flatten());
        emit_op(&mut self.bytecode, b'd', b't');
        self
    }

    /// Draw text with wrap width (left-aligned, wrapping at given RU width).
    /// Stack before call: font_bytes(vb), pos(c44), size(s44), text(x|l), colour
    pub fn dt_wrap(mut self, wrap_width: f32) -> Self {
        // Push wrap value + tag
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::s44(ScalarF4E4::from_f32(wrap_width)).flatten());
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::l("x".to_string()).flatten());
        // Push left-align tag
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::l("l".to_string()).flatten());
        emit_op(&mut self.bytecode, b'd', b't');
        self
    }

    /// Draw text with wrap width and explicit alignment.
    /// align: 0=center, 1=left, 2=right
    pub fn dt_wrap_align(mut self, wrap_width: f32, align: u8) -> Self {
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::s44(ScalarF4E4::from_f32(wrap_width)).flatten());
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::l("x".to_string()).flatten());
        // Alignment tag
        match align {
            1 => {
                emit_op(&mut self.bytecode, b'p', b's');
                self.bytecode.extend_from_slice(&VsfType::l("l".to_string()).flatten());
            }
            2 => {
                emit_op(&mut self.bytecode, b'p', b's');
                self.bytecode.extend_from_slice(&VsfType::l("r".to_string()).flatten());
            }
            _ => {} // center = default, no tag needed
        }
        emit_op(&mut self.bytecode, b'd', b't');
        self
    }

    /// Draw text with leading (line height multiplier).
    /// Stack before call: font_bytes(vb), pos(c44), size(s44), text(x|l), colour
    pub fn dt_leading(mut self, leading: f32) -> Self {
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::s44(ScalarF4E4::from_f32(leading)).flatten());
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::l("e".to_string()).flatten());
        emit_op(&mut self.bytecode, b'd', b't');
        self
    }

    // ==================== DRAW LINE ====================

    /// Draw line with default settings (1px hairline, butt cap).
    /// Stack before call: start(c44), end(c44), colour
    /// VSF: {dl}
    pub fn dl(mut self) -> Self {
        emit_op(&mut self.bytecode, b'd', b'l');
        self
    }

    /// Draw line with weight (thick line, butt caps).
    /// Stack before call: start(c44), end(c44), colour
    /// Tags pushed: s44(weight), l("w") — VM pops tag first, then value
    pub fn dl_weight(mut self, weight: f32) -> Self {
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::s44(ScalarF4E4::from_f32(weight)).flatten());
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::l("w".to_string()).flatten());
        emit_op(&mut self.bytecode, b'd', b'l');
        self
    }

    /// Draw line in pixel mode (always 1 device pixel).
    /// Stack before call: start(c44), end(c44), colour
    pub fn dl_pixel(mut self) -> Self {
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::l("p".to_string()).flatten());
        emit_op(&mut self.bytecode, b'd', b'l');
        self
    }

    /// Draw line with weight and round caps (both ends).
    /// Stack before call: start(c44), end(c44), colour
    pub fn dl_round(mut self, weight: f32) -> Self {
        // Push value, then tag — VM pops tag first, then reads value
        // Weight
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::s44(ScalarF4E4::from_f32(weight)).flatten());
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::l("w".to_string()).flatten());
        // Cap (both)
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::u3(1).flatten());
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::l("c".to_string()).flatten());
        emit_op(&mut self.bytecode, b'd', b'l');
        self
    }

    // ==================== DRAW TABLE ====================

    /// Draw a table. Same base stack as draw_text (font, pos, size, colour),
    /// with all table data via tags.
    ///
    /// Stack before: font_bytes(vb), pos(c44), size(s44)
    /// Tags pushed: cell data, cols, rows, width, styling
    ///
    /// `headers`: column header strings (first row)
    /// `rows`: data rows (each row is a slice of cell strings)
    /// `text_colour`: VSF colour bytes
    /// `width`: table width in RU
    /// `header_bg`: optional header background colour (VSF bytes)
    /// `border_colour`: optional grid/border colour (VSF bytes)
    /// `alt_row_bg`: optional alternating row background colour (VSF bytes)
    pub fn draw_table(
        mut self,
        headers: &[&str],
        rows: &[&[&str]],
        text_colour: &[u8],
        col_widths: Option<&[f32]>,
        header_bg: Option<&[u8]>,
        border_colour: Option<&[u8]>,
        alt_row_bg: Option<&[u8]>,
        h_align: Option<&str>,
        v_align: Option<&str>,
    ) -> Self {
        let cols = headers.len();
        let total_rows = 1 + rows.len();

        // Push text colour (base param, like dt)
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(text_colour);

        // Tags — pushed top-down, VM pops top-first
        // Alignment tags (optional)
        if let Some(va) = v_align {
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::l(va.to_string()).flatten());
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::l("v".to_string()).flatten());
        }
        if let Some(ha) = h_align {
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::l(ha.to_string()).flatten());
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::l("j".to_string()).flatten());
        }
        // Styling tags (optional)
        if let Some(alt) = alt_row_bg {
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(alt);
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::l("a".to_string()).flatten());
        }
        if let Some(border) = border_colour {
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(border);
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::l("b".to_string()).flatten());
        }
        if let Some(header) = header_bg {
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(header);
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::l("h".to_string()).flatten());
        }
        // Per-column widths (optional — omit for all auto-fit. 0 = hidden.)
        if let Some(ws) = col_widths {
            for &w in ws {
                emit_op(&mut self.bytecode, b'p', b's');
                self.bytecode.extend_from_slice(&VsfType::s44(ScalarF4E4::from_f32(w)).flatten());
            }
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::l("x".to_string()).flatten());
        }

        // Cell data — push strings first (they'll be under the 'd' tag)
        // Row-major: header row, then data rows
        for h in headers {
            self = self.ps_str(h);
        }
        for row in rows {
            for cell in *row {
                self = self.ps_str(cell);
            }
        }
        // 'd' tag tells VM to pop cols*rows strings
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::l("d".to_string()).flatten());

        // Row count
        self = self.ps_u32(total_rows as u32);
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::l("r".to_string()).flatten());

        // Column count (must come before 'r' and 'd' in parse order, so pushed last)
        self = self.ps_u32(cols as u32);
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::l("c".to_string()).flatten());

        // Emit draw_table opcode
        emit_op(&mut self.bytecode, b't', b'b');
        self
    }

    /// Raw draw_table opcode — expects stack already set up.
    /// VSF: {tb}
    pub fn tb(mut self) -> Self {
        emit_op(&mut self.bytecode, b't', b'b');
        self
    }

    /// Clear canvas: pop VSF colour (rc*, ra, rw) and fill canvas
    /// VSF: {cr}
    pub fn cr(mut self) -> Self {
        emit_op(&mut self.bytecode, b'c', b'r');
        self
    }

    /// Render Loom: pop scene graph from stack and render to canvas
    /// VSF: {rl}
    pub fn rl(mut self) -> Self {
        emit_op(&mut self.bytecode, b'r', b'l');
        self
    }

    // ==================== SCENE GRAPH CONSTRUCTION ====================

    /// Build row: pop children (ron), rotate (s44), translate (c44) → push row
    /// VSF: {kw}
    pub fn kw(mut self) -> Self {
        emit_op(&mut self.bytecode, b'k', b'w');
        self
    }

    /// Build rob: pop children (ron), fill (colour), size (c44), pos (c44) → push rob
    /// VSF: {kb}
    pub fn kb(mut self) -> Self {
        emit_op(&mut self.bytecode, b'k', b'b');
        self
    }

    /// Build roc: pop fill (colour), radius (s44), center (c44) → push roc
    /// VSF: {kc}
    pub fn kc(mut self) -> Self {
        emit_op(&mut self.bytecode, b'k', b'c');
        self
    }

    // ==================== CONTEXT VARIABLES (Reactive) ====================

    /// Push current time (Unix timestamp in seconds as S44)
    /// VSF: {tm}
    pub fn tm(mut self) -> Self {
        emit_op(&mut self.bytecode, b't', b'm');
        self
    }

    /// Push mouse/pointer X position (in RU)
    /// VSF: {ox}
    pub fn ox(mut self) -> Self {
        emit_op(&mut self.bytecode, b'o', b'x');
        self
    }

    /// Push mouse/pointer Y position (in RU)
    /// VSF: {oy}
    pub fn oy(mut self) -> Self {
        emit_op(&mut self.bytecode, b'o', b'y');
        self
    }

    /// Push scroll offset X (in RU)
    /// VSF: {sx}
    pub fn sx(mut self) -> Self {
        emit_op(&mut self.bytecode, b's', b'x');
        self
    }

    /// Push scroll offset Y (in RU)
    /// VSF: {sy}
    pub fn sy(mut self) -> Self {
        emit_op(&mut self.bytecode, b's', b'y');
        self
    }

    // ==================== HALT ====================

    /// Halt execution
    /// VSF: {hl}
    pub fn hl(mut self) -> Self {
        emit_op(&mut self.bytecode, b'h', b'l');
        self
    }

    // ==================== DEBUG ====================

    /// Debug print: pop value; print to stdout
    /// VSF: {db}
    pub fn db(mut self) -> Self {
        emit_op(&mut self.bytecode, b'd', b'b');
        self
    }

    /// Debug stack: print entire stack
    /// VSF: {ds}
    pub fn ds(mut self) -> Self {
        emit_op(&mut self.bytecode, b'd', b's');
        self
    }

    /// No operation
    /// VSF: {np}
    pub fn np(mut self) -> Self {
        emit_op(&mut self.bytecode, b'n', b'p');
        self
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        // 1 + 1 = 2
        let bytecode = Program::new()
            .ps_s44(1) // push 1
            .ps_s44(1) // push 1
            .ad() // add
            .hl() // halt
            .build();

        assert!(bytecode.len() > 0);
        // Bytecode contains push opcodes + s44 scalar encodings + add + halt
    }

    #[test]
    fn test_push_s44() {
        let bytecode = Program::new().ps_s44(3.14).hl().build();

        // VSF format: {ps} (4 bytes) + s44 type marker (3) + fraction (2) + exponent (2) + {hl} (4 bytes) = 15 bytes
        assert_eq!(bytecode[0], b'{');
        assert_eq!(bytecode[1], b'p');
        assert_eq!(bytecode[2], b's');
        assert_eq!(bytecode[3], b'}');
        assert_eq!(bytecode[4], b's'); // s44 type marker
        assert_eq!(bytecode[5], b'4');
        assert_eq!(bytecode[6], b'4');
        // 4 bytes of S44 data (i16 fraction + i16 exponent)
        assert_eq!(bytecode.len(), 15); // total length
                                        // halt opcode at end
        assert_eq!(bytecode[11], b'{');
        assert_eq!(bytecode[12], b'h');
        assert_eq!(bytecode[13], b'l');
        assert_eq!(bytecode[14], b'}');
    }

    #[test]
    fn test_chainable() {
        // Test that methods can be chained
        let _bytecode = Program::new()
            .ps_s44(0)
            .ps_s44(1)
            .ad()
            .dp()
            .ml()
            .hl()
            .build();
    }

    #[test]
    fn test_vm_integration() {
        // Test that builder-generated bytecode runs in the VM
        use crate::vm::VM;

        let bytecode = Program::new()
            .ps_s44(1) // push 1
            .ps_s44(1) // push 1
            .ad() // add → 2
            .ps_s44(1) // push 1
            .ad() // add → 3
            .hl() // halt
            .build();

        let mut vm = VM::new(bytecode);
        vm.run().unwrap();

        assert_eq!(vm.stack_depth(), 1);
        match vm.peek().unwrap() {
            vsf::types::VsfType::s44(s) => assert_eq!(*s, ScalarF4E4::from(3)),
            _ => panic!("Expected s44"),
        }
    }
}
