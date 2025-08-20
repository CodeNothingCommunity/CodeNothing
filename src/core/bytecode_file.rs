//! CodeNothing Bytecode File Format (.comcn)
//! 
//! This module defines the format for compiled CodeNothing bytecode files
//! that can be executed on any platform with a CodeNothing VM.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Write, BufReader, BufWriter};
use std::path::Path;

use crate::core::{Chunk, Value};

/// Magic number for CodeNothing bytecode files
pub const COMCN_MAGIC: &[u8; 8] = b"COMCN\x01\x00\x00";

/// Version of the bytecode format
pub const COMCN_VERSION: u32 = 1;

/// CodeNothing Compiled Bytecode File
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComcnFile {
    /// File format version
    pub version: u32,
    
    /// Source file name (for debugging)
    pub source_file: String,
    
    /// Compilation timestamp
    pub timestamp: u64,
    
    /// Entry point function name (usually "main")
    pub entry_point: String,
    
    /// All compiled chunks (functions)
    pub chunks: Vec<Chunk>,
    
    /// Function name to chunk index mapping
    pub function_map: HashMap<String, usize>,
    
    /// Global constants pool
    pub constants: Vec<Value>,
    
    /// Metadata for debugging and optimization
    pub metadata: ComcnMetadata,
}

/// Metadata for the compiled bytecode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComcnMetadata {
    /// Compiler version that generated this file
    pub compiler_version: String,
    
    /// Optimization level used
    pub optimization_level: u8,
    
    /// Debug information included
    pub debug_info: bool,
    
    /// Target VM version
    pub target_vm_version: String,
    
    /// Additional compilation flags
    pub flags: Vec<String>,

    /// Names of memoized functions
    pub memo_functions: HashSet<String>,
}

impl ComcnFile {
    /// Create a new bytecode file
    pub fn new(source_file: String, entry_point: String) -> Self {
        Self {
            version: COMCN_VERSION,
            source_file,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            entry_point,
            chunks: Vec::new(),
            function_map: HashMap::new(),
            constants: Vec::new(),
            metadata: ComcnMetadata {
                compiler_version: env!("CARGO_PKG_VERSION").to_string(),
                optimization_level: 0,
                debug_info: true,
                target_vm_version: "1.0".to_string(),
                flags: Vec::new(),
                memo_functions: HashSet::new(),
            },
        }
    }
    
    /// Add a chunk to the bytecode file
    pub fn add_chunk(&mut self, name: String, chunk: Chunk) -> usize {
        let index = self.chunks.len();
        self.chunks.push(chunk);
        self.function_map.insert(name, index);
        index
    }
    
    /// Get chunk by function name
    pub fn get_chunk(&self, name: &str) -> Option<&Chunk> {
        self.function_map.get(name)
            .and_then(|&index| self.chunks.get(index))
    }
    
    /// Get chunk by index
    pub fn get_chunk_by_index(&self, index: usize) -> Option<&Chunk> {
        self.chunks.get(index)
    }
    
    /// Save bytecode file to disk
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        
        // Write magic number
        writer.write_all(COMCN_MAGIC)?;
        
        // Serialize and write the bytecode data
        let data = bincode::serialize(self)?;
        let data_size = data.len() as u64;
        
        // Write data size (8 bytes, little endian)
        writer.write_all(&data_size.to_le_bytes())?;
        
        // Write serialized data
        writer.write_all(&data)?;
        
        writer.flush()?;
        Ok(())
    }
    
    /// Load bytecode file from disk
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        
        // Read and verify magic number
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        
        if &magic != COMCN_MAGIC {
            return Err("Invalid CodeNothing bytecode file: wrong magic number".into());
        }
        
        // Read data size
        let mut size_bytes = [0u8; 8];
        reader.read_exact(&mut size_bytes)?;
        let data_size = u64::from_le_bytes(size_bytes) as usize;
        
        // Read serialized data
        let mut data = vec![0u8; data_size];
        reader.read_exact(&mut data)?;
        
        // Deserialize bytecode
        let bytecode: ComcnFile = bincode::deserialize(&data)?;
        
        // Verify version compatibility
        if bytecode.version > COMCN_VERSION {
            return Err(format!(
                "Bytecode version {} is newer than supported version {}",
                bytecode.version, COMCN_VERSION
            ).into());
        }
        
        Ok(bytecode)
    }
    
    /// Get file info as a formatted string
    pub fn info(&self) -> String {
        format!(
            "CodeNothing Bytecode File (.comcn)\n\
             Source: {}\n\
             Version: {}\n\
             Entry Point: {}\n\
             Functions: {}\n\
             Constants: {}\n\
             Compiled: {}\n\
             Compiler: v{}",
            self.source_file,
            self.version,
            self.entry_point,
            self.chunks.len(),
            self.constants.len(),
            chrono::DateTime::from_timestamp(self.timestamp as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            self.metadata.compiler_version
        )
    }

    /// Set memoized functions
    pub fn set_memo_functions(&mut self, memo_functions: HashSet<String>) {
        self.metadata.memo_functions = memo_functions;
    }

    /// Get memoized functions
    pub fn get_memo_functions(&self) -> &HashSet<String> {
        &self.metadata.memo_functions
    }
}
