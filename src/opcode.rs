//! Toka VM Opcodes
//!
//! All opcodes are encoded as VSF `{}` blocks with two lowercase letters.
//! Format: {ab} where a,b ∈ [a-z]
//!
//! This gives 676 possible opcodes with no collision with other VSF types.

use std::fmt;

/// Toka VM instruction opcodes
///
/// Each opcode is represented by a two-character lowercase identifier.
/// The VSF bytecode parser matches `{ab}` patterns to decode instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Opcode {
    // ==================== STACK MANIPULATION ====================
    /// Push VSF-encoded constant to stack
    /// VSF: {ps}[value]
    push,

    /// Discard top of stack
    /// VSF: {pp}
    pop,

    /// Duplicate top value
    /// VSF: {dp}
    dup,

    /// Duplicate value at depth N
    /// VSF: {dn}[depth:u]
    dup_n,

    /// Swap top two values
    /// VSF: {sw}
    swap,

    /// Rotate top three (a b c → b c a)
    /// VSF: {rt}
    rotate,

    // ==================== LOCAL VARIABLES ====================
    /// Allocate N local variable slots
    /// VSF: {la}[n:u]
    local_alloc,

    /// Push local[id] to stack
    /// VSF: {lg}[id:u]
    local_get,

    /// Pop stack to local[id]
    /// VSF: {ls}[id:u]
    local_set,

    /// Copy top to local[id] without popping
    /// VSF: {lt}[id:u]
    local_tee,

    // ==================== ARITHMETIC (Spirix S44) ====================
    /// Pop b, a; push a + b
    /// VSF: {ad}
    add,

    /// Pop b, a; push a - b
    /// VSF: {sb}
    sub,

    /// Pop b, a; push a * b
    /// VSF: {ml}
    mul,

    /// Pop b, a; push a / b
    /// VSF: {dv}
    div,

    /// Pop a; push 1/a (faster than div)
    /// VSF: {rc}
    recip,

    /// Pop b, a; push a % b
    /// VSF: {md}
    mod_,

    /// Pop a; push -a
    /// VSF: {ng}
    neg,

    /// Pop a; push |a|
    /// VSF: {ab}
    abs,

    /// Pop a; push √a
    /// VSF: {sq}
    sqrt,

    /// Pop exp, base; push base^exp
    /// VSF: {pw}
    pow,

    /// Pop b, a; push min(a, b)
    /// VSF: {mn}
    min,

    /// Pop b, a; push max(a, b)
    /// VSF: {mx}
    max,

    /// Pop max, min, value; push clamped value
    /// VSF: {cm}
    clamp,

    /// Pop a; push ⌊a⌋
    /// VSF: {fl}
    floor,

    /// Pop a; push ⌈a⌉
    /// VSF: {cl}
    ceil,

    /// Pop a; push round(a)
    /// VSF: {rn}
    round,

    /// Pop a; push fractional part
    /// VSF: {fa}
    frac,

    /// Pop t, b, a; push a + t*(b-a)
    /// VSF: {lp}
    lerp,

    // ==================== TRIGONOMETRY ====================
    /// Pop a; push sin(a)
    /// VSF: {sn}
    sin,

    /// Pop a; push cos(a)
    /// VSF: {cs}
    cos,

    /// Pop a; push tan(a)
    /// VSF: {tn}
    tan,

    /// Pop a; push arcsin(a)
    /// VSF: {is}
    asin,

    /// Pop a; push arccos(a)
    /// VSF: {ic}
    acos,

    /// Pop a; push arctan(a)
    /// VSF: {ia}
    atan,

    /// Pop y, x; push atan2(y, x)
    /// VSF: {a2}
    atan2,

    // ==================== COMPARISON ====================
    /// Pop b, a; push 1.0 if a == b else 0.0
    /// VSF: {eq}
    eq,

    /// Pop b, a; push 1.0 if a != b else 0.0
    /// VSF: {ne}
    ne,

    /// Pop b, a; push 1.0 if a < b else 0.0
    /// VSF: {lo}
    lt,

    /// Pop b, a; push 1.0 if a ≤ b else 0.0
    /// VSF: {le}
    le,

    /// Pop b, a; push 1.0 if a > b else 0.0
    /// VSF: {gt}
    gt,

    /// Pop b, a; push 1.0 if a ≥ b else 0.0
    /// VSF: {ge}
    ge,

    // ==================== LOGIC (Logical/Boolean) ====================
    /// Logical AND: pop b, a; push 1 if both truthy else 0
    /// VSF: {an}
    and,

    /// Logical OR: pop b, a; push 1 if either truthy else 0
    /// VSF: {or}
    or,

    /// Logical NOT: pop a; push 1 if falsy else 0
    /// VSF: {nt}
    not,

    // ==================== BITWISE ====================
    /// Bitwise AND: pop b, a; push a & b (bit-level AND)
    /// VSF: {ba}
    bit_and,

    /// Bitwise OR: pop b, a; push a | b (bit-level OR)
    /// VSF: {bo}
    bit_or,

    /// Bitwise XOR: pop b, a; push a ^ b (bit-level XOR)
    /// VSF: {bx}
    bit_xor,

    /// Bitwise NOT: pop a; push ~a (bit-level complement)
    /// VSF: {bn}
    bit_not,

    // ==================== TYPE SYSTEM ====================
    /// Pop value; push type identifier as string (d-type)
    /// VSF: {ty}
    typeof_,

    /// Pop value; convert to s44, push result
    /// VSF: {ts}
    to_s44,

    /// Pop value; convert to u5 (u32), push result
    /// VSF: {tu}
    to_u32,

    /// Pop value; convert to x (UTF-8 string), push result
    /// VSF: {tx}
    to_string,

    // ==================== ARRAYS ====================
    /// Pop count; create array with count elements from stack
    /// VSF: {aw}
    array_new,

    /// Pop array; push length (S44)
    /// VSF: {al}
    array_len,

    /// Pop index, array; push element
    /// VSF: {ag}
    array_get,

    /// Pop value, index, array; modify in place
    /// VSF: {ae}
    array_set,

    /// Pop value, array; append element
    /// VSF: {ap}
    array_push,

    /// Pop array; push & remove last element
    /// VSF: {ao}
    array_pop,

    // ==================== STRINGS ====================
    /// Pop b, a; push a + b
    /// VSF: {sc}
    string_concat,

    /// Pop string; push byte length (S44)
    /// VSF: {sl}
    string_len,

    /// Pop end, start, string; push substring
    /// VSF: {ss}
    string_slice,

    // ==================== HANDLES ====================
    /// Pop handle; push referenced value
    /// VSF: {hr}
    handle_read,

    /// Pop value, handle; write to handle (if writable)
    /// VSF: {hw}
    handle_write,

    /// Pop args..., handle; invoke capability function
    /// VSF: {hc}
    handle_call,

    /// Pop handle; push metadata struct
    /// VSF: {hq}
    handle_query,

    // ==================== DRAWING ====================
    /// Pop rgba_u32; fill entire viewport
    /// VSF: {cr}
    clear,

    /// Pop rgba_u32, h, w, y, x; fill rectangle
    /// VSF: {fr}
    fill_rect,

    /// Pop rgba_u32, stroke_w, h, w, y, x; stroke outline
    /// VSF: {sr}
    stroke_rect,

    /// Pop rgba_u32, r, cy, cx; fill circle
    /// VSF: {fc}
    fill_circle,

    /// Pop rgba_u32, stroke_w, r, cy, cx; stroke outline
    /// VSF: {so}
    stroke_circle,

    /// Pop rgba_u32, stroke_w, y2, x2, y1, x1; draw line
    /// VSF: {dl}
    draw_line,

    /// Pop rgba_u32, size, y, x, string; render text
    /// VSF: {dt}
    draw_text,

    /// Pop font_handle; set current font
    /// VSF: {sf}
    set_font,

    // ==================== COLOUR UTILITIES ====================
    /// Pop a, b, g, r (S44 0.0-1.0); push u32 RGBA
    /// VSF: {ca}
    rgba,

    /// Pop b, g, r (S44 0.0-1.0); push u32 RGBA (alpha=1.0)
    /// VSF: {cb}
    rgb,

    // ==================== CONTROL FLOW ====================
    /// Call function at bytecode offset
    /// VSF: {cn}[offset:u]
    call,

    /// Pop function_handle; call it
    /// VSF: {cd}
    call_indirect,

    /// Return from function (no value)
    /// VSF: {re}
    return_,

    /// Pop value; return it from function
    /// VSF: {rv}
    return_value,

    /// Unconditional jump to offset
    /// VSF: {jm}[offset:u]
    jump,

    /// Pop condition; jump if non-zero
    /// VSF: {ji}[offset:u]
    jump_if,

    /// Pop condition; jump if zero
    /// VSF: {jz}[offset:u]
    jump_zero,

    // ==================== RANDOM NUMBERS ====================
    /// Push random S44 in [-1.0, 1.0]
    /// VSF: {rd}
    random,

    /// Push random S44 (Gaussian distribution)
    /// VSF: {rg}
    random_gauss,

    /// Pop max, min; push random S44 in [min, max]
    /// VSF: {rr}
    random_range,

    // ==================== CRYPTOGRAPHY ====================
    /// Pop data; push 32-byte BLAKE3 hash
    /// VSF: {bh}
    blake3,

    // ==================== TIME ====================
    /// Push current Unix timestamp (S44 seconds)
    /// VSF: {tm}
    timestamp,

    // ==================== ERROR HANDLING ====================
    /// Pop condition; halt if zero
    /// VSF: {ar}
    assert,

    /// Stop execution immediately
    /// VSF: {hl}
    halt,

    // ==================== DEBUG ====================
    /// Pop value; print to debug console
    /// VSF: {db}
    debug_print,

    /// Print entire stack state
    /// VSF: {ds}
    debug_stack,

    /// No operation
    /// VSF: {np}
    nop,
}

// Helper to pack two bytes into u16 for efficient matching
const fn pack(a: u8, b: u8) -> u16 {
    ((a as u16) << 8) | (b as u16)
}

impl Opcode {
    /// Parse opcode from packed u16
    ///
    /// Efficient single-match lookup for all opcodes.
    /// Format: (first_letter << 8) | second_letter
    pub fn from_u16(op: u16) -> Option<Self> {

        match op {
            // Stack manipulation
            0x7073 => Some(Self::push),   // ps
            0x7070 => Some(Self::pop),    // pp
            0x6470 => Some(Self::dup),    // dp
            0x646e => Some(Self::dup_n),  // dn
            0x7377 => Some(Self::swap),   // sw
            0x7274 => Some(Self::rotate), // rt

            // Local variables
            0x6c61 => Some(Self::local_alloc), // la
            0x6c67 => Some(Self::local_get),   // lg
            0x6c73 => Some(Self::local_set),   // ls
            0x6c74 => Some(Self::local_tee),   // lt

            // Arithmetic
            0x6164 => Some(Self::add),   // ad
            0x7362 => Some(Self::sub),   // sb
            0x6d6c => Some(Self::mul),   // ml
            0x6476 => Some(Self::div),   // dv
            0x7263 => Some(Self::recip), // rc
            0x6d64 => Some(Self::mod_),  // md
            0x6e67 => Some(Self::neg),   // ng
            0x6162 => Some(Self::abs),   // ab
            0x7371 => Some(Self::sqrt),  // sq
            0x7077 => Some(Self::pow),   // pw
            0x6d6e => Some(Self::min),   // mn
            0x6d78 => Some(Self::max),   // mx
            0x636d => Some(Self::clamp), // cm
            0x666c => Some(Self::floor), // fl
            0x636c => Some(Self::ceil),  // cl
            0x726e => Some(Self::round), // rn
            0x6661 => Some(Self::frac),  // fa
            0x6c70 => Some(Self::lerp),  // lp

            // Trigonometry
            0x736e => Some(Self::sin),   // sn
            0x6373 => Some(Self::cos),   // cs
            0x746e => Some(Self::tan),   // tn
            0x6973 => Some(Self::asin),  // is
            0x6963 => Some(Self::acos),  // ic
            0x6961 => Some(Self::atan),  // ia
            0x6132 => Some(Self::atan2), // a2

            // Comparison
            0x6571 => Some(Self::eq), // eq
            0x6e65 => Some(Self::ne), // ne
            0x6c6f => Some(Self::lt), // lo
            0x6c65 => Some(Self::le), // le
            0x6774 => Some(Self::gt), // gt
            0x6765 => Some(Self::ge), // ge

            // Logic
            0x616e => Some(Self::and), // an
            0x6f72 => Some(Self::or),  // or
            0x6e74 => Some(Self::not), // nt

            // Type system
            0x7479 => Some(Self::typeof_),   // ty
            0x7473 => Some(Self::to_s44),    // ts
            0x7475 => Some(Self::to_u32),    // tu
            0x7478 => Some(Self::to_string), // tx

            // Arrays
            0x6177 => Some(Self::array_new),  // aw
            0x616c => Some(Self::array_len),  // al
            0x6167 => Some(Self::array_get),  // ag
            0x6165 => Some(Self::array_set),  // ae
            0x6170 => Some(Self::array_push), // ap
            0x616f => Some(Self::array_pop),  // ao

            // Strings
            0x7363 => Some(Self::string_concat), // sc
            0x736c => Some(Self::string_len),    // sl
            0x7373 => Some(Self::string_slice),  // ss

            // Handles
            0x6872 => Some(Self::handle_read),  // hr
            0x6877 => Some(Self::handle_write), // hw
            0x6863 => Some(Self::handle_call),  // hc
            0x6871 => Some(Self::handle_query), // hq

            // Drawing
            0x6372 => Some(Self::clear),         // cr
            0x6672 => Some(Self::fill_rect),     // fr
            0x7372 => Some(Self::stroke_rect),   // sr
            0x6663 => Some(Self::fill_circle),   // fc
            0x736f => Some(Self::stroke_circle), // so
            0x646c => Some(Self::draw_line),     // dl
            0x6474 => Some(Self::draw_text),     // dt
            0x7366 => Some(Self::set_font),      // sf

            // Colour utilities
            0x6361 => Some(Self::rgba), // ca
            0x6362 => Some(Self::rgb),  // cb

            // Control flow
            0x636e => Some(Self::call),          // cn
            0x6364 => Some(Self::call_indirect), // cd
            0x7265 => Some(Self::return_),       // re
            0x7276 => Some(Self::return_value),  // rv
            0x6a6d => Some(Self::jump),          // jm
            0x6a69 => Some(Self::jump_if),       // ji
            0x6a7a => Some(Self::jump_zero),     // jz

            // Random numbers
            0x7264 => Some(Self::random),       // rd
            0x7267 => Some(Self::random_gauss), // rg
            0x7272 => Some(Self::random_range), // rr

            // Cryptography
            0x6268 => Some(Self::blake3), // bh

            // Time
            0x746d => Some(Self::timestamp), // tm

            // Error handling
            0x6172 => Some(Self::assert), // ar
            0x686c => Some(Self::halt),   // hl

            // Debug
            0x6462 => Some(Self::debug_print), // db
            0x6473 => Some(Self::debug_stack), // ds
            0x6e70 => Some(Self::nop),         // np

            _ => None,
        }
    }

    /// Parse opcode from two-character identifier
    ///
    /// VSF bytecode format: {ab} where a,b are lowercase letters
    /// Packed into u16 for efficient single-comparison matching
    pub fn from_bytes(bytes: &[u8; 2]) -> Option<Self> {
        let op = pack(bytes[0], bytes[1]);
        Self::from_u16(op)
    }

    /// Parse opcode from VSF type
    ///
    /// Converts VsfType::op(a, b) into Opcode enum
    /// Returns error if not an opcode type or unknown opcode
    pub fn from_vsf(value: &vsf::VsfType) -> Result<Self, String> {
        match value {
            vsf::VsfType::op(a, b) => {
                let packed = pack(*a, *b);
                Self::from_u16(packed)
                    .ok_or_else(|| format!("Unknown opcode: {{{}{}}} (0x{:04x})",
                        *a as char, *b as char, packed))
            }
            _ => Err(format!("Expected opcode, got VSF type: {:?}", value)),
        }
    }

    /// Convert opcode to two-character identifier
    pub fn to_bytes(&self) -> [u8; 2] {
        match self {
            Self::push => *b"ps",
            Self::pop => *b"pp",
            Self::dup => *b"dp",
            Self::dup_n => *b"dn",
            Self::swap => *b"sw",
            Self::rotate => *b"rt",
            Self::local_alloc => *b"la",
            Self::local_get => *b"lg",
            Self::local_set => *b"ls",
            Self::local_tee => *b"lt",
            Self::add => *b"ad",
            Self::sub => *b"sb",
            Self::mul => *b"ml",
            Self::div => *b"dv",
            Self::recip => *b"rc",
            Self::mod_ => *b"md",
            Self::neg => *b"ng",
            Self::abs => *b"ab",
            Self::sqrt => *b"sq",
            Self::pow => *b"pw",
            Self::min => *b"mn",
            Self::max => *b"mx",
            Self::clamp => *b"cm",
            Self::floor => *b"fl",
            Self::ceil => *b"cl",
            Self::round => *b"rn",
            Self::frac => *b"fa",
            Self::lerp => *b"lp",
            Self::sin => *b"sn",
            Self::cos => *b"cs",
            Self::tan => *b"tn",
            Self::asin => *b"is",
            Self::acos => *b"ic",
            Self::atan => *b"ia",
            Self::atan2 => *b"a2",
            Self::eq => *b"eq",
            Self::ne => *b"ne",
            Self::lt => *b"lo",
            Self::le => *b"le",
            Self::gt => *b"gt",
            Self::ge => *b"ge",
            Self::and => *b"an",
            Self::or => *b"or",
            Self::not => *b"nt",
            Self::bit_and => *b"ba",
            Self::bit_or => *b"bo",
            Self::bit_xor => *b"bx",
            Self::bit_not => *b"bn",
            Self::typeof_ => *b"ty",
            Self::to_s44 => *b"ts",
            Self::to_u32 => *b"tu",
            Self::to_string => *b"tx",
            Self::array_new => *b"aw",
            Self::array_len => *b"al",
            Self::array_get => *b"ag",
            Self::array_set => *b"ae",
            Self::array_push => *b"ap",
            Self::array_pop => *b"ao",
            Self::string_concat => *b"sc",
            Self::string_len => *b"sl",
            Self::string_slice => *b"ss",
            Self::handle_read => *b"hr",
            Self::handle_write => *b"hw",
            Self::handle_call => *b"hc",
            Self::handle_query => *b"hq",
            Self::clear => *b"cr",
            Self::fill_rect => *b"fr",
            Self::stroke_rect => *b"sr",
            Self::fill_circle => *b"fc",
            Self::stroke_circle => *b"so",
            Self::draw_line => *b"dl",
            Self::draw_text => *b"dt",
            Self::set_font => *b"sf",
            Self::rgba => *b"ca",
            Self::rgb => *b"cb",
            Self::call => *b"cn",
            Self::call_indirect => *b"cd",
            Self::return_ => *b"re",
            Self::return_value => *b"rv",
            Self::jump => *b"jm",
            Self::jump_if => *b"ji",
            Self::jump_zero => *b"jz",
            Self::random => *b"rd",
            Self::random_gauss => *b"rg",
            Self::random_range => *b"rr",
            Self::blake3 => *b"bh",
            Self::timestamp => *b"tm",
            Self::assert => *b"ar",
            Self::halt => *b"hl",
            Self::debug_print => *b"db",
            Self::debug_stack => *b"ds",
            Self::nop => *b"np",
        }
    }
}

impl fmt::Display for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.to_bytes();
        write!(f, "{{{}}}", std::str::from_utf8(&bytes).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_roundtrip() {
        let opcodes = [
            Opcode::push,
            Opcode::add,
            Opcode::fill_rect,
            Opcode::jump_if,
            Opcode::halt,
        ];

        for opcode in opcodes {
            let bytes = opcode.to_bytes();
            let parsed = Opcode::from_bytes(&bytes);
            assert_eq!(parsed, Some(opcode));
        }
    }

    #[test]
    fn test_opcode_display() {
        assert_eq!(format!("{}", Opcode::push), "{ps}");
        assert_eq!(format!("{}", Opcode::add), "{ad}");
        assert_eq!(format!("{}", Opcode::fill_rect), "{fr}");
    }

    #[test]
    fn test_invalid_opcode() {
        assert_eq!(Opcode::from_bytes(b"zz"), None);
        assert_eq!(Opcode::from_bytes(b"XX"), None);
    }
}
