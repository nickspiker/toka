//! Toka VM execution engine
//!
//! Stack-based VM with:
//! - Value stack (operands)
//! - Local variables (function-scoped)
//! - Instruction pointer
//! - Capability-checked handle system

use crate::opcode::Opcode;
use crate::value::Value;
use spirix::ScalarF4E4;

/// VM execution state
pub struct VM {
    /// Value stack (operands pushed/popped here)
    stack: Vec<Value>,

    /// Bytecode being executed
    bytecode: Vec<u8>,

    /// Instruction pointer (offset into bytecode)
    ip: usize,

    /// Local variables (function-scoped)
    /// Outer vec is call stack frames, inner vec is locals within frame
    locals: Vec<Vec<Value>>,

    /// Whether execution has halted
    halted: bool,
}

impl VM {
    /// Create a new VM with the given bytecode
    pub fn new(bytecode: Vec<u8>) -> Self {
        Self {
            stack: Vec::new(),
            bytecode,
            ip: 0,
            locals: vec![Vec::new()], // Start with one frame
            halted: false,
        }
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
        if self.ip + 1 >= self.bytecode.len() {
            return Err("Unexpected end of bytecode".to_string());
        }

        // Read opcode (2 bytes)
        let op_bytes = [self.bytecode[self.ip], self.bytecode[self.ip + 1]];
        self.ip += 2;

        let opcode = Opcode::from_bytes(&op_bytes)
            .ok_or_else(|| format!("Unknown opcode: {:02x}{:02x}", op_bytes[0], op_bytes[1]))?;

        // Execute the opcode
        self.execute(opcode)?;

        Ok(())
    }

    /// Execute a single opcode
    fn execute(&mut self, opcode: Opcode) -> Result<(), String> {
        match opcode {
            // Stack manipulation
            Opcode::push => {
                // TODO: Read VSF-encoded value from bytecode
                // For now, placeholder - will implement VSF decoding next
                return Err("push not yet implemented".to_string());
            }

            Opcode::push_zero => {
                self.stack.push(Value::S44(ScalarF4E4::from(0.0)));
            }

            Opcode::push_one => {
                self.stack.push(Value::S44(ScalarF4E4::from(1.0)));
            }

            Opcode::pop => {
                self.stack.pop()
                    .ok_or_else(|| "Stack underflow on pop".to_string())?;
            }

            Opcode::dup => {
                let val = self.stack.last()
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

            // Arithmetic (S44)
            Opcode::add => {
                let b = self.pop_s44()?;
                let a = self.pop_s44()?;
                self.stack.push(Value::S44(a + b));
            }

            Opcode::sub => {
                let b = self.pop_s44()?;
                let a = self.pop_s44()?;
                self.stack.push(Value::S44(a - b));
            }

            Opcode::mul => {
                let b = self.pop_s44()?;
                let a = self.pop_s44()?;
                self.stack.push(Value::S44(a * b));
            }

            Opcode::div => {
                let b = self.pop_s44()?;
                let a = self.pop_s44()?;
                self.stack.push(Value::S44(a / b));
            }

            Opcode::neg => {
                let a = self.pop_s44()?;
                self.stack.push(Value::S44(-a));
            }

            // Comparison (returns 1.0 or 0.0 as S44)
            Opcode::eq => {
                let b = self.pop_s44()?;
                let a = self.pop_s44()?;
                let result = if a == b { 1.0 } else { 0.0 };
                self.stack.push(Value::S44(ScalarF4E4::from(result)));
            }

            Opcode::lt => {
                let b = self.pop_s44()?;
                let a = self.pop_s44()?;
                let result = if a < b { 1.0 } else { 0.0 };
                self.stack.push(Value::S44(ScalarF4E4::from(result)));
            }

            // Control flow
            Opcode::halt => {
                self.halted = true;
            }

            // Everything else is not yet implemented
            _ => {
                return Err(format!("Opcode not yet implemented: {:?}", opcode));
            }
        }

        Ok(())
    }

    /// Pop a value and convert to S44
    fn pop_s44(&mut self) -> Result<ScalarF4E4, String> {
        let val = self.stack.pop()
            .ok_or_else(|| "Stack underflow".to_string())?;
        val.to_s44()
    }

    /// Peek at top of stack without popping
    pub fn peek(&self) -> Option<&Value> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_constants() {
        let bytecode = vec![
            0x70, 0x7a, // push_zero
            0x70, 0x6f, // push_one
        ];

        let mut vm = VM::new(bytecode);
        vm.step().unwrap(); // push_zero
        assert_eq!(vm.stack_depth(), 1);

        vm.step().unwrap(); // push_one
        assert_eq!(vm.stack_depth(), 2);

        // Check values
        let one = vm.stack[1].to_s44().unwrap();
        let zero = vm.stack[0].to_s44().unwrap();
        assert_eq!(Into::<f64>::into(one), 1.0);
        assert_eq!(Into::<f64>::into(zero), 0.0);
    }

    #[test]
    fn test_arithmetic() {
        // Test: 2 + 3 = 5
        // We'll push 1, dup to get 2, push 1 three times and add to get 3, then add
        let bytecode = vec![
            0x70, 0x6f, // push_one
            0x64, 0x70, // dup
            0x61, 0x64, // add  (1+1=2)
            0x70, 0x6f, // push_one
            0x70, 0x6f, // push_one
            0x61, 0x64, // add  (1+1=2)
            0x70, 0x6f, // push_one
            0x61, 0x64, // add  (2+1=3)
            0x61, 0x64, // add  (2+3=5)
            0x68, 0x6c, // halt (hl)
        ];

        let mut vm = VM::new(bytecode);
        vm.run().unwrap();

        assert_eq!(vm.stack_depth(), 1);
        let result = vm.peek().unwrap().to_s44().unwrap();
        assert_eq!(Into::<f64>::into(result), 5.0);
    }

    #[test]
    fn test_comparison() {
        // Test: 2 < 3 (should be 1.0 = true)
        let bytecode = vec![
            0x70, 0x6f, // push_one
            0x70, 0x6f, // push_one
            0x61, 0x64, // add (1+1=2)
            0x70, 0x6f, // push_one
            0x70, 0x6f, // push_one
            0x70, 0x6f, // push_one
            0x61, 0x64, // add (1+1=2)
            0x61, 0x64, // add (2+1=3)
            0x6c, 0x6f, // lo (less-than: 2 < 3)
            0x68, 0x6c, // halt (hl)
        ];

        let mut vm = VM::new(bytecode);
        vm.run().unwrap();

        let result = vm.peek().unwrap().to_s44().unwrap();
        assert_eq!(Into::<f64>::into(result), 1.0); // true
    }
}
