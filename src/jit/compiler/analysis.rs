//! Code analysis for JIT compilation

use crate::core::bytecode::{Chunk, Instruction, OpCode};
use super::config::JitConfig;

/// Code analysis utilities for JIT compilation
pub struct CodeAnalyzer<'a> {
    config: &'a JitConfig,
}

impl<'a> CodeAnalyzer<'a> {
    pub fn new(config: &'a JitConfig) -> Self {
        Self { config }
    }

    /// Check if a chunk is suitable for JIT compilation
    pub fn is_suitable_for_jit(&self, chunk: &Chunk) -> bool {
        self.is_suitable_for_jit_with_debug(chunk, false)
    }

    /// Check if a chunk is suitable for JIT compilation with debug output
    pub fn is_suitable_for_jit_with_debug(&self, chunk: &Chunk, debug: bool) -> bool {
        // Check function size
        if chunk.instruction_count() > self.config.max_function_size {
            return false;
        }

        // Check for unsupported operations
        for instruction in &chunk.instructions {
            match instruction.opcode {
                // Unsupported operations that prevent JIT compilation
                OpCode::CallMethod | OpCode::CreateArray | OpCode::CreateObject |
                OpCode::LoadField | OpCode::Throw | OpCode::TryStart | OpCode::TryEnd |
                OpCode::Catch | OpCode::Finally => {
                    if debug {
                        println!("🚫 JIT: Unsupported operation: {:?}", instruction.opcode);
                    }
                    return false;
                }
                _ => {}
            }
        }

        // Additional checks for complex control flow
        let has_complex_jumps = self.has_complex_control_flow(chunk);
        if has_complex_jumps {
            if debug {
                println!("🚫 JIT: Complex control flow detected");
            }
            return false;
        }

        true
    }

    /// Count the number of parameters a function expects
    pub fn count_function_parameters(&self, chunk: &Chunk) -> usize {
        // Simple heuristic: count the highest LoadLocal index + 1
        let mut max_local = 0;
        for instruction in &chunk.instructions {
            if instruction.opcode == OpCode::LoadLocal && !instruction.operands.is_empty() {
                let local_index = instruction.operands[0] as usize;
                max_local = max_local.max(local_index);
            }
        }

        // For two parameter functions, we expect LoadLocal 0 and LoadLocal 1
        if max_local == 0 { 1 } else { max_local + 1 }
    }

    /// Check if a chunk has recursive call patterns
    pub fn has_recursive_pattern(&self, chunk: &Chunk) -> bool {
        let mut has_call = false;
        let mut has_conditional = false;

        for instruction in &chunk.instructions {
            match instruction.opcode {
                OpCode::Call => has_call = true,
                OpCode::JumpIfFalse | OpCode::JumpIfTrue => has_conditional = true,
                _ => {}
            }
        }

        // Heuristic: function with both calls and conditionals is likely recursive
        has_call && has_conditional
    }

    /// Check if a recursive function can be compiled using block-based JIT
    pub fn is_simple_recursive_function(&self, chunk: &Chunk) -> bool {
        self.is_simple_recursive_function_with_debug(chunk, false)
    }

    /// Check if a recursive function can be compiled using block-based JIT with debug output
    pub fn is_simple_recursive_function_with_debug(&self, chunk: &Chunk, debug: bool) -> bool {
        // New approach: Enable recursive functions for block-based JIT compilation
        // We'll split them into independent basic blocks
        let has_recursive_calls = self.has_recursive_pattern(chunk);
        if has_recursive_calls {
            // Check if the function is suitable for block-based compilation
            return self.can_compile_as_blocks_with_debug(chunk, debug);
        }
        false
    }

    /// Check if a function can be compiled as independent basic blocks
    pub fn can_compile_as_blocks(&self, chunk: &Chunk) -> bool {
        self.can_compile_as_blocks_with_debug(chunk, false)
    }

    /// Check if a function can be compiled as independent basic blocks with debug output
    pub fn can_compile_as_blocks_with_debug(&self, chunk: &Chunk, debug: bool) -> bool {
        // Analyze the function structure for block-based compilation
        let mut has_simple_if = false;
        let mut has_arithmetic = false;
        let mut has_return = false;
        let mut has_call = false;

        for instruction in &chunk.instructions {
            match instruction.opcode {
                OpCode::JumpIfFalse | OpCode::JumpIfTrue => has_simple_if = true,
                OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div | OpCode::Mod => has_arithmetic = true,
                OpCode::Return | OpCode::ReturnValue => has_return = true,
                OpCode::Call => has_call = true,
                // Unsupported operations for block-based compilation
                OpCode::CallMethod | OpCode::CreateArray | OpCode::CreateObject |
                OpCode::LoadField | OpCode::Throw | OpCode::TryStart | OpCode::TryEnd |
                OpCode::Catch | OpCode::Finally => {
                    if debug {
                        println!("🚫 Block JIT: Unsupported operation: {:?}", instruction.opcode);
                    }
                    return false;
                }
                _ => {}
            }
        }

        let is_suitable = has_simple_if && has_arithmetic && has_return;

        if debug {
            println!("🔧 Block JIT Analysis:");
            println!("  - Simple conditionals: {}", has_simple_if);
            println!("  - Arithmetic operations: {}", has_arithmetic);
            println!("  - Return statements: {}", has_return);
            println!("  - Function calls: {}", has_call);
            println!("  - Suitable for block compilation: {}", is_suitable);
        }

        is_suitable
    }

    /// Analyze function and split into basic blocks
    pub fn analyze_basic_blocks<'b>(&self, chunk: &'b Chunk) -> Result<Vec<Vec<&'b Instruction>>, String> {
        let mut blocks = Vec::new();
        let mut current_block = Vec::new();

        for instruction in &chunk.instructions {
            current_block.push(instruction);

            // End block on control flow instructions
            match instruction.opcode {
                OpCode::Jump | OpCode::JumpIfFalse | OpCode::JumpIfTrue |
                OpCode::Return | OpCode::ReturnValue | OpCode::Call => {
                    blocks.push(current_block);
                    current_block = Vec::new();
                }
                _ => {}
            }
        }

        // Add remaining instructions as final block
        if !current_block.is_empty() {
            blocks.push(current_block);
        }

        Ok(blocks)
    }

    /// Check if a block contains only simple arithmetic operations
    pub fn is_simple_arithmetic_block(&self, block: &[&Instruction]) -> bool {
        for instruction in block {
            match instruction.opcode {
                OpCode::Const | OpCode::LoadLocal | OpCode::StoreLocal |
                OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div |
                OpCode::Eq | OpCode::Lt | OpCode::Le => {
                    // These are safe operations
                }
                _ => {
                    return false;
                }
            }
        }
        true
    }

    /// Extract only the safe instructions that can be JIT compiled
    pub fn extract_safe_instructions<'b>(&self, chunk: &'b Chunk) -> Vec<&'b Instruction> {
        let mut safe_instructions = Vec::new();

        for instruction in &chunk.instructions {
            match instruction.opcode {
                // Safe operations for JIT compilation
                OpCode::Const | OpCode::LoadLocal | OpCode::StoreLocal |
                OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div |
                OpCode::Eq | OpCode::Lt | OpCode::Le | OpCode::ReturnValue => {
                    safe_instructions.push(instruction);
                }
                _ => {} // Skip unsafe operations
            }
        }

        safe_instructions
    }

    /// Check for complex control flow patterns
    fn has_complex_control_flow(&self, chunk: &Chunk) -> bool {
        let mut jump_count = 0;
        let mut has_nested_calls = false;

        for instruction in &chunk.instructions {
            match instruction.opcode {
                OpCode::Jump | OpCode::JumpIfFalse | OpCode::JumpIfTrue => {
                    jump_count += 1;
                }
                OpCode::Call => {
                    if jump_count > 0 {
                        has_nested_calls = true;
                    }
                }
                _ => {}
            }
        }

        // Consider complex if too many jumps or nested calls
        jump_count > 5 || has_nested_calls
    }
}
