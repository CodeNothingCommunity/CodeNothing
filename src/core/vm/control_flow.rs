//! Control flow operations for the VM

use crate::core::{
    bytecode::{Instruction, OpCode},
    value::Value,
};
use super::{
    errors::{VMError, VMResult},
    types::{VirtualMachine, ExecutionResult, ExceptionHandler},
};

impl VirtualMachine {
    /// Execute control flow instructions
    pub(super) fn execute_control_flow_instruction(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        match instruction.opcode {
            OpCode::Jump => {
                let target = instruction.get_u16(0)
                    .ok_or_else(|| VMError::InvalidOperand("Missing jump target".to_string()))?;

                let chunk = self.current_chunk.as_ref().unwrap();
                if target as usize >= chunk.instruction_count() {
                    return Err(VMError::InvalidJump(format!("Jump target {} is out of bounds", target)));
                }

                self.ip = target as usize;
                Ok(ExecutionResult::Continue)
            }

            OpCode::JumpIfFalse => {
                let target = instruction.get_u16(0)
                    .ok_or_else(|| VMError::InvalidOperand("Missing jump target".to_string()))?;
                let condition = self.stack.pop()?;
                if condition.is_falsy() {
                    let chunk = self.current_chunk.as_ref().unwrap();
                    if target as usize >= chunk.instruction_count() {
                        return Err(VMError::InvalidJump(format!("Jump target {} is out of bounds", target)));
                    }
                    self.ip = target as usize;
                } else {
                    self.ip += 1;
                }
                Ok(ExecutionResult::Continue)
            }

            OpCode::JumpIfTrue => {
                let target = instruction.get_u16(0)
                    .ok_or_else(|| VMError::InvalidOperand("Missing jump target".to_string()))?;
                let condition = self.stack.pop()?;
                if condition.is_truthy() {
                    let chunk = self.current_chunk.as_ref().unwrap();
                    if target as usize >= chunk.instruction_count() {
                        return Err(VMError::InvalidJump(format!("Jump target {} is out of bounds", target)));
                    }
                    self.ip = target as usize;
                } else {
                    self.ip += 1;
                }
                Ok(ExecutionResult::Continue)
            }

            OpCode::JumpIfNull => {
                let offset = instruction.get_i16(0)
                    .ok_or_else(|| VMError::InvalidOperand("Missing jump offset".to_string()))?;
                let value = self.stack.pop()?;
                if matches!(value, Value::Null) {
                    self.ip = self.calculate_jump_target(offset)?;
                } else {
                    self.ip += 1;
                }
                Ok(ExecutionResult::Continue)
            }

            OpCode::JumpIfNotNull => {
                let offset = instruction.get_i16(0)
                    .ok_or_else(|| VMError::InvalidOperand("Missing jump offset".to_string()))?;
                let value = self.stack.pop()?;
                if !matches!(value, Value::Null) {
                    self.ip = self.calculate_jump_target(offset)?;
                } else {
                    self.ip += 1;
                }
                Ok(ExecutionResult::Continue)
            }

            // Exception handling operations
            OpCode::Throw => {
                let exception = self.stack.pop()?;
                self.throw_exception(exception)
            }

            OpCode::TryStart => {
                let catch_offset = instruction.get_u16(0)
                    .ok_or_else(|| VMError::InvalidOperand("Missing catch offset".to_string()))?;

                // Calculate absolute catch address from the TryStart instruction
                let catch_address = self.ip + 1 + catch_offset as usize;
                let handler = ExceptionHandler {
                    catch_address,
                    finally_address: None, // Will be set by Finally instruction
                    stack_size: self.stack.len(),
                    finally_executed: false,
                };

                self.exception_handlers.push(handler);
                self.ip += 1;
                Ok(ExecutionResult::Continue)
            }

            OpCode::TryEnd => {
                // This marks the end of a try block
                // We need to check if there's a finally block to execute
                if !self.exception_handlers.is_empty() {
                    let handler = self.exception_handlers.last().unwrap().clone();

                    // If there's a finally block and it hasn't been executed yet
                    if let Some(finally_address) = handler.finally_address {
                        if !handler.finally_executed {
                            // Mark finally as executed and jump to it
                            self.exception_handlers.last_mut().unwrap().finally_executed = true;
                            self.ip = finally_address;
                            return Ok(ExecutionResult::Continue);
                        }
                    }

                    // Remove the exception handler since we're done with the try block
                    self.exception_handlers.pop();
                }

                self.ip += 1;
                Ok(ExecutionResult::Continue)
            }

            OpCode::Catch => {
                // This should only be reached when an exception is thrown
                // The exception should already be on the stack
                self.ip += 1;
                Ok(ExecutionResult::Continue)
            }

            OpCode::Finally => {
                // Mark the finally address in the current exception handler
                if let Some(handler) = self.exception_handlers.last_mut() {
                    handler.finally_address = Some(self.ip + 1);
                }

                self.ip += 1;
                Ok(ExecutionResult::Continue)
            }

            _ => Err(VMError::InvalidInstruction(format!("Not a control flow instruction: {:?}", instruction.opcode)))
        }
    }

    /// Calculate jump target address
    pub(super) fn calculate_jump_target(&self, offset: i16) -> VMResult<usize> {
        let current_ip = self.ip as i32;
        let target = current_ip + offset as i32;

        if target < 0 {
            return Err(VMError::InvalidJump("Jump target is negative".to_string()));
        }

        let target = target as usize;
        let chunk = self.current_chunk.as_ref().unwrap();

        if target >= chunk.instruction_count() {
            return Err(VMError::InvalidJump(format!("Jump target {} is out of bounds", target)));
        }

        Ok(target)
    }
}
