//! Recursive function compiler for JIT

use crate::core::bytecode::{Chunk, Instruction};
use crate::jit::native_function::NativeFunction;
use super::{
    analysis::CodeAnalyzer,
    linear_compiler::LinearCompiler,
    config::JitConfig,
};

use cranelift::prelude::*;

type CraneliftValue = cranelift::prelude::Value;

/// Recursive function compiler
pub struct RecursiveCompiler<'a> {
    analyzer: &'a CodeAnalyzer<'a>,
    linear_compiler: &'a LinearCompiler<'a>,
}

impl<'a> RecursiveCompiler<'a> {
    pub fn new(analyzer: &'a CodeAnalyzer<'a>, linear_compiler: &'a LinearCompiler<'a>) -> Self {
        Self {
            analyzer,
            linear_compiler,
        }
    }

    /// Compile recursive function using block-based approach
    pub fn compile_recursive_function_blocks(&self, chunk: &Chunk, params: Vec<CraneliftValue>) -> Result<NativeFunction, String> {
        self.compile_recursive_function_blocks_with_debug(chunk, params, false)
    }

    /// Compile recursive function using block-based approach with debug output
    pub fn compile_recursive_function_blocks_with_debug(&self, chunk: &Chunk, params: Vec<CraneliftValue>, debug: bool) -> Result<NativeFunction, String> {
        if debug {
            println!("🔧 Block-based JIT: Compiling recursive function in independent blocks");
        }

        // Analyze and split the function into basic blocks
        let blocks = self.analyzer.analyze_basic_blocks(chunk)?;

        if debug {
            println!("🔧 Block-based JIT: Found {} basic blocks", blocks.len());
            for (i, block) in blocks.iter().enumerate() {
                println!("  Block {}: {} instructions", i, block.len());
                let is_arithmetic = self.analyzer.is_simple_arithmetic_block(block);
                println!("    - Simple arithmetic: {}", is_arithmetic);
            }
        }

        // For now, compile only the arithmetic parts, use interpretation for control flow
        self.compile_arithmetic_only_function_with_debug(chunk, params, debug)
    }

    /// Compile only the arithmetic parts of a recursive function
    pub fn compile_arithmetic_only_function(&self, chunk: &Chunk, params: Vec<CraneliftValue>) -> Result<NativeFunction, String> {
        self.compile_arithmetic_only_function_with_debug(chunk, params, false)
    }

    /// Compile only the arithmetic parts of a recursive function with debug output
    pub fn compile_arithmetic_only_function_with_debug(&self, chunk: &Chunk, params: Vec<CraneliftValue>, debug: bool) -> Result<NativeFunction, String> {
        if debug {
            println!("🎯 Arithmetic-only JIT: Compiling safe operations");
        }

        // Create a simplified version that only includes safe operations
        let safe_instructions = self.analyzer.extract_safe_instructions(chunk);

        if debug {
            println!("🎯 Arithmetic-only JIT: Extracted {} safe instructions from {} total", 
                     safe_instructions.len(), chunk.instructions.len());
        }

        // Compile the safe instructions as a linear sequence
        // but with a linear control flow (no jumps)
        self.linear_compiler.compile_linear_instructions_with_debug(&safe_instructions, params, debug)
    }
}
