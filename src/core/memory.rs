//! Memory management for the CodeNothing VM
//!
//! This module provides memory management capabilities including:
//! - Global variable storage
//! - Object heap management
//! - Garbage collection (future)
//! - Memory statistics and monitoring
/// VM Memory configuration similar to JVM memory settings
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Initial heap size (like JVM -Xms)
    pub initial_heap_size: usize,

    /// Maximum heap size (like JVM -Xmx)
    pub max_heap_size: usize,

    /// Stack size per thread (like JVM -Xss)
    pub stack_size: usize,

    /// Maximum call stack depth
    pub max_call_depth: usize,

    /// Maximum number of global variables
    pub max_globals: usize,

    /// Local variables per function call
    pub locals_per_function: usize,

    /// Enable memory usage monitoring
    pub enable_monitoring: bool,
}

impl MemoryConfig {
    /// Create default memory configuration (highly optimized for performance)
    pub fn default() -> Self {
        Self {
            initial_heap_size: 4096,        // 4K objects initially (aggressive pre-allocation)
            max_heap_size: 262144,          // 256K objects max (~2GB)
            stack_size: 65536,              // 64K values (~512MB) - very large for deep recursion
            max_call_depth: 100000,         // 100K call frames - extremely high for fibonacci(30)
            max_globals: 16384,             // 16K global variables
            locals_per_function: 1024,      // 1024 locals per function
            enable_monitoring: false,       // Disable monitoring for maximum performance
        }
    }

    /// Create small memory configuration (embedded/testing)
    pub fn small() -> Self {
        Self {
            initial_heap_size: 256,         // 256 objects initially
            max_heap_size: 4096,            // 4K objects max (~32MB)
            stack_size: 1024,               // 1K values (~8MB)
            max_call_depth: 256,            // 256 call frames
            max_globals: 512,               // 512 global variables
            locals_per_function: 64,        // 64 locals per function
            enable_monitoring: true,
        }
    }

    /// Create large memory configuration (server/production)
    pub fn large() -> Self {
        Self {
            initial_heap_size: 8192,        // 8K objects initially
            max_heap_size: 1048576,         // 1M objects max (~8GB)
            stack_size: 65536,              // 64K values (~512MB) - increased for deep recursion
            max_call_depth: 100000,         // 100K call frames - much larger for complex recursion
            max_globals: 16384,             // 16K global variables
            locals_per_function: 512,       // 512 locals per function
            enable_monitoring: true,
        }
    }

    /// Create custom memory configuration
    pub fn custom(
        initial_heap_mb: usize,
        max_heap_mb: usize,
        stack_mb: usize,
    ) -> Self {
        // Rough conversion: assume 8 bytes per Value on average
        let bytes_per_value = 8;
        let mb_to_bytes = 1024 * 1024;

        Self {
            initial_heap_size: (initial_heap_mb * mb_to_bytes) / bytes_per_value,
            max_heap_size: (max_heap_mb * mb_to_bytes) / bytes_per_value,
            stack_size: (stack_mb * mb_to_bytes) / bytes_per_value,
            max_call_depth: 1024,
            max_globals: 4096,
            locals_per_function: 256,
            enable_monitoring: true,
        }
    }

    /// Create unlimited memory configuration (for deep recursion and complex programs)
    /// ⚠️ WARNING: This removes most safety limits - use with caution!
    pub fn unlimited() -> Self {
        Self {
            initial_heap_size: 65536,          // 64K objects initially (512MB)
            max_heap_size: usize::MAX / 1024,  // Practically unlimited heap
            stack_size: usize::MAX / 1024,     // Practically unlimited stack
            max_call_depth: usize::MAX / 1024, // Practically unlimited call depth
            max_globals: usize::MAX / 1024,    // Practically unlimited globals
            locals_per_function: 65536,        // 64K locals per function
            enable_monitoring: false,          // Disable monitoring for performance
        }
    }

    /// Create deep recursion configuration (optimized for recursive algorithms)
    pub fn deep_recursion() -> Self {
        Self {
            initial_heap_size: 8192,           // 8K objects initially
            max_heap_size: 1048576,            // 1M objects max (~8GB)
            stack_size: 1048576,               // 1M values (~8GB stack)
            max_call_depth: 1000000,           // 1M call frames - for deep recursion
            max_globals: 16384,                // 16K global variables
            locals_per_function: 1024,         // 1K locals per function
            enable_monitoring: true,
        }
    }

    /// Estimate total memory usage in bytes
    pub fn estimate_memory_usage(&self) -> usize {
        let value_size = std::mem::size_of::<crate::core::value::Value>();
        let frame_size = std::mem::size_of::<crate::core::stack::CallFrame>();

        let heap_memory = self.max_heap_size * value_size;
        let stack_memory = self.stack_size * value_size;
        let call_stack_memory = self.max_call_depth * frame_size;
        let globals_memory = self.max_globals * value_size;

        heap_memory + stack_memory + call_stack_memory + globals_memory
    }

    /// Get memory usage in human-readable format
    pub fn memory_usage_string(&self) -> String {
        let bytes = self.estimate_memory_usage();
        if bytes >= 1024 * 1024 * 1024 {
            format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        } else if bytes >= 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else if bytes >= 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{} bytes", bytes)
        }
    }
}

use crate::core::value::Value;
use std::collections::HashMap;
use std::fmt;

/// Memory allocation error
#[derive(Debug, Clone)]
pub struct MemoryError {
    pub message: String,
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Memory error: {}", self.message)
    }
}

impl std::error::Error for MemoryError {}

impl MemoryError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Global variable storage
///
/// Stores global variables that can be accessed from any scope
#[derive(Debug, Clone)]
pub struct GlobalMemory {
    variables: Vec<Value>,
    name_to_index: HashMap<String, usize>,
    max_variables: usize,
}

impl GlobalMemory {
    /// Create a new global memory instance with default limits
    pub fn new() -> Self {
        Self::with_config(&MemoryConfig::default())
    }

    /// Create a new global memory instance with specified configuration
    pub fn with_config(config: &MemoryConfig) -> Self {
        Self {
            variables: Vec::with_capacity(config.max_globals.min(1024)),
            name_to_index: HashMap::with_capacity(config.max_globals.min(1024)),
            max_variables: config.max_globals,
        }
    }

    /// Define a new global variable
    pub fn define(&mut self, name: String, value: Value) -> Result<usize, MemoryError> {
        if let Some(&index) = self.name_to_index.get(&name) {
            // Variable already exists, update it
            self.variables[index] = value;
            Ok(index)
        } else {
            // Check memory limit
            if self.variables.len() >= self.max_variables {
                return Err(MemoryError::new(format!(
                    "Global variable limit exceeded: {} >= {}",
                    self.variables.len(),
                    self.max_variables
                )));
            }

            // New variable
            let index = self.variables.len();
            self.variables.push(value);
            self.name_to_index.insert(name, index);
            Ok(index)
        }
    }

    /// Get a global variable by index
    pub fn get(&self, index: usize) -> Result<&Value, MemoryError> {
        self.variables.get(index)
            .ok_or_else(|| MemoryError::new(format!("Global variable index {} out of bounds", index)))
    }

    /// Set a global variable by index
    pub fn set(&mut self, index: usize, value: Value) -> Result<(), MemoryError> {
        if index < self.variables.len() {
            self.variables[index] = value;
            Ok(())
        } else {
            Err(MemoryError::new(format!("Global variable index {} out of bounds", index)))
        }
    }

    /// Get a global variable by name
    pub fn get_by_name(&self, name: &str) -> Result<&Value, MemoryError> {
        let index = self.name_to_index.get(name)
            .ok_or_else(|| MemoryError::new(format!("Global variable '{}' not found", name)))?;
        self.get(*index)
    }

    /// Set a global variable by name
    pub fn set_by_name(&mut self, name: &str, value: Value) -> Result<(), MemoryError> {
        let index = self.name_to_index.get(name)
            .ok_or_else(|| MemoryError::new(format!("Global variable '{}' not found", name)))?;
        self.set(*index, value)
    }

    /// Get the index of a global variable by name
    pub fn get_index(&self, name: &str) -> Option<usize> {
        self.name_to_index.get(name).copied()
    }

    /// Get the number of global variables
    pub fn count(&self) -> usize {
        self.variables.len()
    }

    /// Clear all global variables
    pub fn clear(&mut self) {
        self.variables.clear();
        self.name_to_index.clear();
    }

    /// Get all variable names
    pub fn variable_names(&self) -> Vec<&String> {
        self.name_to_index.keys().collect()
    }

    /// Get the number of global variables
    pub fn len(&self) -> usize {
        self.variables.len()
    }

    /// Reserve space for global variables and initialize them with null
    pub fn reserve(&mut self, count: usize) -> Result<(), MemoryError> {
        if self.variables.len() + count > self.max_variables {
            return Err(MemoryError::new(format!(
                "Global variable limit exceeded: {} + {} > {}",
                self.variables.len(),
                count,
                self.max_variables
            )));
        }

        // Add null values for the reserved space
        for _ in 0..count {
            self.variables.push(Value::Null);
        }
        Ok(())
    }
}

impl Default for GlobalMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// Local variable storage for function scopes
///
/// Each function call creates a new local scope
#[derive(Debug, Clone)]
pub struct LocalMemory {
    variables: Vec<Value>,
}

impl LocalMemory {
    /// Create a new local memory with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            variables: Vec::with_capacity(capacity), // Don't pre-allocate, grow as needed
        }
    }

    /// Get a local variable by index
    pub fn get(&self, index: usize) -> Result<&Value, MemoryError> {
        self.variables.get(index)
            .ok_or_else(|| MemoryError::new(format!("Local variable index {} out of bounds", index)))
    }

    /// Set a local variable by index
    pub fn set(&mut self, index: usize, value: Value) -> Result<(), MemoryError> {
        // Grow the vector if needed
        if index >= self.variables.len() {
            self.variables.resize(index + 1, Value::Null);
        }
        self.variables[index] = value;
        Ok(())
    }

    /// Get the number of local variables
    pub fn count(&self) -> usize {
        self.variables.len()
    }

    /// Get all local variables (for debugging)
    pub fn variables(&self) -> &[Value] {
        &self.variables
    }
}

/// Heap memory manager for dynamic allocations
///
/// Manages dynamic memory allocation for objects, arrays, and strings
#[derive(Debug, Clone)]
pub struct HeapMemory {
    objects: Vec<Value>,
    free_list: Vec<usize>, // Indices of freed objects for reuse
    allocation_count: usize,
    deallocation_count: usize,
    max_objects: usize,
    initial_capacity: usize,
}

impl HeapMemory {
    /// Create a new heap memory manager with default configuration
    pub fn new() -> Self {
        Self::with_config(&MemoryConfig::default())
    }

    /// Create a new heap memory manager with specified configuration
    pub fn with_config(config: &MemoryConfig) -> Self {
        Self {
            objects: Vec::with_capacity(config.initial_heap_size),
            free_list: Vec::new(),
            allocation_count: 0,
            deallocation_count: 0,
            max_objects: config.max_heap_size,
            initial_capacity: config.initial_heap_size,
        }
    }

    /// Allocate space for a new object
    pub fn allocate(&mut self, value: Value) -> Result<usize, MemoryError> {
        self.allocation_count += 1;

        if let Some(index) = self.free_list.pop() {
            // Reuse a freed slot
            self.objects[index] = value;
            Ok(index)
        } else {
            // Check memory limit
            if self.objects.len() >= self.max_objects {
                return Err(MemoryError::new(format!(
                    "Heap memory limit exceeded: {} >= {}",
                    self.objects.len(),
                    self.max_objects
                )));
            }

            // Allocate new slot
            let index = self.objects.len();
            self.objects.push(value);
            Ok(index)
        }
    }

    /// Deallocate an object (mark as free for reuse)
    pub fn deallocate(&mut self, index: usize) -> Result<(), MemoryError> {
        if index >= self.objects.len() {
            return Err(MemoryError::new(format!("Invalid heap index {}", index)));
        }

        self.deallocation_count += 1;
        self.objects[index] = Value::Null; // Clear the value
        self.free_list.push(index);
        Ok(())
    }

    /// Get an object from the heap
    pub fn get(&self, index: usize) -> Result<&Value, MemoryError> {
        self.objects.get(index)
            .ok_or_else(|| MemoryError::new(format!("Heap index {} out of bounds", index)))
    }

    /// Set an object in the heap
    pub fn set(&mut self, index: usize, value: Value) -> Result<(), MemoryError> {
        if index < self.objects.len() {
            self.objects[index] = value;
            Ok(())
        } else {
            Err(MemoryError::new(format!("Heap index {} out of bounds", index)))
        }
    }

    /// Get the number of allocated objects
    pub fn allocated_count(&self) -> usize {
        self.objects.len() - self.free_list.len()
    }

    /// Get the total number of allocations performed
    pub fn total_allocations(&self) -> usize {
        self.allocation_count
    }

    /// Get the total number of deallocations performed
    pub fn total_deallocations(&self) -> usize {
        self.deallocation_count
    }

    /// Get memory statistics
    pub fn stats(&self) -> HeapStats {
        HeapStats {
            total_objects: self.objects.len(),
            allocated_objects: self.allocated_count(),
            free_objects: self.free_list.len(),
            total_allocations: self.allocation_count,
            total_deallocations: self.deallocation_count,
        }
    }

    /// Clear all heap memory
    pub fn clear(&mut self) {
        self.objects.clear();
        self.free_list.clear();
        self.allocation_count = 0;
        self.deallocation_count = 0;
    }
}

impl Default for HeapMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// Heap memory statistics
#[derive(Debug, Clone)]
pub struct HeapStats {
    pub total_objects: usize,
    pub allocated_objects: usize,
    pub free_objects: usize,
    pub total_allocations: usize,
    pub total_deallocations: usize,
}

impl fmt::Display for HeapStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Heap Statistics:")?;
        writeln!(f, "  Total objects: {}", self.total_objects)?;
        writeln!(f, "  Allocated objects: {}", self.allocated_objects)?;
        writeln!(f, "  Free objects: {}", self.free_objects)?;
        writeln!(f, "  Total allocations: {}", self.total_allocations)?;
        writeln!(f, "  Total deallocations: {}", self.total_deallocations)?;
        writeln!(f, "  Memory efficiency: {:.1}%",
            if self.total_objects > 0 {
                (self.allocated_objects as f64 / self.total_objects as f64) * 100.0
            } else {
                100.0
            }
        )?;
        Ok(())
    }
}

/// Complete memory manager combining all memory types
#[derive(Debug, Clone)]
pub struct MemoryManager {
    pub globals: GlobalMemory,
    pub heap: HeapMemory,
    pub config: MemoryConfig,
}

impl MemoryManager {
    /// Create a new memory manager with default configuration
    pub fn new() -> Self {
        Self::with_config(MemoryConfig::default())
    }

    /// Create a new memory manager with specified configuration
    pub fn with_config(config: MemoryConfig) -> Self {
        Self {
            globals: GlobalMemory::with_config(&config),
            heap: HeapMemory::with_config(&config),
            config,
        }
    }

    /// Get comprehensive memory statistics
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            global_variables: self.globals.count(),
            heap_stats: self.heap.stats(),
        }
    }

    /// Clear all memory
    pub fn clear(&mut self) {
        self.globals.clear();
        self.heap.clear();
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub global_variables: usize,
    pub heap_stats: HeapStats,
}

impl fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Memory Statistics:")?;
        writeln!(f, "  Global variables: {}", self.global_variables)?;
        write!(f, "{}", self.heap_stats)?;
        Ok(())
    }
}