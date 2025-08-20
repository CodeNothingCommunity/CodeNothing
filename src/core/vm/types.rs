//! VM types and data structures

use crate::core::{
    bytecode::Chunk,
    memory::{MemoryManager, LocalMemory, MemoryConfig},
    stack::{Stack, CallStack},
    value::Value,
};
use crate::jit::JitEngine;
use std::collections::{HashMap, HashSet};
use std::fmt;

use super::errors::{NativeFunction, VMResult};

/// Exception handler information
#[derive(Debug, Clone)]
pub struct ExceptionHandler {
    /// Address of the catch block
    pub catch_address: usize,
    /// Address of the finally block (if any)
    pub finally_address: Option<usize>,
    /// Stack size when the try block started
    pub stack_size: usize,
    /// Whether the finally block has been executed
    pub finally_executed: bool,
}

/// Virtual Machine state
#[derive(Debug)]
pub struct VirtualMachine {
    /// Operand stack
    pub(super) stack: Stack,

    /// Call stack for function calls
    pub(super) call_stack: CallStack,

    /// Memory manager
    pub(super) memory: MemoryManager,

    /// Local variables for current function
    pub(super) locals: Option<LocalMemory>,

    /// Instruction pointer
    pub(super) ip: usize,

    /// Current chunk being executed
    pub(super) current_chunk: Option<Chunk>,

    /// All chunks (functions)
    pub(super) chunks: Vec<Chunk>,

    /// Native functions registry
    pub(super) native_functions: HashMap<u32, NativeFunction>,

    /// Exception handling stack
    pub(super) exception_handlers: Vec<ExceptionHandler>,

    /// Current exception being handled
    pub(super) current_exception: Option<Value>,

    /// Memoization cache: function_index -> (args_hash -> result)
    pub(super) memo_cache: HashMap<usize, HashMap<u64, Value>>,

    /// Set of memoized function indices
    pub(super) memo_functions: HashSet<usize>,

    /// JIT compilation engine
    pub(super) jit_engine: JitEngine,

    /// Execution statistics
    pub(super) instructions_executed: u64,

    /// Debug mode flag
    pub(super) debug_mode: bool,
}

/// Execution result for instruction processing
#[derive(Debug, Clone)]
pub(super) enum ExecutionResult {
    Continue,
    Return(Value),
    Halt,
}

/// VM execution statistics
#[derive(Debug, Clone)]
pub struct VMStats {
    pub instructions_executed: u64,
    pub stack_size: usize,
    pub call_depth: usize,
    pub memory_stats: crate::core::memory::MemoryStats,
    pub memory_config: MemoryConfig,
}

/// Detailed memory usage information
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub stack_usage: StackUsage,
    pub heap_usage: HeapUsage,
    pub call_stack_usage: CallStackUsage,
    pub globals_usage: GlobalsUsage,
    pub estimated_total_memory: usize,
    pub memory_config_name: String,
}

#[derive(Debug, Clone)]
pub struct StackUsage {
    pub current: usize,
    pub max: usize,
    pub percentage: f64,
}

#[derive(Debug, Clone)]
pub struct HeapUsage {
    pub current: usize,
    pub max: usize,
    pub percentage: f64,
    pub total_allocations: usize,
    pub total_deallocations: usize,
}

#[derive(Debug, Clone)]
pub struct CallStackUsage {
    pub current: usize,
    pub max: usize,
    pub percentage: f64,
}

#[derive(Debug, Clone)]
pub struct GlobalsUsage {
    pub current: usize,
    pub max: usize,
    pub percentage: f64,
}

impl fmt::Display for MemoryInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Memory Usage Information:")?;
        writeln!(f, "  Configuration: {}", self.memory_config_name)?;
        writeln!(f, "  Estimated Total: {} bytes", self.estimated_total_memory)?;
        writeln!(f)?;

        writeln!(f, "Stack Usage:")?;
        writeln!(f, "  Current: {} / {} values ({:.1}%)",
                 self.stack_usage.current,
                 self.stack_usage.max,
                 self.stack_usage.percentage)?;

        writeln!(f, "Heap Usage:")?;
        writeln!(f, "  Current: {} / {} objects ({:.1}%)",
                 self.heap_usage.current,
                 self.heap_usage.max,
                 self.heap_usage.percentage)?;
        writeln!(f, "  Total Allocations: {}", self.heap_usage.total_allocations)?;
        writeln!(f, "  Total Deallocations: {}", self.heap_usage.total_deallocations)?;

        writeln!(f, "Call Stack Usage:")?;
        writeln!(f, "  Current: {} / {} frames ({:.1}%)",
                 self.call_stack_usage.current,
                 self.call_stack_usage.max,
                 self.call_stack_usage.percentage)?;

        writeln!(f, "Global Variables Usage:")?;
        writeln!(f, "  Current: {} / {} variables ({:.1}%)",
                 self.globals_usage.current,
                 self.globals_usage.max,
                 self.globals_usage.percentage)?;

        Ok(())
    }
}

impl fmt::Display for VMStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "VM Statistics:")?;
        writeln!(f, "  Instructions executed: {}", self.instructions_executed)?;
        writeln!(f, "  Stack size: {}", self.stack_size)?;
        writeln!(f, "  Call depth: {}", self.call_depth)?;
        write!(f, "{}", self.memory_stats)?;
        Ok(())
    }
}
