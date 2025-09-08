mod core;
mod repl;
mod lang;
mod compiler;
mod executor;
mod jit;
mod library_loader;


use core::{
    bytecode::{Chunk, Instruction, OpCode},
    memory::MemoryConfig,
    value::Value,
    vm::VirtualMachine,
};
use lang::{
    lexer::Lexer,
    parser::Parser,
    semantic::SemanticAnalyzer,
    codegen::CodeGenerator,
    ast::Declaration,
};
use compiler::{Compiler, CompileOptions};
use std::fs;
use repl::Repl;
use std::env;
use std::rc::Rc;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse command line arguments
    let mut memory_config = MemoryConfig::default();
    let mut repl_mode = false;
    let mut show_help = false;
    let mut input_file: Option<String> = None;
    let mut verbose = false;
    let mut build_mode = false;
    let mut run_mode = false;
    let mut output_file: Option<String> = None;
    let mut disasm_mode = false;
    let mut jit_enabled = true;
    let mut jit_threshold: Option<u32> = None;
    let mut aggressive_jit = false;
    let mut jit_debug = false;
    let mut debug = false;
    let mut show_stats = false;
    let mut show_all_details = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--repl" | "-r" | "repl" => {
                repl_mode = true;
            }
            "--memory" | "-m" => {
                if i + 1 < args.len() {
                    match args[i + 1].as_str() {
                        "small" => memory_config = MemoryConfig::small(),
                        "large" => memory_config = MemoryConfig::large(),
                        "default" => memory_config = MemoryConfig::default(),
                        _ => {
                            println!("Invalid memory configuration. Use: small, default, large");
                            return;
                        }
                    }
                    i += 1;
                } else {
                    println!("--memory requires an argument: small, default, large");
                    return;
                }
            }
            "--heap" => {
                if i + 2 < args.len() {
                    let initial: usize = args[i + 1].parse().unwrap_or_else(|_| {
                        println!("Invalid initial heap size");
                        std::process::exit(1);
                    });
                    let max: usize = args[i + 2].parse().unwrap_or_else(|_| {
                        println!("Invalid max heap size");
                        std::process::exit(1);
                    });
                    memory_config = MemoryConfig::custom(initial, max, 64); // 64MB stack default
                    i += 2;
                } else {
                    println!("--heap requires two arguments: <initial_mb> <max_mb>");
                    return;
                }
            }
            "--unlimited" => {
                memory_config = MemoryConfig::unlimited();
                println!("⚠️  WARNING: Using unlimited memory mode - can consume all system memory!");
            }
            "--deep-recursion" => {
                memory_config = MemoryConfig::deep_recursion();
                println!("🔄 Using deep recursion mode - optimized for recursive algorithms");
            }
            "--help" | "-h" => {
                show_help = true;
            }
            "--verbose" | "--log" | "-v" => {
                verbose = true;
            }
            "--build" | "-b" => {
                build_mode = true;
            }
            "--run" => {
                run_mode = true;
            }
            "--disasm" | "--disassemble" | "-d" => {
                disasm_mode = true;
            }
            "--output" | "-o" => {
                if i + 1 < args.len() {
                    output_file = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    println!("--output requires a filename argument");
                    return;
                }
            }
            "--jit" => {
                jit_enabled = true;
            }
            "--no-jit" => {
                jit_enabled = false;
            }
            "--jit-threshold" => {
                if i + 1 < args.len() {
                    jit_threshold = Some(args[i + 1].parse().unwrap_or_else(|_| {
                        println!("Invalid JIT threshold value");
                        std::process::exit(1);
                    }));
                    i += 1;
                } else {
                    println!("--jit-threshold requires a number argument");
                    return;
                }
            }
            "--aggressive-jit" => {
                aggressive_jit = true;
                jit_threshold = Some(5); // Very aggressive threshold
            }
            "--jit-debug" => {
                jit_debug = true;
            }
            "--debug" => {
                debug = true;
            }
            "--stats" => {
                show_stats = true;
            }
            "--all" => {
                show_all_details = true;
            }
            arg if arg.ends_with(".cn") => {
                // CodeNothing source file
                input_file = Some(arg.to_string());
            }
            arg if arg.ends_with(".comcn") => {
                // CodeNothing bytecode file
                input_file = Some(arg.to_string());
                if !build_mode {
                    run_mode = true; // Default to run mode for .comcn files
                }
            }
            _ => {
                println!("Unknown argument: {}", args[i]);
                show_help = true;
            }
        }
        i += 1;
    }

    if show_help {
        print_help();
        return;
    }

    // Check if input file is provided
    if let Some(file_path) = input_file {
        if build_mode {
            // Compile to bytecode mode
            handle_build_mode(&file_path, output_file, verbose);
        } else if run_mode {
            // Run bytecode mode
            handle_run_mode(&file_path, memory_config, verbose);
        } else if disasm_mode {
            // Disassemble bytecode mode
            handle_disasm_mode(&file_path, verbose);
        } else {
            // Default: compile and run immediately
            if verbose {
                println!("Compiling and running CodeNothing file: {}", file_path);
                println!("Memory Configuration: {}", memory_config.memory_usage_string());
            }

            // Set library debug mode based on debug flag
            crate::library_loader::set_library_debug_mode(debug);

            match compile_and_run_file_new(&file_path, memory_config, verbose, jit_enabled, jit_threshold, aggressive_jit, jit_debug, debug, show_stats, show_all_details) {
                Ok(()) => {
                    if verbose {
                        println!("Program executed successfully.");
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        return;
    }

    // Check if REPL mode is requested
    if repl_mode {
        // Set library debug mode based on debug flag
        crate::library_loader::set_library_debug_mode(debug);

        println!("Starting CodeNothing VM REPL...");
        println!("Memory Configuration: {}", memory_config.memory_usage_string());
        let mut repl = Repl::new();
        repl.run();
        return;
    }

    // Default: Run demo program
    // Set library debug mode based on debug flag
    crate::library_loader::set_library_debug_mode(debug);
    run_demo_program(memory_config, jit_enabled, jit_threshold, aggressive_jit, verbose, show_stats);
}

/// Run demo program
fn run_demo_program(memory_config: MemoryConfig, jit_enabled: bool, jit_threshold: Option<u32>, aggressive_jit: bool, verbose: bool, show_stats: bool) {
    println!("CodeNothing");
    println!("==============================");
    println!("Use --repl or -r to start interactive mode");
    println!();

    // Create a simple test program
    let mut chunk = Chunk::new("demo".to_string());

    // Add some constants
    let const_42 = chunk.add_constant(Value::Int(42));
    let const_10 = chunk.add_constant(Value::Int(10));
    let const_hello = chunk.add_constant(Value::String(Rc::new("Hello, VM!".to_string())));

    // Create a simple program: print "Hello, VM!", then 42 + 10
    chunk.add_instruction(Instruction::with_u16(OpCode::Const, const_hello, 1));
    chunk.add_instruction(Instruction::new(OpCode::Print, 2));
    chunk.add_instruction(Instruction::with_u16(OpCode::Const, const_42, 3));
    chunk.add_instruction(Instruction::with_u16(OpCode::Const, const_10, 4));
    chunk.add_instruction(Instruction::new(OpCode::Add, 5));
    chunk.add_instruction(Instruction::new(OpCode::Print, 6));
    chunk.add_instruction(Instruction::new(OpCode::Halt, 7));

    // Print disassembly
    println!("Bytecode Disassembly:");
    println!("{}", chunk.disassemble());

    // Create and run VM with specified memory configuration
    let mut vm = VirtualMachine::with_memory_config(memory_config.clone());
    vm.set_debug_mode(false); // Disable debug for demo

    // Configure JIT
    vm.set_jit_enabled(jit_enabled);
    if aggressive_jit {
        vm.enable_aggressive_jit();
    } else if let Some(threshold) = jit_threshold {
        vm.set_jit_threshold(threshold);
    }

    println!("VM Memory Configuration: {}", memory_config.memory_usage_string());

    let chunk_index = vm.add_chunk(chunk);

    println!("Executing program...");
    println!("====================");

    match vm.execute(chunk_index) {
        Ok(result) => {
            println!("\nProgram completed successfully!");
            println!("Result: {}", result);
        }
        Err(e) => {
            println!("\nExecution error: {}", e);
        }
    }

    // Print VM statistics and performance report if requested
    if show_stats || verbose {
        println!("\n{}", vm.stats());
        vm.print_performance_report();
    }

    println!("\nTry running with --repl for interactive mode!");
}

/// Compile and run a CodeNothing source file using the new compiler
fn compile_and_run_file_new(file_path: &str, memory_config: MemoryConfig, verbose: bool, jit_enabled: bool, jit_threshold: Option<u32>, aggressive_jit: bool, jit_debug: bool, debug: bool, show_stats: bool, show_all_details: bool) -> Result<(), String> {
    if verbose {
        println!("Compiling and running CodeNothing file: {}", file_path);
        println!("Memory Configuration: {} KB stack, {} KB heap",
                 memory_config.stack_size / 1024,
                 memory_config.initial_heap_size / 1024);
    }

    // Create compiler with options
    let mut options = CompileOptions::default();
    options.verbose = verbose;
    options.debug_info = debug;

    let compiler = Compiler::with_options(options);

    // Compile the file
    let result = compiler.compile(file_path);

    if !result.success {
        return Err(format!("Compilation failed:\n{}", result.messages.join("\n")));
    }

    if verbose {
        for message in &result.messages {
            println!("{}", message);
        }
    }

    // Get the bytecode
    let bytecode_file = result.bytecode.ok_or("No bytecode generated")?;

    if bytecode_file.chunks.is_empty() {
        return Err("No code generated".to_string());
    }

    // Execute the bytecode
    execute_bytecode(bytecode_file.chunks, memory_config, verbose, jit_enabled, jit_threshold, aggressive_jit, jit_debug, debug, show_stats, show_all_details)
}

/// Execute bytecode chunks
fn execute_bytecode(chunks: Vec<Chunk>, memory_config: MemoryConfig, verbose: bool, jit_enabled: bool, jit_threshold: Option<u32>, aggressive_jit: bool, jit_debug: bool, debug: bool, show_stats: bool, show_all_details: bool) -> Result<(), String> {
    // Execution
    if verbose {
        println!("5. Execution...");
    }

    // Create VM with specified memory configuration
    let mut vm = VirtualMachine::with_memory_config(memory_config);

    // Note: Class inheritance and definitions would need to be extracted from the compiler
    // For now, we'll proceed without them for basic functionality

    // Configure JIT
    if jit_enabled {
        vm.set_jit_enabled(true);
        if let Some(threshold) = jit_threshold {
            vm.set_jit_threshold(threshold);
        }
        if aggressive_jit {
            vm.enable_aggressive_jit();
        }
        if jit_debug {
            vm.set_jit_debug(true);
        }
    } else {
        vm.set_jit_enabled(false);
    }

    // Load chunks into VM
    for chunk in chunks {
        vm.add_chunk(chunk);
    }

    // Execute main function (chunk 0 is typically the main chunk)
    let result = vm.execute(0);

    if verbose {
        println!("   Execution completed successfully");
    }

    // Show statistics if requested
    if show_stats || verbose {
        vm.print_performance_report();
    }

    result.map(|_| ()).map_err(|e| format!("Runtime error: {}", e))
}

/// Compile and run a CodeNothing source file (legacy version)
fn compile_and_run_file(file_path: &str, memory_config: MemoryConfig, verbose: bool, jit_enabled: bool, jit_threshold: Option<u32>, aggressive_jit: bool, jit_debug: bool, debug: bool, show_stats: bool, show_all_details: bool) -> Result<(), String> {
    // Read source file
    if verbose {
        println!("Reading source file: {}", file_path);
    }
    let source = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;

    if source.trim().is_empty() {
        return Err("Source file is empty".to_string());
    }

    // Lexical analysis
    if verbose {
        println!("1. Lexical analysis...");
    }
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize().map_err(|e| format!("Lexer error: {}", e))?;
    if verbose {
        println!("   Generated {} tokens", tokens.len());
    }

    // Syntax analysis
    if verbose {
        println!("2. Syntax analysis...");
    }
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().map_err(|e| format!("Parser error: {}", e))?;
    if verbose {
        println!("   Parsed {} declarations", program.declarations.len());
    }

    // Semantic analysis
    if verbose {
        println!("3. Semantic analysis...");
    }
    let mut analyzer = SemanticAnalyzer::new();
    // Set debug mode for semantic analysis
    analyzer.set_debug_mode(debug);
    analyzer.analyze_program(&program).map_err(|errors| {
        let error_messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        format!("Semantic errors:\n{}", error_messages.join("\n"))
    })?;
    if verbose {
        println!("   Type checking completed successfully");
    }

    // Code generation
    if verbose {
        println!("4. Code generation...");
    }
    let mut generator = CodeGenerator::new();
    // Set debug mode for code generation
    generator.set_debug_mode(debug);
    // Set show all details mode for arrow function display
    generator.set_show_all_details(show_all_details);
    // Pass the type environment from semantic analysis to code generation
    generator.set_type_environment(analyzer.get_type_environment().clone());
    let chunks = generator.generate_program(&program).map_err(|errors| {
        let error_messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        format!("Code generation errors:\n{}", error_messages.join("\n"))
    })?;
    if verbose {
        println!("   Generated {} chunks", chunks.len());
    }

    // Execution
    if verbose {
        println!("5. Execution...");
    }
    if chunks.is_empty() {
        return Err("No code generated".to_string());
    }

    // Create VM with specified memory configuration
    let mut vm = VirtualMachine::with_memory_config(memory_config);

    // Register class inheritance relationships
    for (child_class, parent_class) in generator.get_class_inheritance() {
        vm.register_class_inheritance(child_class.clone(), parent_class.clone());
        if verbose {
            println!("  Registered inheritance: {} extends {}", child_class, parent_class);
        }
    }

    // Register class definitions
    for (class_name, class_def) in generator.get_class_definitions() {
        vm.register_class_definition(class_def.clone());
        if verbose {
            println!("  Registered class definition: {}", class_name);
        }
    }

    // Build VTables for all classes
    for class_name in generator.get_class_definitions().keys() {
        if let Err(e) = vm.build_vtable(class_name) {
            if verbose {
                println!("  Warning: Failed to build VTable for '{}': {}", class_name, e);
            }
        } else if verbose {
            println!("  Built VTable for class: {}", class_name);
        }
    }

    // Validate that all concrete classes implement required abstract methods
    for class_name in generator.get_class_definitions().keys() {
        if let Err(e) = vm.validate_concrete_class(class_name) {
            if verbose {
                println!("  Warning: Abstract method validation failed for '{}': {}", class_name, e);
            }
        } else if verbose {
            println!("  Validated concrete class: {}", class_name);
        }
    }

    // Configure JIT and debug mode
    vm.set_jit_enabled(jit_enabled);
    vm.set_jit_debug(jit_debug);
    vm.set_debug_mode(debug);
    if aggressive_jit {
        vm.enable_aggressive_jit();
    } else if let Some(threshold) = jit_threshold {
        vm.set_jit_threshold(threshold);
    }

    // Count global variables and allocate space
    let global_count = program.declarations.iter()
        .filter(|decl| matches!(decl, Declaration::Variable(_)))
        .count();

    if global_count > 0 {
        vm.allocate_globals(global_count).map_err(|e| format!("Failed to allocate globals: {}", e))?;
        if verbose {
            println!("   Allocated {} global variables", global_count);
        }
    }

    // Load main chunk into VM
    let main_chunk = &chunks[0];
    vm.add_chunk(main_chunk.clone());

    // Load other chunks as functions
    for (_i, chunk) in chunks.iter().enumerate().skip(1) {
        vm.add_chunk(chunk.clone());
        if verbose {
            println!("   Loaded function chunk: {}", chunk.name);
        }
    }

    // Execute main chunk (index 0)
    if jit_debug {
        println!("🔧 MAIN: Starting VM execution with chunk 0");
    }
    match vm.execute(0) {
        Ok(_) => {
            if verbose {
                println!("   Execution completed successfully");
            }

            // Print VM statistics and performance report if requested
            if show_stats || verbose {
                println!("\n{}", vm.stats());
                vm.print_performance_report();
            }

            Ok(())
        }
        Err(e) => Err(format!("Runtime error: {}", e)),
    }
}

fn print_help() {
    println!("CodeNothing VM & JIT Compiler");
    println!("==============================");
    println!();
    println!("USAGE:");
    println!("  CodeNothing [OPTIONS] [FILE]");
    println!();
    println!("ARGUMENTS:");
    println!("  FILE                        CodeNothing source file (.cn) or bytecode (.comcn)");
    println!();
    println!("OPTIONS:");
    println!("  --repl, -r, repl           Start interactive REPL mode");
    println!("  --build, -b                 Compile source to bytecode (.comcn)");
    println!("  --run                       Execute bytecode file (.comcn)");
    println!("  --disasm, -d                Disassemble bytecode file (.comcn)");
    println!("  --output, -o <file>         Specify output file for --build");
    println!("  --memory, -m <config>       Set memory configuration");
    println!("                              Options: small, default, large");
    println!("  --heap <initial> <max>      Set custom heap size in MB");
    println!("  --unlimited                 Use unlimited memory mode (⚠️ WARNING: Can use all system memory!)");
    println!("  --deep-recursion            Use deep recursion mode (optimized for recursive algorithms)");
    println!("  --jit                       Enable JIT compilation (default)");
    println!("  --no-jit                    Disable JIT compilation");
    println!("  --jit-threshold <count>     Set JIT compilation threshold (default: 50)");
    println!("  --aggressive-jit            Enable aggressive JIT mode (threshold: 5)");
    println!("  --jit-debug                 Show JIT compilation debug output");
    println!("  --debug                     Show VM execution debug output");
    println!("  --stats                     Show VM statistics after execution");
    println!("  --all                       Show full details for arrow functions (not abbreviated)");
    println!("  --verbose, --log, -v        Show detailed compilation steps and stats");
    println!("  --help, -h                  Show this help message");
    println!();
    println!("MEMORY CONFIGURATIONS:");
    println!("  small         - For embedded/testing (~32MB max)");
    println!("  default       - Balanced configuration (~512MB max)");
    println!("  large         - For production/server (~8GB max)");
    println!("  unlimited     - No memory limits (⚠️ WARNING: Can consume all system memory!)");
    println!("  deep-recursion - Optimized for deep recursive algorithms (~8GB, 1M call depth)");
    println!();
    println!("EXAMPLES:");
    println!("  CodeNothing hello.cn        # Compile and run hello.cn (quiet mode)");
    println!("  CodeNothing -v hello.cn     # Compile and run with verbose output");
    println!("  CodeNothing -m large hello.cn # Run with large memory config");
    println!("  CodeNothing --repl          # Start REPL with default memory");
    println!("  CodeNothing -m small --repl # Start REPL with small memory");
    println!("  CodeNothing --heap 64 512 hello.cn # Custom heap: 64MB initial, 512MB max");
    println!("  CodeNothing --build hello.cn        # Compile to bytecode");
    println!("  CodeNothing --run hello.comcn       # Run bytecode file");
    println!("  CodeNothing --disasm hello.comcn    # Show bytecode disassembly");
    println!("  CodeNothing --no-jit hello.cn       # Run without JIT compilation");
    println!("  CodeNothing --jit-threshold 20 hello.cn # JIT compile after 20 executions");
    println!("  CodeNothing --aggressive-jit hello.cn   # Use aggressive JIT mode");
    println!("  CodeNothing --jit-debug hello.cn        # Show JIT debug output");
    println!("  CodeNothing --stats hello.cn            # Show VM statistics");
    println!("  CodeNothing --all hello.cn              # Show full arrow function details");
    println!("  CodeNothing --unlimited fibonacci.cn    # Run with unlimited memory for deep recursion");
    println!("  CodeNothing --deep-recursion fib30.cn   # Run with deep recursion optimization");
    println!();
}

/// Handle build mode - compile source to bytecode
fn handle_build_mode(input_file: &str, output_file: Option<String>, verbose: bool) {
    use compiler::{Compiler, CompileOptions};
    use std::path::PathBuf;

    let options = CompileOptions {
        output: output_file.map(PathBuf::from),
        optimization_level: 0,
        debug_info: true,
        verbose,
        flags: Vec::new(),
    };

    let compiler = Compiler::with_options(options);
    let result = compiler.compile(input_file);

    // Print compilation messages
    for message in &result.messages {
        println!("{}", message);
    }

    if result.success {
        println!("✅ Compilation successful!");
        println!("📦 Bytecode saved to: {}", result.output_file.display());

        if let Some(bytecode) = &result.bytecode {
            println!("📊 File info:");
            println!("   Functions: {}", bytecode.chunks.len());
            println!("   Constants: {}", bytecode.constants.len());
            println!("   Entry point: {}", bytecode.entry_point);
        }
    } else {
        eprintln!("❌ Compilation failed!");
        std::process::exit(1);
    }
}

/// Handle run mode - execute bytecode file
fn handle_run_mode(input_file: &str, memory_config: MemoryConfig, verbose: bool) {
    use executor::{Executor, ExecuteOptions};

    if !input_file.ends_with(".comcn") {
        eprintln!("Error: Run mode requires a .comcn bytecode file");
        eprintln!("Use --build to compile .cn files to .comcn first");
        std::process::exit(1);
    }

    let options = ExecuteOptions {
        verbose,
        memory_mb: Some(memory_config.initial_heap_size),
        stack_mb: Some(memory_config.stack_size),
        debug: false,
    };

    let executor = Executor::with_options(options);
    let result = executor.execute(input_file);

    // Print execution messages
    for message in &result.messages {
        println!("{}", message);
    }

    if result.success {
        if verbose {
            println!("✅ Execution completed successfully!");
            println!("⏱️  Execution time: {} ms", result.execution_time_ms);
            println!("🚪 Exit code: {}", result.exit_code);
        }
        std::process::exit(result.exit_code);
    } else {
        eprintln!("❌ Execution failed!");
        std::process::exit(result.exit_code);
    }
}

/// Handle disassemble mode - show bytecode content
fn handle_disasm_mode(input_file: &str, verbose: bool) {
    if !input_file.ends_with(".comcn") {
        eprintln!("Error: Disassemble mode requires a .comcn bytecode file");
        std::process::exit(1);
    }

    if verbose {
        println!("Disassembling bytecode file: {}", input_file);
    }

    // Read and parse bytecode file
    match fs::read(input_file) {
        Ok(data) => {
            match bincode::deserialize::<Vec<Chunk>>(&data) {
                Ok(chunks) => {
                    println!("🔍 Bytecode Disassembly: {}", input_file);
                    println!("═══════════════════════════════════════");
                    println!("📊 Total chunks: {}", chunks.len());
                    println!();

                    for (chunk_index, chunk) in chunks.iter().enumerate() {
                        println!("📦 Chunk {}: \"{}\"", chunk_index, chunk.name);
                        println!("   Instructions: {}", chunk.instructions.len());
                        println!("   Constants: {}", chunk.constants.len());
                        println!();

                        // Show constants
                        if !chunk.constants.is_empty() {
                            println!("   📋 Constants:");
                            for (i, constant) in chunk.constants.iter().enumerate() {
                                println!("     [{}] {:?}", i, constant);
                            }
                            println!();
                        }

                        // Show instructions
                        println!("   🔧 Instructions:");
                        for (i, instruction) in chunk.instructions.iter().enumerate() {
                            println!("     {:04} {:?}", i, instruction);
                        }
                        println!();
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to parse bytecode file: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to read bytecode file: {}", e);
            std::process::exit(1);
        }
    }
}
