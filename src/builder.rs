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
use vsf::types::{ButtonVariant, VsfType};

/// Cell content for mixed-content tables (text, buttons, text inputs).
///
/// Used with `draw_table_mixed` to embed interactive widgets directly in table cells.
/// Widget cells are drawn by the table renderer with proper padding and alignment.
pub enum CellData<'a> {
    /// Plain text cell
    Text(&'a str),
    /// Async image cell — the VM resolves `key` against its resource table (fetch-at-render),
    /// blitting the decoded pixels or a placeholder. Sized square to the cell by the VM.
    Image {
        /// Resource key (e.g. an avatar storage key) the host fetches over the VSF wire.
        key: &'a str,
    },
    /// Text cell with custom colour and optional size override
    Styled {
        /// Cell text content
        text: &'a str,
        /// VSF colour bytes
        colour: &'a [u8],
        /// Optional font size override (RU)
        size: Option<f32>,
    },
    /// Button cell — draws stroke_rect + centered label, pushes s44(1/0) for click
    Button {
        /// Button label text
        label: &'a str,
        /// VSF colour bytes for the button border and text
        colour: &'a [u8],
        /// Unique widget ID for hit detection
        id: u32,
        /// If non-empty, POSTs to this URL on click
        action: &'a str,
    },
    /// Text input cell — draws editable text field, pushes current text
    TextInput {
        /// Placeholder text shown when input is empty
        placeholder: &'a str,
        /// VSF colour bytes for the input border and text
        colour: &'a [u8],
        /// Unique widget ID for hit detection and state management
        id: u32,
    },
    /// Sub-table cell — nested table rendered within the parent cell
    SubTable {
        /// Column headers for the sub-table
        headers: &'a [&'a str],
        /// Row data — each row is a slice of CellData
        rows: &'a [&'a [CellData<'a>]],
        /// Optional per-column widths as fractions of sub-table width
        col_widths: Option<&'a [f32]>,
        /// Optional horizontal alignment string (l/c/r per column)
        h_align: Option<&'a str>,
        /// Optional border: (colour bytes, grid mask bytes)
        border: Option<(&'a [u8], &'a [u8])>,
        /// Optional header background colour bytes
        header_bg: Option<&'a [u8]>,
        /// Optional alternating row background colour bytes
        alt_row_bg: Option<&'a [u8]>,
        /// Optional cell padding in RU
        padding: Option<f32>,
    },
}

/// Build a `VsfType::roa` from sub-table cell data.
///
/// Children layout: [cells (cols × total_rows), settings tags...]
/// Settings are encoded as tag pairs: `l("tag"), value`.
fn build_roa_from_cell_data(
    headers: &[&str],
    rows: &[&[CellData]],
    col_widths: Option<&[f32]>,
    h_align: Option<&str>,
    border: Option<(&[u8], &[u8])>,
    header_bg: Option<&[u8]>,
    alt_row_bg: Option<&[u8]>,
    padding: Option<f32>,
) -> VsfType {
    let cols = headers.len();
    let total_rows = 1 + rows.len(); // header row + data rows
    let mut children = Vec::new();

    // Cell content: header row first, then data rows
    // Use VsfType::l (ASCII) not VsfType::x (Huffman) — roa is flattened without 'text' feature
    for h in headers {
        children.push(VsfType::a(h.to_string()));
    }
    for row in rows {
        for cell in *row {
            match cell {
                CellData::Text(s) => children.push(VsfType::a(s.to_string())),
                CellData::Image { key } => {
                    // v-wrapped 'i' → the VM parses this cell as CellContent::Image(key).
                    children.push(VsfType::v(b'i', key.as_bytes().to_vec()));
                }
                CellData::Styled { text: s, colour, size: sz } => {
                    // Encode as: text (l), optional size (s44), colour (ra)
                    // VM parser pops: colour first, then optional s44, then text
                    children.push(VsfType::a(s.to_string()));
                    if let Some(size_val) = sz {
                        children.push(VsfType::s44(ScalarF4E4::from_f32(*size_val)));
                    }
                    let mut ptr = 0;
                    let colour_vsf = vsf::parse::parse(colour, &mut ptr)
                        .expect("SubTable Styled: invalid colour bytes");
                    children.push(colour_vsf);
                }
                CellData::Button { label, colour, id, action } => {
                    let mut ptr = 0;
                    let colour_vsf = vsf::parse::parse(colour, &mut ptr)
                        .expect("SubTable Button: invalid colour bytes");
                    children.push(VsfType::rou(
                        CircleF4E4::ZERO, CircleF4E4::ZERO,
                        label.to_string(), ButtonVariant::Filled,
                        Box::new(colour_vsf),
                    ));
                    children.push(VsfType::u5(*id));
                    children.push(VsfType::a(action.to_string()));
                }
                CellData::TextInput { placeholder, colour, id } => {
                    let mut ptr = 0;
                    let colour_vsf = vsf::parse::parse(colour, &mut ptr)
                        .expect("SubTable TextInput: invalid colour bytes");
                    children.push(VsfType::roq(
                        CircleF4E4::ZERO, CircleF4E4::ZERO,
                        placeholder.to_string(),
                        Box::new(colour_vsf),
                    ));
                    children.push(VsfType::u5(*id));
                }
                CellData::SubTable { headers: sh, rows: sr, col_widths: sw, h_align: sa,
                                     border: sb, header_bg: shb, alt_row_bg: sab, padding: sp } => {
                    children.push(build_roa_from_cell_data(
                        sh, sr, *sw, *sa, *sb, *shb, *sab, *sp,
                    ));
                }
            }
        }
    }

    // Settings tags (after cell data)
    if let Some(widths) = col_widths {
        children.push(VsfType::a("x".to_string()));
        for &w in widths {
            children.push(VsfType::s44(ScalarF4E4::from_f32(w)));
        }
    }
    if let Some(align) = h_align {
        children.push(VsfType::a("j".to_string()));
        children.push(VsfType::a(align.to_string()));
    }
    if let Some((colour, mask)) = border {
        children.push(VsfType::a("b".to_string()));
        let mut ptr = 0;
        children.push(vsf::parse::parse(colour, &mut ptr)
            .expect("SubTable border: invalid colour bytes"));
        children.push(VsfType::v(b'b', mask.to_vec()));
    }
    if let Some(colour) = header_bg {
        children.push(VsfType::a("h".to_string()));
        let mut ptr = 0;
        children.push(vsf::parse::parse(colour, &mut ptr)
            .expect("SubTable header_bg: invalid colour bytes"));
    }
    if let Some(colour) = alt_row_bg {
        children.push(VsfType::a("a".to_string()));
        let mut ptr = 0;
        children.push(vsf::parse::parse(colour, &mut ptr)
            .expect("SubTable alt_row_bg: invalid colour bytes"));
    }
    if let Some(pad) = padding {
        children.push(VsfType::a("p".to_string()));
        children.push(VsfType::s44(ScalarF4E4::from_f32(pad)));
    }

    VsfType::roa(cols, total_rows, children)
}

/// Emit a VSF-encoded opcode: `{ab}` -> 4 bytes
#[inline]
fn emit_op(bytecode: &mut Vec<u8>, a: u8, b: u8) {
    bytecode.extend_from_slice(&VsfType::op(a, b).flatten());
}

/// Builder for bitpacked grid masks (per-segment border control in tables).
///
/// Creates a compact byte blob encoding which horizontal and vertical
/// border segments to draw. Bits are packed MSB-first, row-major.
///
/// # Example: header-only border (line under header row)
/// ```rust
/// let mask = GridMaskBuilder::new(3, 5) // 3 rows, 5 columns
///     .h_row(1, true)   // horizontal line under header (row gap 1)
///     .build();
/// ```
pub struct GridMaskBuilder {
    rows: usize,
    cols: usize,
    h_bits: Vec<u8>,  // (rows+1) × cols bits
    v_bits: Vec<u8>,  // rows × (cols+1) bits
    has_h: bool,
    has_v: bool,
}

impl GridMaskBuilder {
    /// Create a new grid mask for a table with the given dimensions.
    /// All segments start as OFF (no borders).
    pub fn new(rows: usize, cols: usize) -> Self {
        let h_bits_count = (rows + 1) * cols;
        let v_bits_count = rows * (cols + 1);
        Self {
            rows,
            cols,
            h_bits: vec![0u8; (h_bits_count + 7) / 8],
            v_bits: vec![0u8; (v_bits_count + 7) / 8],
            has_h: false,
            has_v: false,
        }
    }

    /// Create a grid mask with all segments ON (full grid).
    pub fn full(rows: usize, cols: usize) -> Self {
        let h_bits_count = (rows + 1) * cols;
        let v_bits_count = rows * (cols + 1);
        Self {
            rows,
            cols,
            h_bits: vec![0xFF; (h_bits_count + 7) / 8],
            v_bits: vec![0xFF; (v_bits_count + 7) / 8],
            has_h: true,
            has_v: true,
        }
    }

    fn set_h_bit(&mut self, row_gap: usize, col: usize, on: bool) {
        let idx = row_gap * self.cols + col;
        let byte = idx / 8;
        let bit = 7 - (idx % 8);
        if byte < self.h_bits.len() {
            if on {
                self.h_bits[byte] |= 1 << bit;
            } else {
                self.h_bits[byte] &= !(1 << bit);
            }
        }
        self.has_h = true;
    }

    fn set_v_bit(&mut self, row: usize, col_gap: usize, on: bool) {
        let idx = row * (self.cols + 1) + col_gap;
        let byte = idx / 8;
        let bit = 7 - (idx % 8);
        if byte < self.v_bits.len() {
            if on {
                self.v_bits[byte] |= 1 << bit;
            } else {
                self.v_bits[byte] &= !(1 << bit);
            }
        }
        self.has_v = true;
    }

    /// Set a single horizontal segment (line spanning one column at a row gap).
    pub fn h(mut self, row_gap: usize, col: usize, on: bool) -> Self {
        self.set_h_bit(row_gap, col, on);
        self
    }

    /// Set an entire horizontal row of segments (all columns at a row gap).
    pub fn h_row(mut self, row_gap: usize, on: bool) -> Self {
        for col in 0..self.cols {
            self.set_h_bit(row_gap, col, on);
        }
        self
    }

    /// Set a single vertical segment (line spanning one row at a column gap).
    pub fn v(mut self, row: usize, col_gap: usize, on: bool) -> Self {
        self.set_v_bit(row, col_gap, on);
        self
    }

    /// Set an entire vertical column of segments (all rows at a column gap).
    pub fn v_col(mut self, col_gap: usize, on: bool) -> Self {
        for row in 0..self.rows {
            self.set_v_bit(row, col_gap, on);
        }
        self
    }

    /// Explicitly enable horizontal mask with all bits off (no horizontal lines).
    pub fn no_h(mut self) -> Self {
        self.has_h = true;
        self
    }

    /// Explicitly enable vertical mask with all bits off (no vertical lines).
    pub fn no_v(mut self) -> Self {
        self.has_v = true;
        self
    }

    /// Set outer border only (top, bottom, left, right edges).
    pub fn outer(mut self, on: bool) -> Self {
        // Top and bottom horizontal rows
        for col in 0..self.cols {
            self.set_h_bit(0, col, on);
            self.set_h_bit(self.rows, col, on);
        }
        // Left and right vertical columns
        for row in 0..self.rows {
            self.set_v_bit(row, 0, on);
            self.set_v_bit(row, self.cols, on);
        }
        self
    }

    /// Build the grid mask byte blob for use with `draw_table_ex`.
    pub fn build(self) -> Vec<u8> {
        let mut out = Vec::new();
        let flags = (if self.has_h { 1u8 } else { 0 }) | (if self.has_v { 2u8 } else { 0 });
        out.push(flags);
        if self.has_h {
            out.extend_from_slice(&self.h_bits);
        }
        if self.has_v {
            out.extend_from_slice(&self.v_bits);
        }
        out
    }
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
            .extend_from_slice(&VsfType::a(s.to_string()).flatten());
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
        self.bytecode.extend_from_slice(&VsfType::a("x".to_string()).flatten());
        // Push left-align tag
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::a("l".to_string()).flatten());
        emit_op(&mut self.bytecode, b'd', b't');
        self
    }

    /// Draw text with wrap width and explicit alignment.
    /// align: 0=center, 1=left, 2=right
    pub fn dt_wrap_align(mut self, wrap_width: f32, align: u8) -> Self {
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::s44(ScalarF4E4::from_f32(wrap_width)).flatten());
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::a("x".to_string()).flatten());
        // Alignment tag
        match align {
            1 => {
                emit_op(&mut self.bytecode, b'p', b's');
                self.bytecode.extend_from_slice(&VsfType::a("l".to_string()).flatten());
            }
            2 => {
                emit_op(&mut self.bytecode, b'p', b's');
                self.bytecode.extend_from_slice(&VsfType::a("r".to_string()).flatten());
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
        self.bytecode.extend_from_slice(&VsfType::a("e".to_string()).flatten());
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
        self.bytecode.extend_from_slice(&VsfType::a("w".to_string()).flatten());
        emit_op(&mut self.bytecode, b'd', b'l');
        self
    }

    /// Draw line in pixel mode (always 1 device pixel).
    /// Stack before call: start(c44), end(c44), colour
    pub fn dl_pixel(mut self) -> Self {
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::a("p".to_string()).flatten());
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
        self.bytecode.extend_from_slice(&VsfType::a("w".to_string()).flatten());
        // Cap (both)
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::u3(1).flatten());
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::a("c".to_string()).flatten());
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
    /// `col_widths`: optional per-column widths in RU (0 = hidden, None = auto-fit)
    /// `header_bg`: optional header background colour (VSF bytes)
    /// `border`: optional (colour, grid_mask) pair — colour + bitpacked mask from GridMaskBuilder
    /// `alt_row_bg`: optional alternating row background colour (VSF bytes)
    /// `h_align`: per-column horizontal justify string (l/c/r per column)
    /// `v_align`: per-column vertical alignment string (t/m/b per column)
    pub fn draw_table(
        self,
        headers: &[&str],
        rows: &[&[&str]],
        text_colour: &[u8],
        col_widths: Option<&[f32]>,
        header_bg: Option<&[u8]>,
        border: Option<(&[u8], &[u8])>,
        alt_row_bg: Option<&[u8]>,
        h_align: Option<&str>,
        v_align: Option<&str>,
    ) -> Self {
        self.draw_table_inner(headers, rows, text_colour, col_widths, None,
            header_bg, border, alt_row_bg, h_align, v_align, None, None)
    }

    /// Like `draw_table` but column widths scale with canvas width.
    /// Each fraction is multiplied by `{cw}` at runtime: `{cw} frac {ml}`.
    /// Fractions are relative to viewport width in RU.
    pub fn draw_table_responsive(
        self,
        headers: &[&str],
        rows: &[&[&str]],
        text_colour: &[u8],
        col_fractions: &[f32],
        header_bg: Option<&[u8]>,
        border: Option<(&[u8], &[u8])>,
        alt_row_bg: Option<&[u8]>,
        h_align: Option<&str>,
        v_align: Option<&str>,
    ) -> Self {
        self.draw_table_inner(headers, rows, text_colour, None, Some(col_fractions),
            header_bg, border, alt_row_bg, h_align, v_align, None, None)
    }

    /// Same as `draw_table_responsive` but with explicit cell padding (in RU)
    pub fn draw_table_responsive_padded(
        self,
        headers: &[&str],
        rows: &[&[&str]],
        text_colour: &[u8],
        col_fractions: &[f32],
        header_bg: Option<&[u8]>,
        border: Option<(&[u8], &[u8])>,
        alt_row_bg: Option<&[u8]>,
        h_align: Option<&str>,
        v_align: Option<&str>,
        padding: f32,
    ) -> Self {
        self.draw_table_inner(headers, rows, text_colour, None, Some(col_fractions),
            header_bg, border, alt_row_bg, h_align, v_align, Some(padding), None)
    }

    /// Draw a table with some cells marked as widget slots.
    /// `query_cells` is a list of `(row, col)` pairs (0-indexed, row 0 = header).
    /// After drawing, the table pushes `font, pos(c44), size(c44), colour` for each
    /// queried cell onto the stack (first cell on top). Use with `cell_button()` etc.
    pub fn draw_table_widget(
        self,
        headers: &[&str],
        rows: &[&[&str]],
        text_colour: &[u8],
        col_widths: Option<&[f32]>,
        header_bg: Option<&[u8]>,
        border: Option<(&[u8], &[u8])>,
        alt_row_bg: Option<&[u8]>,
        h_align: Option<&str>,
        v_align: Option<&str>,
        query_cells: &[(usize, usize)],
    ) -> Self {
        self.draw_table_inner(headers, rows, text_colour, col_widths, None,
            header_bg, border, alt_row_bg, h_align, v_align, None, Some(query_cells))
    }

    /// Draw a responsive table with widget slots and optional padding.
    pub fn draw_table_responsive_widget(
        self,
        headers: &[&str],
        rows: &[&[&str]],
        text_colour: &[u8],
        col_fractions: &[f32],
        header_bg: Option<&[u8]>,
        border: Option<(&[u8], &[u8])>,
        alt_row_bg: Option<&[u8]>,
        h_align: Option<&str>,
        v_align: Option<&str>,
        query_cells: &[(usize, usize)],
    ) -> Self {
        self.draw_table_inner(headers, rows, text_colour, None, Some(col_fractions),
            header_bg, border, alt_row_bg, h_align, v_align, None, Some(query_cells))
    }

    /// Draw a responsive table with widget slots and explicit padding.
    pub fn draw_table_responsive_widget_padded(
        self,
        headers: &[&str],
        rows: &[&[&str]],
        text_colour: &[u8],
        col_fractions: &[f32],
        header_bg: Option<&[u8]>,
        border: Option<(&[u8], &[u8])>,
        alt_row_bg: Option<&[u8]>,
        h_align: Option<&str>,
        v_align: Option<&str>,
        padding: f32,
        query_cells: &[(usize, usize)],
    ) -> Self {
        self.draw_table_inner(headers, rows, text_colour, None, Some(col_fractions),
            header_bg, border, alt_row_bg, h_align, v_align, Some(padding), Some(query_cells))
    }

    fn draw_table_inner(
        mut self,
        headers: &[&str],
        rows: &[&[&str]],
        text_colour: &[u8],
        col_widths: Option<&[f32]>,
        col_fractions: Option<&[f32]>,
        header_bg: Option<&[u8]>,
        border: Option<(&[u8], &[u8])>,
        alt_row_bg: Option<&[u8]>,
        h_align: Option<&str>,
        v_align: Option<&str>,
        padding: Option<f32>,
        query_cells: Option<&[(usize, usize)]>,
    ) -> Self {
        let cols = headers.len();
        let total_rows = 1 + rows.len();

        // Push text colour (base param, like dt)
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(text_colour);

        // Tags — pushed top-down, VM pops top-first
        // Query cells tag (optional) — push geometry for these cells after drawing
        if let Some(qc) = query_cells {
            // Push (row, col) pairs then count, then "q" tag
            // VM pops: "q", count, then count×(row, col)
            for &(row, col) in qc {
                self = self.ps_u32(row as u32);
                self = self.ps_u32(col as u32);
            }
            self = self.ps_u32(qc.len() as u32);
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a("q".to_string()).flatten());
        }

        // Alignment tags (optional)
        if let Some(va) = v_align {
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a(va.to_string()).flatten());
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a("v".to_string()).flatten());
        }
        if let Some(ha) = h_align {
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a(ha.to_string()).flatten());
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a("j".to_string()).flatten());
        }
        // Padding (optional)
        if let Some(pad) = padding {
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::s44(ScalarF4E4::from_f32(pad)).flatten());
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a("p".to_string()).flatten());
        }
        // Border: grid mask + colour (always paired)
        if let Some((colour, mask)) = border {
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::v(b'b', mask.to_vec()).flatten());
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a("g".to_string()).flatten());
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(colour);
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a("b".to_string()).flatten());
        }
        // Styling tags (optional)
        if let Some(alt) = alt_row_bg {
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(alt);
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a("a".to_string()).flatten());
        }
        if let Some(header) = header_bg {
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(header);
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a("h".to_string()).flatten());
        }
        // Per-column widths
        if let Some(ws) = col_widths {
            for &w in ws {
                emit_op(&mut self.bytecode, b'p', b's');
                self.bytecode.extend_from_slice(&VsfType::s44(ScalarF4E4::from_f32(w)).flatten());
            }
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a("x".to_string()).flatten());
        } else if let Some(fracs) = col_fractions {
            // Responsive widths — emit {cw} frac {ml} per column
            for &frac in fracs {
                emit_op(&mut self.bytecode, b'c', b'w'); // {cw}
                emit_op(&mut self.bytecode, b'p', b's');
                self.bytecode.extend_from_slice(&VsfType::s44(ScalarF4E4::from_f32(frac)).flatten());
                emit_op(&mut self.bytecode, b'm', b'l'); // {ml}
            }
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a("x".to_string()).flatten());
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
        self.bytecode.extend_from_slice(&VsfType::a("d".to_string()).flatten());

        // Row count
        self = self.ps_u32(total_rows as u32);
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::a("r".to_string()).flatten());

        // Column count (must come before 'r' and 'd' in parse order, so pushed last)
        self = self.ps_u32(cols as u32);
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::a("c".to_string()).flatten());

        // Emit draw_table opcode
        emit_op(&mut self.bytecode, b't', b'b');
        self
    }

    /// Draw a table with mixed cell content (text, buttons, text inputs).
    /// Headers are always text. Data cells can be any `CellData` type.
    /// Widget cells push results onto the stack after the table:
    /// - Button: s44(1) if clicked, s44(0) otherwise
    /// - TextInput: current text content (string)
    pub fn draw_table_mixed(
        mut self,
        headers: &[&str],
        rows: &[&[CellData]],
        text_colour: &[u8],
        col_fractions: &[f32],
        header_bg: Option<&[u8]>,
        border: Option<(&[u8], &[u8])>,
        alt_row_bg: Option<&[u8]>,
        h_align: Option<&str>,
        v_align: Option<&str>,
        padding: Option<f32>,
    ) -> Self {
        let cols = headers.len();
        let total_rows = 1 + rows.len();

        // Push text colour (base param)
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(text_colour);

        // Tags — pushed top-down, VM pops top-first
        if let Some(va) = v_align {
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a(va.to_string()).flatten());
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a("v".to_string()).flatten());
        }
        if let Some(ha) = h_align {
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a(ha.to_string()).flatten());
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a("j".to_string()).flatten());
        }
        if let Some(pad) = padding {
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::s44(ScalarF4E4::from_f32(pad)).flatten());
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a("p".to_string()).flatten());
        }
        if let Some((colour, mask)) = border {
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::v(b'b', mask.to_vec()).flatten());
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a("g".to_string()).flatten());
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(colour);
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a("b".to_string()).flatten());
        }
        if let Some(alt) = alt_row_bg {
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(alt);
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a("a".to_string()).flatten());
        }
        if let Some(header) = header_bg {
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(header);
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::a("h".to_string()).flatten());
        }
        // Responsive column widths
        for &frac in col_fractions {
            emit_op(&mut self.bytecode, b'c', b'w');
            emit_op(&mut self.bytecode, b'p', b's');
            self.bytecode.extend_from_slice(&VsfType::s44(ScalarF4E4::from_f32(frac)).flatten());
            emit_op(&mut self.bytecode, b'm', b'l');
        }
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::a("x".to_string()).flatten());

        // Cell data — headers (always text), then data rows (mixed)
        for h in headers {
            self = self.ps_str(h);
        }
        for row in rows {
            for cell in *row {
                match cell {
                    CellData::Text(s) => {
                        self = self.ps_str(s);
                    }
                    CellData::Image { key } => {
                        // Push v-wrapped 'i' with the key → VM parses CellContent::Image.
                        emit_op(&mut self.bytecode, b'p', b's');
                        self.bytecode
                            .extend_from_slice(&VsfType::v(b'i', key.as_bytes().to_vec()).flatten());
                    }
                    CellData::Styled { text, colour, size: sz } => {
                        // Push: text, then optional size, then colour on top
                        self = self.ps_str(text);
                        if let Some(s) = sz {
                            emit_op(&mut self.bytecode, b'p', b's');
                            self.bytecode.extend_from_slice(&VsfType::s44(ScalarF4E4::from_f32(*s)).flatten());
                        }
                        self = self.ps(colour);
                    }
                    CellData::Button { label, colour, id, action } => {
                        // Push: id, action_url, then rou drawable (label + colour)
                        self = self.ps_u32(*id);
                        self = self.ps_str(action);
                        // Parse colour bytes into VsfType, embed in rou drawable
                        let mut ptr = 0;
                        let colour_vsf = vsf::parse::parse(colour, &mut ptr)
                            .expect("CellData::Button: invalid colour bytes");
                        let rou = VsfType::rou(
                            CircleF4E4::ZERO, CircleF4E4::ZERO,
                            label.to_string(), ButtonVariant::Filled,
                            Box::new(colour_vsf),
                        );
                        emit_op(&mut self.bytecode, b'p', b's');
                        self.bytecode.extend_from_slice(&rou.flatten());
                    }
                    CellData::TextInput { placeholder, colour, id } => {
                        // Push: id, then roq drawable (placeholder + colour)
                        self = self.ps_u32(*id);
                        // Parse colour bytes into VsfType, embed in roq drawable
                        let mut ptr = 0;
                        let colour_vsf = vsf::parse::parse(colour, &mut ptr)
                            .expect("CellData::TextInput: invalid colour bytes");
                        let roq = VsfType::roq(
                            CircleF4E4::ZERO, CircleF4E4::ZERO,
                            placeholder.to_string(),
                            Box::new(colour_vsf),
                        );
                        emit_op(&mut self.bytecode, b'p', b's');
                        self.bytecode.extend_from_slice(&roq.flatten());
                    }
                    CellData::SubTable { headers, rows: sub_rows, col_widths, h_align,
                                         border, header_bg, alt_row_bg, padding } => {
                        let roa = build_roa_from_cell_data(
                            headers, sub_rows, *col_widths, *h_align,
                            *border, *header_bg, *alt_row_bg, *padding,
                        );
                        emit_op(&mut self.bytecode, b'p', b's');
                        self.bytecode.extend_from_slice(&roa.flatten());
                    }
                }
            }
        }
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::a("d".to_string()).flatten());

        // Row count
        self = self.ps_u32(total_rows as u32);
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::a("r".to_string()).flatten());

        // Column count
        self = self.ps_u32(cols as u32);
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(&VsfType::a("c".to_string()).flatten());

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

    /// Push canvas width (in RU)
    /// VSF: {cw}
    pub fn cw(mut self) -> Self {
        emit_op(&mut self.bytecode, b'c', b'w');
        self
    }

    /// Push canvas height (in RU)
    /// VSF: {ch}
    pub fn ch(mut self) -> Self {
        emit_op(&mut self.bytecode, b'c', b'h');
        self
    }

    /// Push aspect ratio (width / height, dimensionless)
    /// VSF: {ar}
    pub fn ar(mut self) -> Self {
        emit_op(&mut self.bytecode, b'a', b'r');
        self
    }

    // ==================== INTERACTIVE WIDGETS ====================

    /// Button: draws 1px rect with label, pushes clicked bool
    /// Stack: font, pos(c44), size(c44), label(string), colour, id(u)
    /// VSF: {bt}
    pub fn bt(mut self) -> Self { emit_op(&mut self.bytecode, b'b', b't'); self }

    /// Text input: draws 1px rect with editable text, pushes current text
    /// Stack: font, pos(c44), size(c44), placeholder(string), colour, id(u)
    /// VSF: {ti}
    pub fn ti(mut self) -> Self { emit_op(&mut self.bytecode, b't', b'i'); self }

    /// Convenience: emit a button with all params
    /// Pushes font(local), pos, size, label, colour, id, then {bt}
    pub fn draw_button(
        self,
        label: &str,
        pos: (f32, f32),
        size: (f32, f32),
        colour: &[u8],
        id: u32,
    ) -> Self {
        self.lg(0) // font from local 0
            .ps_c44(pos.0, pos.1)
            .ps_c44(size.0, size.1)
            .ps_str(label)
            .ps(colour)
            .ps_u32(id)
            .bt()
    }

    /// Convenience: emit a text input with all params
    /// Pushes font(local), pos, size, placeholder, colour, id, then {ti}
    pub fn draw_text_input(
        self,
        placeholder: &str,
        pos: (f32, f32),
        size: (f32, f32),
        colour: &[u8],
        id: u32,
    ) -> Self {
        self.lg(0) // font from local 0
            .ps_c44(pos.0, pos.1)
            .ps_c44(size.0, size.1)
            .ps_str(placeholder)
            .ps(colour)
            .ps_u32(id)
            .ti()
    }

    /// Action: pop URL, pop condition; if condition != 0, queue URL for JS POST
    /// VSF: {ac}
    pub fn ac(mut self) -> Self { emit_op(&mut self.bytecode, b'a', b'c'); self }

    /// String concat: pop b, pop a; push a+b
    /// VSF: {sc}
    pub fn sc(mut self) -> Self { emit_op(&mut self.bytecode, b's', b'c'); self }

    /// Convenience: emit a button that triggers an HTTP POST action on click
    /// Combines draw_button + action — pushes button, then conditionally queues URL
    pub fn draw_action_button(
        self,
        label: &str,
        action_url: &str,
        pos: (f32, f32),
        size: (f32, f32),
        colour: &[u8],
        id: u32,
    ) -> Self {
        self.draw_button(label, pos, size, colour, id)
            .ps_str(action_url)
            .ac()
    }

    /// Draw a button inside a table cell (consumes geometry pushed by 'q' tag).
    /// Stack input: font(blob), pos(c44), size(c44), colour
    /// Rearranges to: font, pos, size, label, colour, id → {bt}
    /// Pushes: s44(1) if clicked, s44(0) otherwise
    pub fn cell_button(self, label: &str, id: u32) -> Self {
        // Stack: font, pos, size, colour
        // Need:  font, pos, size, label, colour, id
        self.ps_str(label) // font, pos, size, colour, label
            .sw()          // font, pos, size, label, colour
            .ps_u32(id)    // font, pos, size, label, colour, id
            .bt()          // draws button, pushes clicked(s44)
    }

    /// Draw a button inside a table cell that triggers an HTTP POST action.
    /// Consumes geometry pushed by 'q' tag. Pushes nothing (action + click consumed).
    pub fn cell_action_button(self, label: &str, action_url: &str, id: u32) -> Self {
        self.cell_button(label, id)
            .ps_str(action_url)
            .ac()
    }

    /// Draw a button inside a table cell with a custom colour (overrides table colour).
    /// Stack input: font(blob), pos(c44), size(c44), table_colour
    /// Drops table_colour, uses provided colour instead.
    pub fn cell_button_coloured(mut self, label: &str, colour: &[u8], id: u32) -> Self {
        // Stack: font, pos, size, table_colour
        // Drop table_colour, push custom colour
        self = self.pp(); // font, pos, size  (table_colour popped)
        self = self.ps_str(label); // font, pos, size, label
        emit_op(&mut self.bytecode, b'p', b's');
        self.bytecode.extend_from_slice(colour);  // font, pos, size, label, colour
        self = self.ps_u32(id); // font, pos, size, label, colour, id
        self.bt()
    }

    /// Draw an action button inside a table cell with a custom colour.
    pub fn cell_action_button_coloured(self, label: &str, action_url: &str, colour: &[u8], id: u32) -> Self {
        self.cell_button_coloured(label, colour, id)
            .ps_str(action_url)
            .ac()
    }

    /// Draw a text input inside a table cell (consumes geometry pushed by 'q' tag).
    /// Stack input: font(blob), pos(c44), size(c44), colour
    /// Rearranges to: font, pos, size, placeholder, colour, id → {ti}
    /// Pushes: current text content (string)
    pub fn cell_text_input(self, placeholder: &str, id: u32) -> Self {
        // Stack: font, pos, size, colour
        // Need:  font, pos, size, placeholder, colour, id
        self.ps_str(placeholder)
            .sw()
            .ps_u32(id)
            .ti()
    }

    // ==================== ERROR HANDLING ====================

    /// Guard — pop condition; halt if zero
    /// VSF: {gd}
    pub fn gd(mut self) -> Self {
        emit_op(&mut self.bytecode, b'g', b'd');
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
