//! VSF Bytecode Parser
//!
//! Parses VSF-encoded bytecode into an instruction stream for the VM.
//! Uses VSF's type-driven parser to extract opcodes and data values.

use crate::opcode::Opcode;
use vsf::VsfType;

/// VM instruction: either an opcode or a data value
#[derive(Debug, Clone)]
pub enum Instruction {
    /// Executable opcode
    Op(Opcode),
    /// Data value (for push, call arguments, etc.)
    Value(VsfType),
}

/// Bytecode parser with stateful pointer
pub struct BytecodeParser {
    data: Vec<u8>,
    pointer: usize,
}

impl BytecodeParser {
    /// Create parser from raw VSF bytecode
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, pointer: 0 }
    }

    /// Parse next VsfType from bytecode stream
    fn parse_next(&mut self) -> Result<VsfType, String> {
        vsf::parse(&self.data, &mut self.pointer)
            .map_err(|e| format!("VSF parse error at byte {}: {}", self.pointer, e))
    }

    /// Parse entire bytecode into instruction stream
    ///
    /// Pattern: VSF opcodes ({ab}) become Instruction::Op
    ///          All other VSF types become Instruction::Value
    pub fn parse_program(&mut self) -> Result<Vec<Instruction>, String> {
        let mut instructions = Vec::new();

        while self.pointer < self.data.len() {
            let value = self.parse_next()?;

            match &value {
                VsfType::op(_, _) => {
                    // Convert VSF opcode to Toka opcode
                    let opcode = Opcode::from_vsf(&value)?;
                    instructions.push(Instruction::Op(opcode));
                }
                _ => {
                    // Push literal value (data for push opcode, etc.)
                    instructions.push(Instruction::Value(value));
                }
            }
        }

        Ok(instructions)
    }

    /// Get current parse position (for debugging)
    pub fn position(&self) -> usize {
        self.pointer
    }

    /// Check if parsing is complete
    pub fn is_complete(&self) -> bool {
        self.pointer >= self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spirix::ScalarF4E4;

    #[test]
    fn test_parse_simple_program() {
        // Build bytecode: {ps} s44{1.0} {ps} s44{2.0} {ad} {hl}
        let mut bytecode = Vec::new();

        // {ps} - push opcode
        bytecode.extend(VsfType::op(b'p', b's').flatten());
        // s44{1.0} - scalar value
        bytecode.extend(VsfType::s44(ScalarF4E4::from(1)).flatten());

        // {ps}
        bytecode.extend(VsfType::op(b'p', b's').flatten());
        // s44{2.0}
        bytecode.extend(VsfType::s44(ScalarF4E4::from(2)).flatten());

        // {ad} - add opcode
        bytecode.extend(VsfType::op(b'a', b'd').flatten());

        // {hl} - halt opcode
        bytecode.extend(VsfType::op(b'h', b'l').flatten());

        let mut parser = BytecodeParser::new(bytecode);
        let instructions = parser.parse_program().expect("Parse should succeed");

        // Should get: Op(push), Value(1.0), Op(push), Value(2.0), Op(add), Op(halt)
        assert_eq!(instructions.len(), 6);

        matches!(instructions[0], Instruction::Op(Opcode::push));
        matches!(instructions[1], Instruction::Value(VsfType::s44(_)));
        matches!(instructions[2], Instruction::Op(Opcode::push));
        matches!(instructions[3], Instruction::Value(VsfType::s44(_)));
        matches!(instructions[4], Instruction::Op(Opcode::add));
        matches!(instructions[5], Instruction::Op(Opcode::halt));
    }

    #[test]
    fn test_parse_unknown_opcode() {
        // Build bytecode with invalid opcode {zz}
        let mut bytecode = Vec::new();
        bytecode.extend(VsfType::op(b'z', b'z').flatten());

        let mut parser = BytecodeParser::new(bytecode);
        let result = parser.parse_program();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown opcode"));
    }

    #[test]
    fn test_parse_mixed_types() {
        // {ps} u3{42} {ps} l"hello" {hl}
        let mut bytecode = Vec::new();

        bytecode.extend(VsfType::op(b'p', b's').flatten());
        bytecode.extend(VsfType::u3(42).flatten());

        bytecode.extend(VsfType::op(b'p', b's').flatten());
        bytecode.extend(VsfType::l("hello".to_string()).flatten());

        bytecode.extend(VsfType::op(b'h', b'l').flatten());

        let mut parser = BytecodeParser::new(bytecode);
        let instructions = parser.parse_program().expect("Parse should succeed");

        assert_eq!(instructions.len(), 5);
        matches!(instructions[1], Instruction::Value(VsfType::u3(42)));
        matches!(instructions[3], Instruction::Value(VsfType::l(_)));
    }
}
