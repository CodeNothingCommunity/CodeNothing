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
                // Safe path for local variable access with proper bounds checking
                let index = instruction.get_u8(0)
                    .ok_or_else(|| VMError::InvalidOperand("Missing local index".to_string()))? as usize;

                if self.debug_mode {
                    println!("🔧 VM: LoadLocal - index: {}, operands: {:?}", index, instruction.operands);
                }

                let locals = self.locals.as_ref()
                    .ok_or_else(|| VMError::RuntimeError("No local variables".to_string()))?;

                // Check if the local variable exists
                if index >= locals.count() {
                    if self.debug_mode {
                        println!("🔧 VM: LoadLocal - index {} out of bounds (count: {})", index, locals.count());
                    }
                    return Err(VMError::MemoryError(format!("Local variable index {} out of bounds", index)));
                }

                let value = locals.get(index)?.clone();

                // Check if this is an object and if we have an updated version in the registry
                let final_value = if let Value::Object(obj_map) = &value {
                    if let Some(Value::Int(object_id)) = obj_map.get("__object_id__") {
                        let obj_id = *object_id as usize;
                        if let Some(updated_object) = self.object_registry.get(&obj_id) {
                            if self.jit_engine.debug {
                                println!("🔧 VM: LoadLocal - using updated object from registry for ID {}", obj_id);
                            }
                            updated_object.clone()
                        } else {
                            value.clone()
                        }
                    } else {
                        value.clone()
                    }
                } else {
                    value.clone()
                };

                if self.debug_mode {
                    println!("🔧 VM: LoadLocal - loaded value: {:?}", final_value);
                }
                self.stack.push(final_value)?;
                self.ip += 1;
                Ok(ExecutionResult::Continue)
            }

            crate::core::bytecode::OpCode::StoreLocal => {
                let index = instruction.get_u8(0)
                    .ok_or_else(|| VMError::InvalidOperand("Missing local index".to_string()))? as usize;

                if self.debug_mode {
                    println!("🔧 VM: StoreLocal - index: {}", index);
                }

                let value = self.stack.pop()?;
                let locals = self.locals.as_mut()
                    .ok_or_else(|| VMError::RuntimeError("No local variables".to_string()))?;

                // StoreLocal will automatically grow the local memory if needed
                // No bounds checking required as LocalMemory::set handles growth

                locals.set(index, value)?;
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

            crate::core::bytecode::OpCode::StoreField => {
                self.execute_store_field(instruction)
            }

            crate::core::bytecode::OpCode::SuperFieldLoad => {
                self.execute_super_field_load(instruction)
            }

            crate::core::bytecode::OpCode::SuperFieldStore => {
                self.execute_super_field_store(instruction)
            }

            crate::core::bytecode::OpCode::LoadIndex => {
                self.execute_load_index()
            }

            crate::core::bytecode::OpCode::StoreIndex => {
                self.execute_store_index()
            }

            crate::core::bytecode::OpCode::NewObject => {
                self.execute_new_object(instruction)
            }

            crate::core::bytecode::OpCode::NewArray => {
                self.execute_new_array(instruction)
            }

            crate::core::bytecode::OpCode::ArrayLength => {
                self.execute_array_length()
            }

            _ => Err(VMError::InvalidInstruction(format!("Not a memory instruction: {:?}", instruction.opcode)))
        }
    }

    /// Execute field access operations
    fn execute_field_access(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        let field_index = instruction.get_u16(0)
            .ok_or_else(|| VMError::InvalidOperand("Missing field index".to_string()))?;

        // Object field access: object is on top of stack
        let object = self.stack.pop()?;

        // Debug: Print load field operation
        if self.debug_mode {
            println!("🔧 VM: LoadField - object: {:?}", object);
        }

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
                    // Get the target class from the object
                    let target_class = if let Some(Value::String(class_name)) = obj_map.get("__class__") {
                        class_name.as_str()
                    } else {
                        "Unknown" // Fallback for objects without class information
                    };

                    // Get caller information from execution context
                    let (caller_class, caller_package) = self.execution_context.get_caller_info();

                    // Check field access permission
                    if let Err(e) = self.check_field_access(target_class, field_name_str.as_ref(), caller_class, caller_package) {
                        return Err(e);
                    }

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

    /// Execute store field operation
    fn execute_store_field(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        let field_index = instruction.get_u16(0)
            .ok_or_else(|| VMError::InvalidOperand("Missing field index".to_string()))?;

        if self.debug_mode {
            println!("🔧 VM: StoreField - stack size before pop: {}", self.stack.len());
        }

        if self.stack.len() < 2 {
            return Err(VMError::StackUnderflow);
        }

        let value = self.stack.pop()?;
        let object = self.stack.pop()?;

        // Debug: Print store field operation
        if self.debug_mode {
            println!("🔧 VM: StoreField - object: {:?}, value: {:?}", object, value);
        }

        match object {
            Value::Null => {
                let exception = Value::null_pointer_error("Cannot set field of null object".to_string());
                return self.throw_exception(exception);
            }
            Value::Object(obj_map) => {
                // Get field name from constants
                let chunk = self.current_chunk.as_ref().unwrap();
                let field_name = chunk.get_constant(field_index)
                    .ok_or_else(|| VMError::InvalidOperand(format!("Invalid field index: {}", field_index)))?;

                if let Value::String(field_name_str) = field_name {
                    // Get the target class from the object
                    let target_class = if let Some(Value::String(class_name)) = obj_map.get("__class__") {
                        class_name.as_str()
                    } else {
                        "Unknown" // Fallback for objects without class information
                    };

                    // Get caller information from execution context
                    let (caller_class, caller_package) = self.execution_context.get_caller_info();

                    // Check field access permission
                    if let Err(e) = self.check_field_access(target_class, field_name_str.as_ref(), caller_class, caller_package) {
                        return Err(e);
                    }

                    // Since Rc<HashMap> is immutable, we need to clone and modify
                    let mut new_map = obj_map.as_ref().clone();
                    new_map.insert(field_name_str.as_ref().clone(), value.clone());
                    let new_object = Value::Object(std::rc::Rc::new(new_map));

                    // For field assignment, we need to update the object reference
                    // We need to find and update all references to this object

                    // Check if the object has an object ID for tracking
                    if let Some(Value::Int(object_id)) = obj_map.get("__object_id__") {
                        let obj_id = *object_id as usize;

                        // Update the object in the object tracker if it exists
                        if let Some(_lifecycle_info) = self.object_tracker.get_mut(&obj_id) {
                            // Update the tracked object reference
                            if self.jit_engine.debug {
                                println!("🔧 VM: Updating tracked object {} with new field value", object_id);
                            }
                        }

                        // Update the global object registry with the new object state
                        self.object_registry.insert(obj_id, new_object.clone());
                        if self.jit_engine.debug {
                            println!("🔧 VM: Updated object registry for object {}", object_id);
                        }
                    }

                    // For the current implementation, we need to update the local variable
                    // that contains this object. Since we don't have a direct way to do this,
                    // we'll update local variable 0 (which should be 'self' in method calls)
                    if let Some(ref mut locals) = self.locals {
                        if let Ok(_) = locals.set(0, new_object.clone()) {
                            if self.jit_engine.debug {
                                println!("🔧 VM: Updated local variable 0 (self) with modified object");
                            }
                        }
                    }

                    // Push the modified object back to stack for subsequent operations
                    // This is needed because the code generator expects the modified object
                    // to be available on the stack after StoreField
                    self.stack.push(new_object)?;

                    if self.debug_mode {
                        println!("🔧 VM: StoreField completed successfully");
                    }
                } else {
                    return Err(VMError::TypeMismatch("Field name must be a string".to_string()));
                }
            }
            _ => {
                return Err(VMError::TypeMismatch(format!("Cannot set field of {}", object.type_name())));
            }
        }

        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    /// Execute load index operation (array[index])
    fn execute_load_index(&mut self) -> VMResult<ExecutionResult> {
        let index = self.stack.pop()?;
        let array = self.stack.pop()?;

        match array {
            Value::Null => {
                let exception = Value::null_pointer_error("Cannot index into null array".to_string());
                return self.throw_exception(exception);
            }
            Value::Array(arr) => {
                if let Value::Int(idx) = index {
                    if idx >= 0 && (idx as usize) < arr.len() {
                        self.stack.push(arr[idx as usize].clone())?;
                    } else {
                        let exception = Value::index_error(format!("Array index {} out of bounds (array length: {})", idx, arr.len()));
                        return self.throw_exception(exception);
                    }
                } else {
                    let exception = Value::type_error(format!("Array index must be an integer, got {}", index.type_name()));
                    return self.throw_exception(exception);
                }
            }
            _ => {
                let exception = Value::type_error(format!("Cannot index into {}", array.type_name()));
                return self.throw_exception(exception);
            }
        }

        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    /// Execute store index operation (array[index] = value)
    fn execute_store_index(&mut self) -> VMResult<ExecutionResult> {
        let value = self.stack.pop()?;
        let index = self.stack.pop()?;
        let array = self.stack.pop()?;

        match array {
            Value::Null => {
                let exception = Value::null_pointer_error("Cannot assign to null array".to_string());
                return self.throw_exception(exception);
            }
            Value::Array(arr) => {
                if let Value::Int(idx) = index {
                    if idx >= 0 && (idx as usize) < arr.len() {
                        // Since Rc<Vec> is immutable, we need to clone and modify
                        let mut new_array = arr.as_ref().clone();
                        new_array[idx as usize] = value;
                        let new_array_value = Value::Array(std::rc::Rc::new(new_array));
                        self.stack.push(new_array_value)?;
                    } else {
                        let exception = Value::index_error(format!("Array index {} out of bounds (array length: {})", idx, arr.len()));
                        return self.throw_exception(exception);
                    }
                } else {
                    let exception = Value::type_error(format!("Array index must be an integer, got {}", index.type_name()));
                    return self.throw_exception(exception);
                }
            }
            _ => {
                let exception = Value::type_error(format!("Cannot assign to index of {}", array.type_name()));
                return self.throw_exception(exception);
            }
        }

        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    /// Execute new object operation
    fn execute_new_object(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        let class_index = instruction.get_u16(0)
            .ok_or_else(|| VMError::InvalidOperand("Missing class index".to_string()))?;

        // Get the class name from the constant pool
        let class_name = if let Some(chunk) = &self.current_chunk {
            if let Some(constant) = chunk.constants.get(class_index as usize) {
                if let Value::String(name) = constant {
                    name.as_ref().clone()
                } else {
                    return Err(VMError::TypeMismatch("Class name must be a string".to_string()));
                }
            } else {
                return Err(VMError::InvalidOperand(format!("Invalid class index: {}", class_index)));
            }
        } else {
            return Err(VMError::RuntimeError("No current chunk available".to_string()));
        };

        // Check if the class can be instantiated (not abstract)
        if let Err(e) = self.can_instantiate_class(&class_name) {
            return Err(e);
        }

        // Create object with __class__ field for proper type identification
        let mut fields = HashMap::new();
        fields.insert("__class__".to_string(), Value::String(std::rc::Rc::new(class_name.clone())));

        // Add object ID for lifecycle tracking
        let object_id = self.next_object_id;
        self.next_object_id += 1;
        fields.insert("__object_id__".to_string(), Value::Int(object_id as i64));

        let object = Value::Object(std::rc::Rc::new(fields));

        // Add to object registry for state tracking
        self.object_registry.insert(object_id, object.clone());

        self.stack.push(object)?;

        // Track object lifecycle
        self.track_object_creation(object_id, &class_name);

        if self.debug_mode {
            println!("🔧 VM: Created new object of class '{}' with ID {} and __class__ field", class_name, object_id);
        }

        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    /// Track object creation for lifecycle management
    fn track_object_creation(&mut self, object_id: usize, class_name: &str) {
        use crate::core::vm::types::ObjectLifecycleInfo;

        // Get current stack trace for debugging
        let stack_trace = self.get_current_stack_trace();

        let lifecycle_info = ObjectLifecycleInfo {
            object_id,
            class_name: class_name.to_string(),
            created_at: self.instructions_executed,
            created_gc_cycle: self.gc_cycle_count,
            gc_survival_count: 0,
            creation_stack_trace: stack_trace,
            suspected_leak: false,
        };

        self.object_tracker.insert(object_id, lifecycle_info);

        if self.debug_mode {
            println!("🔧 VM: Tracking object {} of class '{}' created at instruction {}",
                    object_id, class_name, self.instructions_executed);
        }
    }

    /// Get current stack trace for debugging
    fn get_current_stack_trace(&self) -> Vec<String> {
        let mut trace = Vec::new();

        // Add current chunk info
        if let Some(chunk) = &self.current_chunk {
            trace.push(format!("{}::{}", chunk.name, self.ip));
        }

        // Add call stack info
        for frame in self.call_stack.frames() {
            if let Some(chunk) = self.chunks.get(frame.function_index) {
                trace.push(format!("{}::{}", chunk.name, frame.return_address));
            }
        }

        trace
    }

    /// Perform garbage collection cycle and update object tracking
    pub fn garbage_collect(&mut self) -> Result<usize, String> {
        self.gc_cycle_count += 1;
        if self.debug_mode {
            println!("🔧 VM: Starting GC cycle {}", self.gc_cycle_count);
        }

        let mut objects_collected = 0;
        let mut suspected_leaks = Vec::new();

        // Update survival counts and detect potential leaks
        for (object_id, lifecycle_info) in self.object_tracker.iter_mut() {
            lifecycle_info.gc_survival_count += 1;

            // Check for potential memory leaks
            if lifecycle_info.gc_survival_count >= self.leak_detection_threshold {
                if !lifecycle_info.suspected_leak {
                    lifecycle_info.suspected_leak = true;
                    suspected_leaks.push(lifecycle_info.clone());
                }
            }
        }

        // Report suspected leaks
        if !suspected_leaks.is_empty() {
            println!("⚠️ VM: Detected {} potential memory leaks:", suspected_leaks.len());
            for leak in &suspected_leaks {
                println!("   Object {} ({}): alive for {} GC cycles, created at instruction {}",
                        leak.object_id, leak.class_name, leak.gc_survival_count, leak.created_at);
                println!("     Creation stack: {:?}", leak.creation_stack_trace);
            }
        }

        // TODO: Implement actual garbage collection logic
        // For now, just simulate some collection

        if self.debug_mode {
            println!("🔧 VM: GC cycle {} completed, {} objects collected, {} suspected leaks",
                    self.gc_cycle_count, objects_collected, suspected_leaks.len());
        }

        Ok(objects_collected)
    }

    /// Get memory leak report
    pub fn get_memory_leak_report(&self) -> Vec<crate::core::vm::types::ObjectLifecycleInfo> {
        self.object_tracker.values()
            .filter(|info| info.suspected_leak)
            .cloned()
            .collect()
    }

    /// Get object lifecycle statistics
    pub fn get_object_lifecycle_stats(&self) -> (usize, usize, f64) {
        let total_objects = self.object_tracker.len();
        let suspected_leaks = self.object_tracker.values()
            .filter(|info| info.suspected_leak)
            .count();
        let leak_percentage = if total_objects > 0 {
            (suspected_leaks as f64 / total_objects as f64) * 100.0
        } else {
            0.0
        };

        (total_objects, suspected_leaks, leak_percentage)
    }



    /// Simulate garbage collection cycle and detect potential memory leaks
    pub fn run_gc_cycle(&mut self) {
        self.gc_cycle_count += 1;
        if self.debug_mode {
            println!("🔧 VM: Running GC cycle #{}", self.gc_cycle_count);
        }

        let mut leaked_objects = Vec::new();
        let mut total_objects = 0;
        let mut suspected_leaks = 0;

        // Update survival counts and detect potential leaks
        for (object_id, lifecycle_info) in self.object_tracker.iter_mut() {
            total_objects += 1;
            lifecycle_info.gc_survival_count += 1;

            // Mark as suspected leak if it has survived too many GC cycles
            if lifecycle_info.gc_survival_count >= self.leak_detection_threshold {
                if !lifecycle_info.suspected_leak {
                    lifecycle_info.suspected_leak = true;
                    leaked_objects.push(lifecycle_info.clone());
                }
                suspected_leaks += 1;
            }
        }

        // Report findings
        if self.debug_mode {
            println!("🔧 VM: GC cycle #{} complete - {} objects tracked, {} suspected leaks",
                    self.gc_cycle_count, total_objects, suspected_leaks);
        }

        if !leaked_objects.is_empty() {
            println!("⚠️ VM: Potential memory leaks detected:");
            for leak in &leaked_objects {
                println!("   - Object {} ({}): alive for {} GC cycles, created at instruction {}",
                        leak.object_id, leak.class_name, leak.gc_survival_count, leak.created_at);
                if !leak.creation_stack_trace.is_empty() {
                    println!("     Stack trace: {:?}", leak.creation_stack_trace);
                }
            }
        }
    }

    /// Get memory leak statistics
    pub fn get_memory_leak_stats(&self) -> (usize, usize, Vec<String>) {
        let total_objects = self.object_tracker.len();
        let suspected_leaks = self.object_tracker.values()
            .filter(|info| info.suspected_leak)
            .count();

        let leak_details = self.object_tracker.values()
            .filter(|info| info.suspected_leak)
            .map(|info| format!("Object {} ({}): {} GC cycles",
                               info.object_id, info.class_name, info.gc_survival_count))
            .collect();

        (total_objects, suspected_leaks, leak_details)
    }

    /// Clear object tracking (useful for testing)
    pub fn clear_object_tracking(&mut self) {
        self.object_tracker.clear();
        self.next_object_id = 1;
        self.gc_cycle_count = 0;
        if self.debug_mode {
            println!("🔧 VM: Object tracking cleared");
        }
    }

    /// Execute new array operation
    fn execute_new_array(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        let size = instruction.get_u16(0)
            .ok_or_else(|| VMError::InvalidOperand("Missing array size".to_string()))?;

        // Create array filled with null values
        let array = vec![Value::Null; size as usize];
        let array_value = Value::Array(std::rc::Rc::new(array));
        self.stack.push(array_value)?;

        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    /// Execute array length operation
    fn execute_array_length(&mut self) -> VMResult<ExecutionResult> {
        let array = self.stack.pop()?;

        match array {
            Value::Null => {
                let exception = Value::null_pointer_error("Cannot get length of null array".to_string());
                return self.throw_exception(exception);
            }
            Value::Array(arr) => {
                self.stack.push(Value::Int(arr.len() as i64))?;
            }
            Value::String(s) => {
                // Also support string length
                self.stack.push(Value::Int(s.len() as i64))?;
            }
            _ => {
                let exception = Value::type_error(format!("Cannot get length of {}", array.type_name()));
                return self.throw_exception(exception);
            }
        }

        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    /// Execute super field load operation
    fn execute_super_field_load(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        let field_index = instruction.get_u16(0)
            .ok_or_else(|| VMError::InvalidOperand("Missing field index".to_string()))?;

        // Object field access: object is on top of stack
        let object = self.stack.pop()?;

        // Debug: Print super field load operation
        if self.debug_mode {
            println!("🔧 VM: SuperFieldLoad - object: {:?}", object);
        }

        match object {
            Value::Null => {
                let exception = Value::null_pointer_error("Cannot access super field of null object".to_string());
                return self.throw_exception(exception);
            }
            Value::Object(obj_map) => {
                // Get field name from constants
                let chunk = self.current_chunk.as_ref().unwrap();
                let field_name = chunk.get_constant(field_index)
                    .ok_or_else(|| VMError::InvalidOperand(format!("Invalid field index: {}", field_index)))?;

                if let Value::String(field_name_str) = field_name {
                    // For super field access, we need to look in the parent class
                    // For now, we'll use the same logic as regular field access
                    // TODO: Implement proper super field resolution
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
                return Err(VMError::TypeMismatch(format!("Cannot access super field of {}", object.type_name())));
            }
        }

        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }

    /// Execute super field store operation
    fn execute_super_field_store(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        let field_index = instruction.get_u16(0)
            .ok_or_else(|| VMError::InvalidOperand("Missing field index".to_string()))?;

        if self.debug_mode {
            println!("🔧 VM: SuperFieldStore - stack size before pop: {}", self.stack.len());
        }

        if self.stack.len() < 2 {
            return Err(VMError::StackUnderflow);
        }

        let value = self.stack.pop()?;
        let object = self.stack.pop()?;

        // Debug: Print super field store operation
        if self.debug_mode {
            println!("🔧 VM: SuperFieldStore - object: {:?}, value: {:?}", object, value);
        }

        match object {
            Value::Null => {
                let exception = Value::null_pointer_error("Cannot set super field of null object".to_string());
                return self.throw_exception(exception);
            }
            Value::Object(obj_map) => {
                // Get field name from constants
                let chunk = self.current_chunk.as_ref().unwrap();
                let field_name = chunk.get_constant(field_index)
                    .ok_or_else(|| VMError::InvalidOperand(format!("Invalid field index: {}", field_index)))?;

                if let Value::String(field_name_str) = field_name {
                    // Create a new object with the updated field
                    let mut new_fields = obj_map.as_ref().clone();
                    new_fields.insert(field_name_str.as_ref().clone(), value);
                    let new_object = Value::Object(std::rc::Rc::new(new_fields));

                    // Push the modified object back to stack
                    self.stack.push(new_object)?;

                    if self.debug_mode {
                        println!("🔧 VM: SuperFieldStore completed successfully");
                    }
                } else {
                    return Err(VMError::TypeMismatch("Field name must be a string".to_string()));
                }
            }
            _ => {
                return Err(VMError::TypeMismatch(format!("Cannot set super field of {}", object.type_name())));
            }
        }

        self.ip += 1;
        Ok(ExecutionResult::Continue)
    }
}
