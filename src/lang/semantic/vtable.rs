//! Virtual Method Table (VTable) implementation for polymorphism
//! 
//! Manages virtual method dispatch for class inheritance and method overriding

use std::collections::HashMap;
use crate::lang::ast::{ClassDeclaration, ClassMethod};
use crate::lang::types::{Type, TypeEnvironment};

/// Virtual method entry in the VTable
#[derive(Debug, Clone)]
pub struct VTableEntry {
    /// Method name
    pub method_name: String,
    /// Class that defines this method implementation
    pub defining_class: String,
    /// Method signature
    pub method_type: Type,
    /// Index in the VTable for efficient lookup
    pub vtable_index: usize,
    /// Whether this method is virtual
    pub is_virtual: bool,
    /// Whether this method is abstract
    pub is_abstract: bool,
}

/// Virtual Method Table for a class
#[derive(Debug, Clone)]
pub struct VTable {
    /// Class name this VTable belongs to
    pub class_name: String,
    /// Base class VTable (if any)
    pub base_vtable: Option<Box<VTable>>,
    /// Virtual method entries indexed by method name
    pub methods: HashMap<String, VTableEntry>,
    /// Ordered list of virtual methods for index-based lookup
    pub virtual_methods: Vec<VTableEntry>,
}

impl VTable {
    /// Create a new VTable for a class
    pub fn new(class_name: String) -> Self {
        Self {
            class_name,
            base_vtable: None,
            methods: HashMap::new(),
            virtual_methods: Vec::new(),
        }
    }

    /// Create VTable with base class inheritance
    pub fn with_base(class_name: String, base_vtable: VTable) -> Self {
        let mut vtable = Self {
            class_name,
            base_vtable: Some(Box::new(base_vtable.clone())),
            methods: HashMap::new(),
            virtual_methods: base_vtable.virtual_methods.clone(),
        };

        // Copy base class methods
        for (method_name, entry) in &base_vtable.methods {
            vtable.methods.insert(method_name.clone(), entry.clone());
        }

        vtable
    }

    /// Add a virtual method to the VTable
    pub fn add_virtual_method(&mut self, method: &ClassMethod, method_type: Type) {
        let method_name = method.function.name.clone();
        
        // Check if this method overrides a base class method
        if method.is_override {
            if let Some(existing_entry) = self.methods.get_mut(&method_name) {
                // Update the existing entry with new implementation
                existing_entry.defining_class = self.class_name.clone();
                existing_entry.method_type = method_type.clone();
                existing_entry.is_abstract = method.is_abstract; // Update abstract flag

                // Update in virtual_methods list as well
                for virtual_method in &mut self.virtual_methods {
                    if virtual_method.method_name == method_name {
                        virtual_method.defining_class = self.class_name.clone();
                        virtual_method.method_type = method_type.clone();
                        virtual_method.is_abstract = method.is_abstract; // Update abstract flag
                        break;
                    }
                }
            }
        } else {
            // Add new virtual method
            let vtable_index = self.virtual_methods.len();
            let entry = VTableEntry {
                method_name: method_name.clone(),
                defining_class: self.class_name.clone(),
                method_type: method_type.clone(),
                vtable_index,
                is_virtual: method.is_virtual,
                is_abstract: method.is_abstract,
            };

            self.methods.insert(method_name, entry.clone());
            self.virtual_methods.push(entry);
        }
    }

    /// Add a non-virtual method to the VTable
    pub fn add_method(&mut self, method: &ClassMethod, method_type: Type) {
        let method_name = method.function.name.clone();
        
        let entry = VTableEntry {
            method_name: method_name.clone(),
            defining_class: self.class_name.clone(),
            method_type,
            vtable_index: 0, // Non-virtual methods don't need vtable index
            is_virtual: false,
            is_abstract: method.is_abstract,
        };

        self.methods.insert(method_name, entry);
    }

    /// Get method entry by name
    pub fn get_method(&self, method_name: &str) -> Option<&VTableEntry> {
        self.methods.get(method_name)
    }

    /// Get virtual method by index
    pub fn get_virtual_method(&self, index: usize) -> Option<&VTableEntry> {
        self.virtual_methods.get(index)
    }

    /// Check if a method is virtual
    pub fn is_virtual_method(&self, method_name: &str) -> bool {
        if let Some(entry) = self.methods.get(method_name) {
            entry.is_virtual
        } else {
            false
        }
    }

    /// Get all virtual methods
    pub fn get_virtual_methods(&self) -> &Vec<VTableEntry> {
        &self.virtual_methods
    }

    /// Get method implementation class
    pub fn get_method_implementation(&self, method_name: &str) -> Option<String> {
        self.methods.get(method_name).map(|entry| entry.defining_class.clone())
    }

    /// Validate that all abstract methods are implemented
    pub fn validate_abstract_methods(&self) -> Vec<String> {
        let mut unimplemented = Vec::new();
        
        for entry in &self.virtual_methods {
            if entry.is_abstract && entry.defining_class != self.class_name {
                unimplemented.push(entry.method_name.clone());
            }
        }
        
        unimplemented
    }
}

/// VTable Manager for managing all class VTables
#[derive(Debug)]
pub struct VTableManager {
    /// VTables indexed by class name
    vtables: HashMap<String, VTable>,
}

impl VTableManager {
    pub fn new() -> Self {
        Self {
            vtables: HashMap::new(),
        }
    }

    /// Build VTable for a class
    pub fn build_vtable(&mut self, class_decl: &ClassDeclaration, type_env: &TypeEnvironment) -> Result<(), String> {
        let class_name = class_decl.name.clone();

        // Check if VTable already exists
        if self.vtables.contains_key(&class_name) {
            return Ok(());
        }

        // Create VTable with base class if exists
        let mut vtable = if let Some(base_class_name) = &class_decl.base_class {
            if let Some(base_vtable) = self.vtables.get(base_class_name) {
                VTable::with_base(class_name.clone(), base_vtable.clone())
            } else {
                // Base class VTable not found, create a basic VTable for now
                // This will be updated when the base class is processed
                VTable::new(class_name.clone())
            }
        } else {
            VTable::new(class_name.clone())
        };

        // Add methods to VTable
        for method in &class_decl.methods {
            let param_types: Vec<Type> = method.function.parameters.iter()
                .map(|p| type_env.resolve_type_annotation(&p.type_annotation))
                .collect();

            let return_type = method.function.return_type.as_ref()
                .map(|t| type_env.resolve_type_annotation(t))
                .unwrap_or(Type::Void);

            let method_type = Type::Function {
                parameters: param_types,
                return_type: Box::new(return_type),
            };

            if method.is_virtual || method.is_override || method.is_abstract {
                vtable.add_virtual_method(method, method_type);
            } else {
                vtable.add_method(method, method_type);
            }
        }

        self.vtables.insert(class_name, vtable);
        Ok(())
    }

    /// Build VTable for a class with all dependencies
    pub fn build_vtable_with_dependencies(&mut self, class_decls: &[&ClassDeclaration], type_env: &TypeEnvironment) -> Result<(), String> {
        // Build a dependency graph and process classes in topological order
        let mut processed = std::collections::HashSet::new();
        let mut processing = std::collections::HashSet::new();

        for class_decl in class_decls {
            self.build_vtable_recursive(class_decl, class_decls, type_env, &mut processed, &mut processing)?;
        }

        Ok(())
    }

    /// Recursively build VTable with dependency resolution
    fn build_vtable_recursive(
        &mut self,
        class_decl: &ClassDeclaration,
        all_classes: &[&ClassDeclaration],
        type_env: &TypeEnvironment,
        processed: &mut std::collections::HashSet<String>,
        processing: &mut std::collections::HashSet<String>,
    ) -> Result<(), String> {
        let class_name = &class_decl.name;

        // Check if already processed
        if processed.contains(class_name) {
            return Ok(());
        }

        // Check for circular dependency
        if processing.contains(class_name) {
            return Err(format!("Circular inheritance detected involving class '{}'", class_name));
        }

        processing.insert(class_name.clone());

        // Process base class first if it exists
        if let Some(base_class_name) = &class_decl.base_class {
            if let Some(base_class_decl) = all_classes.iter().find(|c| c.name == *base_class_name) {
                self.build_vtable_recursive(base_class_decl, all_classes, type_env, processed, processing)?;
            }
        }

        // Now build this class's VTable
        self.build_vtable(class_decl, type_env)?;

        processing.remove(class_name);
        processed.insert(class_name.clone());

        Ok(())
    }

    /// Get VTable for a class
    pub fn get_vtable(&self, class_name: &str) -> Option<&VTable> {
        self.vtables.get(class_name)
    }

    /// Check if a method call should use virtual dispatch
    pub fn should_use_virtual_dispatch(&self, class_name: &str, method_name: &str) -> bool {
        if let Some(vtable) = self.vtables.get(class_name) {
            vtable.is_virtual_method(method_name)
        } else {
            false
        }
    }

    /// Get method implementation class for virtual dispatch
    pub fn get_method_implementation(&self, class_name: &str, method_name: &str) -> Option<String> {
        if let Some(vtable) = self.vtables.get(class_name) {
            vtable.get_method_implementation(method_name)
        } else {
            None
        }
    }
}

impl Default for VTableManager {
    fn default() -> Self {
        Self::new()
    }
}
