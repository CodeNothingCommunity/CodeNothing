//! VM statistics and monitoring

use super::types::{VirtualMachine, VMStats, MemoryInfo, StackUsage, HeapUsage, CallStackUsage, GlobalsUsage};

impl VirtualMachine {
    /// Get VM statistics
    pub fn stats(&self) -> VMStats {
        VMStats {
            instructions_executed: self.instructions_executed,
            stack_size: self.stack.size(),
            call_depth: self.call_stack.depth(),
            memory_stats: self.memory.stats(),
            memory_config: self.memory.config.clone(),
        }
    }

    /// Get memory usage information
    pub fn memory_info(&self) -> MemoryInfo {
        let config = &self.memory.config;
        let heap_stats = self.memory.heap.stats();

        MemoryInfo {
            stack_usage: StackUsage {
                current: self.stack.size(),
                max: self.stack.max_size(),
                percentage: (self.stack.size() as f64 / self.stack.max_size() as f64) * 100.0,
            },
            heap_usage: HeapUsage {
                current: heap_stats.allocated_objects,
                max: config.max_heap_size,
                percentage: (heap_stats.allocated_objects as f64 / config.max_heap_size as f64) * 100.0,
                total_allocations: heap_stats.total_allocations,
                total_deallocations: heap_stats.total_deallocations,
            },
            call_stack_usage: CallStackUsage {
                current: self.call_stack.depth(),
                max: config.max_call_depth,
                percentage: (self.call_stack.depth() as f64 / config.max_call_depth as f64) * 100.0,
            },
            globals_usage: GlobalsUsage {
                current: self.memory.globals.count(),
                max: config.max_globals,
                percentage: (self.memory.globals.count() as f64 / config.max_globals as f64) * 100.0,
            },
            estimated_total_memory: config.estimate_memory_usage(),
            memory_config_name: config.memory_usage_string(),
        }
    }

    /// Reset the VM to initial state
    pub fn reset(&mut self) {
        self.stack.clear();
        self.call_stack.clear();
        self.memory.clear();
        self.locals = None;
        self.ip = 0;
        self.current_chunk = None;
        self.instructions_executed = 0;
    }
}
