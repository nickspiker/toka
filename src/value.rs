//! Stack values for Toka VM
//!
//! Values are VSF-typed data that can be pushed/popped from the stack.
//!
//! **IMPORTANT: Type restrictions for determinism**
//!
//! Toka ONLY supports deterministic types. The following VSF types are FORBIDDEN:
//! - `f5`, `f6` (IEEE-754 floats - non-deterministic)
//! - `j5`, `j6` (IEEE-754 complex - non-deterministic)
//!
//! Allowed types:
//! - All Spirix Scalars: `s33`-`s77` (25 combinations of F×E)
//! - All Spirix Circles: `c33`-`c77` (25 combinations of F×E)
//! - Unsigned integers: `u0`, `u3`-`u7` (bool, u8, u16, u32, u64, u128)
//! - Signed integers: `i3`-`i7` (i8, i16, i32, i64, i128)
//! - Strings: `x` (UTF-8), `l` (ASCII), `d` (dict keys)
//! - Tensors of allowed types: `t_s44`, `t_u5`, etc.
//!
//! The VM will reject any bytecode that attempts to use IEEE-754 types.

use spirix::ScalarF4E4; // S44: Our primary numeric type
use std::fmt;

/// Stack value wrapping deterministic VSF types
///
/// For v0, we support a minimal subset:
/// - s44: Spirix Scalar<i16, i16> (primary numeric type - deterministic)
/// - Unsigned/signed integers (u3-u7, i3-i7)
/// - Strings (UTF-8)
/// - Arrays of allowed types
///
/// IEEE-754 types (f5, f6, j5, j6) are explicitly FORBIDDEN for determinism.
#[derive(Debug, Clone)]
pub enum Value {
    /// Spirix S44 (16-bit fraction, 16-bit exponent)
    /// This is our primary numeric type for all floating-point math.
    /// Deterministic across all platforms (unlike IEEE-754).
    S44(ScalarF4E4),

    /// 8-bit unsigned integer (VSF type: u3)
    U8(u8),
    /// 16-bit unsigned integer (VSF type: u4)
    U16(u16),
    /// 32-bit unsigned integer (VSF type: u5) - used for colours, indices
    U32(u32),
    /// 64-bit unsigned integer (VSF type: u6)
    U64(u64),
    /// 128-bit unsigned integer (VSF type: u7)
    U128(u128),

    /// 8-bit signed integer (VSF type: i3)
    I8(i8),
    /// 16-bit signed integer (VSF type: i4)
    I16(i16),
    /// 32-bit signed integer (VSF type: i5)
    I32(i32),
    /// 64-bit signed integer (VSF type: i6)
    I64(i64),
    /// 128-bit signed integer (VSF type: i7)
    I128(i128),

    /// UTF-8 string
    String(String),

    /// Array of values (homogeneous for now)
    Array(Vec<Value>),
}

impl Value {
    /// Convert value to S44 (best-effort conversion)
    pub fn to_s44(&self) -> Result<ScalarF4E4, String> {
        match self {
            Value::S44(v) => Ok(*v),
            Value::U8(v) => Ok(ScalarF4E4::from(*v)),
            Value::U16(v) => Ok(ScalarF4E4::from(*v)),
            Value::U32(v) => Ok(ScalarF4E4::from(*v)),
            Value::U64(v) => Ok(ScalarF4E4::from(*v)),
            Value::U128(v) => Ok(ScalarF4E4::from(*v)),
            Value::I8(v) => Ok(ScalarF4E4::from(*v)),
            Value::I16(v) => Ok(ScalarF4E4::from(*v)),
            Value::I32(v) => Ok(ScalarF4E4::from(*v)),
            Value::I64(v) => Ok(ScalarF4E4::from(*v)),
            Value::I128(v) => Ok(ScalarF4E4::from(*v)),
            Value::String(s) => {
                // Try parsing as f64 first, then convert to S44
                let f: f64 = s.parse()
                    .map_err(|e| format!("Cannot convert string to S44: {}", e))?;
                Ok(ScalarF4E4::from(f))
            }
            Value::Array(_) => Err("Cannot convert array to S44".to_string()),
        }
    }

    /// Convert value to u32 (best-effort conversion)
    pub fn to_u32(&self) -> Result<u32, String> {
        match self {
            Value::S44(v) => {
                // Convert S44 to f64 first, then to u32
                let f: f64 = (*v).into();
                Ok(f as u32)
            }
            Value::U8(v) => Ok(*v as u32),
            Value::U16(v) => Ok(*v as u32),
            Value::U32(v) => Ok(*v),
            Value::U64(v) => Ok(*v as u32),
            Value::U128(v) => Ok(*v as u32),
            Value::I8(v) => Ok(*v as u32),
            Value::I16(v) => Ok(*v as u32),
            Value::I32(v) => Ok(*v as u32),
            Value::I64(v) => Ok(*v as u32),
            Value::I128(v) => Ok(*v as u32),
            Value::String(s) => s
                .parse::<u32>()
                .map_err(|e| format!("Cannot convert string to u32: {}", e)),
            Value::Array(_) => Err("Cannot convert array to u32".to_string()),
        }
    }

    /// Convert value to string
    pub fn to_string(&self) -> String {
        match self {
            Value::S44(v) => {
                let f: f64 = (*v).into();
                format!("{}", f)
            }
            Value::U8(v) => format!("{}", v),
            Value::U16(v) => format!("{}", v),
            Value::U32(v) => format!("{}", v),
            Value::U64(v) => format!("{}", v),
            Value::U128(v) => format!("{}", v),
            Value::I8(v) => format!("{}", v),
            Value::I16(v) => format!("{}", v),
            Value::I32(v) => format!("{}", v),
            Value::I64(v) => format!("{}", v),
            Value::I128(v) => format!("{}", v),
            Value::String(s) => s.clone(),
            Value::Array(arr) => {
                let elements: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                format!("[{}]", elements.join(", "))
            }
        }
    }

    /// Check if value is "truthy" (non-zero for logic ops)
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::S44(v) => !v.is_zero(),
            Value::U8(v) => *v != 0,
            Value::U16(v) => *v != 0,
            Value::U32(v) => *v != 0,
            Value::U64(v) => *v != 0,
            Value::U128(v) => *v != 0,
            Value::I8(v) => *v != 0,
            Value::I16(v) => *v != 0,
            Value::I32(v) => *v != 0,
            Value::I64(v) => *v != 0,
            Value::I128(v) => *v != 0,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.is_empty(),
        }
    }

    /// Get type name for typeof operation
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::S44(_) => "s44",
            Value::U8(_) => "u8",
            Value::U16(_) => "u16",
            Value::U32(_) => "u32",
            Value::U64(_) => "u64",
            Value::U128(_) => "u128",
            Value::I8(_) => "i8",
            Value::I16(_) => "i16",
            Value::I32(_) => "i32",
            Value::I64(_) => "i64",
            Value::I128(_) => "i128",
            Value::String(_) => "string",
            Value::Array(_) => "array",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

// Convenience constructors
// NOTE: f32/f64 are converted to S44 (Spirix) for determinism
// Direct IEEE-754 usage is NOT supported
impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::S44(ScalarF4E4::from(v))
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::S44(ScalarF4E4::from(v))
    }
}

impl From<ScalarF4E4> for Value {
    fn from(v: ScalarF4E4) -> Self {
        Value::S44(v)
    }
}

// Unsigned integer conversions
impl From<u8> for Value {
    fn from(v: u8) -> Self {
        Value::U8(v)
    }
}

impl From<u16> for Value {
    fn from(v: u16) -> Self {
        Value::U16(v)
    }
}

impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Value::U32(v)
    }
}

impl From<u64> for Value {
    fn from(v: u64) -> Self {
        Value::U64(v)
    }
}

impl From<u128> for Value {
    fn from(v: u128) -> Self {
        Value::U128(v)
    }
}

// Signed integer conversions
impl From<i8> for Value {
    fn from(v: i8) -> Self {
        Value::I8(v)
    }
}

impl From<i16> for Value {
    fn from(v: i16) -> Self {
        Value::I16(v)
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::I32(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::I64(v)
    }
}

impl From<i128> for Value {
    fn from(v: i128) -> Self {
        Value::I128(v)
    }
}

// String conversions
impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_string())
    }
}

// Array conversion
impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Value::Array(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s44_arithmetic() {
        let a = Value::from(3.0);
        let b = Value::from(4.0);

        assert_eq!(a.to_s44().unwrap(), 3.0);
        assert_eq!(b.to_s44().unwrap(), 4.0);
    }

    #[test]
    fn test_type_conversions() {
        let s44 = Value::from(42.5);
        let u32_val = Value::from(100u32);
        let string = Value::from("hello");

        assert_eq!(s44.to_u32().unwrap(), 42);
        assert_eq!(u32_val.to_s44().unwrap(), 100.0);
        assert_eq!(string.to_string(), "hello");
    }

    #[test]
    fn test_truthy() {
        assert!(Value::from(1.0).is_truthy());
        assert!(!Value::from(0.0).is_truthy());
        assert!(Value::from(42u32).is_truthy());
        assert!(!Value::from(0u32).is_truthy());
        assert!(Value::from("text").is_truthy());
        assert!(!Value::from("").is_truthy());
    }

    #[test]
    fn test_type_name() {
        assert_eq!(Value::from(1.0).type_name(), "s44");
        assert_eq!(Value::from(1u32).type_name(), "u32");
        assert_eq!(Value::from("x").type_name(), "string");
        assert_eq!(Value::from(vec![]).type_name(), "array");
    }
}
