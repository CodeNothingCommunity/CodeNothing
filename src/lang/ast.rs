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
}

/// Unary operators
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Minus,
    Not,
    BitNot, // Bitwise NOT (~)
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
    pub type_annotation: TypeAnnotation,
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

/// Items that can appear in a namespace
#[derive(Debug, Clone, PartialEq)]
pub enum NamespaceItem {
    Function(FunctionDeclaration),
    Variable(Statement), // VarDeclaration
    Namespace(NamespaceDeclaration), // Nested namespace
}

/// Using declarations
#[derive(Debug, Clone, PartialEq)]
pub struct UsingDeclaration {
    pub namespace: String,
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
    Variable(Statement), // Global variable declaration
    Memo(MemoDeclaration), // 记忆化声明
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
            Declaration::Variable(stmt) => self.print_statement(stmt),
            Declaration::Memo(memo) => format!("memo {{ ... }}"), // 简化打印
        }
    }
    
    pub fn print_function(&mut self, function: &FunctionDeclaration) -> String {
        let params: Vec<String> = function.parameters.iter()
            .map(|p| format!("{} : {}", p.name, p.type_annotation))
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
