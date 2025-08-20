//! Core VirtualMachine implementation

use crate::core::{
    bytecode::Chunk,
    memory::{MemoryManager, LocalMemory, MemoryConfig},
    stack::{Stack, CallStack},
    value::Value,
};
use crate::jit::JitEngine;
use std::collections::{HashMap, HashSet};

use super::{
    errors::{VMError, VMResult, NativeFunction},
    types::{VirtualMachine, ExceptionHandler},
};

impl VirtualMachine {
    /// Create a new virtual machine with default memory configuration
    pub fn new() -> Self {
        Self::with_memory_config(MemoryConfig::default())
    }

    /// Create a new virtual machine with specified memory configuration
    pub fn with_memory_config(config: MemoryConfig) -> Self {
        Self {
            stack: Stack::new(config.stack_size),
            call_stack: CallStack::new(config.max_call_depth),
            memory: MemoryManager::with_config(config.clone()),
            locals: None,
            ip: 0,
            current_chunk: None,
            chunks: Vec::new(),
            native_functions: HashMap::new(),
            exception_handlers: Vec::new(),
            current_exception: None,
            memo_cache: HashMap::new(),
            memo_functions: HashSet::new(),
            jit_engine: JitEngine::new(),
            instructions_executed: 0,
            debug_mode: false,
        }
    }

    /// Create a small VM for testing/embedded use
    pub fn small() -> Self {
        Self::with_memory_config(MemoryConfig::small())
    }

    /// Create a large VM for production use
    pub fn large() -> Self {
        Self::with_memory_config(MemoryConfig::large())
    }

    /// Create a custom VM with specified memory limits (in MB)
    pub fn custom(initial_heap_mb: usize, max_heap_mb: usize, stack_mb: usize) -> Self {
        Self::with_memory_config(MemoryConfig::custom(initial_heap_mb, max_heap_mb, stack_mb))
    }

    /// Enable or disable debug mode
    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_mode = enabled;
    }

    /// Add a chunk (function) to the VM
    pub fn add_chunk(&mut self, chunk: Chunk) -> usize {
        self.chunks.push(chunk);
        self.chunks.len() - 1
    }

    /// Execute a chunk
    pub fn execute(&mut self, chunk_index: usize) -> VMResult<Value> {
        // Debug output when JIT debug is enabled
        if self.jit_engine.debug {
            println!("🔧 VM: execute() method called for chunk {}", chunk_index);
            println!("🔧 VM: chunks.len() = {}", self.chunks.len());

            // Show all chunks
            for (i, chunk) in self.chunks.iter().enumerate() {
                println!("🔧 VM: Chunk {}: name='{}', instructions={}", i, chunk.name, chunk.instructions.len());
            }
        }

        if self.jit_engine.debug {
            println!("🔧 VM: execute() called for chunk {} (DEBUG MODE)", chunk_index);
        }

        if chunk_index >= self.chunks.len() {
            println!("🔧 VM: ERROR - Invalid chunk index: {} >= {}", chunk_index, self.chunks.len());
            return Err(VMError::RuntimeError(format!("Invalid chunk index: {}", chunk_index)));
        }

        if self.jit_engine.debug {
            println!("🔧 VM: Chunk index {} is valid, continuing execution", chunk_index);
        }

        let start_time = std::time::Instant::now();
        if self.jit_engine.debug {
            println!("🔧 VM: About to record execution for chunk {}", chunk_index);
        }

        // Record execution for hot spot detection first
        self.record_jit_execution(chunk_index, 0);
        if self.jit_engine.debug {
            println!("🔧 VM: Recorded execution for chunk {}", chunk_index);
            println!("🔧 VM: About to check if should compile chunk {}", chunk_index);
        }

        // Check if we should try JIT compilation (with improved logic)
        self.check_jit_compilation(chunk_index);

        // Try to execute with JIT if available
        if let Some(result) = self.try_jit_execute(chunk_index) {
            let execution_time = start_time.elapsed().as_micros() as u64;
            self.record_jit_execution(chunk_index, execution_time);
            return result;
        }

        // Fall back to interpretation
        let chunk = &self.chunks[chunk_index];
        self.current_chunk = Some(chunk.clone());
        self.ip = 0;
        self.locals = Some(LocalMemory::new(self.memory.config.locals_per_function));

        if self.jit_engine.debug {
            println!("🔧 VM: About to call run() method");
        }
        let result = self.run();
        if self.jit_engine.debug {
            println!("🔧 VM: run() method completed");
        }

        // Record execution for hot spot detection
        let execution_time = start_time.elapsed().as_micros() as u64;
        self.record_jit_execution(chunk_index, execution_time);

        result
    }
}

impl Default for VirtualMachine {
    fn default() -> Self {
        Self::new()
    }
}
