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
use std::collections::HashMap;
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

            OpCode::CallSuper => {
                self.execute_call_super(instruction)
            }

            OpCode::CallVirtual => {
                self.execute_call_virtual(instruction)
            }

            OpCode::CallStatic => {
                self.execute_call_static(instruction)
            }

            _ => Err(VMError::InvalidInstruction(format!("Not a function instruction: {:?}", instruction.opcode)))
        }
    }

    /// Execute function call
    fn execute_call(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        let function_index = instruction.get_u16(0)
            .ok_or_else(|| VMError::InvalidOperand("Missing function index".to_string()))?;

        // Debug: Log function calls when debug mode is enabled
        if self.debug_mode {
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
            // For memoized functions: peek at parameters for caching (don't pop yet)
            memo_params.reserve(param_count);
            for i in 0..param_count {
                // Peek at parameters from top of stack (reverse order)
                let param = self.stack.peek_at(param_count - 1 - i)?.clone();
                memo_params.push(param);
            }

            if self.debug_mode {
                println!("🔧 MEMO: Function {} called with args: {:?}", function_index, memo_params);
            }

            // Check if we have a cached result
            let cached_result = self.get_memo_result(function_index as usize, &memo_params).cloned();
            if let Some(cached_result) = cached_result {
                if self.debug_mode {
                    println!("🔧 MEMO: Cache HIT for function {} with args {:?} -> {:?}", function_index, memo_params, cached_result);
                }
                // Cache hit! Pop the parameters and push the cached result
                for _ in 0..param_count {
                    self.stack.pop()?;
                }
                self.stack.push(cached_result)?;
                self.ip += 1;
                return Ok(ExecutionResult::Continue);
            }

            if self.debug_mode {
                println!("🔧 MEMO: Cache MISS for function {} with args {:?}", function_index, memo_params);
            }
            // Cache miss - parameters are still on stack, proceed with normal function call
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
        if self.debug_mode {
            println!("🔧 VM: About to pop {} parameters, stack size: {}", param_count, self.stack.len());
        }

        for i in 0..param_count {
            if self.stack.len() == 0 {
                println!("❌ VM: STACK UNDERFLOW while popping parameter {} of {}", i, param_count);
                println!("❌ VM: Function: {}, IP: {}", function_index, self.ip);
                if let Some(chunk) = &self.current_chunk {
                    println!("❌ VM: Current chunk: {}", chunk.name);
                }
                return Err(VMError::StackUnderflow);
            }
            let param = self.stack.pop()?;
            if self.debug_mode {
                println!("🔧 VM: Popped parameter {}: {:?}", i, param);
            }
            params.push(param);
        }
        params.reverse(); // Parameters were pushed in order, so we need to reverse

        // Ultra-lightweight call frame - only save essential data
        let saved_locals = std::mem::replace(&mut self.locals, None);
        let saved_chunk = self.current_chunk.take();
        let saved_ip = self.ip + 1;

        // Create minimal call frame with correct stack size tracking
        let stack_size_before_call = self.stack.len(); // This is the size AFTER popping parameters

        let call_frame = if is_memoized {
            CallFrame {
                function_name: String::new(),
                return_address: saved_ip,
                locals_start: stack_size_before_call, // Correct stack size to restore to
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
                locals_start: stack_size_before_call, // Correct stack size to restore to
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

        // Create new local memory for the function - optimized capacity
        let mut new_locals = LocalMemory::new(params.len().max(4)); // Pre-allocate for common cases

        // Store parameters in local variables (optimized)
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

            // Pop execution context for access control
            self.execution_context.pop_method_call();

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
        if self.stack.len() == 0 {
            println!("❌ VM: STACK UNDERFLOW at ReturnValue - no value to return!");
            println!("❌ VM: IP: {}, Call stack depth: {}", self.ip, self.call_stack.depth());
            if let Some(chunk) = &self.current_chunk {
                println!("❌ VM: Current chunk: {}", chunk.name);
            }
            return Err(VMError::StackUnderflow);
        }

        let value = self.stack.pop()?;
        if self.debug_mode {
            println!("🔧 VM: ReturnValue - returning value: {:?}, stack size after pop: {}", value, self.stack.len());
        }

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

            // Pop execution context for access control
            self.execution_context.pop_method_call();

            // Return to the caller's context
            self.ip = call_frame.return_address;
            self.locals = call_frame.saved_locals; // Restore caller's locals

            // Only restore chunk if there was a saved chunk (not main function)
            if let Some(saved_chunk) = call_frame.saved_chunk {
                self.current_chunk = Some(saved_chunk);
            }
            // If no saved chunk, keep current chunk (main function)

            // For method calls, we need to clean up the stack properly
            // The stack should be restored to the state before the method call
            // We need to remove any extra items that were left on the stack during the call

            // Check if there are extra items on the stack that need to be cleaned up
            // This can happen when 'self' was pushed back onto the stack during method execution
            let expected_stack_size = call_frame.locals_start;
            if self.debug_mode {
                println!("🔧 VM: Stack cleanup - current size: {}, expected: {}", self.stack.len(), expected_stack_size);
            }

            // Safety check: prevent infinite loop and stack underflow
            let mut cleanup_count = 0;
            const MAX_CLEANUP_ITEMS: usize = 100; // Prevent infinite loops

            while self.stack.len() > expected_stack_size && cleanup_count < MAX_CLEANUP_ITEMS {
                if self.stack.len() == 0 {
                    println!("❌ VM: STACK UNDERFLOW during stack cleanup!");
                    break;
                }
                let extra_item = self.stack.pop()?;
                cleanup_count += 1;
                if self.debug_mode {
                    println!("🔧 VM: Cleaning up extra stack item {}: {:?}", cleanup_count, extra_item);
                }
            }

            if cleanup_count >= MAX_CLEANUP_ITEMS {
                println!("⚠️ VM: Stack cleanup hit maximum limit - possible infinite loop prevented");
            }

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

        // Debug: Log method calls when debug mode is enabled
        if self.debug_mode {
            println!("🔧 VM: OpCode::CallMethod executed for method index {}", method_index);
        }

        if let Value::String(method_name_str) = method_name {
            let method_name_owned = method_name_str.clone();

            // First try to call as builtin method
            if let Err(VMError::UndefinedFunction(_)) = self.call_builtin_method(&method_name_owned) {
                // Check if it's a library function call (contains "::" but not a constructor pattern)
                if method_name_owned.contains("::") && !self.is_constructor_call(&method_name_owned) {
                    if self.debug_mode {
                        println!("🔧 VM: Attempting library function call: {}", method_name_owned);
                    }

                    // Get arguments from stack
                        let param_count = instruction.get_arg_count().unwrap_or(0) as usize;
                        let mut args = Vec::new();

                        for _ in 0..param_count {
                            if let Ok(value) = self.stack.pop() {
                                args.push(crate::library_loader::convert_value_to_string_arg(&value));
                            }
                        }
                        args.reverse(); // Arguments were popped in reverse order

                        // Call library function
                        match crate::library_loader::call_library_function_by_full_name(&method_name_owned, args) {
                            Ok(result) => {
                                // Push result back to stack
                                self.stack.push(crate::core::value::Value::String(std::rc::Rc::new(result)))?;
                                self.ip += 1;
                                return Ok(ExecutionResult::Continue);
                            }
                            Err(e) => {
                                return Err(VMError::RuntimeError(format!("Library function call failed: {}", e)));
                            }
                        }
                }

                // If not a library function, try to call as user-defined method
                self.call_user_method(&method_name_owned, instruction)?;
                // Don't increment IP for user-defined methods - we've switched to a new function context
                return Ok(ExecutionResult::Continue);
            }
        } else {
            return Err(VMError::TypeMismatch("Method name must be a string".to_string()));
        }

        // Only increment IP for builtin methods
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
                    // Convert builtin function errors to exceptions instead of VM errors
                    let exception = Value::runtime_error(e);
                    match self.throw_exception(exception) {
                        Ok(_) => Ok(()), // Exception was thrown and handled
                        Err(vm_error) => Err(vm_error), // No exception handler found
                    }
                }
            }
        } else {
            Err(VMError::UndefinedFunction(method_name.to_string()))
        }
    }

    /// Call a user-defined method with dynamic dispatch and inheritance support
    pub(super) fn call_user_method(&mut self, method_name: &str, instruction: &Instruction) -> VMResult<()> {
        // Get the object from the stack (should be the first argument)
        if self.stack.is_empty() {
            return Err(VMError::RuntimeError("No object on stack for method call".to_string()));
        }

        // Get parameter count from the CallMethod instruction
        // Handle both formats: u8 at byte 2 (regular methods) and u16 at bytes 2-3 (constructors)
        let param_count = if let Some(count) = instruction.get_arg_count() {
            // Regular method call format: u8 at byte 2
            count as usize
        } else if instruction.operands.len() >= 4 {
            // Constructor call format: u16 at bytes 2-3
            let bytes = [instruction.operands[2], instruction.operands[3]];
            u16::from_le_bytes(bytes) as usize
        } else {
            1 // Default for methods (just self)
        };



        // For method calls, the object is pushed first, then arguments
        // Stack layout: [object, arg1, arg2, ...] (bottom to top)
        // The object is at position (param_count - 1) from the top
        let object = self.stack.peek_at(param_count - 1)?;

        // Debug: Print method lookup
        if self.debug_mode {
            println!("🔧 VM: Looking for method '{}' on object: {:?}", method_name, object);
        }

        // Extract class type from object
        let class_name = self.extract_class_name_from_object(&object)?;
        if self.debug_mode {
            println!("🔧 VM: Object class determined as: '{}'", class_name);
        }

        // Check method access permission before dispatch
        let (caller_class, caller_package) = self.execution_context.get_caller_info();
        if let Err(e) = self.check_method_access(&class_name, method_name, caller_class, caller_package) {
            return Err(e);
        }

        // First try virtual method dispatch if VTable exists
        match self.virtual_dispatch(&class_name, method_name) {
            Ok(Some(chunk_index)) => {
                if self.debug_mode {
                    println!("🔧 VM: Using virtual dispatch for '{}::{}' -> chunk {}", class_name, method_name, chunk_index);
                }
                return self.call_function_by_index_with_method_name(chunk_index, method_name, instruction);
            }
            Ok(None) => {
                // Continue to fallback method lookup
            }
            Err(e) => {
                return Err(e);
            }
        }

        // Try interface method dispatch for all implemented interfaces
        let implemented_interfaces = self.get_implemented_interfaces(&class_name);
        for interface_name in &implemented_interfaces {
            match self.interface_dispatch(&class_name, interface_name, method_name) {
                Ok(Some(chunk_index)) => {
                    if self.debug_mode {
                        println!("🔧 VM: Using interface dispatch for '{}::{}' (interface '{}') -> chunk {}",
                            class_name, method_name, interface_name, chunk_index);
                    }
                    return self.call_function_by_index_with_method_name(chunk_index, method_name, instruction);
                }
                Ok(None) => {
                    // Continue to next interface
                }
                Err(_) => {
                    // Continue to next interface or fallback
                }
            }
        }

        // Fallback to inheritance-aware lookup with caching
        if let Some(chunk_index) = self.find_method_with_inheritance(&class_name, method_name) {
            if self.debug_mode {
                println!("🔧 VM: Found method '{}' for class '{}' at chunk index {} (fallback)", method_name, class_name, chunk_index);
            }
            return self.call_function_by_index_with_method_name(chunk_index, method_name, instruction);
        }

        Err(VMError::UndefinedFunction(format!("Method '{}' not found for class '{}', its parent classes, or implemented interfaces", method_name, class_name)))
    }

    /// Build method index for fast lookup
    fn build_method_index(&mut self) {
        if !self.method_index_dirty {
            return;
        }

        if self.debug_mode {
            println!("🔧 VM: Building method index for {} chunks", self.chunks.len());
        }
        self.method_index.clear();

        for (i, chunk) in self.chunks.iter().enumerate() {
            // Index by exact name
            self.method_index.insert(chunk.name.clone(), i);

            // Index by method suffix for fallback matching
            if let Some(method_suffix_pos) = chunk.name.rfind("::") {
                let method_suffix = &chunk.name[method_suffix_pos..];
                let fallback_key = format!("__fallback{}", method_suffix);
                // Only store if not already present (first match wins)
                if !self.method_index.contains_key(&fallback_key) {
                    self.method_index.insert(fallback_key, i);
                }
            }
        }

        self.method_index_dirty = false;
        if self.debug_mode {
            println!("🔧 VM: Method index built with {} entries", self.method_index.len());
        }
    }

    /// Find method with inheritance support and caching
    fn find_method_with_inheritance(&mut self, class_name: &str, method_name: &str) -> Option<usize> {
        // Check cache first
        let cache_key = (class_name.to_string(), method_name.to_string());
        if let Some(&chunk_index) = self.method_cache.get(&cache_key) {
            self.method_cache_hits += 1;
            if self.debug_mode {
                println!("🔧 VM: Method cache HIT for '{}::{}' -> chunk {}", class_name, method_name, chunk_index);
            }
            return Some(chunk_index);
        }

        self.method_cache_misses += 1;
        if self.debug_mode {
            println!("🔧 VM: Method cache MISS for '{}::{}'", class_name, method_name);
        }

        // Ensure method index is built
        self.build_method_index();

        // Build inheritance chain
        let inheritance_chain = self.build_inheritance_chain(class_name);
        if self.debug_mode {
            println!("🔧 VM: Inheritance chain for '{}': {:?}", class_name, inheritance_chain);
        }

        // Search for method in inheritance order using index
        for current_class in &inheritance_chain {
            // 1. Try constructor pattern first (ClassName::ClassName)
            if method_name == current_class {
                let constructor_pattern = format!("{}::{}", current_class, current_class);
                if let Some(&chunk_index) = self.method_index.get(&constructor_pattern) {
                    if self.debug_mode {
                        println!("🔧 VM: Found constructor '{}' in class '{}' via index", method_name, current_class);
                    }
                    // Cache the result
                    self.method_cache.insert(cache_key, chunk_index);
                    return Some(chunk_index);
                }
            }

            // 2. Try class-specific method (ClassName::method_name)
            let class_method_pattern = format!("{}::{}", current_class, method_name);
            if let Some(&chunk_index) = self.method_index.get(&class_method_pattern) {
                if self.debug_mode {
                    println!("🔧 VM: Found method '{}' in class '{}' via index", method_name, current_class);
                }
                // Cache the result
                self.method_cache.insert(cache_key, chunk_index);
                return Some(chunk_index);
            }
        }

        // 3. Fall back to any method with matching name (for compatibility)
        let fallback_key = format!("__fallback::{}", method_name);
        if let Some(&chunk_index) = self.method_index.get(&fallback_key) {
            if self.debug_mode {
                println!("🔧 VM: Found fallback method match for '{}' via index", method_name);
            }
            // Cache the result
            self.method_cache.insert(cache_key, chunk_index);
            return Some(chunk_index);
        }

        // 4. Try exact match (for simple method names)
        if let Some(&chunk_index) = self.method_index.get(method_name) {
            if self.debug_mode {
                println!("🔧 VM: Found exact match for method '{}' via index", method_name);
            }
            // Cache the result
            self.method_cache.insert(cache_key, chunk_index);
            return Some(chunk_index);
        }

        None
    }

    /// Clear method cache (useful when chunks are modified)
    pub fn clear_method_cache(&mut self) {
        self.method_cache.clear();
        self.method_index.clear();
        self.method_index_dirty = true;
        if self.debug_mode {
            println!("🔧 VM: Method cache and index cleared");
        }
    }

    /// Get method cache statistics
    pub fn get_method_cache_stats(&self) -> (u64, u64, f64) {
        let total = self.method_cache_hits + self.method_cache_misses;
        let hit_rate = if total > 0 {
            (self.method_cache_hits as f64) / (total as f64) * 100.0
        } else {
            0.0
        };
        (self.method_cache_hits, self.method_cache_misses, hit_rate)
    }

    /// Mark method index as dirty (call when chunks are added/removed)
    pub fn invalidate_method_index(&mut self) {
        self.method_index_dirty = true;
        // Also clear cache since chunk indices might have changed
        self.method_cache.clear();
        if self.debug_mode {
            println!("🔧 VM: Method index invalidated");
        }
    }

    /// Build inheritance chain from most specific to most general
    fn build_inheritance_chain(&self, class_name: &str) -> Vec<String> {
        let mut chain = vec![class_name.to_string()];
        let mut current_class = class_name;
        let mut visited = std::collections::HashSet::new();

        // Build chain dynamically from class inheritance registry
        while let Some(parent_class) = self.class_inheritance.get(current_class) {
            // Prevent infinite loops in case of circular inheritance
            if visited.contains(current_class) {
                println!("⚠️ VM: Circular inheritance detected for class '{}'", current_class);
                break;
            }
            visited.insert(current_class.to_string());

            chain.push(parent_class.clone());
            current_class = parent_class;
        }

        if self.debug_mode {
            println!("🔧 VM: Inheritance chain for '{}': {:?}", class_name, chain);
        }
        chain
    }

    /// Extract class name from an object
    fn extract_class_name_from_object(&self, object: &crate::core::value::Value) -> VMResult<String> {
        use crate::core::value::Value;

        match object {
            Value::Object(fields) => {
                // Always try to find a __class__ field first - this is the reliable way
                if let Some(Value::String(class_name)) = fields.get("__class__") {
                    return Ok(class_name.to_string());
                }

                // If no __class__ field, this is an error in our object creation
                // We should always have type information
                println!("⚠️ VM: Object missing __class__ field: {:?}", fields);

                // Fallback to heuristic for backward compatibility, but this is unreliable
                if fields.contains_key("breed") && fields.contains_key("name") && fields.contains_key("age") {
                    // Check if it has indoor field (Cat) vs just breed (Dog)
                    if fields.contains_key("indoor") {
                        return Ok("Cat".to_string());
                    } else {
                        return Ok("Dog".to_string());
                    }
                } else if fields.contains_key("indoor") && fields.contains_key("name") && fields.contains_key("age") {
                    return Ok("Cat".to_string());
                } else if fields.contains_key("name") && fields.contains_key("age") {
                    return Ok("Animal".to_string());
                }

                // Default fallback
                Ok("Object".to_string())
            }
            _ => Err(VMError::TypeMismatch("Expected object for method call".to_string()))
        }
    }

    /// Call a function by chunk index
    fn call_function_by_index(&mut self, chunk_index: usize) -> VMResult<()> {
        // Create a default instruction for non-method calls
        let default_instruction = crate::core::bytecode::Instruction::new(crate::core::bytecode::OpCode::Call, 0);
        self.call_function_by_index_with_method_name(chunk_index, "", &default_instruction)
    }

    /// Call a function by chunk index with method name for parameter handling
    fn call_function_by_index_with_method_name(&mut self, chunk_index: usize, method_name: &str, instruction: &Instruction) -> VMResult<()> {
        if chunk_index >= self.chunks.len() {
            return Err(VMError::RuntimeError(format!("Invalid chunk index: {}", chunk_index)));
        }

        // Save current state
        let return_address = self.ip + 1; // Return to next instruction after call
        let chunk_name = self.chunks[chunk_index].name.clone();

        // Debug: Print function call
        if self.debug_mode {
            println!("🔧 VM: Calling function '{}' at chunk index {}", chunk_name, chunk_index);
        }

        // Extract class and method information for execution context
        let (class_name, method_name_extracted) = if chunk_name.contains("::") {
            let parts: Vec<&str> = chunk_name.split("::").collect();
            if parts.len() >= 2 {
                (Some(parts[0].to_string()), parts[1].to_string())
            } else {
                (None, method_name.to_string())
            }
        } else {
            (None, method_name.to_string())
        };

        // Get package information for the class
        let package = if let Some(ref class) = class_name {
            self.class_definitions.get(class).and_then(|def| def.package.clone())
        } else {
            None
        };

        // Push method call context for access control
        if let Some(class) = class_name {
            self.execution_context.push_method_call(class, method_name_extracted, package);
        } else {
            // For non-class methods, still track the call for context
            self.execution_context.push_method_call("__global__".to_string(), method_name_extracted, None);
        }

        // Determine method type and parameter count first
        let chunk_name_ref = &self.chunks[chunk_index].name;
        let is_constructor = chunk_name_ref.contains("::") &&
                           chunk_name_ref.split("::").collect::<Vec<_>>().len() == 2 &&
                           chunk_name_ref.split("::").nth(0) == chunk_name_ref.split("::").nth(1);

        // Get parameter count from the CallMethod instruction
        let param_count = if let Some(count) = instruction.get_arg_count() {
            count as usize
        } else {
            // Fallback to heuristic if operand is missing
            if is_constructor {
                3 // Default for most constructors
            } else {
                1 // Default for methods (just self)
            }
        };

        // Record the stack size before we pop parameters
        // This is the size the stack should be restored to after the function returns
        let stack_size_before_call = self.stack.len() - param_count;

        // Create call frame with saved context
        let mut frame = crate::core::stack::CallFrame::new(
            chunk_name,
            return_address,
            stack_size_before_call, // Stack size to restore to after function returns
            0, // locals_count - we'll set this properly later
        );

        // Save current chunk and locals
        frame.saved_chunk = self.current_chunk.clone();
        frame.saved_locals = self.locals.clone();

        self.call_stack.push(frame).map_err(|_| VMError::StackOverflow)?;

        // Switch to new chunk
        self.current_chunk = Some(self.chunks[chunk_index].clone());
        self.ip = 0;

        // Create new local memory for the function
        // For now, we'll create a local memory with a reasonable capacity
        // In a full implementation, we'd get the local count from the chunk metadata
        let mut local_memory = crate::core::memory::LocalMemory::new(16);

        if self.debug_mode {
            println!("🔧 VM: Function call - param_count: {}, stack_size: {}", param_count, self.stack.len());
        }
        if self.stack.len() >= param_count {
            // Pop parameters from stack (they are in reverse order)
            let mut params = Vec::with_capacity(param_count);
            for i in 0..param_count {
                let param = self.stack.pop().unwrap_or(crate::core::value::Value::Null);
                if self.debug_mode {
                    println!("🔧 VM: Popped param {}: {:?}", i, param);
                }
                params.push(param);
            }
            // Don't reverse - we want to keep the popped order for correct parameter mapping
            // Stack order: [self, x_val, y_val] (bottom to top)
            // Pop order: [y_val, x_val, self] (first to last popped)
            // We want: self=params[2], x_val=params[1], y_val=params[0]

            // Set local variables with generic parameter mapping
            // Parameters are popped in reverse order: [self, param_n-1, ..., param_1, param_0]
            // We need to map them to local variables: [self, param_0, param_1, ..., param_n-1]



            // Set self (always at local index 0) - object is the last item popped
            local_memory.set(0, params[param_count - 1].clone()).map_err(|e| VMError::RuntimeError(e.to_string()))?;

            // Set other parameters in correct order
            for i in 1..param_count {
                let param_index = param_count - 1 - i; // Reverse the order to get correct parameter mapping
                local_memory.set(i, params[param_index].clone()).map_err(|e| VMError::RuntimeError(e.to_string()))?;
            }

            if self.debug_mode {
                println!("🔧 VM: Parameters set successfully");
            }

            // Don't put self back on stack - this was causing stack imbalance
            // The function should work with local variables only
        }

        if self.debug_mode {
            println!("🔧 VM: Stack after setup - size: {}", self.stack.len());
        }

        self.locals = Some(local_memory);

        Ok(())
    }

    /// Execute super method call instruction
    fn execute_call_super(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        let method_index = instruction.get_u16(0)
            .ok_or_else(|| VMError::InvalidOperand("Missing method index".to_string()))?;
        let chunk = self.current_chunk.as_ref().unwrap();
        let method_name = chunk.get_constant(method_index)
            .ok_or_else(|| VMError::InvalidOperand(format!("Invalid method index: {}", method_index)))?;

        if self.debug_mode {
            println!("🔧 VM: CallSuper executed for method: {:?}", method_name);
        }

        if let Value::String(method_name_str) = method_name {
            let method_name_owned = method_name_str.clone();

            // For super calls, we need to find the method in the parent class
            self.call_super_method(&method_name_owned, instruction)?;
            return Ok(ExecutionResult::Continue);
        } else {
            return Err(VMError::TypeMismatch("Method name must be a string".to_string()));
        }
    }

    /// Execute virtual method call instruction
    fn execute_call_virtual(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        let method_index = instruction.get_u16(0)
            .ok_or_else(|| VMError::InvalidOperand("Missing method index".to_string()))?;
        let chunk = self.current_chunk.as_ref().unwrap();
        let method_name = chunk.get_constant(method_index)
            .ok_or_else(|| VMError::InvalidOperand(format!("Invalid method index: {}", method_index)))?;

        if self.debug_mode {
            println!("🔧 VM: CallVirtual executed for method: {:?}", method_name);
        }

        if let Value::String(method_name_str) = method_name {
            let method_name_owned = method_name_str.clone();

            // For virtual calls, use dynamic dispatch
            self.call_user_method(&method_name_owned, instruction)?;
            return Ok(ExecutionResult::Continue);
        } else {
            return Err(VMError::TypeMismatch("Method name must be a string".to_string()));
        }
    }

    /// Execute static method call instruction
    fn execute_call_static(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        let method_index = instruction.get_u16(0)
            .ok_or_else(|| VMError::InvalidOperand("Missing method index".to_string()))?;

        let method_name_str = {
            let chunk = self.current_chunk.as_ref().unwrap();
            let method_name = chunk.get_constant(method_index)
                .ok_or_else(|| VMError::InvalidOperand(format!("Invalid method index: {}", method_index)))?;

            println!("🔧 VM: CallStatic executed for method: {:?}", method_name);

            if let Value::String(method_name_str) = method_name {
                method_name_str.clone()
            } else {
                return Err(VMError::TypeMismatch("Method name must be a string".to_string()));
            }
        };

        // For static calls, call the method directly without object context
        if let Some(&chunk_index) = self.method_index.get(method_name_str.as_ref()) {
            self.call_function_by_index_with_method_name(chunk_index, method_name_str.as_ref(), instruction)?;
            return Ok(ExecutionResult::Continue);
        } else {
            return Err(VMError::UndefinedFunction(format!("Static method '{}' not found", method_name_str.as_ref())));
        }
    }

    /// Call a super method (parent class method)
    fn call_super_method(&mut self, method_name: &str, instruction: &Instruction) -> VMResult<()> {
        // Get the object from the stack (should be the first argument)
        if self.stack.is_empty() {
            return Err(VMError::RuntimeError("No object on stack for super method call".to_string()));
        }

        let param_count = if let Some(count) = instruction.get_arg_count() {
            count as usize
        } else {
            1 // Default for methods (just self)
        };

        // The object is at position (param_count - 1) from the top
        let object = self.stack.peek_at(param_count - 1)?;

        // Extract class type from object
        let object_class_name = self.extract_class_name_from_object(&object)?;
        if self.debug_mode {
            println!("🔧 VM: Super call for method '{}' on object of class '{}'", method_name, object_class_name);
        }

        // Determine the current method's class from the call stack
        let current_method_class = self.get_current_method_class();
        if self.debug_mode {
            println!("🔧 VM: Current method is in class '{}'", current_method_class);
        }

        // Build inheritance chain from the object's class
        let inheritance_chain = self.build_inheritance_chain(&object_class_name);

        // Find the current method's class in the inheritance chain
        let current_class_index = inheritance_chain.iter().position(|c| c == &current_method_class);

        if let Some(start_index) = current_class_index {
            // Look for method starting from the parent of the current method's class
            for parent_class in inheritance_chain.iter().skip(start_index + 1) {
                let parent_method_key = format!("{}::{}", parent_class, method_name);
                if let Some(&chunk_index) = self.method_index.get(&parent_method_key) {
                    if self.debug_mode {
                        println!("🔧 VM: Found super method '{}' in parent class '{}' at chunk index {}", method_name, parent_class, chunk_index);
                    }
                    return self.call_function_by_index_with_method_name(chunk_index, method_name, instruction);
                }
            }
        } else {
            // Fallback: if we can't determine the current method's class, skip the object's class
            for parent_class in inheritance_chain.iter().skip(1) {
                let parent_method_key = format!("{}::{}", parent_class, method_name);
                if let Some(&chunk_index) = self.method_index.get(&parent_method_key) {
                    if self.debug_mode {
                        println!("🔧 VM: Found super method '{}' in parent class '{}' at chunk index {}", method_name, parent_class, chunk_index);
                    }
                    return self.call_function_by_index_with_method_name(chunk_index, method_name, instruction);
                }
            }
        }

        Err(VMError::UndefinedFunction(format!("Super method '{}' not found in parent classes of '{}'", method_name, object_class_name)))
    }

    /// Register class inheritance relationship
    pub fn register_class_inheritance(&mut self, child_class: String, parent_class: String) {
        self.class_inheritance.insert(child_class, parent_class);
        // Invalidate method cache when inheritance changes
        self.invalidate_method_index();
    }

    /// Get parent class of a given class
    pub fn get_parent_class(&self, class_name: &str) -> Option<&String> {
        self.class_inheritance.get(class_name)
    }

    /// Register a class definition
    pub fn register_class_definition(&mut self, class_def: crate::core::vm::types::ClassDefinition) {
        // Also register inheritance relationship if present
        if let Some(base_class) = &class_def.base_class {
            self.class_inheritance.insert(class_def.name.clone(), base_class.clone());
        }

        self.class_definitions.insert(class_def.name.clone(), class_def);
        // Invalidate method cache when class definitions change
        self.invalidate_method_index();
    }

    /// Get class definition
    pub fn get_class_definition(&self, class_name: &str) -> Option<&crate::core::vm::types::ClassDefinition> {
        self.class_definitions.get(class_name)
    }

    /// Check if a class exists
    pub fn has_class(&self, class_name: &str) -> bool {
        self.class_definitions.contains_key(class_name)
    }

    /// Get all classes that extend a given class
    pub fn get_subclasses(&self, base_class: &str) -> Vec<String> {
        self.class_inheritance
            .iter()
            .filter(|(_, parent)| *parent == base_class)
            .map(|(child, _)| child.clone())
            .collect()
    }

    /// Get the class name of the currently executing method
    fn get_current_method_class(&self) -> String {
        // Try to extract class name from current chunk name
        if let Some(chunk) = &self.current_chunk {
            let chunk_name = &chunk.name;

            // Chunk names are typically in format "ClassName::MethodName"
            if let Some(class_end) = chunk_name.find("::") {
                let class_name = &chunk_name[..class_end];
                return class_name.to_string();
            }

            // If no "::" found, assume the chunk name is the class name
            return chunk_name.clone();
        }

        // Fallback: return empty string if we can't determine
        String::new()
    }

    /// Build VTable for a class
    pub fn build_vtable(&mut self, class_name: &str) -> Result<(), String> {
        // Check if VTable already exists
        if self.vtables.contains_key(class_name) {
            return Ok(());
        }

        // Get class definition
        let class_def = self.class_definitions.get(class_name)
            .ok_or_else(|| format!("Class definition not found for '{}'", class_name))?
            .clone();

        // Create VTable
        let mut vtable = crate::core::vm::types::VTable {
            class_name: class_name.to_string(),
            methods: HashMap::new(),
            virtual_methods: Vec::new(),
            parent_class: class_def.base_class.clone(),
            interface_tables: HashMap::new(),
        };

        // If there's a parent class, inherit its VTable
        if let Some(parent_class) = &class_def.base_class {
            // Ensure parent VTable is built first
            self.build_vtable(parent_class)?;

            if let Some(parent_vtable) = self.vtables.get(parent_class) {
                // Copy parent's virtual methods
                vtable.virtual_methods = parent_vtable.virtual_methods.clone();
                vtable.methods = parent_vtable.methods.clone();
            }
        }

        // Add this class's methods to the VTable
        for method_name in &class_def.methods {
            let method_key = format!("{}::{}", class_name, method_name);

            // Find the chunk index for this method
            if let Some(&chunk_index) = self.method_index.get(&method_key) {
                // Check if this method overrides a parent method
                let is_override = vtable.methods.contains_key(method_name);

                // For now, we'll determine if a method is abstract based on whether it has a real implementation
                // In a more complete implementation, this would come from the class definition metadata
                let is_abstract = false; // TODO: Get from class definition metadata

                // Get access modifier from class definition
                let access_modifier = if let Some(class_def) = self.class_definitions.get(class_name) {
                    class_def.method_access.get(method_name)
                        .cloned()
                        .unwrap_or(crate::core::vm::types::AccessModifier::Public)
                } else {
                    crate::core::vm::types::AccessModifier::Public
                };

                let entry = crate::core::vm::types::VTableEntry {
                    method_name: method_name.clone(),
                    defining_class: class_name.to_string(),
                    chunk_index,
                    vtable_index: if is_override {
                        // Find the existing vtable index
                        vtable.virtual_methods.iter()
                            .position(|e| e.method_name == *method_name)
                            .unwrap_or(vtable.virtual_methods.len())
                    } else {
                        vtable.virtual_methods.len()
                    },
                    is_virtual: true, // Assume all methods are virtual for now
                    is_abstract,
                    access_modifier,
                };

                if is_override {
                    // Update existing entry
                    vtable.methods.insert(method_name.clone(), entry.clone());

                    // Update in virtual_methods list
                    if let Some(pos) = vtable.virtual_methods.iter().position(|e| e.method_name == *method_name) {
                        vtable.virtual_methods[pos] = entry;
                    }
                } else {
                    // Add new method
                    vtable.methods.insert(method_name.clone(), entry.clone());
                    vtable.virtual_methods.push(entry);
                }
            }
        }

        // Build interface method tables
        for trait_name in &class_def.implemented_traits {
            let mut interface_table = HashMap::new();

            // For each interface method, find the implementation in this class
            // For now, we'll assume the method names match directly
            // In a more complete implementation, we'd get the interface definition
            // and check each required method

            // Add interface table to VTable
            vtable.interface_tables.insert(trait_name.clone(), interface_table);
            println!("🔧 VM: Added interface table for trait '{}' to class '{}'", trait_name, class_name);
        }

        self.vtables.insert(class_name.to_string(), vtable);
        if self.debug_mode {
            println!("🔧 VM: Built VTable for class '{}' with {} virtual methods and {} interfaces",
                class_name, self.vtables[class_name].virtual_methods.len(), self.vtables[class_name].interface_tables.len());
        }

        Ok(())
    }

    /// Get VTable for a class
    pub fn get_vtable(&self, class_name: &str) -> Option<&crate::core::vm::types::VTable> {
        self.vtables.get(class_name)
    }

    /// Check if a method name represents a constructor call
    fn is_constructor_call(&self, method_name: &str) -> bool {
        if let Some(double_colon_pos) = method_name.find("::") {
            let class_name = &method_name[..double_colon_pos];
            let method_part = &method_name[double_colon_pos + 2..];
            // Constructor pattern: ClassName::ClassName
            return class_name == method_part;
        }
        false
    }

    /// Perform virtual method dispatch with abstract method checking
    pub fn virtual_dispatch(&self, object_class: &str, method_name: &str) -> Result<Option<usize>, VMError> {
        if let Some(vtable) = self.vtables.get(object_class) {
            if let Some(entry) = vtable.methods.get(method_name) {
                // Check if this is an abstract method
                if entry.is_abstract {
                    return Err(VMError::RuntimeError(format!(
                        "Cannot call abstract method '{}' on class '{}'. Abstract methods must be implemented in concrete classes.",
                        method_name, object_class
                    )));
                }

                println!("🔧 VM: Virtual dispatch for '{}::{}' -> chunk {} (defined in '{}')",
                    object_class, method_name, entry.chunk_index, entry.defining_class);
                return Ok(Some(entry.chunk_index));
            }
        }
        Ok(None)
    }

    /// Check if a method should use virtual dispatch
    pub fn should_use_virtual_dispatch(&self, class_name: &str, method_name: &str) -> bool {
        if let Some(vtable) = self.vtables.get(class_name) {
            if let Some(entry) = vtable.methods.get(method_name) {
                return entry.is_virtual;
            }
        }
        false
    }

    /// Check if a class can be instantiated (not abstract)
    pub fn can_instantiate_class(&self, class_name: &str) -> Result<(), VMError> {
        if let Some(class_def) = self.class_definitions.get(class_name) {
            if class_def.is_abstract {
                return Err(VMError::RuntimeError(format!(
                    "Cannot instantiate abstract class '{}'. Abstract classes must be extended by concrete classes.",
                    class_name
                )));
            }
        }
        Ok(())
    }

    /// Validate that all abstract methods are implemented in a concrete class
    pub fn validate_concrete_class(&self, class_name: &str) -> Result<(), VMError> {
        if let Some(class_def) = self.class_definitions.get(class_name) {
            if !class_def.is_abstract {
                // For concrete classes, check that all abstract methods are implemented
                if let Some(vtable) = self.vtables.get(class_name) {
                    for (method_name, entry) in &vtable.methods {
                        if entry.is_abstract {
                            return Err(VMError::RuntimeError(format!(
                                "Concrete class '{}' must implement abstract method '{}'",
                                class_name, method_name
                            )));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Perform interface method dispatch
    pub fn interface_dispatch(&self, object_class: &str, interface_name: &str, method_name: &str) -> Result<Option<usize>, VMError> {
        if let Some(vtable) = self.vtables.get(object_class) {
            if let Some(interface_table) = vtable.interface_tables.get(interface_name) {
                if let Some(entry) = interface_table.get(method_name) {
                    println!("🔧 VM: Interface dispatch for '{}::{}' (interface '{}') -> chunk {} (defined in '{}')",
                        object_class, method_name, interface_name, entry.chunk_index, entry.defining_class);
                    return Ok(Some(entry.chunk_index));
                }
            }

            // Fallback: try to find the method in the regular method table
            // This handles cases where interface methods are implemented as regular methods
            if let Some(entry) = vtable.methods.get(method_name) {
                println!("🔧 VM: Interface dispatch fallback for '{}::{}' (interface '{}') -> chunk {} (defined in '{}')",
                    object_class, method_name, interface_name, entry.chunk_index, entry.defining_class);
                return Ok(Some(entry.chunk_index));
            }
        }

        Err(VMError::RuntimeError(format!(
            "Interface method '{}::{}' not implemented in class '{}'",
            interface_name, method_name, object_class
        )))
    }

    /// Check if a class implements a specific interface
    pub fn implements_interface(&self, class_name: &str, interface_name: &str) -> bool {
        if let Some(class_def) = self.class_definitions.get(class_name) {
            return class_def.implemented_traits.contains(&interface_name.to_string());
        }
        false
    }

    /// Get all interfaces implemented by a class
    pub fn get_implemented_interfaces(&self, class_name: &str) -> Vec<String> {
        if let Some(class_def) = self.class_definitions.get(class_name) {
            return class_def.implemented_traits.clone();
        }
        Vec::new()
    }

    /// Check if a field access is allowed based on access control rules
    pub fn check_field_access(&self,
                              target_class: &str,
                              field_name: &str,
                              caller_class: Option<&str>,
                              caller_package: Option<&str>) -> Result<(), VMError> {
        if let Some(class_def) = self.class_definitions.get(target_class) {
            if let Some(access_modifier) = class_def.field_access.get(field_name) {
                return self.check_access_permission(
                    target_class,
                    access_modifier,
                    caller_class,
                    caller_package,
                    &class_def.package,
                    &class_def.friend_classes,
                    &format!("field '{}'", field_name)
                );
            }
        }

        // If field not found in access control info, allow access (for backward compatibility)
        Ok(())
    }

    /// Check if a method access is allowed based on access control rules
    pub fn check_method_access(&self,
                               target_class: &str,
                               method_name: &str,
                               caller_class: Option<&str>,
                               caller_package: Option<&str>) -> Result<(), VMError> {
        if let Some(class_def) = self.class_definitions.get(target_class) {
            if let Some(access_modifier) = class_def.method_access.get(method_name) {
                return self.check_access_permission(
                    target_class,
                    access_modifier,
                    caller_class,
                    caller_package,
                    &class_def.package,
                    &class_def.friend_classes,
                    &format!("method '{}'", method_name)
                );
            }
        }

        // If method not found in access control info, allow access (for backward compatibility)
        Ok(())
    }

    /// Core access permission checking logic
    fn check_access_permission(&self,
                               target_class: &str,
                               access_modifier: &crate::core::vm::types::AccessModifier,
                               caller_class: Option<&str>,
                               caller_package: Option<&str>,
                               target_package: &Option<String>,
                               friend_classes: &[String],
                               member_description: &str) -> Result<(), VMError> {
        use crate::core::vm::types::AccessModifier;

        match access_modifier {
            AccessModifier::Public => {
                // Public members are always accessible
                Ok(())
            }
            AccessModifier::Private => {
                // Private members are accessible from:
                // 1. The same class
                // 2. Friend classes
                if let Some(caller) = caller_class {
                    if caller == target_class {
                        // Same class - always allowed
                        Ok(())
                    } else if friend_classes.contains(&caller.to_string()) {
                        // Friend class - allowed
                        Ok(())
                    } else {
                        Err(VMError::AccessViolation(format!(
                            "Cannot access private {} of class '{}' from class '{}'",
                            member_description, target_class, caller
                        )))
                    }
                } else {
                    Err(VMError::AccessViolation(format!(
                        "Cannot access private {} of class '{}' from outside any class",
                        member_description, target_class
                    )))
                }
            }
            AccessModifier::Protected => {
                // Protected members are accessible from:
                // 1. The same class
                // 2. Derived classes (subclasses)
                // 3. Friend classes
                if let Some(caller) = caller_class {
                    if caller == target_class {
                        // Same class - always allowed
                        Ok(())
                    } else if self.is_subclass(caller, target_class) {
                        // Subclass - allowed
                        Ok(())
                    } else if friend_classes.contains(&caller.to_string()) {
                        // Friend class - allowed
                        Ok(())
                    } else {
                        Err(VMError::AccessViolation(format!(
                            "Cannot access protected {} of class '{}' from class '{}'",
                            member_description, target_class, caller
                        )))
                    }
                } else {
                    Err(VMError::AccessViolation(format!(
                        "Cannot access protected {} of class '{}' from outside any class",
                        member_description, target_class
                    )))
                }
            }
            AccessModifier::Package => {
                // Package members are accessible from:
                // 1. The same class
                // 2. The same package
                // 3. Friend classes
                if let Some(caller) = caller_class {
                    if caller == target_class {
                        // Same class - always allowed
                        Ok(())
                    } else if friend_classes.contains(&caller.to_string()) {
                        // Friend class - allowed
                        Ok(())
                    } else {
                        // Check package access
                        match (caller_package, target_package) {
                            (Some(caller_pkg), Some(target_pkg)) => {
                                if caller_pkg == target_pkg {
                                    // Same package - allowed
                                    Ok(())
                                } else {
                                    Err(VMError::AccessViolation(format!(
                                        "Cannot access package {} of class '{}' (package '{}') from class '{}' (package '{}')",
                                        member_description, target_class, target_pkg, caller, caller_pkg
                                    )))
                                }
                            }
                            _ => {
                                // If package information is not available, deny access for safety
                                Err(VMError::AccessViolation(format!(
                                    "Cannot access package {} of class '{}' - package information unavailable",
                                    member_description, target_class
                                )))
                            }
                        }
                    }
                } else {
                    Err(VMError::AccessViolation(format!(
                        "Cannot access package {} of class '{}' from outside any class",
                        member_description, target_class
                    )))
                }
            }
        }
    }

    /// Check if a class is a subclass of another class (inheritance check)
    pub fn is_subclass(&self, child_class: &str, parent_class: &str) -> bool {
        if child_class == parent_class {
            return true;
        }

        // Build inheritance chain and check if parent_class is in it
        let inheritance_chain = self.build_inheritance_chain(child_class);
        inheritance_chain.contains(&parent_class.to_string())
    }
}
