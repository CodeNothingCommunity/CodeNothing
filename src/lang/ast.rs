//! Abstract Syntax Tree definitions for CodeNothing language

use std::fmt;

/// Source position information
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Type annotations
#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    Int,
    Long,                  // 64-bit integer
    Float(Option<u32>),    // float(32) or just float
    String,                // String type
    Num(Option<u32>),      // num(16) or just num
    Bool,
    Array(Box<TypeAnnotation>),
    Obj,
    Auto,
    Null,
    Custom(String),        // User-defined types
    Void,                  // Void type for functions with no return value
    Enum(String),          // Enum type
    Trait(String),         // Trait type
    Generic {              // Generic type with parameters
        name: String,
        parameters: Vec<TypeAnnotation>,
    },
    TypeParameter(String), // Generic type parameter (e.g., T, U, K, V)
    Function {             // Function type for closures
        parameters: Vec<TypeAnnotation>,
        return_type: Box<TypeAnnotation>,
    },
    FunctionPointer {      // Function pointer type (fn(T1, T2) -> T3)
        parameters: Vec<TypeAnnotation>,
        return_type: Box<TypeAnnotation>,
    },

    // Pointer and reference types
    Reference {            // Immutable reference (&T)
        target_type: Box<TypeAnnotation>,
        lifetime: Option<String>, // Optional lifetime annotation
    },
    MutableReference {     // Mutable reference (&mut T)
        target_type: Box<TypeAnnotation>,
        lifetime: Option<String>, // Optional lifetime annotation
    },
    RawPointer {           // Raw pointer (*T or *mut T)
        target_type: Box<TypeAnnotation>,
        is_mutable: bool,  // *mut T vs *const T
    },
}

impl fmt::Display for TypeAnnotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeAnnotation::Int => write!(f, "int"),
            TypeAnnotation::Long => write!(f, "long"),
            TypeAnnotation::Float(Some(size)) => write!(f, "float({})", size),
            TypeAnnotation::Float(None) => write!(f, "float"),
            TypeAnnotation::String => write!(f, "string"),
            TypeAnnotation::Num(Some(size)) => write!(f, "num({})", size),
            TypeAnnotation::Num(None) => write!(f, "num"),
            TypeAnnotation::Bool => write!(f, "bool"),
            TypeAnnotation::Array(inner) => write!(f, "array<{}>", inner),
            TypeAnnotation::Obj => write!(f, "obj"),
            TypeAnnotation::Auto => write!(f, "auto"),
            TypeAnnotation::Null => write!(f, "null"),
            TypeAnnotation::Custom(name) => write!(f, "{}", name),
            TypeAnnotation::Void => write!(f, "void"),
            TypeAnnotation::Enum(name) => write!(f, "{}", name),
            TypeAnnotation::Trait(name) => write!(f, "{}", name),
            TypeAnnotation::Generic { name, parameters } => {
                write!(f, "{}<", name)?;
                for (i, param) in parameters.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", param)?;
                }
                write!(f, ">")
            },
            TypeAnnotation::Function { parameters, return_type } => {
                write!(f, "|")?;
                for (i, param) in parameters.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", param)?;
                }
                write!(f, "| -> {}", return_type)
            },
            TypeAnnotation::FunctionPointer { parameters, return_type } => {
                write!(f, "fn(")?;
                for (i, param) in parameters.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", return_type)
            },
            TypeAnnotation::TypeParameter(name) => write!(f, "{}", name),
            TypeAnnotation::Reference { target_type, lifetime } => {
                if let Some(lifetime) = lifetime {
                    write!(f, "&'{} {}", lifetime, target_type)
                } else {
                    write!(f, "&{}", target_type)
                }
            }
            TypeAnnotation::MutableReference { target_type, lifetime } => {
                if let Some(lifetime) = lifetime {
                    write!(f, "&'{}mut {}", lifetime, target_type)
                } else {
                    write!(f, "&mut {}", target_type)
                }
            }
            TypeAnnotation::RawPointer { target_type, is_mutable } => {
                if *is_mutable {
                    write!(f, "*mut {}", target_type)
                } else {
                    write!(f, "*const {}", target_type)
                }
            }
        }
    }
}

/// Variable declaration scope
#[derive(Debug, Clone, PartialEq)]
pub enum VarScope {
    Var,    // Function scope (can redeclare)
    Let,    // Block scope (cannot redeclare)
    Const,  // Constant (cannot change)
    Global, // Global scope (cannot redeclare)
}

/// Expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    // Literals
    Integer(i64, Position),
    Float(f64, Position),
    String(String, Position),
    InterpolatedString(Vec<crate::lang::lexer::InterpolationPart>, Position), // 字符串插值
    Boolean(bool, Position),
    Null(Position),
    Array(Vec<Expression>, Position), // Array literal
    Object(Vec<(String, Expression)>, Position), // Object literal: {key: value, ...}

    // Identifiers
    Identifier(String, Position),
    
    // Binary operations
    Binary {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
        position: Position,
    },
    
    // Unary operations
    Unary {
        operator: UnaryOperator,
        operand: Box<Expression>,
        position: Position,
    },

    // Pointer operations
    AddressOf {            // Take address (&expr)
        operand: Box<Expression>,
        is_mutable: bool,  // &mut expr vs &expr
        position: Position,
    },
    Dereference {          // Dereference (*expr)
        operand: Box<Expression>,
        position: Position,
    },
    
    // Function calls
    Call {
        function: Box<Expression>,
        arguments: Vec<Expression>,
        position: Position,
    },
    
    // Namespace access
    NamespaceAccess {
        namespace_path: Vec<String>, // Support nested namespaces like ["ns1", "ns2"]
        name: String,
        position: Position,
    },
    
    // Array access
    ArrayAccess {
        array: Box<Expression>,
        index: Box<Expression>,
        position: Position,
    },
    
    // Object field access
    FieldAccess {
        object: Box<Expression>,
        field: String,
        position: Position,
    },

    // Range expression
    Range {
        start: Box<Expression>,
        end: Box<Expression>,
        inclusive: bool, // true for ..=, false for ..
        position: Position,
    },

    // Lambda/Closure expression
    Lambda {
        parameters: Vec<Parameter>,
        body: Box<Expression>,
        position: Position,
    },

    // Match expression
    Match {
        expression: Box<Expression>,
        arms: Vec<MatchArm>,
        position: Position,
    },

    // Ternary conditional expression (condition ? true_expr : false_expr)
    Ternary {
        condition: Box<Expression>,
        true_expr: Box<Expression>,
        false_expr: Box<Expression>,
        position: Position,
    },

    // Null coalescing expression (left ?? right)
    NullCoalescing {
        left: Box<Expression>,
        right: Box<Expression>,
        position: Position,
    },

    // When expression (类似 match 但语法不同)
    When {
        arms: Vec<WhenArm>,
        else_expr: Option<Box<Expression>>,
        position: Position,
    },

    // Arrow function expression (x, y) => x + y
    ArrowFunction {
        parameters: Vec<Parameter>,
        return_type: Option<TypeAnnotation>,
        body: ArrowFunctionBody,
        position: Position,
    },

    // Enum variant construction
    EnumVariant {
        enum_name: String,
        variant: String,
        value: Option<Box<Expression>>,
        position: Position,
    },

    // Class instantiation
    ClassInstantiation {
        class_name: String,
        arguments: Vec<Expression>,
        position: Position,
    },

    // Super method call
    SuperCall {
        method_name: String,
        arguments: Vec<Expression>,
        position: Position,
    },

    // Super field access
    SuperFieldAccess {
        field_name: String,
        position: Position,
    },

    // Method call on object
    MethodCall {
        object: Box<Expression>,
        method_name: String,
        arguments: Vec<Expression>,
        position: Position,
    },

    // Static method call (ClassName::method_name)
    StaticMethodCall {
        class_name: String,
        method_name: String,
        arguments: Vec<Expression>,
        position: Position,
    },

    // Static field access (ClassName::field_name)
    StaticFieldAccess {
        class_name: String,
        field_name: String,
        position: Position,
    },
}

impl Expression {
    pub fn position(&self) -> &Position {
        match self {
            Expression::Integer(_, pos) => pos,
            Expression::Float(_, pos) => pos,
            Expression::String(_, pos) => pos,
            Expression::Boolean(_, pos) => pos,
            Expression::Null(pos) => pos,
            Expression::Array(_, pos) => pos,
            Expression::Object(_, pos) => pos,
            Expression::Identifier(_, pos) => pos,
            Expression::Binary { position, .. } => position,
            Expression::Unary { position, .. } => position,
            Expression::Call { position, .. } => position,
            Expression::NamespaceAccess { position, .. } => position,
            Expression::ArrayAccess { position, .. } => position,
            Expression::FieldAccess { position, .. } => position,
            Expression::Range { position, .. } => position,
            Expression::Lambda { position, .. } => position,
            Expression::Match { position, .. } => position,
            Expression::Ternary { position, .. } => position,
            Expression::EnumVariant { position, .. } => position,
            Expression::ClassInstantiation { position, .. } => position,
            Expression::SuperCall { position, .. } => position,
            Expression::SuperFieldAccess { position, .. } => position,
            Expression::MethodCall { position, .. } => position,
            Expression::StaticMethodCall { position, .. } => position,
            Expression::StaticFieldAccess { position, .. } => position,
            Expression::InterpolatedString(_, position) => position,
            Expression::NullCoalescing { position, .. } => position,
            Expression::When { position, .. } => position,
            Expression::ArrowFunction { position, .. } => position,
            Expression::AddressOf { position, .. } => position,
            Expression::Dereference { position, .. } => position,
        }
    }
}

/// Binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    
    // Comparison
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    
    // Logical
    And,
    Or,

    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    LeftShift,
    RightShift,

    // Assignment
    Assign,

    // Compound assignment
    AddAssign,      // +=
    SubtractAssign, // -=
    MultiplyAssign, // *=
    DivideAssign,   // /=
    ModuloAssign,   // %=

    // Function composition
    PipeForward,    // |> (pipe operator)
}

/// Unary operators
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Minus,
    Not,
    BitNot, // Bitwise NOT (~)

    // Increment/Decrement
    PreIncrement,   // ++x
    PostIncrement,  // x++
    PreDecrement,   // --x
    PostDecrement,  // x--
}

/// Statements
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    // Variable declarations
    VarDeclaration {
        scope: VarScope,
        name: String,
        type_annotation: Option<TypeAnnotation>,
        initializer: Option<Expression>,
        position: Position,
    },
    
    // Expression statements
    Expression(Expression),
    
    // Return statements
    Return {
        value: Option<Expression>,
        position: Position,
    },
    
    // If statements
    If {
        condition: Expression,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
        position: Position,
    },
    
    // While loops
    While {
        condition: Expression,
        body: Box<Statement>,
        position: Position,
    },
    
    // For loops
    For {
        initializer: Option<Box<Statement>>,
        condition: Option<Expression>,
        increment: Option<Expression>,
        body: Box<Statement>,
        position: Position,
    },

    // Range-based for loops: for (variable : start...end) { ... }
    ForRange {
        variable: String,
        range: Expression, // Should be a Range expression
        body: Box<Statement>,
        position: Position,
    },

    // For-in loops: for (variable in iterable) { ... }
    ForIn {
        variable: String,
        iterable: Expression,
        body: Box<Statement>,
        position: Position,
    },

    // Do-while loops: do { ... } while (condition);
    DoWhile {
        body: Box<Statement>,
        condition: Expression,
        position: Position,
    },


    
    // Block statements
    Block {
        statements: Vec<Statement>,
        position: Position,
    },
    
    // Break and continue
    Break(Position),
    Continue(Position),

    // Switch statement
    Switch {
        expression: Expression,
        cases: Vec<CaseClause>,
        default_case: Option<Box<Statement>>,
        position: Position,
    },

    // Exception handling
    Try {
        try_block: Box<Statement>,
        catch_clauses: Vec<CatchClause>,
        finally_block: Option<Box<Statement>>,
        position: Position,
    },

    // Throw statement
    Throw {
        value: Expression,
        position: Position,
    },

    // Library import statements
    LibImport {
        library_name: String,
        import_type: LibImportType, // System library (<>) or local library ("")
        position: Position,
    },
}

/// Library import type
#[derive(Debug, Clone, PartialEq)]
pub enum LibImportType {
    System,  // lib <library_name>; - from interpreter directory
    Local,   // lib "library_name"; - from current directory
}

impl Statement {
    pub fn position(&self) -> &Position {
        match self {
            Statement::VarDeclaration { position, .. } => position,
            Statement::Expression(expr) => expr.position(),
            Statement::Return { position, .. } => position,
            Statement::If { position, .. } => position,
            Statement::While { position, .. } => position,
            Statement::For { position, .. } => position,
            Statement::Block { position, .. } => position,
            Statement::Break(position) => position,
            Statement::Continue(position) => position,
            Statement::Switch { position, .. } => position,
            Statement::Try { position, .. } => position,
            Statement::Throw { position, .. } => position,
            Statement::ForRange { position, .. } => position,
            Statement::ForIn { position, .. } => position,
            Statement::DoWhile { position, .. } => position,
            Statement::LibImport { position, .. } => position,
        }
    }
}

/// Catch clause for exception handling
#[derive(Debug, Clone, PartialEq)]
pub struct CatchClause {
    pub exception_variable: Option<String>, // Variable to bind the exception to
    pub exception_type: Option<String>,     // Type of exception to catch (None = catch all)
    pub body: Box<Statement>,
    pub position: Position,
}

/// Case clause for switch statements
#[derive(Debug, Clone, PartialEq)]
pub struct CaseClause {
    pub value: Expression,
    pub body: Statement,
    pub position: Position,
}

/// Function parameters
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<TypeAnnotation>,
    pub position: Position,
}

/// Function declarations
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDeclaration {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Statement,
    pub position: Position,
}

/// Namespace declarations
#[derive(Debug, Clone, PartialEq)]
pub struct NamespaceDeclaration {
    pub name: String,
    pub items: Vec<NamespaceItem>,
    pub position: Position,
}

/// Enum variant
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub value: Option<Expression>, // Optional associated value
    pub position: Position,
}

/// Enum declarations
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDeclaration {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub position: Position,
}

/// Struct field
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub type_annotation: TypeAnnotation,
    pub position: Position,
}

/// Struct declarations
#[derive(Debug, Clone, PartialEq)]
pub struct StructDeclaration {
    pub name: String,
    pub fields: Vec<StructField>,
    pub position: Position,
}

/// Class field with access modifier
#[derive(Debug, Clone, PartialEq)]
pub struct ClassField {
    pub name: String,
    pub type_annotation: TypeAnnotation,
    pub access_modifier: AccessModifier,
    pub is_static: bool,
    pub initializer: Option<Expression>,  // Field initialization expression
    pub position: Position,
}

/// Access modifiers for class members
#[derive(Debug, Clone, PartialEq)]
pub enum AccessModifier {
    Public,
    Private,
    Protected,
    Package,  // Package-level access (visible within the same package/namespace)
}

/// Constructor declaration
#[derive(Debug, Clone, PartialEq)]
pub struct Constructor {
    pub parameters: Vec<Parameter>,
    pub body: Statement,
    pub access_modifier: AccessModifier,
    pub position: Position,
}

/// Destructor declaration
#[derive(Debug, Clone, PartialEq)]
pub struct Destructor {
    pub body: Statement,
    pub position: Position,
}

/// Class method with access modifier
#[derive(Debug, Clone, PartialEq)]
pub struct ClassMethod {
    pub function: FunctionDeclaration,
    pub access_modifier: AccessModifier,
    pub is_static: bool,
    pub is_virtual: bool,
    pub is_override: bool,  // 是否重写父类方法
    pub is_abstract: bool,  // 是否为抽象方法
    pub position: Position,
}

/// Generic parameter for classes and functions
#[derive(Debug, Clone, PartialEq)]
pub struct GenericParameter {
    pub name: String,
    pub bounds: Vec<String>,  // 类型约束，如 T: Drawable + Movable
    pub position: Position,
}

/// Class declarations
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDeclaration {
    pub name: String,
    pub generic_parameters: Vec<GenericParameter>,  // 泛型参数
    pub base_class: Option<String>,  // 父类名称
    pub implemented_traits: Vec<String>,  // 实现的接口/trait列表
    pub is_abstract: bool,  // 是否为抽象类
    pub friend_classes: Vec<String>,  // 友元类列表
    pub fields: Vec<ClassField>,
    pub constructor: Option<Constructor>,
    pub destructor: Option<Destructor>,
    pub methods: Vec<ClassMethod>,
    pub nested_classes: Vec<ClassDeclaration>,  // 内部类
    pub position: Position,
}

/// Trait method signature
#[derive(Debug, Clone, PartialEq)]
pub struct TraitMethod {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<TypeAnnotation>,
    pub default_body: Option<Statement>, // Optional default implementation
    pub is_required: bool,  // 是否必须实现（没有默认实现时为true）
    pub position: Position,
}

/// Trait declarations (interfaces)
#[derive(Debug, Clone, PartialEq)]
pub struct TraitDeclaration {
    pub name: String,
    pub super_traits: Vec<String>,  // 继承的父接口
    pub methods: Vec<TraitMethod>,
    pub default_methods: Vec<TraitMethod>,  // 带默认实现的方法
    pub associated_types: Vec<String>,  // 关联类型
    pub position: Position,
}

/// Implementation declarations
#[derive(Debug, Clone, PartialEq)]
pub struct ImplDeclaration {
    pub trait_name: Option<String>, // None for inherent impl, Some for trait impl
    pub type_name: String,
    pub methods: Vec<FunctionDeclaration>,
    pub position: Position,
}

/// Match arm for pattern matching
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expression>, // Optional guard condition
    pub body: Expression,
    pub position: Position,
}

/// When arm for when expressions (条件 => 表达式)
#[derive(Debug, Clone, PartialEq)]
pub struct WhenArm {
    pub condition: Expression,  // 条件表达式
    pub body: Expression,       // 结果表达式
    pub position: Position,
}

/// Arrow function body - either expression or block
#[derive(Debug, Clone, PartialEq)]
pub enum ArrowFunctionBody {
    Expression(Box<Expression>),  // x => x + 1
    Block(Vec<Statement>),        // x => { return x + 1; }
}

/// Patterns for match expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard(Position),                    // _
    Literal(Expression),                   // 42, "hello", true
    Identifier(String, Position),          // x
    EnumVariant {                         // Color::Red(value)
        enum_name: String,
        variant: String,
        value: Option<Box<Pattern>>,
        position: Position,
    },
    Tuple(Vec<Pattern>, Position),        // (a, b, c)
    Array(Vec<Pattern>, Position),        // [a, b, c]
}

/// Items that can appear in a namespace
#[derive(Debug, Clone, PartialEq)]
pub enum NamespaceItem {
    Function(FunctionDeclaration),
    Variable(Statement), // VarDeclaration
    Namespace(NamespaceDeclaration), // Nested namespace
    Enum(EnumDeclaration),
    Struct(StructDeclaration),
    Class(ClassDeclaration),
    Trait(TraitDeclaration),
    Impl(ImplDeclaration),
}

/// Using declarations
#[derive(Debug, Clone, PartialEq)]
pub struct UsingDeclaration {
    pub namespace: String,
    pub position: Position,
}

/// Include path specification
#[derive(Debug, Clone, PartialEq)]
pub enum IncludePath {
    /// Simple file path: "file.cn"
    Simple(String),
    /// Wildcard path: "utils/*.cn" or "**/*.cn"
    Wildcard(String),
    /// Search path: <std/io> or <math>
    Search(String),
}

/// Import specification for selective includes
#[derive(Debug, Clone, PartialEq)]
pub enum ImportSpec {
    /// Import specific symbol: func1
    Symbol(String),
    /// Import with alias: func1 as my_func
    Aliased { name: String, alias: String },
    /// Import all: *
    All,
}

/// Include declarations
#[derive(Debug, Clone, PartialEq)]
pub struct IncludeDeclaration {
    pub path: IncludePath,
    pub alias: Option<String>,
    pub selective_imports: Option<Vec<ImportSpec>>, // For selective imports: include "file.cn" { func1, Class1 as MyClass }
    pub search_paths: Vec<String>, // Additional search paths for resolution
    pub position: Position,
}

/// Package path specification for imports
#[derive(Debug, Clone, PartialEq)]
pub enum PackagePath {
    /// Absolute path: std::io
    Absolute(Vec<String>),
    /// Relative path: ./utils, ../common
    Relative(Vec<String>),
    /// Local path: math (in current project)
    Local(String),
}

/// Package declaration
#[derive(Debug, Clone, PartialEq)]
pub struct PackageDeclaration {
    pub name: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub dependencies: Vec<String>,
    pub position: Position,
}

/// Import declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ImportDeclaration {
    pub package_path: PackagePath,
    pub alias: Option<String>,
    pub selective_imports: Option<Vec<ImportSpec>>, // Reuse ImportSpec from include system
    pub position: Position,
}

/// Export declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ExportDeclaration {
    pub declaration: Box<Declaration>,
    pub position: Position,
}

/// Memo declaration parameters
#[derive(Debug, Clone, PartialEq)]
pub struct MemoParams {
    pub max_size: Option<i64>, // 最大缓存大小
    pub ttl: Option<i64>,      // 生存时间（秒）
}

/// Memo declaration
#[derive(Debug, Clone, PartialEq)]
pub struct MemoDeclaration {
    pub params: Option<MemoParams>,
    pub content: MemoContent,
    pub position: Position,
}

/// Memo content can be functions or namespaces
#[derive(Debug, Clone, PartialEq)]
pub enum MemoContent {
    Function(FunctionDeclaration),
    Namespace(NamespaceDeclaration),
    Block(Vec<Declaration>), // 多个声明的块
}

/// Top-level declarations
#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    Function(FunctionDeclaration),
    Namespace(NamespaceDeclaration),
    Using(UsingDeclaration),
    Include(IncludeDeclaration), // Include declaration
    Package(PackageDeclaration), // Package declaration
    Import(ImportDeclaration), // Import declaration
    Export(ExportDeclaration), // Export declaration
    Variable(Statement), // Global variable declaration
    Memo(MemoDeclaration), // 记忆化声明
    Enum(EnumDeclaration),
    Struct(StructDeclaration),
    Class(ClassDeclaration),
    Trait(TraitDeclaration),
    Impl(ImplDeclaration),
    LibImport(Statement), // Library import statement
}

/// Complete program
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub declarations: Vec<Declaration>,
}

impl Program {
    pub fn new() -> Self {
        Self {
            declarations: Vec::new(),
        }
    }
    
    pub fn add_declaration(&mut self, declaration: Declaration) {
        self.declarations.push(declaration);
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

/// Visitor trait for AST traversal
pub trait AstVisitor<T> {
    fn visit_program(&mut self, program: &Program) -> T;
    fn visit_declaration(&mut self, declaration: &Declaration) -> T;
    fn visit_function(&mut self, function: &FunctionDeclaration) -> T;
    fn visit_namespace(&mut self, namespace: &NamespaceDeclaration) -> T;
    fn visit_using(&mut self, using: &UsingDeclaration) -> T;
    fn visit_statement(&mut self, statement: &Statement) -> T;
    fn visit_expression(&mut self, expression: &Expression) -> T;
}

/// Pretty printer for AST
pub struct AstPrinter {
    indent: usize,
}

impl AstPrinter {
    pub fn new() -> Self {
        Self { indent: 0 }
    }
    
    fn indent_str(&self) -> String {
        "  ".repeat(self.indent)
    }
    
    fn with_indent<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        self.indent += 1;
        let result = f(self);
        self.indent -= 1;
        result
    }
    
    pub fn print_program(&mut self, program: &Program) -> String {
        let mut result = String::new();
        result.push_str("Program:\n");
        
        for declaration in &program.declarations {
            result.push_str(&self.with_indent(|printer| {
                format!("{}{}\n", printer.indent_str(), printer.print_declaration(declaration))
            }));
        }
        
        result
    }
    
    pub fn print_declaration(&mut self, declaration: &Declaration) -> String {
        match declaration {
            Declaration::Function(func) => self.print_function(func),
            Declaration::Namespace(ns) => self.print_namespace(ns),
            Declaration::Using(using) => format!("using ns {};", using.namespace),
            Declaration::Include(include) => {
                let mut result = String::from("include ");

                // Print path
                match &include.path {
                    IncludePath::Simple(path) => result.push_str(&format!("\"{}\"", path)),
                    IncludePath::Wildcard(pattern) => result.push_str(&format!("\"{}\"", pattern)),
                    IncludePath::Search(name) => result.push_str(&format!("<{}>", name)),
                }

                // Print alias
                if let Some(alias) = &include.alias {
                    result.push_str(&format!(" as {}", alias));
                }

                // Print selective imports
                if let Some(imports) = &include.selective_imports {
                    let import_strs: Vec<String> = imports.iter().map(|spec| {
                        match spec {
                            ImportSpec::Symbol(name) => name.clone(),
                            ImportSpec::Aliased { name, alias } => format!("{} as {}", name, alias),
                            ImportSpec::All => "*".to_string(),
                        }
                    }).collect();
                    result.push_str(&format!(" {{ {} }}", import_strs.join(", ")));
                }

                result.push(';');
                result
            },
            Declaration::Variable(stmt) => self.print_statement(stmt),
            Declaration::Memo(memo) => format!("memo {{ ... }}"), // 简化打印
            Declaration::Enum(enum_decl) => format!("enum {} {{ ... }}", enum_decl.name),
            Declaration::Struct(struct_decl) => format!("struct {} {{ ... }}", struct_decl.name),
            Declaration::Class(class_decl) => format!("class {} {{ ... }}", class_decl.name),
            Declaration::Trait(trait_decl) => format!("trait {} {{ ... }}", trait_decl.name),
            Declaration::Impl(impl_decl) => format!("impl {} {{ ... }}", impl_decl.type_name),
            Declaration::LibImport(stmt) => self.print_statement(stmt),
            Declaration::Package(package) => format!("package {} version {:?};", package.name, package.version),
            Declaration::Import(import) => format!("import {:?};", import.package_path),
            Declaration::Export(export) => format!("export {};", self.print_declaration(&export.declaration)),
        }
    }
    
    pub fn print_function(&mut self, function: &FunctionDeclaration) -> String {
        let params: Vec<String> = function.parameters.iter()
            .map(|p| {
                if let Some(type_annotation) = &p.type_annotation {
                    format!("{} : {}", p.name, type_annotation)
                } else {
                    p.name.clone()
                }
            })
            .collect();
        
        let return_type = function.return_type.as_ref()
            .map(|t| format!(" : {}", t))
            .unwrap_or_default();
        
        format!("fn {}({}){} {{ ... }}", 
                function.name, 
                params.join(", "), 
                return_type)
    }
    
    pub fn print_namespace(&mut self, namespace: &NamespaceDeclaration) -> String {
        format!("ns {} {{ {} items }}", namespace.name, namespace.items.len())
    }
    
    pub fn print_statement(&mut self, statement: &Statement) -> String {
        match statement {
            Statement::VarDeclaration { scope, name, type_annotation, .. } => {
                let scope_str = match scope {
                    VarScope::Var => "var",
                    VarScope::Let => "let",
                    VarScope::Const => "const",
                    VarScope::Global => "",
                };
                let type_str = type_annotation.as_ref()
                    .map(|t| format!(" : {}", t))
                    .unwrap_or_default();
                format!("{} {}{}", scope_str, name, type_str)
            }
            Statement::Return { .. } => "return".to_string(),
            Statement::Block { statements, .. } => format!("{{ {} statements }}", statements.len()),
            Statement::Try { catch_clauses, finally_block, .. } => {
                let catch_count = catch_clauses.len();
                let finally_str = if finally_block.is_some() { " with finally" } else { "" };
                format!("try with {} catch clause(s){}", catch_count, finally_str)
            }
            Statement::Throw { .. } => "throw".to_string(),
            _ => "statement".to_string(),
        }
    }
}

impl Default for AstPrinter {
    fn default() -> Self {
        Self::new()
    }
}
