//! Core instruction execution for the VM

use crate::core::{
    bytecode::{Instruction, OpCode},
    value::Value,
};
use super::{
    errors::{VMError, VMResult},
    types::{VirtualMachine, ExecutionResult},
};

impl VirtualMachine {
    /// Main execution loop
    pub(super) fn run(&mut self) -> VMResult<Value> {
        loop {
            if self.debug_mode {
                self.print_debug_info();
            }

            let instruction = self.fetch_instruction()?;
            self.instructions_executed += 1;

            match self.execute_instruction(&instruction)? {
                ExecutionResult::Continue => continue,
                ExecutionResult::Return(value) => return Ok(value),
                ExecutionResult::Halt => return Ok(Value::Null),
            }
        }
    }

    /// Fetch the next instruction
    pub(super) fn fetch_instruction(&self) -> VMResult<Instruction> {
        let chunk = self.current_chunk.as_ref()
            .ok_or_else(|| VMError::RuntimeError("No current chunk".to_string()))?;

        chunk.get_instruction(self.ip)
            .cloned()
            .ok_or_else(|| VMError::RuntimeError(format!("Invalid instruction pointer: {}", self.ip)))
    }

    /// Print debug information
    pub(super) fn print_debug_info(&self) {
        if let Some(chunk) = &self.current_chunk {
            if let Some(instruction) = chunk.get_instruction(self.ip) {
                println!("IP: {:04} | {}", self.ip, instruction);
                println!("Stack: {} values", self.stack.size());
                if self.stack.size() > 0 {
                    if let Ok(top) = self.stack.peek() {
                        println!("  Top: {}", top);
                    }
                }
            }
        }
    }

    /// Execute a single instruction
    pub(super) fn execute_instruction(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        // Debug: Log all instructions when JIT debug is enabled
        if self.jit_engine.debug {
            println!("🔧 VM: execute_instruction - opcode: {:?}", instruction.opcode);
        }

        match instruction.opcode {
            // Stack operations
            OpCode::Const => self.execute_const(instruction),
            OpCode::Null => self.execute_null(),
            OpCode::True => self.execute_true(),
            OpCode::False => self.execute_false(),
            OpCode::Dup => self.execute_dup(),
            OpCode::Pop => self.execute_pop(),
            OpCode::Swap => self.execute_swap(),

            // Arithmetic operations
            OpCode::Add => self.execute_add(),
            OpCode::Sub => self.execute_sub(),
            OpCode::Mul => self.execute_mul(),
            OpCode::Div => self.execute_div(),
            OpCode::Mod => self.execute_mod(),
            OpCode::Neg => self.execute_neg(),

            // Comparison operations
            OpCode::Eq => self.execute_eq(),
            OpCode::Ne => self.execute_ne(),
            OpCode::Lt => self.execute_lt(),
            OpCode::Le => self.execute_le(),
            OpCode::Gt => self.execute_gt(),
            OpCode::Ge => self.execute_ge(),

            // Logical operations
            OpCode::And => self.execute_and(),
            OpCode::Or => self.execute_or(),
            OpCode::Not => self.execute_not(),

            // Bitwise operations
            OpCode::BitAnd => self.execute_bit_and(),
            OpCode::BitOr => self.execute_bit_or(),
            OpCode::BitXor => self.execute_bit_xor(),
            OpCode::BitNot => self.execute_bit_not(),
            OpCode::Shl => self.execute_shl(),
            OpCode::Shr => self.execute_shr(),

            // Memory operations
            OpCode::LoadLocal | OpCode::StoreLocal | OpCode::LoadGlobal | OpCode::StoreGlobal |
            OpCode::CreateArray | OpCode::CreateObject | OpCode::LoadField | OpCode::StoreField |
            OpCode::LoadIndex | OpCode::StoreIndex | OpCode::NewObject | OpCode::NewArray |
            OpCode::ArrayLength => {
                self.execute_memory_instruction(instruction)
            }

            // Control flow operations
            OpCode::Jump | OpCode::JumpIfFalse | OpCode::JumpIfTrue | OpCode::JumpIfNull |
            OpCode::JumpIfNotNull | OpCode::Throw | OpCode::TryStart | OpCode::TryEnd |
            OpCode::Catch | OpCode::Finally => {
                self.execute_control_flow_instruction(instruction)
            }

            // Function operations
            OpCode::Call | OpCode::Return | OpCode::ReturnValue | OpCode::CallMethod => {
                self.execute_function_instruction(instruction)
            }

            // Type operations
            OpCode::IsInstance => self.execute_is_instance(instruction),
            OpCode::Cast => self.execute_cast(instruction),

            // Special operations
            OpCode::Halt => Ok(ExecutionResult::Halt),
            OpCode::Print => self.execute_print(),
            OpCode::Nop => self.execute_nop(),
            OpCode::Debug => self.execute_debug(),

            _ => Err(VMError::InvalidInstruction(format!("Unimplemented instruction: {:?}", instruction.opcode)))
        }
    }

    // Stack operations
    fn execute_const(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        // Fast path for constant loading
        let index = ((instruction.operands[0] as u16) << 8) | (instruction.operands[1] as u16);
        if let Some(chunk) = &self.current_chunk {
            if let Some(value) = chunk.constants.get(index as usize) {
                self.stack.push(value.clone())?;
                self.ip += 1;
                return Ok(ExecutionResult::Continue);
            }
        }
        // Fallback to safe version
        let index = instruction.get_u16(0)
            .ok_or_else(|| VMError::InvalidOperand("Missing constant index".to_string()))?;
        let chunk = self.current_chunk.as_ref().unwrap();
        let value = chunk.get_constant(index)
            .ok_or_else(|| VMError::InvalidOperand(format!("Invalid constant index: {}", index)))?
            .clone();
        self.stack.push(value)?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_null(&mut self) -> VMResult<ExecutionResult> {
        self.stack.push(Value::Null)?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_true(&mut self) -> VMResult<ExecutionResult> {
        self.stack.push(Value::Bool(true))?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_false(&mut self) -> VMResult<ExecutionResult> {
        self.stack.push(Value::Bool(false))?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_dup(&mut self) -> VMResult<ExecutionResult> {
        self.stack.dup().map_err(|_| VMError::StackUnderflow)?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_pop(&mut self) -> VMResult<ExecutionResult> {
        self.stack.pop()?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_swap(&mut self) -> VMResult<ExecutionResult> {
        self.stack.swap()?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    // Arithmetic operations
    fn execute_add(&mut self) -> VMResult<ExecutionResult> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;

        // Fast path for integer addition
        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => Value::Int(x + y),
            _ => a.add(&b).map_err(|e| VMError::TypeMismatch(e))?,
        };

        self.stack.push(result)?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_sub(&mut self) -> VMResult<ExecutionResult> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;

        // Fast path for integer subtraction
        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => Value::Int(x - y),
            _ => a.sub(&b).map_err(|e| VMError::TypeMismatch(e))?,
        };

        self.stack.push(result)?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_mul(&mut self) -> VMResult<ExecutionResult> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        let result = a.mul(&b).map_err(|e| VMError::TypeMismatch(e))?;
        self.stack.push(result)?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_div(&mut self) -> VMResult<ExecutionResult> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;

        // Check for division by zero and throw appropriate exception
        match a.div(&b) {
            Ok(result) => {
                self.stack.push(result)?;
                self.ip += 1;
                Ok(ExecutionResult::Continue)
            }
            Err(e) => {
                if e.contains("Division by zero") {
                    let exception = Value::arithmetic_error("Division by zero".to_string());
                    self.throw_exception(exception)
                } else {
                    let exception = Value::type_error(format!("Invalid division: {}", e));
                    self.throw_exception(exception)
                }
            }
        }
    }

    fn execute_mod(&mut self) -> VMResult<ExecutionResult> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;

        // Check for modulo by zero and throw appropriate exception
        match a.modulo(&b) {
            Ok(result) => {
                self.stack.push(result)?;
                self.ip += 1;
                Ok(ExecutionResult::Continue)
            }
            Err(e) => {
                if e.contains("Modulo by zero") {
                    let exception = Value::arithmetic_error("Modulo by zero".to_string());
                    self.throw_exception(exception)
                } else {
                    let exception = Value::type_error(format!("Invalid modulo operation: {}", e));
                    self.throw_exception(exception)
                }
            }
        }
    }

    fn execute_neg(&mut self) -> VMResult<ExecutionResult> {
        let a = self.stack.pop()?;
        let result = a.negate().map_err(|e| VMError::TypeMismatch(e))?;
        self.stack.push(result)?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    // Comparison operations
    fn execute_eq(&mut self) -> VMResult<ExecutionResult> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        self.stack.push(Value::Bool(a.equals(&b)))?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_ne(&mut self) -> VMResult<ExecutionResult> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        self.stack.push(Value::Bool(!a.equals(&b)))?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_lt(&mut self) -> VMResult<ExecutionResult> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        let result = self.compare_values(&a, &b)? < 0;
        self.stack.push(Value::Bool(result))?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_le(&mut self) -> VMResult<ExecutionResult> {
        // Ultra-fast path for integer comparison
        if let (Ok(Value::Int(b)), Ok(Value::Int(a))) = (self.stack.pop(), self.stack.pop()) {
            self.stack.push(Value::Bool(a <= b))?;
        } else {
            // Fallback to general case
            let b = self.stack.pop()?;
            let a = self.stack.pop()?;
            let result = match (&a, &b) {
                (Value::Int(x), Value::Int(y)) => *x <= *y,
                _ => self.compare_values(&a, &b)? <= 0,
            };
            self.stack.push(Value::Bool(result))?;
        }
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_gt(&mut self) -> VMResult<ExecutionResult> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        let result = self.compare_values(&a, &b)? > 0;
        self.stack.push(Value::Bool(result))?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_ge(&mut self) -> VMResult<ExecutionResult> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        let result = self.compare_values(&a, &b)? >= 0;
        self.stack.push(Value::Bool(result))?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    // Logical operations
    fn execute_and(&mut self) -> VMResult<ExecutionResult> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        let result = a.is_truthy() && b.is_truthy();
        self.stack.push(Value::Bool(result))?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_or(&mut self) -> VMResult<ExecutionResult> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        let result = a.is_truthy() || b.is_truthy();
        self.stack.push(Value::Bool(result))?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_not(&mut self) -> VMResult<ExecutionResult> {
        let a = self.stack.pop()?;
        self.stack.push(Value::Bool(a.is_falsy()))?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    // Bitwise operations
    fn execute_bit_and(&mut self) -> VMResult<ExecutionResult> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => Value::Int(x & y),
            _ => return Err(VMError::TypeMismatch(format!("Bitwise AND requires integers, got {} and {}", a.type_name(), b.type_name()))),
        };
        self.stack.push(result)?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_bit_or(&mut self) -> VMResult<ExecutionResult> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => Value::Int(x | y),
            _ => return Err(VMError::TypeMismatch(format!("Bitwise OR requires integers, got {} and {}", a.type_name(), b.type_name()))),
        };
        self.stack.push(result)?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_bit_xor(&mut self) -> VMResult<ExecutionResult> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => Value::Int(x ^ y),
            _ => return Err(VMError::TypeMismatch(format!("Bitwise XOR requires integers, got {} and {}", a.type_name(), b.type_name()))),
        };
        self.stack.push(result)?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_bit_not(&mut self) -> VMResult<ExecutionResult> {
        let a = self.stack.pop()?;
        let result = match a {
            Value::Int(x) => Value::Int(!x),
            _ => return Err(VMError::TypeMismatch(format!("Bitwise NOT requires integer, got {}", a.type_name()))),
        };
        self.stack.push(result)?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_shl(&mut self) -> VMResult<ExecutionResult> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => {
                if *y < 0 || *y >= 64 {
                    return Err(VMError::RuntimeError(format!("Shift amount {} is out of range", y)));
                }
                Value::Int(x << y)
            },
            _ => return Err(VMError::TypeMismatch(format!("Left shift requires integers, got {} and {}", a.type_name(), b.type_name()))),
        };
        self.stack.push(result)?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_shr(&mut self) -> VMResult<ExecutionResult> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => {
                if *y < 0 || *y >= 64 {
                    return Err(VMError::RuntimeError(format!("Shift amount {} is out of range", y)));
                }
                Value::Int(x >> y)
            },
            _ => return Err(VMError::TypeMismatch(format!("Right shift requires integers, got {} and {}", a.type_name(), b.type_name()))),
        };
        self.stack.push(result)?;
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    // Special operations
    fn execute_print(&mut self) -> VMResult<ExecutionResult> {
        let value = self.stack.pop()?;
        println!("{}", value);
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_nop(&mut self) -> VMResult<ExecutionResult> {
        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    // Type operations
    fn execute_is_instance(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        let type_index = instruction.get_u16(0)
            .ok_or_else(|| VMError::InvalidOperand("Missing type index".to_string()))?;

        let value = self.stack.pop()?;

        // Get type name from constants
        let chunk = self.current_chunk.as_ref().unwrap();
        let type_name = chunk.get_constant(type_index)
            .ok_or_else(|| VMError::InvalidOperand(format!("Invalid type index: {}", type_index)))?;

        if let Value::String(type_name_str) = type_name {
            let is_instance = match type_name_str.as_str() {
                "int" => matches!(value, Value::Int(_)),
                "float" => matches!(value, Value::Float(_)),
                "string" => matches!(value, Value::String(_)),
                "bool" => matches!(value, Value::Bool(_)),
                "array" => matches!(value, Value::Array(_)),
                "object" => matches!(value, Value::Object(_)),
                "null" => matches!(value, Value::Null),
                _ => false, // Unknown type
            };
            self.stack.push(Value::Bool(is_instance))?;
        } else {
            return Err(VMError::TypeMismatch("Type name must be a string".to_string()));
        }

        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_cast(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        let type_index = instruction.get_u16(0)
            .ok_or_else(|| VMError::InvalidOperand("Missing type index".to_string()))?;

        let value = self.stack.pop()?;

        // Get type name from constants
        let chunk = self.current_chunk.as_ref().unwrap();
        let type_name = chunk.get_constant(type_index)
            .ok_or_else(|| VMError::InvalidOperand(format!("Invalid type index: {}", type_index)))?;

        if let Value::String(type_name_str) = type_name {
            let result = match type_name_str.as_str() {
                "int" => match value {
                    Value::Int(i) => Value::Int(i),
                    Value::Float(f) => Value::Int(f as i64),
                    Value::String(s) => {
                        match s.parse::<i64>() {
                            Ok(i) => Value::Int(i),
                            Err(_) => return Err(VMError::TypeMismatch(format!("Cannot cast '{}' to int", s))),
                        }
                    }
                    Value::Bool(b) => Value::Int(if b { 1 } else { 0 }),
                    _ => return Err(VMError::TypeMismatch(format!("Cannot cast {} to int", value.type_name()))),
                },
                "float" => match value {
                    Value::Int(i) => Value::Float(i as f64),
                    Value::Float(f) => Value::Float(f),
                    Value::String(s) => {
                        match s.parse::<f64>() {
                            Ok(f) => Value::Float(f),
                            Err(_) => return Err(VMError::TypeMismatch(format!("Cannot cast '{}' to float", s))),
                        }
                    }
                    _ => return Err(VMError::TypeMismatch(format!("Cannot cast {} to float", value.type_name()))),
                },
                "string" => Value::String(std::rc::Rc::new(value.to_string())),
                "bool" => Value::Bool(value.is_truthy()),
                _ => return Err(VMError::TypeMismatch(format!("Unknown cast type: {}", type_name_str))),
            };
            self.stack.push(result)?;
        } else {
            return Err(VMError::TypeMismatch("Type name must be a string".to_string()));
        }

        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    fn execute_debug(&mut self) -> VMResult<ExecutionResult> {
        // Debug breakpoint - print current state
        println!("=== DEBUG BREAKPOINT ===");
        println!("IP: {}", self.ip);
        println!("Stack size: {}", self.stack.size());
        if self.stack.size() > 0 {
            if let Ok(top) = self.stack.peek() {
                println!("Stack top: {}", top);
            }
        }
        if let Some(locals) = &self.locals {
            println!("Local variables: {}", locals.count());
        }
        println!("Call stack depth: {}", self.call_stack.depth());
        println!("========================");

        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    /// Compare two values for ordering
    pub(super) fn compare_values(&self, a: &Value, b: &Value) -> VMResult<i32> {
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => Ok(a.cmp(b) as i32),
            (Value::Float(a), Value::Float(b)) => Ok(a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal) as i32),
            (Value::Int(a), Value::Float(b)) => Ok((*a as f64).partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal) as i32),
            (Value::Float(a), Value::Int(b)) => Ok(a.partial_cmp(&(*b as f64)).unwrap_or(std::cmp::Ordering::Equal) as i32),
            (Value::String(a), Value::String(b)) => Ok(a.cmp(b) as i32),
            _ => Err(VMError::TypeMismatch(format!("Cannot compare {} and {}", a.type_name(), b.type_name()))),
        }
    }
}
