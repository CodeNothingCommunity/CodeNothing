//! Function call operations for the VM

use crate::core::{
    bytecode::{Instruction, OpCode},
    memory::LocalMemory,
    stack::CallFrame,
    value::Value,
};
use super::{
    errors::{VMError, VMResult, NativeFunction},
    types::{VirtualMachine, ExecutionResult},
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

impl VirtualMachine {
    /// Register a native function
    pub fn register_native_function(&mut self, id: u32, function: NativeFunction) {
        self.native_functions.insert(id, function);
    }

    /// Mark a function as memoized
    pub fn mark_function_as_memoized(&mut self, function_index: usize) {
        self.memo_functions.insert(function_index);
        self.memo_cache.insert(function_index, std::collections::HashMap::new());
    }

    /// Check if a function is memoized
    pub fn is_function_memoized(&self, function_index: usize) -> bool {
        self.memo_functions.contains(&function_index)
    }

    /// Hash function arguments for memoization
    pub(super) fn hash_args(&self, args: &[Value]) -> u64 {
        let mut hasher = DefaultHasher::new();
        for arg in args {
            match arg {
                Value::Int(i) => i.hash(&mut hasher),
                Value::Float(f) => f.to_bits().hash(&mut hasher),
                Value::String(s) => s.hash(&mut hasher),
                Value::Bool(b) => b.hash(&mut hasher),
                Value::Null => 0u8.hash(&mut hasher),
                _ => format!("{:?}", arg).hash(&mut hasher), // Fallback for complex types
            }
        }
        hasher.finish()
    }

    /// Get cached result for memoized function
    pub(super) fn get_memo_result(&self, function_index: usize, args: &[Value]) -> Option<&Value> {
        let args_hash = self.hash_args(args);
        self.memo_cache.get(&function_index)?.get(&args_hash)
    }

    /// Cache result for memoized function
    pub(super) fn cache_memo_result(&mut self, function_index: usize, args: &[Value], result: Value) {
        let args_hash = self.hash_args(args);
        if let Some(cache) = self.memo_cache.get_mut(&function_index) {
            cache.insert(args_hash, result);
        }
    }

    /// Execute function call instructions
    pub(super) fn execute_function_instruction(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        match instruction.opcode {
            OpCode::Call => {
                self.execute_call(instruction)
            }

            OpCode::Return => {
                self.execute_return()
            }

            OpCode::ReturnValue => {
                self.execute_return_value()
            }

            OpCode::CallMethod => {
                self.execute_call_method(instruction)
            }

            _ => Err(VMError::InvalidInstruction(format!("Not a function instruction: {:?}", instruction.opcode)))
        }
    }

    /// Execute function call
    fn execute_call(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        let function_index = instruction.get_u16(0)
            .ok_or_else(|| VMError::InvalidOperand("Missing function index".to_string()))?;

        // Debug: Log function calls when JIT debug is enabled
        if self.jit_engine.debug {
            println!("🔧 VM: OpCode::Call executed for function {}", function_index);
        }

        if function_index as usize >= self.chunks.len() {
            return Err(VMError::RuntimeError(format!("Invalid function index: {}", function_index)));
        }

        // Check and potentially compile with JIT
        self.check_jit_compilation(function_index as usize);

        // Get parameter count from the top of the stack
        let param_count_value = self.stack.pop()?;
        let param_count = match param_count_value {
            Value::Int(n) => n as usize,
            _ => return Err(VMError::TypeMismatch("Parameter count must be an integer".to_string())),
        };

        // Handle memoization for memoized functions
        let mut memo_params = Vec::new();
        let is_memoized = self.is_function_memoized(function_index as usize);

        if is_memoized {
            if self.debug_mode {
                println!("🔧 MEMO: Function {} is memoized, checking cache", function_index);
            }
            // Only for memoized functions: pop parameters for caching
            memo_params.reserve(param_count);
            for _ in 0..param_count {
                memo_params.push(self.stack.pop()?);
            }
            memo_params.reverse(); // Parameters were pushed in order, so we need to reverse

            if self.debug_mode {
                println!("🔧 MEMO: Function {} called with args: {:?}", function_index, memo_params);
            }

            // Check if we have a cached result
            if let Some(cached_result) = self.get_memo_result(function_index as usize, &memo_params) {
                if self.debug_mode {
                    println!("🔧 MEMO: Cache HIT for function {} with args {:?} -> {:?}", function_index, memo_params, cached_result);
                }
                // Cache hit! Push the cached result and continue
                self.stack.push(cached_result.clone())?;
                self.ip += 1;
                return Ok(ExecutionResult::Continue);
            }

            if self.debug_mode {
                println!("🔧 MEMO: Cache MISS for function {} with args {:?}", function_index, memo_params);
            }
            // Cache miss - we need to call the function
            // Parameters are already popped, so we need to push them back for the function call
            for param in memo_params.iter().rev() {
                self.stack.push(param.clone())?;
            }
        } else if self.debug_mode {
            println!("🔧 MEMO: Function {} is NOT memoized", function_index);
        }

        // Record execution for hot spot detection (only for actual function calls, not cache hits)
        self.record_jit_execution(function_index as usize, 0);

        // Check call stack depth to prevent stack overflow
        if self.call_stack.depth() >= self.memory.config.max_call_depth {
            let exception = Value::runtime_error("Call stack overflow - maximum recursion depth exceeded".to_string());
            return self.throw_exception(exception);
        }

        // Pop parameters from stack and store them in local variables (in reverse order)
        let mut params = Vec::with_capacity(param_count);
        for _ in 0..param_count {
            params.push(self.stack.pop()?);
        }
        params.reverse(); // Parameters were pushed in order, so we need to reverse

        // Ultra-lightweight call frame - only save essential data
        let saved_locals = std::mem::replace(&mut self.locals, None);
        let saved_chunk = self.current_chunk.take();
        let saved_ip = self.ip + 1;

        // Create minimal call frame
        let call_frame = if is_memoized {
            CallFrame {
                function_name: String::new(),
                return_address: saved_ip,
                locals_start: 0,
                locals_count: 0,
                saved_locals,
                saved_chunk,
                function_index: function_index as usize,
                function_args: memo_params,
            }
        } else {
            CallFrame {
                function_name: String::new(),
                return_address: saved_ip,
                locals_start: 0,
                locals_count: 0,
                saved_locals,
                saved_chunk,
                function_index: 0,
                function_args: Vec::new(),
            }
        };

        // Push call frame to call stack
        self.call_stack.push(call_frame).map_err(|_| {
            VMError::RuntimeError("Call stack overflow".to_string())
        })?;

        // Create new local memory for the function - use minimal capacity
        let mut new_locals = LocalMemory::new(1); // Fibonacci only needs 1 local variable

        // Store parameters in local variables
        for (i, param_value) in params.into_iter().enumerate() {
            new_locals.set(i, param_value)?;
        }

        // Save current state and set up new function context
        self.locals = Some(new_locals);
        let function_chunk = &self.chunks[function_index as usize];
        self.current_chunk = Some(function_chunk.clone());
        self.ip = 0;

        Ok(ExecutionResult::Continue)
    }

    /// Execute return instruction
    fn execute_return(&mut self) -> VMResult<ExecutionResult> {
        // Check if we're in a function call
        if self.call_stack.depth() > 0 {
            // Pop the call frame
            let call_frame = self.call_stack.pop().map_err(|_| {
                VMError::RuntimeError("Call stack underflow".to_string())
            })?;

            // Return to the caller's context
            self.ip = call_frame.return_address;
            self.locals = call_frame.saved_locals; // Restore caller's locals

            // Only restore chunk if there was a saved chunk (not main function)
            if let Some(saved_chunk) = call_frame.saved_chunk {
                self.current_chunk = Some(saved_chunk);
            }
            // If no saved chunk, keep current chunk (main function)

            // Push null as return value
            self.stack.push(Value::Null)?;

            Ok(ExecutionResult::Continue)
        } else {
            // Main function return - terminate execution
            Ok(ExecutionResult::Return(Value::Null))
        }
    }

    /// Execute return with value instruction
    fn execute_return_value(&mut self) -> VMResult<ExecutionResult> {
        let value = self.stack.pop()?;

        // Check if we're in a function call
        if self.call_stack.depth() > 0 {
            // Pop the call frame
            let call_frame = self.call_stack.pop().map_err(|_| {
                VMError::RuntimeError("Call stack underflow".to_string())
            })?;

            // Cache the result if this is a memoized function
            if self.is_function_memoized(call_frame.function_index) {
                if self.debug_mode {
                    println!("🔧 MEMO: Caching result for function {} with args {:?} -> {:?}", call_frame.function_index, call_frame.function_args, value);
                }
                self.cache_memo_result(call_frame.function_index, &call_frame.function_args, value.clone());
            }

            // Return to the caller's context
            self.ip = call_frame.return_address;
            self.locals = call_frame.saved_locals; // Restore caller's locals

            // Only restore chunk if there was a saved chunk (not main function)
            if let Some(saved_chunk) = call_frame.saved_chunk {
                self.current_chunk = Some(saved_chunk);
            }
            // If no saved chunk, keep current chunk (main function)

            // Push return value to stack
            self.stack.push(value)?;

            Ok(ExecutionResult::Continue)
        } else {
            // Main function return - terminate execution
            Ok(ExecutionResult::Return(value))
        }
    }

    /// Execute method call instruction
    fn execute_call_method(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        let method_index = instruction.get_u16(0)
            .ok_or_else(|| VMError::InvalidOperand("Missing method index".to_string()))?;
        let chunk = self.current_chunk.as_ref().unwrap();
        let method_name = chunk.get_constant(method_index)
            .ok_or_else(|| VMError::InvalidOperand(format!("Invalid method index: {}", method_index)))?;

        // Debug: Log method calls when JIT debug is enabled
        if self.jit_engine.debug {
            println!("🔧 VM: OpCode::CallMethod executed for method index {}", method_index);
        }

        if let Value::String(method_name_str) = method_name {
            let method_name_owned = method_name_str.clone();
            self.call_builtin_method(&method_name_owned)?;
        } else {
            return Err(VMError::TypeMismatch("Method name must be a string".to_string()));
        }

        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    /// Call a built-in method
    pub(super) fn call_builtin_method(&mut self, method_name: &str) -> VMResult<()> {
        use crate::lang::builtins::BuiltinRegistry;

        // Get the builtin function
        let registry = BuiltinRegistry::new();
        if let Some((builtin_fn, function_type)) = registry.get(method_name) {
            // Get parameter count from function type
            let param_count = match function_type {
                crate::lang::types::Type::Function { parameters, .. } => parameters.len(),
                _ => 0,
            };

            // Collect arguments from stack
            let mut args = Vec::new();
            for _ in 0..param_count {
                if !self.stack.is_empty() {
                    args.push(self.stack.pop()?);
                }
            }

            // Reverse args since we popped them in reverse order
            args.reverse();

            // Call the builtin function directly
            match builtin_fn(&args) {
                Ok(result) => {
                    // Always push result back to stack, even if it's null
                    // The caller expects a return value on the stack
                    self.stack.push(result)?;
                    Ok(())
                }
                Err(e) => {
                    Err(VMError::RuntimeError(e))
                }
            }
        } else {
            Err(VMError::UndefinedFunction(method_name.to_string()))
        }
    }
}
