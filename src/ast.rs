#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Long,
    Void,    // 添加void类型
    Auto,    // 新增：自动类型推断（弱类型）
    Array(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Exception, // 新增：异常类型
    Class(String), // 新增：类类型
    Function(Vec<Type>, Box<Type>), // 新增：函数类型 (参数类型列表, 返回类型)
    Enum(String), // 新增：枚举类型
    Pointer(Box<Type>), // 新增：指针类型 (*Type)
    OptionalPointer(Box<Type>), // 新增：可选指针类型 (?*Type)
    FunctionPointer(Vec<Type>, Box<Type>), // 新增：函数指针类型 (*fn(args) : ret)
    ArrayPointer(Box<Type>, usize), // 新增：数组指针类型 (*[size]Type)
    PointerArray(Box<Type>, usize), // 新增：指针数组类型 ([size]*Type)

    // 泛型类型支持
    Generic(String), // 泛型类型参数 (T, U, K, V等)
    GenericClass(String, Vec<Type>), // 泛型类实例化 (Container<T>, Map<K,V>)
    GenericEnum(String, Vec<Type>), // 泛型枚举实例化 (Option<T>, Result<T,E>)
    GenericFunction(String, Vec<Type>), // 泛型函数实例化 (max<i32>, sort<String>)

    // 未来可以扩展更多类型
}

// 泛型参数定义
#[derive(Debug, Clone, PartialEq)]
pub struct GenericParameter {
    pub name: String,                    // 类型参数名 (T, U, K, V等)
    pub constraints: Vec<TypeConstraint>, // 类型约束列表
    pub default_type: Option<Type>,      // 默认类型 (可选)
}

// 类型约束
#[derive(Debug, Clone, PartialEq)]
pub enum TypeConstraint {
    Trait(String),                       // trait约束 (Comparable, Clone等)
    Lifetime(String),                    // 生命周期约束 (暂时保留)
    Sized,                              // 大小约束
    Copy,                               // 复制约束
    Send,                               // 发送约束 (多线程)
    Sync,                               // 同步约束 (多线程)
}

// 泛型实例化信息
#[derive(Debug, Clone, PartialEq)]
pub struct GenericInstantiation {
    pub type_arguments: Vec<Type>,       // 类型参数实例化
    pub inferred: bool,                  // 是否为类型推导得出
}

#[derive(Debug, Clone)]
pub enum Expression {
    IntLiteral(i32),
    FloatLiteral(f64),
    BoolLiteral(bool),
    StringLiteral(String),
    RawStringLiteral(String), // 新增：原始字符串字面量
    LongLiteral(i64),
    ArrayLiteral(Vec<Expression>),
    ArrayAccess(Box<Expression>, Box<Expression>), // 数组索引访问 array[index]
    MapLiteral(Vec<(Expression, Expression)>),
    FunctionCall(String, Vec<Expression>),
    FunctionPointerCall(Box<Expression>, Vec<Expression>), // 函数指针调用 func_ptr(args)
    NamespacedFunctionCall(Vec<String>, Vec<Expression>), // 命名空间函数调用
    GlobalFunctionCall(String, Vec<Expression>), // 全局函数明确调用 (::func)
    LibraryFunctionCall(String, String, Vec<Expression>), // 库函数调用 (lib::func)
    Variable(String),
    BinaryOp(Box<Expression>, BinaryOperator, Box<Expression>),
    CompareOp(Box<Expression>, CompareOperator, Box<Expression>), // 比较操作
    LogicalOp(Box<Expression>, LogicalOperator, Box<Expression>), // 逻辑操作
    PreIncrement(String),  // 前置自增 (++var)
    PreDecrement(String),  // 前置自减 (--var)
    PostIncrement(String), // 后置自增 (var++)
    PostDecrement(String), // 后置自减 (var--)
    TernaryOp(Box<Expression>, Box<Expression>, Box<Expression>), // 三元条件运算符 (cond ? expr1 : expr2)
    Throw(Box<Expression>), // 新增：抛出异常
    // 链式调用相关
    MethodCall(Box<Expression>, String, Vec<Expression>), // 方法调用 (obj.method(args))
    ChainCall(Box<Expression>, Vec<(String, Vec<Expression>)>), // 链式调用 (obj.method1().method2())
    // OOP相关表达式
    ObjectCreation(String, Vec<Expression>), // 对象创建 (new ClassName(args))
    FieldAccess(Box<Expression>, String), // 字段访问 (obj.field)
    This, // this 关键字
    Super, // super 关键字
    SuperCall(Vec<Expression>), // super() 构造函数调用
    SuperMethodCall(String, Vec<Expression>), // super.methodName() 父类方法调用
    StaticAccess(String, String), // 静态访问 (ClassName::member)
    StaticMethodCall(String, String, Vec<Expression>), // 静态方法调用 (ClassName::method(args))
    // Lambda表达式和函数式编程
    Lambda(Vec<Parameter>, Box<Expression>), // Lambda表达式 (参数列表, 表达式体)
    LambdaBlock(Vec<Parameter>, Vec<Statement>), // Lambda块 (参数列表, 语句块)
    FunctionValue(String), // 函数值引用 (函数名)
    Apply(Box<Expression>, Vec<Expression>), // 函数应用 (函数表达式, 参数列表)
    // 高阶函数调用
    ArrayMap(Box<Expression>, Box<Expression>), // array.map(lambda)
    ArrayFilter(Box<Expression>, Box<Expression>), // array.filter(lambda)
    ArrayReduce(Box<Expression>, Box<Expression>, Box<Expression>), // array.reduce(lambda, initial)
    ArrayForEach(Box<Expression>, Box<Expression>), // array.forEach(lambda)
    // Switch 表达式
    SwitchExpression(Box<Expression>, Vec<SwitchCase>, Option<Box<Expression>>), // switch表达式：表达式、case列表、default表达式
    // 字符串插值
    StringInterpolation(Vec<StringInterpolationSegment>), // 字符串插值表达式
    // 模式匹配表达式
    MatchExpression(Box<Expression>, Vec<MatchArm>), // match表达式：匹配表达式和匹配分支列表
    // Enum 相关表达式
    EnumVariantCreation(String, String, Vec<Expression>), // 枚举变体创建 (枚举名, 变体名, 参数)
    EnumVariantAccess(String, String), // 枚举变体访问 (枚举名::变体名)
    // Pointer 相关表达式
    AddressOf(Box<Expression>), // 取地址操作 (&expression)
    Dereference(Box<Expression>), // 解引用操作 (*expression)
    PointerArithmetic(Box<Expression>, PointerArithmeticOp, Box<Expression>), // 指针算术
    PointerMemberAccess(Box<Expression>, String), // 指针成员访问 (ptr->member 或 ptr.member)
    ArrayPointerAccess(Box<Expression>, Box<Expression>), // 数组指针访问 ((*arrayPtr)[index])
    PointerArrayAccess(Box<Expression>, Box<Expression>), // 指针数组访问 (ptrArray[index])

    // 函数指针相关表达式
    FunctionReference(String), // 函数引用 (函数名)
    LambdaFunction(Vec<Parameter>, Box<Type>, Box<Statement>), // Lambda函数表达式

    // 泛型相关表达式
    GenericFunctionCall(String, Vec<Type>, Vec<Expression>), // 泛型函数调用 (函数名, 类型参数, 参数)
    GenericMethodCall(Box<Expression>, String, Vec<Type>, Vec<Expression>), // 泛型方法调用
    GenericObjectCreation(String, Vec<Type>, Vec<Expression>), // 泛型对象创建
    TypeCast(Box<Expression>, Type), // 类型转换 (expression as Type)
    TypeOf(Box<Expression>), // 类型查询 (typeof(expression))

    None, // 空表达式（用于未初始化的变量）
    // 未来可以扩展更多表达式类型
}

// 字符串插值片段
#[derive(Debug, Clone)]
pub enum StringInterpolationSegment {
    Text(String),                 // 普通文本
    Expression(Box<Expression>),  // 插入的表达式
}

// 模式匹配分支
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,                    // 匹配模式
    pub guard: Option<Expression>,           // 可选的守卫条件 (if condition)
    pub body: Vec<Statement>,                // 匹配成功时执行的语句块
}

// 模式定义
#[derive(Debug, Clone)]
pub enum Pattern {
    // 字面量模式
    IntLiteral(i32),                         // 整数字面量模式
    FloatLiteral(f64),                       // 浮点数字面量模式
    BoolLiteral(bool),                       // 布尔字面量模式
    StringLiteral(String),                   // 字符串字面量模式

    // 变量模式
    Variable(String),                        // 变量绑定模式 (x)
    Wildcard,                               // 通配符模式 (_)

    // 复合模式
    Tuple(Vec<Pattern>),                     // 元组模式 (x, y, z)
    Array(Vec<Pattern>),                     // 数组模式 [x, y, z]

    // 范围模式
    Range(Box<Pattern>, Box<Pattern>),       // 范围模式 (1..10)

    // 多选模式
    Or(Vec<Pattern>),                        // 或模式 (1 | 2 | 3)

    // 结构化模式
    Struct(String, Vec<(String, Pattern)>),  // 结构体模式 Person { name, age }

    // 枚举模式
    EnumVariant(String, String, Vec<Pattern>), // 枚举变体模式 Option::Some(x)

    // 引用模式
    Reference(Box<Pattern>),                 // 引用模式 &pattern

    // 守卫模式 (在MatchArm中处理)
}

// 命名空间类型
#[derive(Debug, Clone, PartialEq)]
pub enum NamespaceType {
    Code,    // 代码命名空间 (ns xxx)
    Library, // 库命名空间 (lib xxx)
}

#[derive(Debug, Clone)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    // 位运算操作符 - v0.7.2新增
    BitwiseAnd,    // &
    BitwiseOr,     // |
    BitwiseXor,    // ^
    LeftShift,     // <<
    RightShift,    // >>
}

#[derive(Debug, Clone)]
pub enum CompareOperator {
    Equal,        // ==
    NotEqual,     // !=
    Greater,      // >
    Less,         // <
    GreaterEqual, // >=
    LessEqual,    // <=
}

#[derive(Debug, Clone)]
pub enum LogicalOperator {
    And,  // &&
    Or,   // ||
    Not,  // !
}

#[derive(Debug, Clone, PartialEq)]
pub enum PointerArithmeticOp {
    Add,    // ptr + offset
    Sub,    // ptr - offset
    Diff,   // ptr1 - ptr2
}

#[derive(Debug, Clone)]
pub enum Statement {
    Return(Option<Expression>),
    VariableDeclaration(String, Type, Expression),
    ConstantDeclaration(String, Type, Expression), // 新增：常量声明
    VariableAssignment(String, Expression),
    Increment(String), // 后置自增语句 (var++)
    Decrement(String), // 后置自减语句 (var--)
    PreIncrement(String), // 前置自增语句 (++var)
    PreDecrement(String), // 前置自减语句 (--var)
    CompoundAssignment(String, BinaryOperator, Expression), // 复合赋值 (+=, -=, *=, /=, %=)
    ImportNamespace(NamespaceType, Vec<String>), // 统一的命名空间导入，第一个参数表示类型，第二个参数是路径
    FileImport(String),    // 导入文件 (using file "xxx.cn";)
    FunctionCallStatement(Expression), // 函数调用语句
    NamespacedFunctionCallStatement(Vec<String>, Vec<Expression>), // 命名空间函数调用语句 (ns::func())
    LibraryFunctionCallStatement(String, String, Vec<Expression>), // 库函数调用语句 (lib::func())
    IfElse(Expression, Vec<Statement>, Vec<(Option<Expression>, Vec<Statement>)>), // if-else 语句，包含条件、if块和多个else-if/else块
    ForLoop(String, Expression, Expression, Vec<Statement>), // for循环，包含变量名、范围起始值、范围结束值和循环体
    WhileLoop(Expression, Vec<Statement>), // while循环，包含条件和循环体
    Break, // 跳出当前循环
    Continue, // 跳过当前迭代，继续下一次迭代
    ForEachLoop(String, Expression, Vec<Statement>), // foreach循环，包含变量名、集合表达式和循环体
    TryCatch(Vec<Statement>, Vec<(String, Type, Vec<Statement>)>, Option<Vec<Statement>>), // 新增：try-catch-finally 语句
    Throw(Expression), // 新增：抛出异常语句
    // Switch 语句
    Switch(Expression, Vec<SwitchCase>, Option<Vec<Statement>>, SwitchType), // switch语句：表达式、case列表、default块、类型
    // OOP相关语句
    ClassDeclaration(Class), // 类声明
    InterfaceDeclaration(Interface), // 接口声明
    FieldAssignment(Box<Expression>, String, Expression), // 字段赋值 (obj.field = value)
    ArrayElementAssignment(Box<Expression>, Box<Expression>, Expression), // 数组元素赋值 (arr[index] = value)
    // Enum相关语句
    EnumDeclaration(Enum), // 枚举声明
    // 模式匹配语句
    Match(Expression, Vec<MatchArm>), // match语句：匹配表达式和匹配分支列表
    // 未来可以扩展更多语句类型
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub param_type: Type,
    pub default_value: Option<Expression>, // 新增：参数的默认值（可选）
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub generic_parameters: Vec<GenericParameter>, // 泛型参数列表
    pub parameters: Vec<Parameter>,
    pub return_type: Type,
    pub body: Vec<Statement>,
    pub where_clause: Vec<TypeConstraint>, // where子句中的约束
}

#[derive(Debug, Clone)]
pub struct Namespace {
    pub name: String,
    pub ns_type: NamespaceType, // 添加命名空间类型字段
    pub functions: Vec<Function>,
    pub namespaces: Vec<Namespace>, // 嵌套命名空间
}

#[derive(Debug, Clone)]
pub enum Visibility {
    Private,
    Protected,
    Public,
}

// v0.7.2新增：友元声明支持
#[derive(Debug, Clone)]
pub struct FriendDeclaration {
    pub friend_type: FriendType,
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum FriendType {
    Class,
    Function,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub field_type: Type,
    pub visibility: Visibility,
    pub initial_value: Option<Expression>,
    pub is_static: bool, // 是否为静态字段
}

#[derive(Debug, Clone)]
pub struct Method {
    pub name: String,
    pub generic_parameters: Vec<GenericParameter>, // 泛型参数列表
    pub parameters: Vec<Parameter>,
    pub return_type: Type,
    pub body: Vec<Statement>,
    pub visibility: Visibility,
    pub is_static: bool, // 是否为静态方法
    pub is_virtual: bool, // 是否为虚方法
    pub is_override: bool, // 是否重写父类方法
    pub is_abstract: bool, // 是否为抽象方法
    pub where_clause: Vec<TypeConstraint>, // where子句中的约束
}

#[derive(Debug, Clone)]
pub struct Constructor {
    pub generic_parameters: Vec<GenericParameter>, // 泛型参数
    pub parameters: Vec<Parameter>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct Interface {
    pub name: String,
    pub generic_parameters: Vec<GenericParameter>, // 泛型参数列表
    pub methods: Vec<InterfaceMethod>, // 接口方法声明
    pub extends: Vec<String>, // 接口可以继承多个接口
    pub where_clause: Vec<TypeConstraint>, // where子句中的约束
}

#[derive(Debug, Clone)]
pub struct InterfaceMethod {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Type,
    pub visibility: Visibility, // 接口方法默认为public
}

#[derive(Debug, Clone)]
pub struct Class {
    pub name: String,
    pub generic_parameters: Vec<GenericParameter>, // 泛型参数列表
    pub super_class: Option<String>, // 父类名
    pub implements: Vec<String>, // 实现的接口列表
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    pub constructors: Vec<Constructor>,
    pub is_abstract: bool, // 是否为抽象类
    pub friends: Vec<FriendDeclaration>, // v0.7.2新增：友元声明
    pub where_clause: Vec<TypeConstraint>, // where子句中的约束
}

#[derive(Debug, Clone)]
pub struct Program {
    pub functions: Vec<Function>,
    pub namespaces: Vec<Namespace>, // 顶层命名空间
    pub imported_namespaces: Vec<(NamespaceType, Vec<String>)>, // 统一的导入记录
    pub file_imports: Vec<String>,   // 顶层文件导入
    pub constants: Vec<(String, Type, Expression)>, // 新增：顶层常量定义
    pub classes: Vec<Class>, // 新增：类定义
    pub interfaces: Vec<Interface>, // 新增：接口定义
    pub enums: Vec<Enum>, // 新增：枚举定义
}

// Switch case 结构
#[derive(Debug, Clone)]
pub enum CasePattern {
    Value(Expression),           // 原有的值匹配
    Range(Expression, Expression), // 范围匹配: start..end
    Guard(String, Expression),   // Guard条件: x if condition
    Destructure(DestructurePattern), // 解构匹配
}

#[derive(Debug, Clone)]
pub enum DestructurePattern {
    Array(Vec<ArrayElement>),    // 数组解构
    // 未来可扩展对象解构等
}

#[derive(Debug, Clone)]
pub enum ArrayElement {
    Variable(String),            // 变量绑定
    Rest(String),               // 剩余元素 ...name
    Literal(Expression),        // 字面量匹配
}

#[derive(Debug, Clone)]
pub enum SwitchType {
    Statement,                  // 语句形式的 switch
    Expression,                 // 表达式形式的 switch
}

#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub pattern: CasePattern,        // 替换原有的 value
    pub statements: Vec<Statement>,  // case 块中的语句
    pub expression: Option<Expression>, // 表达式形式的返回值
    pub has_break: bool,            // 是否有 break 语句
}

// Enum 相关结构体
#[derive(Debug, Clone)]
pub struct Enum {
    pub name: String,
    pub generic_parameters: Vec<GenericParameter>, // 泛型参数列表
    pub variants: Vec<EnumVariant>,
    pub where_clause: Vec<TypeConstraint>, // where子句中的约束
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<EnumField>, // 枚举变体的字段（支持类似Rust的enum）
}

#[derive(Debug, Clone)]
pub struct EnumField {
    pub name: Option<String>, // 字段名（可选，支持元组式和结构体式）
    pub field_type: Type,
}

// 指针成员访问操作符类型
#[derive(Debug, Clone, PartialEq)]
pub enum PointerMemberAccessOp {
    Arrow,  // -> 操作符
    Dot,    // . 操作符（用于指针的直接成员访问）
}