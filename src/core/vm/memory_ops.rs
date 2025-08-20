//! Memory operations for the VM

use crate::core::{
    bytecode::Instruction,
    value::Value,
};
use super::{
    errors::{VMError, VMResult},
    types::{VirtualMachine, ExecutionResult},
};
use std::collections::HashMap;

impl VirtualMachine {
    /// Pre-allocate global variables
    pub fn allocate_globals(&mut self, count: usize) -> VMResult<()> {
        // Extend the globals vector to accommodate the required number of variables
        for _ in 0..count {
            // Use the define method to add new global variables
            self.memory.globals.define(format!("__global_{}", self.memory.globals.len()), Value::Null)
                .map_err(|e| VMError::MemoryError(e.to_string()))?;
        }
        Ok(())
    }

    /// Execute memory-related instructions
    pub(super) fn execute_memory_instruction(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        match instruction.opcode {
            crate::core::bytecode::OpCode::LoadLocal => {
                // Fast path for local variable access
                let index = instruction.operands[0] as usize; // Direct access, assume valid
                if let Some(locals) = &self.locals {
                    if let Ok(value) = locals.get(index) {
                        self.stack.push(value.clone())?;
                        self.ip += 1;
                        return Ok(ExecutionResult::Continue);
                    }
                }
                // Fallback to safe version
                let index = instruction.get_u8(0)
                    .ok_or_else(|| VMError::InvalidOperand("Missing local index".to_string()))?;
                let locals = self.locals.as_ref()
                    .ok_or_else(|| VMError::RuntimeError("No local variables".to_string()))?;
                let value = locals.get(index as usize)?.clone();
                self.stack.push(value)?;
                self.ip += 1;
                Ok(ExecutionResult::Continue)
            }

            crate::core::bytecode::OpCode::StoreLocal => {
                let index = instruction.get_u8(0)
                    .ok_or_else(|| VMError::InvalidOperand("Missing local index".to_string()))?;
                let value = self.stack.pop()?;
                let locals = self.locals.as_mut()
                    .ok_or_else(|| VMError::RuntimeError("No local variables".to_string()))?;
                locals.set(index as usize, value)?;
                self.ip += 1;
                Ok(ExecutionResult::Continue)
            }

            crate::core::bytecode::OpCode::LoadGlobal => {
                let index = instruction.get_u16(0)
                    .ok_or_else(|| VMError::InvalidOperand("Missing global index".to_string()))?;
                let value = self.memory.globals.get(index as usize)?.clone();
                self.stack.push(value)?;
                self.ip += 1;
                Ok(ExecutionResult::Continue)
            }

            crate::core::bytecode::OpCode::StoreGlobal => {
                let index = instruction.get_u16(0)
                    .ok_or_else(|| VMError::InvalidOperand("Missing global index".to_string()))?;
                let value = self.stack.pop()?;
                self.memory.globals.set(index as usize, value)?;
                self.ip += 1;
                Ok(ExecutionResult::Continue)
            }

            crate::core::bytecode::OpCode::CreateArray => {
                let element_count = instruction.get_u16(0)
                    .ok_or_else(|| VMError::InvalidOperand("Missing element count".to_string()))?;

                // Pop elements from stack in reverse order
                let mut elements = Vec::with_capacity(element_count as usize);
                for _ in 0..element_count {
                    elements.push(self.stack.pop()?);
                }

                // Reverse to get correct order
                elements.reverse();

                // Create array value
                let array = Value::Array(std::rc::Rc::new(elements));
                self.stack.push(array)?;

                self.ip += 1;
                Ok(ExecutionResult::Continue)
            }

            crate::core::bytecode::OpCode::CreateObject => {
                let field_count = instruction.get_u16(0)
                    .ok_or_else(|| VMError::InvalidOperand("Missing field count".to_string()))?;

                // Pop key-value pairs from stack in reverse order
                let mut fields = HashMap::new();
                for _ in 0..field_count {
                    let value = self.stack.pop()?;
                    let key = self.stack.pop()?;

                    if let Value::String(key_str) = key {
                        fields.insert(key_str.as_ref().clone(), value);
                    } else {
                        return Err(VMError::TypeMismatch("Object key must be a string".to_string()));
                    }
                }

                // Create object value
                let object = Value::Object(std::rc::Rc::new(fields));
                self.stack.push(object)?;

                self.ip += 1;
                Ok(ExecutionResult::Continue)
            }

            crate::core::bytecode::OpCode::LoadField => {
                self.execute_field_access(instruction)
            }

            _ => Err(VMError::InvalidInstruction(format!("Not a memory instruction: {:?}", instruction.opcode)))
        }
    }

    /// Execute field access operations
    fn execute_field_access(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        let field_index = instruction.get_u16(0)
            .ok_or_else(|| VMError::InvalidOperand("Missing field index".to_string()))?;

        if field_index == 0 {
            // Array access: index is on top of stack, array is below it
            let index = self.stack.pop()?;
            let array = self.stack.pop()?;

            match array {
                Value::Null => {
                    // Null pointer access - throw a specific exception
                    let exception = Value::null_pointer_error("Cannot index into null array".to_string());
                    return self.throw_exception(exception);
                }
                Value::Array(arr) => {
                    if let Value::Int(idx) = index {
                        if idx >= 0 && (idx as usize) < arr.len() {
                            self.stack.push(arr[idx as usize].clone())?;
                        } else {
                            // Throw IndexError exception instead of generic runtime error
                            let exception = Value::index_error(format!("Array index {} out of bounds (array length: {})", idx, arr.len()));
                            return self.throw_exception(exception);
                        }
                    } else {
                        // Throw TypeError for invalid index type
                        let exception = Value::type_error(format!("Array index must be an integer, got {}", index.type_name()));
                        return self.throw_exception(exception);
                    }
                }
                _ => {
                    // Throw TypeError for trying to index non-array
                    let exception = Value::type_error(format!("Cannot index into {}", array.type_name()));
                    return self.throw_exception(exception);
                }
            }
        } else {
            // Object field access: object is on top of stack
            let object = self.stack.pop()?;

            match object {
                Value::Null => {
                    // Null pointer access - throw a specific exception
                    let exception = Value::null_pointer_error("Cannot access field of null object".to_string());
                    return self.throw_exception(exception);
                }
                Value::Object(obj_map) => {
                    // Use field index to get field name from constants
                    let chunk = self.current_chunk.as_ref().unwrap();
                    let field_name = chunk.get_constant(field_index)
                        .ok_or_else(|| VMError::InvalidOperand(format!("Invalid field index: {}", field_index)))?;

                    if let Value::String(field_name_str) = field_name {
                        if let Some(value) = obj_map.get(field_name_str.as_ref()) {
                            self.stack.push(value.clone())?;
                        } else {
                            self.stack.push(Value::Null)?; // Field doesn't exist
                        }
                    } else {
                        return Err(VMError::TypeMismatch("Field name must be a string".to_string()));
                    }
                }
                _ => {
                    return Err(VMError::TypeMismatch(format!("Cannot access field of {}", object.type_name())));
                }
            }
        }

        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    /// Throw an exception and unwind the stack
    pub(super) fn throw_exception(&mut self, exception: Value) -> VMResult<ExecutionResult> {
        self.current_exception = Some(exception.clone());

        // Find the nearest exception handler
        while let Some(mut handler) = self.exception_handlers.pop() {
            // Restore stack to the state when try block started
            while self.stack.len() > handler.stack_size {
                self.stack.pop()?;
            }

            // Push the exception onto the stack for the catch block
            self.stack.push(exception.clone())?;

            // Jump to catch block
            self.ip = handler.catch_address;

            // If there's a finally block, we need to remember to execute it
            if handler.finally_address.is_some() {
                self.exception_handlers.push(handler);
            }

            return Ok(ExecutionResult::Continue);
        }

        // No exception handler found, terminate with error
        if let Value::Exception(exc_ref) = &exception {
            Err(VMError::RuntimeError(format!("{}: {}", exc_ref.exception_type, exc_ref.message)))
        } else {
            Err(VMError::RuntimeError(format!("Uncaught exception: {}", exception.to_string())))
        }
    }
}
