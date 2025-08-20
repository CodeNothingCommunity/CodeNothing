//! REPL (Read-Eval-Print Loop) for CodeNothing VM
//!
//! This module provides an interactive shell for debugging and testing the VM.
//! Users can directly input bytecode instructions, inspect VM state, and execute programs.

use crate::core::{
    bytecode::{Chunk, Instruction, OpCode},
    memory::MemoryConfig,
    value::Value,
    vm::{VirtualMachine, VMError},
};
use crate::lang::{
    lexer::Lexer,
    parser::Parser,
    semantic::SemanticAnalyzer,
    codegen::CodeGenerator,
};
use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::Rc;
use std::str::FromStr;

/// REPL state and context
pub struct Repl {
    vm: VirtualMachine,
    current_chunk: Chunk,
    instruction_history: Vec<String>,
    variables: HashMap<String, Value>,
    line_number: u32,
}

impl Repl {
    /// Create a new REPL instance
    pub fn new() -> Self {
        Self {
            vm: VirtualMachine::new(),
            current_chunk: Chunk::new("repl".to_string()),
            instruction_history: Vec::new(),
            variables: HashMap::new(),
            line_number: 1,
        }
    }

    /// Start the REPL loop
    pub fn run(&mut self) {
        println!("CodeNothing VM REPL v1.0.0");
        println!("Type 'help' for available commands, 'quit' to exit");
        println!("======================================================");

        loop {
            print!("vm> ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => {
                    let input = input.trim();
                    if input.is_empty() {
                        continue;
                    }

                    self.instruction_history.push(input.to_string());

                    match self.process_command(input) {
                        Ok(should_continue) => {
                            if !should_continue {
                                break;
                            }
                        }
                        Err(e) => {
                            println!("Error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("Error reading input: {}", e);
                    break;
                }
            }
        }

        println!("Goodbye!");
    }

    /// Process a single command
    fn process_command(&mut self, input: &str) -> Result<bool, String> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(true);
        }

        match parts[0].to_lowercase().as_str() {
            "help" | "h" => {
                self.show_help();
                Ok(true)
            }
            "quit" | "exit" | "q" => Ok(false),
            "reset" | "clear" => {
                self.reset();
                println!("VM state reset");
                Ok(true)
            }
            "status" | "state" => {
                self.show_status();
                Ok(true)
            }
            "stack" => {
                self.show_stack();
                Ok(true)
            }
            "memory" | "mem" => {
                self.show_memory();
                Ok(true)
            }
            "meminfo" => {
                self.show_detailed_memory();
                Ok(true)
            }
            "compile" => {
                self.compile_mode();
                Ok(true)
            }
            "history" => {
                self.show_history();
                Ok(true)
            }
            "disasm" | "dis" => {
                self.show_disassembly();
                Ok(true)
            }
            "run" | "execute" => {
                self.execute_chunk()?;
                Ok(true)
            }
            "debug" => {
                let enabled = if parts.len() > 1 {
                    match parts[1] {
                        "on" | "true" | "1" => true,
                        "off" | "false" | "0" => false,
                        _ => return Err("Invalid debug mode. Use 'on' or 'off'".to_string()),
                    }
                } else {
                    true
                };
                self.vm.set_debug_mode(enabled);
                println!("Debug mode: {}", if enabled { "ON" } else { "OFF" });
                Ok(true)
            }
            "const" => {
                if parts.len() < 2 {
                    return Err("Usage: const <value>".to_string());
                }
                self.add_constant(&parts[1..])?;
                Ok(true)
            }
            "set" => {
                if parts.len() < 3 {
                    return Err("Usage: set <variable> <value>".to_string());
                }
                self.set_variable(parts[1], &parts[2..])?;
                Ok(true)
            }
            "get" => {
                if parts.len() < 2 {
                    return Err("Usage: get <variable>".to_string());
                }
                self.get_variable(parts[1])?;
                Ok(true)
            }
            _ => {
                // Try to parse as bytecode instruction
                self.parse_instruction(input)?;
                Ok(true)
            }
        }
    }

    /// Show help information
    fn show_help(&self) {
        println!("Available commands:");
        println!("  help, h              - Show this help");
        println!("  quit, exit, q        - Exit REPL");
        println!("  reset, clear         - Reset VM state");
        println!("  status, state        - Show VM status");
        println!("  stack                - Show stack contents");
        println!("  memory, mem          - Show memory statistics");
        println!("  meminfo              - Show detailed memory usage");
        println!("  compile              - Enter CodeNothing language compile mode");
        println!("  history              - Show instruction history");
        println!("  disasm, dis          - Show current chunk disassembly");
        println!("  run, execute         - Execute current chunk");
        println!("  debug <on|off>       - Toggle debug mode");
        println!("  const <value>        - Add constant to chunk");
        println!("  set <var> <value>    - Set variable");
        println!("  get <var>            - Get variable value");
        println!();
        println!("Bytecode instructions:");
        println!("  const <index>        - Push constant onto stack");
        println!("  null                 - Push null onto stack");
        println!("  true                 - Push true onto stack");
        println!("  false                - Push false onto stack");
        println!("  dup                  - Duplicate top stack value");
        println!("  pop                  - Pop top stack value");
        println!("  swap                 - Swap top two stack values");
        println!("  add                  - Add two values");
        println!("  sub                  - Subtract two values");
        println!("  mul                  - Multiply two values");
        println!("  div                  - Divide two values");
        println!("  mod                  - Modulo operation");
        println!("  neg                  - Negate value");
        println!("  eq                   - Equal comparison");
        println!("  ne                   - Not equal comparison");
        println!("  lt                   - Less than comparison");
        println!("  le                   - Less than or equal");
        println!("  gt                   - Greater than comparison");
        println!("  ge                   - Greater than or equal");
        println!("  and                  - Logical AND");
        println!("  or                   - Logical OR");
        println!("  not                  - Logical NOT");
        println!("  print                - Print top stack value");
        println!("  halt                 - Halt execution");
        println!("  nop                  - No operation");
    }

    /// Reset VM state
    fn reset(&mut self) {
        self.vm = VirtualMachine::new();
        self.current_chunk = Chunk::new("repl".to_string());
        self.variables.clear();
        self.line_number = 1;
    }

    /// Show VM status
    fn show_status(&self) {
        println!("VM Status:");
        println!("{}", self.vm.stats());
        println!("Current chunk: {} ({} instructions)",
                 self.current_chunk.name,
                 self.current_chunk.instruction_count());
        println!("Variables: {}", self.variables.len());
    }

    /// Show stack contents
    fn show_stack(&self) {
        // We need to access the stack through VM stats since it's private
        let stats = self.vm.stats();
        println!("Stack size: {}", stats.stack_size);
        // Note: We can't show actual stack contents without exposing the stack
        // This is a limitation of our current VM design
    }

    /// Show memory statistics
    fn show_memory(&self) {
        let stats = self.vm.stats();
        println!("{}", stats.memory_stats);
    }

    /// Show detailed memory usage information
    fn show_detailed_memory(&self) {
        let memory_info = self.vm.memory_info();
        println!("{}", memory_info);
    }

    /// Enter CodeNothing language compile mode
    fn compile_mode(&mut self) {
        println!("Entering CodeNothing Language Compile Mode");
        println!("Type your CodeNothing source code. End with 'END' on a new line.");
        println!("Example:");
        println!("  fn main() : int {{");
        println!("    lang::println(\"Hello, CodeNothing!\");");
        println!("    return 42;");
        println!("  }}");
        println!("  END");
        println!();

        let mut source_lines = Vec::new();

        loop {
            print!(">>> ");
            io::stdout().flush().unwrap();

            let mut line = String::new();
            match io::stdin().read_line(&mut line) {
                Ok(_) => {
                    let line = line.trim();
                    if line == "END" {
                        break;
                    }
                    source_lines.push(line.to_string());
                }
                Err(e) => {
                    println!("Error reading input: {}", e);
                    return;
                }
            }
        }

        let source = source_lines.join("\n");
        if source.trim().is_empty() {
            println!("No source code provided.");
            return;
        }

        println!("\nCompiling source code...");
        match self.compile_and_run(&source) {
            Ok(()) => println!("Compilation and execution completed successfully."),
            Err(e) => println!("Error: {}", e),
        }
    }

    /// Compile and run CodeNothing source code
    fn compile_and_run(&mut self, source: &str) -> Result<(), String> {
        // Lexical analysis
        println!("1. Lexical analysis...");
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().map_err(|e| format!("Lexer error: {}", e))?;
        println!("   Generated {} tokens", tokens.len());

        // Syntax analysis
        println!("2. Syntax analysis...");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().map_err(|e| format!("Parser error: {}", e))?;
        println!("   Parsed {} declarations", program.declarations.len());

        // Semantic analysis
        println!("3. Semantic analysis...");
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze_program(&program).map_err(|errors| {
            let error_messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            format!("Semantic errors:\n{}", error_messages.join("\n"))
        })?;
        println!("   Type checking completed successfully");

        // Code generation
        println!("4. Code generation...");
        let mut generator = CodeGenerator::new();
        let chunks = generator.generate_program(&program).map_err(|errors| {
            let error_messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            format!("Code generation errors:\n{}", error_messages.join("\n"))
        })?;
        println!("   Generated {} chunks", chunks.len());

        // Execution
        println!("5. Execution...");
        if chunks.is_empty() {
            return Err("No code generated".to_string());
        }

        // Load all chunks
        let mut main_chunk_index = None;
        for (i, chunk) in chunks.iter().enumerate() {
            println!("   Original chunk '{}' has {} instructions", chunk.name, chunk.instructions.len());
            let vm_chunk_index = self.vm.add_chunk(chunk.clone());
            println!("   Loaded chunk '{}' at VM index {} with {} instructions",
                     chunk.name, vm_chunk_index, chunk.instructions.len());

            if chunk.name == "main" {
                main_chunk_index = Some(vm_chunk_index);
            }
        }

        // Execute main chunk
        let main_index = main_chunk_index.ok_or("No main function found")?;
        match self.vm.execute(main_index) {
            Ok(_) => {
                println!("   Execution completed");
                Ok(())
            }
            Err(e) => Err(format!("Runtime error: {}", e)),
        }
    }

    /// Show instruction history
    fn show_history(&self) {
        println!("Instruction History:");
        for (i, instruction) in self.instruction_history.iter().enumerate() {
            println!("  {}: {}", i + 1, instruction);
        }
    }

    /// Show current chunk disassembly
    fn show_disassembly(&self) {
        println!("{}", self.current_chunk.disassemble());
    }

    /// Execute current chunk
    fn execute_chunk(&mut self) -> Result<(), String> {
        if self.current_chunk.instruction_count() == 0 {
            return Err("No instructions to execute".to_string());
        }

        let chunk_index = self.vm.add_chunk(self.current_chunk.clone());

        match self.vm.execute(chunk_index) {
            Ok(result) => {
                println!("Execution completed. Result: {}", result);
                Ok(())
            }
            Err(e) => Err(format!("Execution error: {}", e)),
        }
    }

    /// Add a constant to the chunk
    fn add_constant(&mut self, parts: &[&str]) -> Result<(), String> {
        let value_str = parts.join(" ");
        let value = self.parse_value(&value_str)?;
        let index = self.current_chunk.add_constant(value.clone());
        println!("Added constant [{}]: {}", index, value);
        Ok(())
    }

    /// Set a variable
    fn set_variable(&mut self, name: &str, parts: &[&str]) -> Result<(), String> {
        let value_str = parts.join(" ");
        let value = self.parse_value(&value_str)?;
        self.variables.insert(name.to_string(), value.clone());
        println!("Set variable '{}' = {}", name, value);
        Ok(())
    }

    /// Get a variable
    fn get_variable(&self, name: &str) -> Result<(), String> {
        match self.variables.get(name) {
            Some(value) => {
                println!("Variable '{}' = {}", name, value);
                Ok(())
            }
            None => Err(format!("Variable '{}' not found", name)),
        }
    }

    /// Parse a value from string
    fn parse_value(&self, s: &str) -> Result<Value, String> {
        let s = s.trim();

        // Try to parse as different types
        if s == "null" {
            Ok(Value::Null)
        } else if s == "true" {
            Ok(Value::Bool(true))
        } else if s == "false" {
            Ok(Value::Bool(false))
        } else if let Ok(i) = s.parse::<i64>() {
            Ok(Value::Int(i))
        } else if let Ok(f) = s.parse::<f64>() {
            Ok(Value::Float(f))
        } else if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
            // String literal
            let content = &s[1..s.len()-1];
            Ok(Value::String(Rc::new(content.to_string())))
        } else {
            // Check if it's a variable reference
            if let Some(value) = self.variables.get(s) {
                Ok(value.clone())
            } else {
                Err(format!("Cannot parse value: {}", s))
            }
        }
    }

    /// Parse and add an instruction
    fn parse_instruction(&mut self, input: &str) -> Result<(), String> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        let opcode_str = parts[0].to_lowercase();

        let instruction = match opcode_str.as_str() {
            "const" => {
                if parts.len() != 2 {
                    return Err("Usage: const <index>".to_string());
                }
                let index: u16 = parts[1].parse()
                    .map_err(|_| "Invalid constant index".to_string())?;
                Instruction::with_u16(OpCode::Const, index, self.line_number)
            }
            "null" => Instruction::new(OpCode::Null, self.line_number),
            "true" => Instruction::new(OpCode::True, self.line_number),
            "false" => Instruction::new(OpCode::False, self.line_number),
            "dup" => Instruction::new(OpCode::Dup, self.line_number),
            "pop" => Instruction::new(OpCode::Pop, self.line_number),
            "swap" => Instruction::new(OpCode::Swap, self.line_number),
            "add" => Instruction::new(OpCode::Add, self.line_number),
            "sub" => Instruction::new(OpCode::Sub, self.line_number),
            "mul" => Instruction::new(OpCode::Mul, self.line_number),
            "div" => Instruction::new(OpCode::Div, self.line_number),
            "mod" => Instruction::new(OpCode::Mod, self.line_number),
            "neg" => Instruction::new(OpCode::Neg, self.line_number),
            "eq" => Instruction::new(OpCode::Eq, self.line_number),
            "ne" => Instruction::new(OpCode::Ne, self.line_number),
            "lt" => Instruction::new(OpCode::Lt, self.line_number),
            "le" => Instruction::new(OpCode::Le, self.line_number),
            "gt" => Instruction::new(OpCode::Gt, self.line_number),
            "ge" => Instruction::new(OpCode::Ge, self.line_number),
            "and" => Instruction::new(OpCode::And, self.line_number),
            "or" => Instruction::new(OpCode::Or, self.line_number),
            "not" => Instruction::new(OpCode::Not, self.line_number),
            "print" => Instruction::new(OpCode::Print, self.line_number),
            "halt" => Instruction::new(OpCode::Halt, self.line_number),
            "nop" => Instruction::new(OpCode::Nop, self.line_number),
            _ => return Err(format!("Unknown instruction: {}", opcode_str)),
        };

        let index = self.current_chunk.add_instruction(instruction);
        println!("Added instruction [{}]: {}", index, input);
        self.line_number += 1;
        Ok(())
    }
}

impl Default for Repl {
    fn default() -> Self {
        Self::new()
    }
}