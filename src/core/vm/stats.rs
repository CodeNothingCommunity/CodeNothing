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

        // Reset performance tracking
        self.method_cache.clear();
        self.method_cache_hits = 0;
        self.method_cache_misses = 0;
        self.method_index.clear();
        self.method_index_dirty = true;
        self.object_tracker.clear();
        self.next_object_id = 1;
        self.gc_cycle_count = 0;
        self.object_registry.clear();
    }

    /// Get memory usage information (public version)
    pub fn get_memory_info(&self) -> MemoryInfo {
        self.memory_info()
    }

    /// Print comprehensive performance and memory report
    pub fn print_performance_report(&self) {
        println!("\n🔧 ===== CodeNothing VM Performance Report =====");

        // Method cache statistics
        let (cache_hits, cache_misses, hit_rate) = self.get_method_cache_stats();
        println!("📊 Method Lookup Performance:");
        println!("   Cache hits: {}", cache_hits);
        println!("   Cache misses: {}", cache_misses);
        println!("   Hit rate: {:.2}%", hit_rate);
        println!("   Method index entries: {}", self.method_index.len());

        // Object lifecycle statistics
        let (total_objects, suspected_leaks, leak_percentage) = self.get_object_lifecycle_stats();
        println!("📊 Object Lifecycle:");
        println!("   Total objects created: {}", total_objects);
        println!("   Suspected leaks: {}", suspected_leaks);
        println!("   Leak percentage: {:.2}%", leak_percentage);
        println!("   GC cycles: {}", self.gc_cycle_count);

        // Memory statistics
        let memory_stats = self.get_memory_info();
        println!("📊 Memory Usage:");
        println!("   Stack: {}/{} ({:.1}%)",
                memory_stats.stack_usage.current,
                memory_stats.stack_usage.max,
                memory_stats.stack_usage.percentage);
        println!("   Heap: {}/{} ({:.1}%)",
                memory_stats.heap_usage.current,
                memory_stats.heap_usage.max,
                memory_stats.heap_usage.percentage);
        println!("   Globals: {}/{} ({:.1}%)",
                memory_stats.globals_usage.current,
                memory_stats.globals_usage.max,
                memory_stats.globals_usage.percentage);

        // Performance recommendations
        self.print_performance_recommendations(hit_rate, leak_percentage);

        println!("🔧 ============================================\n");
    }

    /// Print performance recommendations based on statistics
    fn print_performance_recommendations(&self, cache_hit_rate: f64, leak_percentage: f64) {
        println!("💡 Performance Recommendations:");

        if cache_hit_rate < 80.0 {
            println!("   ⚠️ Low method cache hit rate ({:.1}%) - consider optimizing method lookup patterns", cache_hit_rate);
        } else {
            println!("   ✅ Good method cache hit rate ({:.1}%)", cache_hit_rate);
        }

        if leak_percentage > 10.0 {
            println!("   ⚠️ High memory leak percentage ({:.1}%) - review object lifecycle management", leak_percentage);
        } else if leak_percentage > 5.0 {
            println!("   ⚠️ Moderate memory leak percentage ({:.1}%) - monitor object creation patterns", leak_percentage);
        } else {
            println!("   ✅ Low memory leak percentage ({:.1}%)", leak_percentage);
        }

        let memory_info = self.get_memory_info();
        if memory_info.stack_usage.percentage > 80.0 {
            println!("   ⚠️ High stack usage ({:.1}%) - consider reducing recursion depth", memory_info.stack_usage.percentage);
        }

        if memory_info.heap_usage.percentage > 80.0 {
            println!("   ⚠️ High heap usage ({:.1}%) - consider running garbage collection", memory_info.heap_usage.percentage);
        }
    }
}
