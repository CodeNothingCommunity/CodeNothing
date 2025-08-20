//! Linear instruction compiler for JIT

use crate::core::bytecode::Instruction;
use crate::jit::native_function::NativeFunction;
use super::cranelift_backend::CraneliftBackend;

use cranelift::prelude::*;

type CraneliftValue = cranelift::prelude::Value;

/// Linear instruction compiler
pub struct LinearCompiler<'a> {
    backend: &'a CraneliftBackend,
}

impl<'a> LinearCompiler<'a> {
    pub fn new(backend: &'a CraneliftBackend) -> Self {
        Self { backend }
    }

    /// Compile a linear sequence of safe instructions
    pub fn compile_linear_instructions(&self, instructions: &[&Instruction], params: Vec<CraneliftValue>) -> Result<NativeFunction, String> {
        self.compile_linear_instructions_with_debug(instructions, params, false)
    }

    /// Compile a linear sequence of safe instructions with debug output
    pub fn compile_linear_instructions_with_debug(&self, instructions: &[&Instruction], params: Vec<CraneliftValue>, debug: bool) -> Result<NativeFunction, String> {
        if instructions.len() < 3 {
            return Err("Too few instructions for meaningful JIT compilation".to_string());
        }

        if debug {
            println!("🎯 Linear JIT: Compiling {} instructions", instructions.len());
            for (i, instruction) in instructions.iter().enumerate() {
                println!("  {}: {:?}", i, instruction.opcode);
            }
        }

        // Try different compilation strategies
        if let Ok(result) = self.compile_linear_arithmetic(instructions) {
            return Ok(result);
        } else {
            return Err("Failed to compile linear instructions".to_string());
        }
    }

    /// Compile linear arithmetic operations without control flow
    pub fn compile_linear_arithmetic(&self, instructions: &[&Instruction]) -> Result<NativeFunction, String> {
        self.compile_linear_arithmetic_with_debug(instructions, false)
    }

    /// Compile linear arithmetic operations without control flow with debug output
    pub fn compile_linear_arithmetic_with_debug(&self, instructions: &[&Instruction], debug: bool) -> Result<NativeFunction, String> {
        // Create a new Cranelift context for linear compilation
        let mut context = self.backend.create_cranelift_context();
        let mut builder_context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut context.func, &mut builder_context);

        // Create entry block
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);

        // Get the parameter (input value)
        let param = builder.block_params(entry_block)[0];

        if debug {
            println!("🎯 Linear Arithmetic JIT: Starting compilation");
            println!("  - Entry block created");
            println!("  - Parameter: {:?}", param);
        }

        // Simple linear compilation: just return the parameter for now
        // In a real implementation, you would process the instructions
        let mut result = param;

        // Process a subset of safe instructions
        for (i, instruction) in instructions.iter().enumerate() {
            match instruction.opcode {
                crate::core::bytecode::OpCode::Const => {
                    // For simplicity, add a constant value
                    let const_val = builder.ins().iconst(types::I64, 1);
                    result = builder.ins().iadd(result, const_val);
                    if debug {
                        println!("  - Instruction {}: Const -> Add 1", i);
                    }
                }
                crate::core::bytecode::OpCode::Add => {
                    // Add operation (simplified)
                    let const_val = builder.ins().iconst(types::I64, 1);
                    result = builder.ins().iadd(result, const_val);
                    if debug {
                        println!("  - Instruction {}: Add", i);
                    }
                }
                crate::core::bytecode::OpCode::Sub => {
                    // Subtract operation (simplified)
                    let const_val = builder.ins().iconst(types::I64, 1);
                    result = builder.ins().isub(result, const_val);
                    if debug {
                        println!("  - Instruction {}: Sub", i);
                    }
                }
                _ => {
                    if debug {
                        println!("  - Instruction {}: {:?} (skipped)", i, instruction.opcode);
                    }
                }
            }
        }

        // Return the result
        builder.ins().return_(&[result]);
        builder.seal_all_blocks();

        if debug {
            println!("🎯 Linear Arithmetic JIT: Compilation completed");
        }

        // Compile to native code
        let compiled = self.backend.compile_cranelift_function(context)?;

        if debug {
            println!("✅ Linear Arithmetic JIT: Native code generated successfully");
        }
        Ok(compiled)
    }
}
