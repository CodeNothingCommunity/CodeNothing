//! Stack management for the CodeNothing VM
//!
//! This module provides the stack data structure used by the virtual machine
//! for operand storage and function call management.

use crate::core::value::Value;
use std::fmt;

/// Stack overflow error
#[derive(Debug, Clone)]
pub struct StackOverflowError;

impl fmt::Display for StackOverflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Stack overflow")
    }
}

impl std::error::Error for StackOverflowError {}

/// Stack underflow error
#[derive(Debug, Clone)]
pub struct StackUnderflowError;

impl fmt::Display for StackUnderflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Stack underflow")
    }
}

impl std::error::Error for StackUnderflowError {}

/// VM Stack for operand storage
///
/// The stack is used for:
/// - Storing operands for arithmetic and logical operations
/// - Passing arguments to functions
/// - Storing local variables
/// - Managing function call frames
#[derive(Debug, Clone)]
pub struct Stack {
    values: Vec<Value>,
    max_size: usize,
}

impl Stack {
    /// Create a new stack with the specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            values: Vec::with_capacity(max_size.min(8192)), // Pre-allocate more for better performance
            max_size,
        }
    }

    /// Create a new stack with default size (64KB worth of values)
    pub fn with_default_size() -> Self {
        Self::new(8192) // 8K values should be plenty for most programs
    }

    /// Push a value onto the stack
    #[inline(always)]
    pub fn push(&mut self, value: Value) -> Result<(), StackOverflowError> {
        if self.values.len() >= self.max_size {
            return Err(StackOverflowError);
        }
        self.values.push(value);
        Ok(())
    }

    /// Pop a value from the stack
    #[inline(always)]
    pub fn pop(&mut self) -> Result<Value, StackUnderflowError> {
        if self.values.is_empty() {
            eprintln!("❌ STACK: Attempting to pop from empty stack! Stack size: {}", self.values.len());
            return Err(StackUnderflowError);
        }
        self.values.pop().ok_or(StackUnderflowError)
    }

    /// Peek at the top value without removing it
    pub fn peek(&self) -> Result<&Value, StackUnderflowError> {
        self.values.last().ok_or(StackUnderflowError)
    }

    /// Peek at the top value mutably without removing it
    pub fn peek_mut(&mut self) -> Result<&mut Value, StackUnderflowError> {
        self.values.last_mut().ok_or(StackUnderflowError)
    }

    /// Peek at a value at the specified distance from the top
    /// Distance 0 is the top, distance 1 is one below the top, etc.
    pub fn peek_at(&self, distance: usize) -> Result<&Value, StackUnderflowError> {
        if distance >= self.values.len() {
            return Err(StackUnderflowError);
        }
        let index = self.values.len() - 1 - distance;
        Ok(&self.values[index])
    }

    /// Duplicate the top value
    pub fn dup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let value = self.peek()?.clone();
        self.push(value)?;
        Ok(())
    }

    /// Swap the top two values
    pub fn swap(&mut self) -> Result<(), StackUnderflowError> {
        if self.values.len() < 2 {
            return Err(StackUnderflowError);
        }
        let len = self.values.len();
        self.values.swap(len - 1, len - 2);
        Ok(())
    }

    /// Get the current stack size
    pub fn size(&self) -> usize {
        self.values.len()
    }

    /// Get the current length of the stack (alias for size)
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if the stack is empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Get the maximum stack size
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Clear the stack
    pub fn clear(&mut self) {
        self.values.clear();
    }

    /// Get a slice of the stack values (for debugging)
    pub fn values(&self) -> &[Value] {
        &self.values
    }

    /// Reserve space for additional values
    pub fn reserve(&mut self, additional: usize) {
        self.values.reserve(additional);
    }
}

impl fmt::Display for Stack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Stack (size: {}/{})", self.size(), self.max_size())?;
        if self.is_empty() {
            writeln!(f, "  [empty]")?;
        } else {
            for (i, value) in self.values.iter().enumerate().rev() {
                let marker = if i == self.values.len() - 1 { " <- TOP" } else { "" };
                writeln!(f, "  [{}]: {}{}", i, value, marker)?;
            }
        }
        Ok(())
    }
}

/// Call frame for function calls
///
/// Each function call creates a new call frame that tracks:
/// - The return address (instruction pointer)
/// - Local variables
/// - Function metadata
#[derive(Debug, Clone)]
pub struct CallFrame {
    pub function_name: String,
    pub return_address: usize,
    pub locals_start: usize, // Index in stack where locals begin
    pub locals_count: u8,
    pub saved_locals: Option<crate::core::memory::LocalMemory>, // Save caller's local variables
    pub saved_chunk: Option<crate::core::bytecode::Chunk>, // Save caller's chunk
    pub function_index: usize, // Index of the called function for memoization
    pub function_args: Vec<crate::core::value::Value>, // Arguments for memoization
}

impl CallFrame {
    /// Create a new call frame
    pub fn new(function_name: String, return_address: usize, locals_start: usize, locals_count: u8) -> Self {
        Self {
            function_name,
            return_address,
            locals_start,
            locals_count,
            saved_locals: None,
            saved_chunk: None,
            function_index: 0,
            function_args: Vec::new(),
        }
    }

    /// Create a new call frame with saved context
    pub fn with_context(
        function_name: String,
        return_address: usize,
        locals_start: usize,
        locals_count: u8,
        saved_locals: Option<crate::core::memory::LocalMemory>,
        saved_chunk: Option<crate::core::bytecode::Chunk>
    ) -> Self {
        Self {
            function_name,
            return_address,
            locals_start,
            locals_count,
            saved_locals,
            saved_chunk,
            function_index: 0,
            function_args: Vec::new(),
        }
    }

    /// Create a new call frame with memoization info
    pub fn with_memo_context(
        function_name: String,
        return_address: usize,
        locals_start: usize,
        locals_count: u8,
        saved_locals: Option<crate::core::memory::LocalMemory>,
        saved_chunk: Option<crate::core::bytecode::Chunk>,
        function_index: usize,
        function_args: Vec<crate::core::value::Value>
    ) -> Self {
        Self {
            function_name,
            return_address,
            locals_start,
            locals_count,
            saved_locals,
            saved_chunk,
            function_index,
            function_args,
        }
    }
}

/// Call stack for managing function calls
#[derive(Debug, Clone)]
pub struct CallStack {
    frames: Vec<CallFrame>,
    max_depth: usize,
}

impl CallStack {
    /// Create a new call stack with the specified maximum depth
    pub fn new(max_depth: usize) -> Self {
        Self {
            frames: Vec::with_capacity(max_depth.min(256)),
            max_depth,
        }
    }

    /// Create a new call stack with default depth
    pub fn with_default_depth() -> Self {
        Self::new(1024) // Should be enough for most recursive calls
    }

    /// Push a new call frame
    pub fn push(&mut self, frame: CallFrame) -> Result<(), StackOverflowError> {
        if self.frames.len() >= self.max_depth {
            return Err(StackOverflowError);
        }
        self.frames.push(frame);
        Ok(())
    }

    /// Pop the top call frame
    pub fn pop(&mut self) -> Result<CallFrame, StackUnderflowError> {
        self.frames.pop().ok_or(StackUnderflowError)
    }

    /// Peek at the top call frame
    pub fn peek(&self) -> Result<&CallFrame, StackUnderflowError> {
        self.frames.last().ok_or(StackUnderflowError)
    }

    /// Get the current call depth
    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    /// Check if the call stack is empty
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Clear the call stack
    pub fn clear(&mut self) {
        self.frames.clear();
    }

    /// Get all frames (for debugging)
    pub fn frames(&self) -> &[CallFrame] {
        &self.frames
    }
}

impl fmt::Display for CallStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Call Stack (depth: {}/{})", self.depth(), self.max_depth)?;
        if self.is_empty() {
            writeln!(f, "  [empty]")?;
        } else {
            for (i, frame) in self.frames.iter().enumerate().rev() {
                let marker = if i == self.frames.len() - 1 { " <- CURRENT" } else { "" };
                writeln!(f, "  [{}]: {} (return: {}){}", i, frame.function_name, frame.return_address, marker)?;
            }
        }
        Ok(())
    }
}