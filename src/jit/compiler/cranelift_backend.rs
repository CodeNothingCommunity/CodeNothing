//! Cranelift backend for JIT compilation

use crate::core::bytecode::{Instruction, OpCode};
use crate::core::value::Value;
use crate::jit::native_function::{NativeFunction, FunctionSignature, FunctionMetadata};

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use cranelift_codegen::ir::condcodes::IntCC;
use std::collections::HashMap;

type CraneliftValue = cranelift::prelude::Value;

/// Cranelift backend for JIT compilation
pub struct CraneliftBackend {
    /// Cranelift JIT module
    pub module: JITModule,
    /// Function builder context (reused for efficiency)
    pub builder_context: FunctionBuilderContext,
    /// Codegen context (reused for efficiency)
    pub context: codegen::Context,
}

impl CraneliftBackend {
    /// Create a new Cranelift backend
    pub fn new() -> Self {
        // Enable aggressive optimizations in Cranelift
        let mut flag_builder = cranelift::codegen::settings::builder();
        flag_builder.set("opt_level", "speed").unwrap();
        flag_builder.set("enable_verifier", "false").unwrap(); // Disable for performance
        let flags = cranelift::codegen::settings::Flags::new(flag_builder);

        let isa = cranelift::codegen::isa::lookup_by_name("x86_64")
            .unwrap()
            .finish(flags)
            .unwrap();

        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let module = JITModule::new(builder);
        let context = module.make_context();

        Self {
            module,
            builder_context: FunctionBuilderContext::new(),
            context,
        }
    }

    /// Create a new Cranelift context for linear compilation
    pub fn create_cranelift_context(&self) -> codegen::Context {
        let mut context = self.module.make_context();

        // Create a simple function signature for linear arithmetic
        // Takes one i64 parameter (for local variable access) and returns i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        context.func.signature = sig;
        context
    }

    /// Compile a Cranelift function to native code
    pub fn compile_cranelift_function(&self, mut context: codegen::Context) -> Result<NativeFunction, String> {
        // Verify the function (skip verification for now to avoid ISA access issues)
        // TODO: Add proper verification when ISA is accessible

        // Compile to native code (create a temporary module for linear compilation)
        let builder = JITBuilder::new(cranelift_module::default_libcall_names()).unwrap();
        let mut temp_module = JITModule::new(builder);

        let func_id = temp_module.declare_function("jit_func", Linkage::Export, &context.func.signature)
            .map_err(|e| format!("Failed to declare function: {}", e))?;

        temp_module.define_function(func_id, &mut context)
            .map_err(|e| format!("Failed to define function: {}", e))?;

        temp_module.finalize_definitions()
            .map_err(|e| format!("Failed to finalize definitions: {}", e))?;

        let code_ptr = temp_module.get_finalized_function(func_id);

        let signature = FunctionSignature::SingleInt;

        let metadata = FunctionMetadata::new("jit_linear_func".to_string(), 0);

        Ok(NativeFunction::new(code_ptr, signature, metadata))
    }

    /// Compile bytecode instructions to Cranelift IR
    pub fn compile_bytecode_to_cranelift_static(
        builder: &mut FunctionBuilder,
        params: &[CraneliftValue],
        instructions: &[Instruction],
        constants: &[Value]
    ) -> Result<CraneliftValue, String> {
        let mut stack = Vec::new();
        let mut locals = HashMap::new();

        // Initialize parameters as local variables
        for (i, &param) in params.iter().enumerate() {
            locals.insert(i, param);
        }

        // Create blocks for all jump targets first
        let mut blocks = HashMap::new();
        Self::create_blocks_for_jumps_static(builder, instructions, &mut blocks);

        // Compile each instruction
        for (pc, instruction) in instructions.iter().enumerate() {
            // If this instruction is a jump target, switch to its block
            if let Some(&block) = blocks.get(&pc) {
                builder.switch_to_block(block);
            }

            match instruction.opcode {
                OpCode::Const => {
                    let index = Self::decode_u16_operand_static(instruction)?;
                    if let Some(constant) = constants.get(index as usize) {
                        let value = Self::compile_constant_static(builder, constant)?;
                        stack.push(value);
                    } else {
                        return Err(format!("Invalid constant index: {}", index));
                    }
                }

                OpCode::LoadLocal => {
                    if instruction.operands.is_empty() {
                        return Err("LoadLocal missing operand".to_string());
                    }
                    let index = instruction.operands[0] as usize;
                    if let Some(&value) = locals.get(&index) {
                        stack.push(value);
                    } else {
                        return Err(format!("Undefined local variable: {}", index));
                    }
                }

                OpCode::StoreLocal => {
                    if instruction.operands.is_empty() {
                        return Err("StoreLocal missing operand".to_string());
                    }
                    let index = instruction.operands[0] as usize;
                    if let Some(value) = stack.pop() {
                        locals.insert(index, value);
                    } else {
                        return Err("Stack underflow in StoreLocal".to_string());
                    }
                }

                OpCode::Add => {
                    if stack.len() < 2 {
                        return Err("Stack underflow in Add".to_string());
                    }
                    let b = stack.pop().unwrap();
                    let a = stack.pop().unwrap();
                    let result = builder.ins().iadd(a, b);
                    stack.push(result);
                }

                OpCode::Sub => {
                    if stack.len() < 2 {
                        return Err("Stack underflow in Sub".to_string());
                    }
                    let b = stack.pop().unwrap();
                    let a = stack.pop().unwrap();
                    let result = builder.ins().isub(a, b);
                    stack.push(result);
                }

                OpCode::Mul => {
                    if stack.len() < 2 {
                        return Err("Stack underflow in Mul".to_string());
                    }
                    let b = stack.pop().unwrap();
                    let a = stack.pop().unwrap();
                    let result = builder.ins().imul(a, b);
                    stack.push(result);
                }

                OpCode::Div => {
                    if stack.len() < 2 {
                        return Err("Stack underflow in Div".to_string());
                    }
                    let b = stack.pop().unwrap();
                    let a = stack.pop().unwrap();
                    let result = builder.ins().sdiv(a, b);
                    stack.push(result);
                }

                OpCode::Eq => {
                    if stack.len() < 2 {
                        return Err("Stack underflow in Eq".to_string());
                    }
                    let b = stack.pop().unwrap();
                    let a = stack.pop().unwrap();
                    let result = builder.ins().icmp(IntCC::Equal, a, b);
                    let extended = builder.ins().uextend(types::I64, result);
                    stack.push(extended);
                }

                OpCode::Lt => {
                    if stack.len() < 2 {
                        return Err("Stack underflow in Lt".to_string());
                    }
                    let b = stack.pop().unwrap();
                    let a = stack.pop().unwrap();
                    let result = builder.ins().icmp(IntCC::SignedLessThan, a, b);
                    let extended = builder.ins().uextend(types::I64, result);
                    stack.push(extended);
                }

                OpCode::Le => {
                    if stack.len() < 2 {
                        return Err("Stack underflow in Le".to_string());
                    }
                    let b = stack.pop().unwrap();
                    let a = stack.pop().unwrap();
                    let result = builder.ins().icmp(IntCC::SignedLessThanOrEqual, a, b);
                    let extended = builder.ins().uextend(types::I64, result);
                    stack.push(extended);
                }

                OpCode::JumpIfFalse => {
                    if stack.is_empty() {
                        return Err("Stack underflow in JumpIfFalse".to_string());
                    }
                    let condition = stack.pop().unwrap();
                    let target = Self::calculate_jump_target_static(pc, instruction)?;

                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_false = builder.ins().icmp(IntCC::Equal, condition, zero);

                    if let Some(&target_block) = blocks.get(&target) {
                        let next_block = builder.create_block();
                        builder.ins().brif(is_false, target_block, &[], next_block, &[]);
                        builder.switch_to_block(next_block);
                    } else {
                        return Err(format!("Invalid jump target: {}", target));
                    }
                }

                OpCode::Jump => {
                    let target = Self::calculate_jump_target_static(pc, instruction)?;
                    if let Some(&target_block) = blocks.get(&target) {
                        builder.ins().jump(target_block, &[]);
                        let next_block = builder.create_block();
                        builder.switch_to_block(next_block);
                    } else {
                        return Err(format!("Invalid jump target: {}", target));
                    }
                }

                OpCode::ReturnValue => {
                    if let Some(value) = stack.pop() {
                        builder.ins().return_(&[value]);
                        return Ok(value);
                    } else {
                        return Err("Stack underflow in ReturnValue".to_string());
                    }
                }

                _ => {
                    return Err(format!("Unsupported instruction for JIT: {:?}", instruction.opcode));
                }
            }
        }

        // If we reach here without a return, return the top of stack or zero
        if let Some(value) = stack.pop() {
            builder.ins().return_(&[value]);
            Ok(value)
        } else {
            let zero = builder.ins().iconst(types::I64, 0);
            builder.ins().return_(&[zero]);
            Ok(zero)
        }
    }

    /// Compile a constant value to Cranelift IR
    pub fn compile_constant_static(builder: &mut FunctionBuilder, constant: &Value) -> Result<CraneliftValue, String> {
        match constant {
            Value::Int(i) => Ok(builder.ins().iconst(types::I64, *i)),
            Value::Float(f) => Ok(builder.ins().f64const(*f)),
            Value::Bool(b) => Ok(builder.ins().iconst(types::I64, if *b { 1 } else { 0 })),
            _ => Err(format!("Unsupported constant type: {:?}", constant)),
        }
    }

    /// Create blocks for all jump targets - improved for Cranelift stability
    pub fn create_blocks_for_jumps_static(
        builder: &mut FunctionBuilder,
        instructions: &[Instruction],
        blocks: &mut HashMap<usize, Block>
    ) {
        // Collect all jump targets first
        let mut targets = std::collections::HashSet::new();

        for (pc, instruction) in instructions.iter().enumerate() {
            match instruction.opcode {
                OpCode::Jump | OpCode::JumpIfFalse | OpCode::JumpIfTrue => {
                    if let Ok(target) = Self::calculate_jump_target_static(pc, instruction) {
                        targets.insert(target);
                    }
                }
                _ => {}
            }
        }

        // Create blocks for all targets
        for &target in &targets {
            if target < instructions.len() {
                let block = builder.create_block();
                blocks.insert(target, block);
            }
        }
    }

    /// Decode a 16-bit operand from instruction bytes
    pub fn decode_u16_operand_static(instruction: &Instruction) -> Result<u16, String> {
        if instruction.operands.len() < 2 {
            return Err("Instruction missing operands".to_string());
        }
        // Use little-endian byte order to match VM implementation
        let bytes = [instruction.operands[0], instruction.operands[1]];
        Ok(u16::from_le_bytes(bytes))
    }

    /// Calculate jump target address
    pub fn calculate_jump_target_static(pc: usize, instruction: &Instruction) -> Result<usize, String> {
        if instruction.operands.len() < 2 {
            return Err("Jump instruction missing operands".to_string());
        }

        let offset_bytes = [instruction.operands[0], instruction.operands[1]];
        let offset = i16::from_le_bytes(offset_bytes);
        let target = (pc as i32 + offset as i32) as usize; // Match VM implementation: pc + offset
        Ok(target)
    }
}
