// CodeNothing Virtual Machine ONE.

//! Virtual Machine implementation for CodeNothing
//!
//! This module contains the core virtual machine that executes bytecode instructions
//! using a stack-based architecture.

// Module declarations
pub mod errors;
pub mod types;
pub mod vm_impl;
pub mod execution;
pub mod memory_ops;
pub mod control_flow;
pub mod function_calls;
pub mod jit_integration;
pub mod stats;

// Re-export main types and functions
pub use errors::{VMError, VMResult, NativeFunction};
pub use types::{
    VirtualMachine, ExceptionHandler, VMStats, MemoryInfo,
    StackUsage, HeapUsage, CallStackUsage, GlobalsUsage
};

// Re-export the main VirtualMachine struct with all its implementations
pub use types::VirtualMachine as VM;
