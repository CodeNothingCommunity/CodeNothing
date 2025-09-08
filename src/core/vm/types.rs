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

    /// Method lookup cache: (class_name, method_name) -> chunk_index
    pub(super) method_cache: HashMap<(String, String), usize>,

    /// Method lookup statistics
    pub(super) method_cache_hits: u64,
    pub(super) method_cache_misses: u64,

    /// Pre-built method index: method_pattern -> chunk_index
    pub(super) method_index: HashMap<String, usize>,

    /// Flag to track if method index needs rebuilding
    pub(super) method_index_dirty: bool,

    /// Object lifecycle tracking: object_id -> creation_info
    pub(super) object_tracker: HashMap<usize, ObjectLifecycleInfo>,

    /// Next object ID for tracking
    pub(super) next_object_id: usize,

    /// Memory leak detection threshold (objects alive for this many GC cycles)
    pub(super) leak_detection_threshold: u32,

    /// GC cycle counter
    pub(super) gc_cycle_count: u32,

    /// Object registry: object_id -> current_object_state
    /// This is used to maintain consistent object state across method calls
    pub(super) object_registry: HashMap<usize, Value>,

    /// Class inheritance registry: class_name -> parent_class_name
    pub(super) class_inheritance: HashMap<String, String>,

    /// Class definitions registry: class_name -> class_definition
    pub(super) class_definitions: HashMap<String, ClassDefinition>,

    /// Virtual method tables for runtime polymorphism
    pub(super) vtables: HashMap<String, VTable>,

    /// Current execution context for access control
    pub(super) execution_context: ExecutionContext,

    /// Function index mapping: original_index -> vm_index
    /// Used to map compilation-time function indices to runtime VM indices
    pub(super) function_index_mapping: HashMap<usize, usize>,
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



/// Object lifecycle information for memory leak detection
#[derive(Debug, Clone)]
pub struct ObjectLifecycleInfo {
    /// Unique object ID
    pub object_id: usize,

    /// Class name of the object
    pub class_name: String,

    /// Creation timestamp (in VM instructions executed)
    pub created_at: u64,

    /// GC cycle when object was created
    pub created_gc_cycle: u32,

    /// Number of times this object has survived GC
    pub gc_survival_count: u32,

    /// Stack trace when object was created (for debugging)
    pub creation_stack_trace: Vec<String>,

    /// Whether this object is suspected of being leaked
    pub suspected_leak: bool,
}

/// Runtime class definition for VM
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClassDefinition {
    /// Class name
    pub name: String,

    /// Parent class name (if any)
    pub base_class: Option<String>,

    /// Field names and their types (simplified)
    pub fields: Vec<String>,

    /// Method names available in this class
    pub methods: Vec<String>,

    /// Whether this class is abstract
    pub is_abstract: bool,

    /// Implemented traits/interfaces
    pub implemented_traits: Vec<String>,

    /// Field access modifiers: field_name -> access_modifier
    pub field_access: HashMap<String, AccessModifier>,

    /// Method access modifiers: method_name -> access_modifier
    pub method_access: HashMap<String, AccessModifier>,

    /// Abstract methods: method_name -> is_abstract
    pub abstract_methods: HashMap<String, bool>,

    /// Package/namespace this class belongs to
    pub package: Option<String>,

    /// Friend classes that have special access to private/protected members
    pub friend_classes: Vec<String>,
}

/// Access modifier for runtime access control
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AccessModifier {
    Public,
    Private,
    Protected,
    Package,
}

/// Execution context for access control tracking
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Current class being executed (if any)
    pub current_class: Option<String>,

    /// Current package/namespace
    pub current_package: Option<String>,

    /// Stack of method calls for context tracking
    pub call_context_stack: Vec<MethodCallContext>,
}

/// Method call context information
#[derive(Debug, Clone)]
pub struct MethodCallContext {
    /// Class that owns the method being called
    pub class_name: String,

    /// Method name being called
    pub method_name: String,

    /// Package of the class
    pub package: Option<String>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            current_class: None,
            current_package: None,
            call_context_stack: Vec::new(),
        }
    }

    /// Push a new method call context
    pub fn push_method_call(&mut self, class_name: String, method_name: String, package: Option<String>) {
        self.call_context_stack.push(MethodCallContext {
            class_name: class_name.clone(),
            method_name,
            package: package.clone(),
        });

        // Update current context
        self.current_class = Some(class_name);
        self.current_package = package;
    }

    /// Pop the current method call context
    pub fn pop_method_call(&mut self) {
        self.call_context_stack.pop();

        // Update current context to the previous one
        if let Some(context) = self.call_context_stack.last() {
            self.current_class = Some(context.class_name.clone());
            self.current_package = context.package.clone();
        } else {
            self.current_class = None;
            self.current_package = None;
        }
    }

    /// Get the current caller information for access control
    pub fn get_caller_info(&self) -> (Option<&str>, Option<&str>) {
        if let Some(context) = self.call_context_stack.last() {
            (Some(&context.class_name), context.package.as_deref())
        } else {
            (self.current_class.as_deref(), self.current_package.as_deref())
        }
    }
}

/// Virtual method table entry for runtime dispatch
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VTableEntry {
    /// Method name
    pub method_name: String,

    /// Class that defines this method implementation
    pub defining_class: String,

    /// Chunk index for the method implementation
    pub chunk_index: usize,

    /// Index in the VTable for efficient lookup
    pub vtable_index: usize,

    /// Whether this method is virtual
    pub is_virtual: bool,

    /// Whether this method is abstract
    pub is_abstract: bool,

    /// Access modifier for this method
    pub access_modifier: AccessModifier,
}

/// Virtual method table for runtime polymorphism
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VTable {
    /// Class name this VTable belongs to
    pub class_name: String,

    /// Virtual method entries indexed by method name
    pub methods: HashMap<String, VTableEntry>,

    /// Ordered list of virtual methods for index-based lookup
    pub virtual_methods: Vec<VTableEntry>,

    /// Parent class name (for inheritance chain)
    pub parent_class: Option<String>,

    /// Interface method tables for each implemented interface
    pub interface_tables: HashMap<String, HashMap<String, VTableEntry>>,
}
