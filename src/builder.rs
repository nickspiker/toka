//! Rust DSL for building Toka bytecode programs
//!
//! Provides a type-safe, chainable API for constructing Toka bytecode using
//! mnemonic method names that match the opcodes exactly (e.g., `.po()` for push_one).
//!
//! # Example
//!
//! ```rust
//! use toka::builder::Program;
//!
//! // Draw a red square in the center
//! let bytecode = Program::new()
//!     .po()           // push_one (r=1.0)
//!     .pz()           // push_zero (g=0.0)
//!     .pz()           // push_zero (b=0.0)
//!     .cb()           // rgb (create colour)
//!     .ps_s44(0.25)   // push x
//!     .ps_s44(0.25)   // push y
//!     .ps_s44(0.5)    // push width
//!     .ps_s44(0.5)    // push height
//!     .fr()           // fill_rect
//!     .hl()           // halt
//!     .build();
//! ```

use spirix::ScalarF4E4;

/// Bytecode program builder with chainable opcode methods
///
/// Each method corresponds to a Toka opcode and appends the appropriate
/// bytes to the bytecode vector. The builder pattern allows for readable,
/// type-safe program construction with compile-time checking.
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
        self.bytecode.extend_from_slice(&[0x70, 0x73]); // ps
        self.bytecode.extend_from_slice(vsf_bytes);
        self
    }

    /// Push S44 value (encodes as VSF s44)
    /// VSF: {ps}s44[bytes]
    pub fn ps_s44(mut self, value: f64) -> Self {
        self.bytecode.extend_from_slice(&[0x70, 0x73]); // ps
        self.bytecode.push(b's');
        self.bytecode.push(b'4');
        self.bytecode.push(b'4');

        // Encode S44 as VSF
        let s44 = ScalarF4E4::from(value);
        self.bytecode.extend_from_slice(&s44.fraction.to_le_bytes());
        self.bytecode.extend_from_slice(&s44.exponent.to_le_bytes());
        self
    }

    /// Push u32 value (encodes as VSF u5)
    /// VSF: {ps}u5[bytes]
    pub fn ps_u32(mut self, value: u32) -> Self {
        self.bytecode.extend_from_slice(&[0x70, 0x73]); // ps
        self.bytecode.push(b'u');
        self.bytecode.push(b'5');
        self.bytecode.extend_from_slice(&value.to_le_bytes());
        self
    }

    /// Push string value (encodes as VSF x - UTF-8)
    /// VSF: {ps}x[len][bytes]
    pub fn ps_str(mut self, s: &str) -> Self {
        self.bytecode.extend_from_slice(&[0x70, 0x73]); // ps
        self.bytecode.push(b'x');
        let len = s.len() as u64;
        self.bytecode.extend_from_slice(&len.to_le_bytes());
        self.bytecode.extend_from_slice(s.as_bytes());
        self
    }

    /// Push zero (S44)
    /// VSF: {pz}
    pub fn pz(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x70, 0x7a]); // pz
        self
    }

    /// Push one (S44)
    /// VSF: {po}
    pub fn po(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x70, 0x6f]); // po
        self
    }

    /// Pop top of stack
    /// VSF: {pp}
    pub fn pp(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x70, 0x70]); // pp
        self
    }

    /// Duplicate top of stack
    /// VSF: {dp}
    pub fn dp(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x64, 0x70]); // dp
        self
    }

    /// Duplicate N items from stack
    /// VSF: {dn}[count:u]
    pub fn dn(mut self, count: u32) -> Self {
        self.bytecode.extend_from_slice(&[0x64, 0x6e]); // dn
        self.bytecode.extend_from_slice(&count.to_le_bytes());
        self
    }

    /// Swap top two stack items
    /// VSF: {sw}
    pub fn sw(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x73, 0x77]); // sw
        self
    }

    /// Rotate top N stack items
    /// VSF: {rt}[count:u]
    pub fn rt(mut self, count: u32) -> Self {
        self.bytecode.extend_from_slice(&[0x72, 0x74]); // rt
        self.bytecode.extend_from_slice(&count.to_le_bytes());
        self
    }

    // ==================== LOCAL VARIABLES ====================

    /// Allocate N local variables
    /// VSF: {la}[count:u]
    pub fn la(mut self, count: u32) -> Self {
        self.bytecode.extend_from_slice(&[0x6c, 0x61]); // la
        self.bytecode.extend_from_slice(&count.to_le_bytes());
        self
    }

    /// Get local variable at index
    /// VSF: {lg}[index:u]
    pub fn lg(mut self, index: u32) -> Self {
        self.bytecode.extend_from_slice(&[0x6c, 0x67]); // lg
        self.bytecode.extend_from_slice(&index.to_le_bytes());
        self
    }

    /// Set local variable at index
    /// VSF: {ls}[index:u]
    pub fn ls(mut self, index: u32) -> Self {
        self.bytecode.extend_from_slice(&[0x6c, 0x73]); // ls
        self.bytecode.extend_from_slice(&index.to_le_bytes());
        self
    }

    /// Tee local variable (set without popping)
    /// VSF: {lt}[index:u]
    pub fn lt(mut self, index: u32) -> Self {
        self.bytecode.extend_from_slice(&[0x6c, 0x74]); // lt
        self.bytecode.extend_from_slice(&index.to_le_bytes());
        self
    }

    // ==================== ARITHMETIC ====================

    /// Add: pop b, a; push a+b
    /// VSF: {ad}
    pub fn ad(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x61, 0x64]); // ad
        self
    }

    /// Subtract: pop b, a; push a-b
    /// VSF: {sb}
    pub fn sb(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x73, 0x62]); // sb
        self
    }

    /// Multiply: pop b, a; push a*b
    /// VSF: {ml}
    pub fn ml(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x6d, 0x6c]); // ml
        self
    }

    /// Divide: pop b, a; push a/b
    /// VSF: {dv}
    pub fn dv(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x64, 0x76]); // dv
        self
    }

    /// Reciprocal: pop a; push 1/a
    /// VSF: {rc}
    pub fn rc(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x72, 0x63]); // rc
        self
    }

    /// Modulo: pop b, a; push a%b
    /// VSF: {md}
    pub fn md(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x6d, 0x64]); // md
        self
    }

    /// Negate: pop a; push -a
    /// VSF: {ng}
    pub fn ng(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x6e, 0x67]); // ng
        self
    }

    /// Absolute value: pop a; push |a|
    /// VSF: {ab}
    pub fn ab(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x61, 0x62]); // ab
        self
    }

    /// Square root: pop a; push sqrt(a)
    /// VSF: {sq}
    pub fn sq(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x73, 0x71]); // sq
        self
    }

    /// Power: pop b, a; push a^b
    /// VSF: {pw}
    pub fn pw(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x70, 0x77]); // pw
        self
    }

    /// Minimum: pop b, a; push min(a, b)
    /// VSF: {mn}
    pub fn mn(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x6d, 0x6e]); // mn
        self
    }

    /// Maximum: pop b, a; push max(a, b)
    /// VSF: {mx}
    pub fn mx(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x6d, 0x78]); // mx
        self
    }

    /// Clamp: pop max, min, a; push clamp(a, min, max)
    /// VSF: {cm}
    pub fn cm(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x63, 0x6d]); // cm
        self
    }

    /// Floor: pop a; push floor(a)
    /// VSF: {fl}
    pub fn fl(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x66, 0x6c]); // fl
        self
    }

    /// Ceiling: pop a; push ceil(a)
    /// VSF: {cl}
    pub fn cl(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x63, 0x6c]); // cl
        self
    }

    /// Round: pop a; push round(a)
    /// VSF: {rn}
    pub fn rn(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x72, 0x6e]); // rn
        self
    }

    /// Fractional part: pop a; push frac(a)
    /// VSF: {fa}
    pub fn fa(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x66, 0x61]); // fa
        self
    }

    /// Linear interpolation: pop t, b, a; push a + t*(b-a)
    /// VSF: {lp}
    pub fn lp(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x6c, 0x70]); // lp
        self
    }

    // ==================== TRIGONOMETRY ====================

    /// Sine: pop a; push sin(a)
    /// VSF: {sn}
    pub fn sn(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x73, 0x6e]); // sn
        self
    }

    /// Cosine: pop a; push cos(a)
    /// VSF: {cs}
    pub fn cs(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x63, 0x73]); // cs
        self
    }

    /// Tangent: pop a; push tan(a)
    /// VSF: {tn}
    pub fn tn(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x74, 0x6e]); // tn
        self
    }

    /// Arcsine: pop a; push asin(a)
    /// VSF: {is}
    pub fn is(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x69, 0x73]); // is
        self
    }

    /// Arccosine: pop a; push acos(a)
    /// VSF: {ic}
    pub fn ic(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x69, 0x63]); // ic
        self
    }

    /// Arctangent: pop a; push atan(a)
    /// VSF: {ia}
    pub fn ia(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x69, 0x61]); // ia
        self
    }

    /// Arctangent2: pop x, y; push atan2(y, x)
    /// VSF: {a2}
    pub fn a2(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x61, 0x32]); // a2
        self
    }

    // ==================== COMPARISON ====================

    /// Equal: pop b, a; push 1.0 if a==b else 0.0
    /// VSF: {eq}
    pub fn eq(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x65, 0x71]); // eq
        self
    }

    /// Not equal: pop b, a; push 1.0 if a!=b else 0.0
    /// VSF: {ne}
    pub fn ne(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x6e, 0x65]); // ne
        self
    }

    /// Less than: pop b, a; push 1.0 if a<b else 0.0
    /// VSF: {lo}
    pub fn lo(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x6c, 0x6f]); // lo
        self
    }

    /// Less than or equal: pop b, a; push 1.0 if a<=b else 0.0
    /// VSF: {le}
    pub fn le(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x6c, 0x65]); // le
        self
    }

    /// Greater than: pop b, a; push 1.0 if a>b else 0.0
    /// VSF: {gt}
    pub fn gt(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x67, 0x74]); // gt
        self
    }

    /// Greater than or equal: pop b, a; push 1.0 if a>=b else 0.0
    /// VSF: {ge}
    pub fn ge(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x67, 0x65]); // ge
        self
    }

    // ==================== LOGIC ====================

    /// Logical AND: pop b, a; push 1.0 if both non-zero else 0.0
    /// VSF: {an}
    pub fn an(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x61, 0x6e]); // an
        self
    }

    /// Logical OR: pop b, a; push 1.0 if either non-zero else 0.0
    /// VSF: {or}
    pub fn or(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x6f, 0x72]); // or
        self
    }

    /// Logical NOT: pop a; push 1.0 if zero else 0.0
    /// VSF: {nt}
    pub fn nt(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x6e, 0x74]); // nt
        self
    }

    // ==================== TYPE SYSTEM ====================

    /// Type of: pop value; push type identifier string
    /// VSF: {ty}
    pub fn ty(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x74, 0x79]); // ty
        self
    }

    /// Convert to S44: pop value; push s44
    /// VSF: {ts}
    pub fn ts(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x74, 0x73]); // ts
        self
    }

    /// Convert to u32: pop value; push u32
    /// VSF: {tu}
    pub fn tu(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x74, 0x75]); // tu
        self
    }

    /// Convert to string: pop value; push string
    /// VSF: {tx}
    pub fn tx(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x74, 0x78]); // tx
        self
    }

    // ==================== DRAWING ====================

    /// Clear canvas: pop colour (u32 AARRGGBB)
    /// VSF: {cr}
    pub fn cr(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x63, 0x72]); // cr
        self
    }

    /// Fill rectangle: pop colour, h, w, y, x (viewport coords 0.0-1.0)
    /// VSF: {fr}
    pub fn fr(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x66, 0x72]); // fr
        self
    }

    /// Stroke rectangle: pop colour, h, w, y, x
    /// VSF: {sr}
    pub fn sr(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x73, 0x72]); // sr
        self
    }

    /// Fill circle: pop colour, radius, y, x
    /// VSF: {fc}
    pub fn fc(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x66, 0x63]); // fc
        self
    }

    /// Stroke circle: pop colour, radius, y, x
    /// VSF: {so}
    pub fn so(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x73, 0x6f]); // so
        self
    }

    /// Draw line: pop colour, y2, x2, y1, x1
    /// VSF: {dl}
    pub fn dl(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x64, 0x6c]); // dl
        self
    }

    /// Draw text: pop colour, size, y, x, text
    /// VSF: {dt}
    pub fn dt(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x64, 0x74]); // dt
        self
    }

    /// Set font: pop font_handle
    /// VSF: {sf}
    pub fn sf(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x73, 0x66]); // sf
        self
    }

    // ==================== COLOUR UTILITIES ====================

    /// RGBA: pop a, b, g, r (S44 0.0-1.0); push u32 AARRGGBB
    /// VSF: {ca}
    pub fn ca(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x63, 0x61]); // ca
        self
    }

    /// RGB: pop b, g, r (S44 0.0-1.0); push u32 AARRGGBB (alpha=1.0)
    /// VSF: {cb}
    pub fn cb(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x63, 0x62]); // cb
        self
    }

    /// Colour lerp: pop t, colour_b, colour_a; push interpolated colour
    /// VSF: {ci}
    pub fn ci(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x63, 0x69]); // ci
        self
    }

    /// HSLA: pop a, l, s, h; push u32 AARRGGBB
    /// VSF: {ch}
    pub fn ch(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x63, 0x68]); // ch
        self
    }

    // ==================== CONTROL FLOW ====================

    /// Call function at bytecode offset
    /// VSF: {cn}[offset:u]
    pub fn cn(mut self, offset: u32) -> Self {
        self.bytecode.extend_from_slice(&[0x63, 0x6e]); // cn
        self.bytecode.extend_from_slice(&offset.to_le_bytes());
        self
    }

    /// Call indirect: pop function_handle; call it
    /// VSF: {cd}
    pub fn cd(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x63, 0x64]); // cd
        self
    }

    /// Return from function (no value)
    /// VSF: {re}
    pub fn re(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x72, 0x65]); // re
        self
    }

    /// Return value from function
    /// VSF: {rv}
    pub fn rv(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x72, 0x76]); // rv
        self
    }

    /// Jump to bytecode offset
    /// VSF: {jm}[offset:u]
    pub fn jm(mut self, offset: u32) -> Self {
        self.bytecode.extend_from_slice(&[0x6a, 0x6d]); // jm
        self.bytecode.extend_from_slice(&offset.to_le_bytes());
        self
    }

    /// Jump if non-zero: pop condition; jump if truthy
    /// VSF: {ji}[offset:u]
    pub fn ji(mut self, offset: u32) -> Self {
        self.bytecode.extend_from_slice(&[0x6a, 0x69]); // ji
        self.bytecode.extend_from_slice(&offset.to_le_bytes());
        self
    }

    /// Jump if zero: pop condition; jump if falsy
    /// VSF: {jz}[offset:u]
    pub fn jz(mut self, offset: u32) -> Self {
        self.bytecode.extend_from_slice(&[0x6a, 0x7a]); // jz
        self.bytecode.extend_from_slice(&offset.to_le_bytes());
        self
    }

    // ==================== HALT ====================

    /// Halt execution
    /// VSF: {hl}
    pub fn hl(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x68, 0x6c]); // hl
        self
    }

    // ==================== DEBUG ====================

    /// Debug print: pop value; print to stdout
    /// VSF: {db}
    pub fn db(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x64, 0x62]); // db
        self
    }

    /// Debug stack: print entire stack
    /// VSF: {ds}
    pub fn ds(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x64, 0x73]); // ds
        self
    }

    /// No operation
    /// VSF: {np}
    pub fn np(mut self) -> Self {
        self.bytecode.extend_from_slice(&[0x6e, 0x70]); // np
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
            .po()  // push_one
            .po()  // push_one
            .ad()  // add
            .hl()  // halt
            .build();

        assert_eq!(
            bytecode,
            vec![
                0x70, 0x6f, // po
                0x70, 0x6f, // po
                0x61, 0x64, // ad
                0x68, 0x6c, // hl
            ]
        );
    }

    #[test]
    fn test_colour_creation() {
        // Create red colour (1.0, 0.0, 0.0)
        let bytecode = Program::new()
            .po()  // push_one (r)
            .pz()  // push_zero (g)
            .pz()  // push_zero (b)
            .cb()  // rgb
            .hl()  // halt
            .build();

        assert_eq!(
            bytecode,
            vec![
                0x70, 0x6f, // po
                0x70, 0x7a, // pz
                0x70, 0x7a, // pz
                0x63, 0x62, // cb
                0x68, 0x6c, // hl
            ]
        );
    }

    #[test]
    fn test_push_s44() {
        let bytecode = Program::new()
            .ps_s44(3.14)
            .hl()
            .build();

        // Should have: ps opcode (2) + s44 type marker (3) + fraction (2) + exponent (2) + hl opcode (2) = 11 bytes
        assert_eq!(bytecode[0], 0x70); // p
        assert_eq!(bytecode[1], 0x73); // s
        assert_eq!(bytecode[2], b's');  // s
        assert_eq!(bytecode[3], b'4');  // 4
        assert_eq!(bytecode[4], b'4');  // 4
        // 4 bytes of S44 data (i16 fraction + i16 exponent)
        assert_eq!(bytecode.len(), 11); // total length
        assert_eq!(bytecode[9], 0x68); // h (halt)
        assert_eq!(bytecode[10], 0x6c); // l
    }

    #[test]
    fn test_chainable() {
        // Test that methods can be chained
        let _bytecode = Program::new()
            .pz()
            .po()
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
            .po()  // push 1
            .po()  // push 1
            .ad()  // add → 2
            .po()  // push 1
            .ad()  // add → 3
            .hl()  // halt
            .build();

        let mut vm = VM::new(bytecode);
        vm.run().unwrap();

        assert_eq!(vm.stack_depth(), 1);
        let result = vm.peek().unwrap().to_s44().unwrap();
        let result_f64: f64 = result.into();
        assert_eq!(result_f64, 3.0);
    }
}
