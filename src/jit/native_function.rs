//! Native function management for JIT compiled code
//! 
//! This module provides structures and utilities for managing
//! native functions compiled by the JIT compiler.

use std::fmt;

/// Function signature for native compiled functions
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionSignature {
    /// Function takes no parameters and returns an integer
    NoParams,
    /// Function takes a single integer parameter and returns an integer
    SingleInt,
    /// Function takes two integer parameters and returns an integer
    TwoInts,
    /// Function takes multiple integer parameters and returns an integer
    MultipleInts(u8),
    /// Custom signature with specific parameter and return types
    Custom {
        param_types: Vec<NativeType>,
        return_type: NativeType,
    },
}

/// Native data types supported by JIT compilation
#[derive(Debug, Clone, PartialEq)]
pub enum NativeType {
    /// 64-bit signed integer
    I64,
    /// 64-bit floating point
    F64,
    /// Boolean (represented as i8)
    Bool,
    /// Pointer to memory
    Pointer,
    /// Void (no return value)
    Void,
}

/// Metadata about a compiled native function
#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    /// Original function name
    pub name: String,
    /// Number of bytecode instructions in original function
    pub instruction_count: usize,
    /// Compilation timestamp
    pub compiled_at: std::time::SystemTime,
    /// Number of times this function has been called
    pub call_count: u64,
    /// Total execution time in microseconds
    pub total_execution_time_us: u64,
    /// Average execution time per call in nanoseconds
    pub avg_execution_time_ns: u64,
}

impl FunctionMetadata {
    /// Create new function metadata
    pub fn new(name: String, instruction_count: usize) -> Self {
        Self {
            name,
            instruction_count,
            compiled_at: std::time::SystemTime::now(),
            call_count: 0,
            total_execution_time_us: 0,
            avg_execution_time_ns: 0,
        }
    }

    /// Record a function call with execution time
    pub fn record_call(&mut self, execution_time_ns: u64) {
        self.call_count += 1;
        self.total_execution_time_us += execution_time_ns / 1000;
        self.avg_execution_time_ns = (self.total_execution_time_us * 1000) / self.call_count;
    }

    /// Get performance improvement ratio compared to interpretation
    /// (estimated based on instruction count and call frequency)
    pub fn get_performance_ratio(&self) -> f64 {
        if self.call_count == 0 {
            return 1.0;
        }

        // Estimate: JIT is typically 5-50x faster depending on code complexity
        let base_speedup = match self.instruction_count {
            0..=10 => 3.0,      // Simple functions
            11..=50 => 8.0,     // Medium functions
            51..=200 => 15.0,   // Complex functions
            _ => 25.0,          // Very complex functions
        };

        // Adjust based on call frequency (more calls = better amortization)
        let frequency_multiplier = match self.call_count {
            0..=10 => 0.5,
            11..=100 => 1.0,
            101..=1000 => 1.5,
            _ => 2.0,
        };

        base_speedup * frequency_multiplier
    }
}

/// A compiled native function that can be executed directly
#[derive(Clone)]
pub struct NativeFunction {
    /// Function pointer to the compiled code
    function_ptr: *const u8,
    /// Function signature
    signature: FunctionSignature,
    /// Function metadata
    metadata: FunctionMetadata,
}

impl NativeFunction {
    /// Create a new native function
    pub fn new(
        function_ptr: *const u8,
        signature: FunctionSignature,
        metadata: FunctionMetadata,
    ) -> Self {
        Self {
            function_ptr,
            signature,
            metadata,
        }
    }

    /// Get the function signature
    pub fn signature(&self) -> &FunctionSignature {
        &self.signature
    }

    /// Get the function metadata
    pub fn metadata(&self) -> &FunctionMetadata {
        &self.metadata
    }

    /// Get mutable access to metadata (for updating call statistics)
    pub fn metadata_mut(&mut self) -> &mut FunctionMetadata {
        &mut self.metadata
    }

    /// Call the native function with a single integer parameter
    pub fn call(&self, input: i64) -> Result<i64, String> {
        if !matches!(self.signature, FunctionSignature::SingleInt) {
            return Err("Function signature mismatch: expected single integer parameter".to_string());
        }

        unsafe {
            let func: fn(i64) -> i64 = std::mem::transmute(self.function_ptr);
            Ok(func(input))
        }
    }

    /// Call the native function with no parameters
    pub fn call_no_params(&self) -> Result<i64, String> {
        if !matches!(self.signature, FunctionSignature::NoParams) {
            return Err("Function signature mismatch: expected no parameters".to_string());
        }

        unsafe {
            let func: fn() -> i64 = std::mem::transmute(self.function_ptr);
            Ok(func())
        }
    }

    /// Call the native function with two integer parameters
    pub fn call_two_ints(&self, a: i64, b: i64) -> Result<i64, String> {
        if !matches!(self.signature, FunctionSignature::TwoInts) {
            return Err("Function signature mismatch: expected two integer parameters".to_string());
        }

        unsafe {
            let func: fn(i64, i64) -> i64 = std::mem::transmute(self.function_ptr);
            Ok(func(a, b))
        }
    }

    /// Check if the function pointer is valid (non-null)
    pub fn is_valid(&self) -> bool {
        !self.function_ptr.is_null()
    }

    /// Get the raw function pointer
    pub fn as_ptr(&self) -> *const u8 {
        self.function_ptr
    }
}

impl FunctionSignature {
    /// Create a signature for a function with a single integer parameter
    pub fn single_int() -> Self {
        FunctionSignature::SingleInt
    }

    /// Create a signature for a function with no parameters
    pub fn no_params() -> Self {
        FunctionSignature::NoParams
    }

    /// Create a signature for a function with two integer parameters
    pub fn two_ints() -> Self {
        FunctionSignature::TwoInts
    }

    /// Get the number of parameters
    pub fn param_count(&self) -> usize {
        match self {
            FunctionSignature::NoParams => 0,
            FunctionSignature::SingleInt => 1,
            FunctionSignature::TwoInts => 2,
            FunctionSignature::MultipleInts(count) => *count as usize,
            FunctionSignature::Custom { param_types, .. } => param_types.len(),
        }
    }

    /// Check if this signature is compatible with another
    pub fn is_compatible(&self, other: &FunctionSignature) -> bool {
        match (self, other) {
            (FunctionSignature::NoParams, FunctionSignature::NoParams) => true,
            (FunctionSignature::SingleInt, FunctionSignature::SingleInt) => true,
            (FunctionSignature::TwoInts, FunctionSignature::TwoInts) => true,
            (FunctionSignature::MultipleInts(a), FunctionSignature::MultipleInts(b)) => a == b,
            _ => false,
        }
    }
}

impl fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeFunction")
            .field("function_ptr", &format!("{:p}", self.function_ptr))
            .field("signature", &self.signature)
            .field("metadata", &self.metadata)
            .finish()
    }
}

// Safety: NativeFunction can be sent between threads as long as the function
// pointer remains valid. This is generally safe for JIT compiled code.
unsafe impl Send for NativeFunction {}
unsafe impl Sync for NativeFunction {}

/// Native function call result
pub type NativeCallResult = Result<i64, String>;

/// Trait for objects that can be called as native functions
pub trait Callable {
    /// Call the function with the given arguments
    fn call(&self, args: &[i64]) -> NativeCallResult;
    
    /// Get the function signature
    fn signature(&self) -> &FunctionSignature;
    
    /// Check if the function can be called with the given number of arguments
    fn can_call_with(&self, arg_count: usize) -> bool {
        self.signature().param_count() == arg_count
    }
}

impl Callable for NativeFunction {
    fn call(&self, args: &[i64]) -> NativeCallResult {
        match args.len() {
            0 => self.call_no_params(),
            1 => self.call(args[0]),
            2 => self.call_two_ints(args[0], args[1]),
            _ => Err(format!("Unsupported argument count: {}", args.len())),
        }
    }

    fn signature(&self) -> &FunctionSignature {
        &self.signature
    }
}
