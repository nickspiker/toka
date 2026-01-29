//! Stack values for Toka VM
//!
//! Values are VSF-typed data that can be pushed/popped from the stack.
//!
//! Allowed types:
//! - All Spirix Scalars: `s33`-`s77` (25 combinations of F×E)
//! - All Spirix Circles: `c33`-`c77` (25 combinations of F×E)
//! - Boolean: `u0`
//! - Unsigned integers: `u`
//! - Signed integers: `i`
//! - Strings: `x` (UTF-8), `l` (ASCII)
//! - Tensors of allowed types: `t_s54`, `t_u5`, etc.
//!
//! The VM will reject any bytecode that attempts to use IEEE-754 types.

use spirix::ScalarF4E4; // S44: Our primary numeric type
use std::fmt;

impl Value {
    /// Convert value to string
    pub fn to_string(&self) -> String {
        match self {
            Value::S44(v) => {
                // Use Spirix's Display implementation
                format!("{}", v)
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

    /// Check if value is "truthy" for conditional logic (if/while/etc)
    /// - Numbers: non-zero = true, zero = false
    /// - Strings/Arrays: non-empty = true, empty = false
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
    /// Returns the VSF type name (e.g., "s44", "u32", "string")
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

// Convenience constructors from Spirix types
impl From<ScalarF4E4> for Value {
    fn from(v: ScalarF4E4) -> Self {
        Value::S44(v)
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
        let a = Value::from(ScalarF4E4::from(3));
        let b = Value::from(ScalarF4E4::from(4));

        assert_eq!(a.to_s44().unwrap(), ScalarF4E4::from(3));
        assert_eq!(b.to_s44().unwrap(), ScalarF4E4::from(4));
    }

    #[test]
    fn test_type_conversions() {
        let s44 = Value::from(ScalarF4E4::from(42) + ScalarF4E4::from(1) / ScalarF4E4::from(2));
        let u32_val = Value::from(100u32);
        let string = Value::from("hello");

        assert_eq!(s44.to_u32().unwrap(), 42);
        assert_eq!(u32_val.to_s44().unwrap(), ScalarF4E4::from(100));
        assert_eq!(string.to_string(), "hello");
    }

    #[test]
    fn test_truthy() {
        assert!(Value::from(ScalarF4E4::ONE).is_truthy());
        assert!(!Value::from(ScalarF4E4::ZERO).is_truthy());
        assert!(Value::from(42u32).is_truthy());
        assert!(!Value::from(0u32).is_truthy());
        assert!(Value::from("text").is_truthy());
        assert!(!Value::from("").is_truthy());
    }

    #[test]
    fn test_type_name() {
        assert_eq!(Value::from(ScalarF4E4::ONE).type_name(), "s44");
        assert_eq!(Value::from(1u32).type_name(), "u32");
        assert_eq!(Value::from("x").type_name(), "string");
        assert_eq!(Value::from(vec![]).type_name(), "array");
    }
}
