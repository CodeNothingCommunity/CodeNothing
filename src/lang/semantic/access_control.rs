//! Access control checker for class members
//! 
//! Implements visibility rules for public, private, and protected members

use crate::lang::ast::{AccessModifier, Position};
use crate::lang::types::{Type, TypeError, TypeEnvironment};

/// Context information for access control checking
#[derive(Debug, Clone)]
pub struct AccessContext {
    /// Current class being analyzed (if any)
    pub current_class: Option<String>,
    /// Current namespace path
    pub current_namespace: Vec<String>,
    /// Current package (top-level namespace)
    pub current_package: Option<String>,
    /// Whether we're inside a method of the current class
    pub inside_method: bool,
    /// Whether we're inside a derived class
    pub inside_derived_class: Option<String>,
}

impl AccessContext {
    pub fn new() -> Self {
        Self {
            current_class: None,
            current_namespace: Vec::new(),
            current_package: None,
            inside_method: false,
            inside_derived_class: None,
        }
    }

    pub fn enter_class(&mut self, class_name: String) {
        self.current_class = Some(class_name);
    }

    pub fn set_inheritance_context(&mut self, derived_class: String, base_class: String) {
        self.inside_derived_class = Some(base_class);
    }

    pub fn clear_inheritance_context(&mut self) {
        self.inside_derived_class = None;
    }

    pub fn exit_class(&mut self) {
        self.current_class = None;
        self.inside_method = false;
        self.inside_derived_class = None;
    }

    pub fn enter_method(&mut self) {
        self.inside_method = true;
    }

    pub fn exit_method(&mut self) {
        self.inside_method = false;
    }

    pub fn enter_namespace(&mut self, namespace: String) {
        self.current_namespace.push(namespace);
    }

    pub fn exit_namespace(&mut self) {
        self.current_namespace.pop();
    }
}

/// Access control checker
pub struct AccessChecker {
    context: AccessContext,
}

impl AccessChecker {
    pub fn new() -> Self {
        Self {
            context: AccessContext::new(),
        }
    }

    pub fn get_context(&self) -> &AccessContext {
        &self.context
    }

    pub fn get_context_mut(&mut self) -> &mut AccessContext {
        &mut self.context
    }

    /// Check if a member with given access modifier can be accessed from current context
    pub fn can_access_member(
        &self,
        member_class: &str,
        access_modifier: &AccessModifier,
        position: &Position,
    ) -> Result<(), TypeError> {
        self.can_access_member_with_type_env(member_class, access_modifier, position, None)
    }

    /// Check if a member with given access modifier can be accessed from current context
    /// with access to TypeEnvironment for inheritance checking
    pub fn can_access_member_with_type_env(
        &self,
        member_class: &str,
        access_modifier: &AccessModifier,
        position: &Position,
        type_env: Option<&TypeEnvironment>,
    ) -> Result<(), TypeError> {
        match access_modifier {
            AccessModifier::Public => {
                // Public members are always accessible
                Ok(())
            }
            AccessModifier::Private => {
                // Private members are accessible from:
                // 1. The same class
                // 2. Friend classes (if TypeEnvironment is available)
                if let Some(current_class) = &self.context.current_class {
                    if current_class == member_class {
                        // Same class - always allowed
                        Ok(())
                    } else if let Some(type_env) = type_env {
                        // Check if current class is a friend of the member class
                        if type_env.is_friend_class(member_class, current_class) {
                            Ok(())
                        } else {
                            Err(TypeError::new(
                                format!("Cannot access private member of class '{}' from class '{}'",
                                       member_class, current_class),
                                position.clone(),
                            ))
                        }
                    } else {
                        // No type environment available - fallback to old behavior
                        Err(TypeError::new(
                            format!("Cannot access private member of class '{}' from class '{}'",
                                   member_class, current_class),
                            position.clone(),
                        ))
                    }
                } else {
                    Err(TypeError::new(
                        format!("Cannot access private member of class '{}' from outside any class",
                               member_class),
                        position.clone(),
                    ))
                }
            }
            AccessModifier::Protected => {
                // Protected members are accessible from:
                // 1. The same class
                // 2. Derived classes (subclasses)
                // 3. Friend classes (if TypeEnvironment is available)
                if let Some(current_class) = &self.context.current_class {
                    if current_class == member_class {
                        // Same class - always allowed
                        Ok(())
                    } else {
                        // Check if current class is a subclass of member class or a friend class
                        if let Some(type_env) = type_env {
                            if type_env.is_subclass(current_class, member_class) {
                                // Current class is a subclass of member class - allowed
                                Ok(())
                            } else if type_env.is_friend_class(member_class, current_class) {
                                // Current class is a friend of member class - allowed
                                Ok(())
                            } else {
                                Err(TypeError::new(
                                    format!("Cannot access protected member of class '{}' from unrelated class '{}'",
                                           member_class, current_class),
                                    position.clone(),
                                ))
                            }
                        } else {
                            // Fallback to old logic if no type environment available
                            if let Some(base_class) = &self.context.inside_derived_class {
                                if base_class == member_class {
                                    Ok(())
                                } else {
                                    Err(TypeError::new(
                                        format!("Cannot access protected member of class '{}' from unrelated class '{}'",
                                               member_class, current_class),
                                        position.clone(),
                                    ))
                                }
                            } else {
                                Err(TypeError::new(
                                    format!("Cannot access protected member of class '{}' from unrelated class '{}'",
                                           member_class, current_class),
                                    position.clone(),
                                ))
                            }
                        }
                    }
                } else {
                    Err(TypeError::new(
                        format!("Cannot access protected member of class '{}' from outside any class",
                               member_class),
                        position.clone(),
                    ))
                }
            }
            AccessModifier::Package => {
                // Package members are accessible from:
                // 1. The same package (top-level namespace)
                // 2. Same class (if applicable)
                // 3. Friend classes (if TypeEnvironment is available)

                // First check if we're in the same class (always allowed)
                if let Some(current_class) = &self.context.current_class {
                    if current_class == member_class {
                        return Ok(());
                    }

                    // Check friend relationships if available
                    if let Some(type_env) = type_env {
                        if type_env.is_friend_class(member_class, current_class) {
                            return Ok(());
                        }
                    }
                }

                // Check package-level access
                let member_package = self.get_package_for_class(member_class, type_env);
                let current_package = &self.context.current_package;

                match (&member_package, current_package) {
                    (Some(member_pkg), Some(current_pkg)) => {
                        if member_pkg == current_pkg {
                            Ok(())
                        } else {
                            Err(TypeError::new(
                                format!("Cannot access package member of class '{}' from different package", member_class),
                                position.clone(),
                            ))
                        }
                    }
                    (None, None) => {
                        // Both are in the global package (no namespace)
                        Ok(())
                    }
                    _ => {
                        Err(TypeError::new(
                            format!("Cannot access package member of class '{}' from different package context", member_class),
                            position.clone(),
                        ))
                    }
                }
            }
        }
    }

    /// Check if a field can be accessed
    pub fn check_field_access(
        &self,
        field_class: &str,
        field_name: &str,
        access_modifier: &AccessModifier,
        position: &Position,
    ) -> Result<(), TypeError> {
        self.can_access_member(field_class, access_modifier, position)
            .map_err(|mut err| {
                err.message = format!("Cannot access field '{}': {}", field_name, err.message);
                err
            })
    }

    /// Check if a field can be accessed with TypeEnvironment for inheritance checking
    pub fn check_field_access_with_type_env(
        &self,
        field_class: &str,
        field_name: &str,
        access_modifier: &AccessModifier,
        position: &Position,
        type_env: &TypeEnvironment,
    ) -> Result<(), TypeError> {
        self.can_access_member_with_type_env(field_class, access_modifier, position, Some(type_env))
            .map_err(|mut err| {
                err.message = format!("Cannot access field '{}': {}", field_name, err.message);
                err
            })
    }

    /// Check if a method can be accessed
    pub fn check_method_access(
        &self,
        method_class: &str,
        method_name: &str,
        access_modifier: &AccessModifier,
        position: &Position,
    ) -> Result<(), TypeError> {
        self.can_access_member(method_class, access_modifier, position)
            .map_err(|mut err| {
                err.message = format!("Cannot access method '{}': {}", method_name, err.message);
                err
            })
    }

    /// Check if a method can be accessed with TypeEnvironment for inheritance checking
    pub fn check_method_access_with_type_env(
        &self,
        method_class: &str,
        method_name: &str,
        access_modifier: &AccessModifier,
        position: &Position,
        type_env: &TypeEnvironment,
    ) -> Result<(), TypeError> {
        self.can_access_member_with_type_env(method_class, access_modifier, position, Some(type_env))
            .map_err(|mut err| {
                err.message = format!("Cannot access method '{}': {}", method_name, err.message);
                err
            })
    }

    /// Check if a constructor can be accessed
    pub fn check_constructor_access(
        &self,
        constructor_class: &str,
        access_modifier: &AccessModifier,
        position: &Position,
    ) -> Result<(), TypeError> {
        self.can_access_member(constructor_class, access_modifier, position)
            .map_err(|mut err| {
                err.message = format!("Cannot access constructor of class '{}': {}", constructor_class, err.message);
                err
            })
    }

    /// Check if a static field can be accessed
    pub fn check_static_field_access(
        &self,
        field_class: &str,
        field_name: &str,
        access_modifier: &AccessModifier,
        position: &Position,
    ) -> Result<(), TypeError> {
        self.can_access_member(field_class, access_modifier, position)
            .map_err(|mut err| {
                err.message = format!("Cannot access static field '{}': {}", field_name, err.message);
                err
            })
    }

    /// Check if a static method can be accessed
    pub fn check_static_method_access(
        &self,
        method_class: &str,
        method_name: &str,
        access_modifier: &AccessModifier,
        position: &Position,
    ) -> Result<(), TypeError> {
        self.can_access_member(method_class, access_modifier, position)
            .map_err(|mut err| {
                err.message = format!("Cannot access static method '{}': {}", method_name, err.message);
                err
            })
    }

    /// Check if a static method can be accessed with TypeEnvironment for inheritance checking
    pub fn check_static_method_access_with_type_env(
        &self,
        method_class: &str,
        method_name: &str,
        access_modifier: &AccessModifier,
        position: &Position,
        type_env: &TypeEnvironment,
    ) -> Result<(), TypeError> {
        self.can_access_member_with_type_env(method_class, access_modifier, position, Some(type_env))
            .map_err(|mut err| {
                err.message = format!("Cannot access static method '{}': {}", method_name, err.message);
                err
            })
    }

    /// Check if a static field can be accessed with TypeEnvironment for inheritance checking
    pub fn check_static_field_access_with_type_env(
        &self,
        field_class: &str,
        field_name: &str,
        access_modifier: &AccessModifier,
        position: &Position,
        type_env: &TypeEnvironment,
    ) -> Result<(), TypeError> {
        self.can_access_member_with_type_env(field_class, access_modifier, position, Some(type_env))
            .map_err(|mut err| {
                err.message = format!("Cannot access static field '{}': {}", field_name, err.message);
                err
            })
    }

    /// Get the package (top-level namespace) for a given class
    fn get_package_for_class(&self, class_name: &str, type_env: Option<&TypeEnvironment>) -> Option<String> {
        // For now, we'll determine the package based on the current namespace context
        // In a more sophisticated implementation, we could store package information
        // with each class definition

        if let Some(_type_env) = type_env {
            // Try to find the class in the type environment and determine its package
            // For simplicity, we'll use the first part of the current namespace as the package
            if !self.context.current_namespace.is_empty() {
                Some(self.context.current_namespace[0].clone())
            } else {
                None // Global package
            }
        } else {
            // Fallback: use current package context
            self.context.current_package.clone()
        }
    }

    /// Set the current package context
    pub fn set_package_context(&mut self, package: Option<String>) {
        self.context.current_package = package;
    }

    /// Get the current package context
    pub fn get_package_context(&self) -> &Option<String> {
        &self.context.current_package
    }
}

impl Default for AccessChecker {
    fn default() -> Self {
        Self::new()
    }
}
