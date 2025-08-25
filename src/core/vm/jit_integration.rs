//! JIT compilation integration for the VM

use crate::core::value::Value;
use crate::jit::JitEngine;
use super::{
    errors::{VMError, VMResult},
    types::VirtualMachine,
};

impl VirtualMachine {
    /// Enable or disable JIT compilation
    pub fn set_jit_enabled(&mut self, enabled: bool) {
        self.jit_engine.set_enabled(enabled);
    }

    /// Check if JIT compilation is enabled
    pub fn is_jit_enabled(&self) -> bool {
        self.jit_engine.is_enabled()
    }

    /// Set JIT compilation threshold
    pub fn set_jit_threshold(&mut self, threshold: u32) {
        let debug_state = self.jit_engine.debug; // 保存debug状态
        self.jit_engine = JitEngine::with_threshold(threshold);
        self.jit_engine.set_debug(debug_state); // 恢复debug状态
    }

    /// Enable aggressive JIT mode for performance-critical code
    pub fn enable_aggressive_jit(&mut self) {
        let debug_state = self.jit_engine.debug; // 保存debug状态
        self.jit_engine = JitEngine::with_threshold(10); // Very low threshold
        self.jit_engine.set_debug(debug_state); // 恢复debug状态
    }

    /// Enable or disable JIT debug output
    pub fn set_jit_debug(&mut self, debug: bool) {
        self.jit_engine.set_debug(debug);
        if debug {
            println!("🔧 JIT Debug mode enabled");
        }
    }

    /// Get JIT compilation statistics
    pub fn get_jit_stats(&self) -> &crate::jit::JitStats {
        self.jit_engine.get_stats()
    }

    /// Get JIT hot spots information
    pub fn get_jit_hotspots(&self) -> Vec<(usize, u32, u64)> {
        self.jit_engine.get_hotspots()
    }

    /// Reset JIT engine (clear compiled functions and statistics)
    pub fn reset_jit(&mut self) {
        self.jit_engine.reset();
    }

    /// Try to execute a function using JIT if available
    pub(super) fn try_jit_execute(&mut self, chunk_index: usize) -> Option<VMResult<Value>> {
        // For now, JIT functions take a single integer parameter and return an integer
        // This is a simplified implementation - in practice, you'd need more sophisticated
        // parameter passing and return value handling

        if self.debug_mode {
            println!("🔧 VM: try_jit_execute called for chunk {}", chunk_index);
        }

        if let Ok(top_value) = self.stack.peek() {
            if let Value::Int(input) = top_value {
                if let Some(result) = self.jit_engine.execute_native(chunk_index, *input) {
                    match result {
                        Ok(output) => {
                            self.stack.pop().ok(); // Remove input parameter
                            self.stack.push(Value::Int(output)).ok();
                            return Some(Ok(Value::Int(output)));
                        }
                        Err(error) => {
                            return Some(Err(VMError::RuntimeError(format!("JIT execution error: {}", error))));
                        }
                    }
                }
            }
        }

        if self.debug_mode {
            println!("🔧 VM: try_jit_execute returning None (no JIT function available)");
        }
        None
    }

    /// Check and potentially compile a function with JIT
    pub(super) fn check_jit_compilation(&mut self, chunk_index: usize) {
        // Check if we should try JIT compilation (with improved logic)
        if self.jit_engine.should_compile(chunk_index) {
            let chunk = &self.chunks[chunk_index];
            if self.jit_engine.debug {
                println!("🔥 JIT: Attempting to compile function: {}", chunk.name);
            }
            match self.jit_engine.compile_chunk(chunk_index, chunk) {
                crate::jit::JitResult::Compiled(_) => {
                    if self.jit_engine.debug {
                        println!("✅ JIT: Successfully compiled function: {}", chunk.name);
                    }
                }
                crate::jit::JitResult::Failed(error) => {
                    if self.jit_engine.debug {
                        println!("❌ JIT: Compilation failed for {}: {}", chunk.name, error);
                    }
                }
                crate::jit::JitResult::NotSuitable(reason) => {
                    if self.jit_engine.debug {
                        println!("⚠️  JIT: Function {} not suitable: {}", chunk.name, reason);
                    }
                }
            }
        }
    }

    /// Record execution for JIT hot spot detection
    pub(super) fn record_jit_execution(&mut self, chunk_index: usize, execution_time: u64) {
        self.jit_engine.record_execution(chunk_index, execution_time);
    }
}
