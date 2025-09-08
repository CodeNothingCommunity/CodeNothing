//! Lexical analyzer for CodeNothing language
//! 
//! Tokenizes source code into a stream of tokens for parsing

use std::fmt;

/// 字符串插值的组成部分
#[derive(Debug, Clone, PartialEq)]
pub enum InterpolationPart {
    Text(String),       // 普通文本
    Expression(String), // 表达式 ${...}
}

/// Token types in CodeNothing language
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    InterpolatedString(Vec<InterpolationPart>), // 字符串插值
    Boolean(bool),
    Null,
    
    // Identifiers
    Identifier(String),
    
    // Keywords
    Var,        // var
    Let,        // let
    Const,      // const
    Fn,         // fn
    Ns,         // ns
    Using,      // using
    Lib,        // lib
    Import,     // import
    Return,     // return
    If,         // if
    Else,       // else
    While,      // while
    For,        // for
    In,         // in (for..in loops)
    Do,         // do (do-while loops)
    Break,      // break
    Continue,   // continue
    Switch,     // switch
    Case,       // case
    Default,    // default
    Try,        // try
    Catch,      // catch
    Finally,    // finally
    Throw,      // throw
    True,       // true
    False,      // false
    Memo,       // memo
    Enum,       // enum
    Struct,     // struct
    Class,      // class
    Trait,      // trait
    Impl,       // impl
    Match,      // match
    When,       // when (类似 match 但语法不同)
    Lambda,     // lambda (optional, we'll use |x| syntax)

    // Access modifiers
    Public,     // public
    Private,    // private
    Protected,  // protected
    Package,    // package
    Static,     // static
    Virtual,    // virtual
    Override,   // override
    Abstract,   // abstract
    Friend,     // friend
    Extends,    // extends
    Implements, // implements
    Super,      // super
    
    // Types
    Int,        // int
    Long,       // long
    Float32,    // float
    StringType, // string
    Num,        // num
    Bool,       // bool
    Array,      // array
    Obj,        // obj
    Auto,       // auto

    // Pointer keywords
    Mut,        // mut (for &mut references)
    ConstPtr,   // const (for *const pointers)
    
    // Operators
    Plus,       // +
    Minus,      // -
    Multiply,   // *
    Divide,     // /
    Modulo,     // %
    Assign,     // =
    ColonAssign, // := for type inference
    Equal,      // ==
    NotEqual,   // !=
    Less,       // <
    LessEqual,  // <=
    Greater,    // >
    GreaterEqual, // >=
    And,        // &&
    Or,         // ||
    Not,        // !
    Range,      // ...
    DotDot,     // .. (exclusive range)
    DotDotEqual, // ..= (inclusive range)

    // Increment/Decrement operators
    PlusPlus,   // ++
    MinusMinus, // --

    // Compound assignment operators
    PlusEqual,     // +=
    MinusEqual,    // -=
    MultiplyEqual, // *=
    DivideEqual,   // /=
    ModuloEqual,   // %=

    // Bitwise operators
    BitAnd,     // &
    BitOr,      // |
    BitXor,     // ^
    BitNot,     // ~
    LeftShift,  // <<
    RightShift, // >>
    
    // Punctuation
    LeftParen,  // (
    RightParen, // )
    LeftBrace,  // {
    RightBrace, // }
    LeftBracket, // [
    RightBracket, // ]
    Semicolon,  // ;
    Comma,      // ,
    Colon,      // :
    DoubleColon, // ::
    Dot,        // .
    Question,   // ?
    DoubleQuestion, // ?? (null coalescing operator)
    Pipe,       // | (for lambda parameters)
    PipeForward, // |> (pipe operator)
    Arrow,      // -> (for lambda return type)
    FatArrow,   // => (for match expressions)

    // Special
    Newline,
    Eof,
}

/// Token with position information
#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub line: usize,
    pub column: usize,
    pub lexeme: String,
}

impl Token {
    pub fn new(token_type: TokenType, line: usize, column: usize, lexeme: String) -> Self {
        Self {
            token_type,
            line,
            column,
            lexeme,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} at {}:{}", self.token_type, self.line, self.column)
    }
}

/// Lexical analyzer
pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }
    
    /// Get current character without advancing
    fn peek(&self) -> Option<char> {
        self.input.get(self.position).copied()
    }
    
    /// Get character at offset without advancing
    fn peek_ahead(&self, offset: usize) -> Option<char> {
        self.input.get(self.position + offset).copied()
    }
    
    /// Advance to next character
    fn advance(&mut self) -> Option<char> {
        if let Some(ch) = self.input.get(self.position) {
            self.position += 1;
            if *ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            Some(*ch)
        } else {
            None
        }
    }
    
    /// Skip all whitespace including newlines (C-style)
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }
    
    /// Skip line comment
    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }
    
    /// Read identifier or keyword
    fn read_identifier(&mut self) -> String {
        let mut result = String::new();
        
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                result.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        result
    }
    
    /// Read number (integer or float)
    fn read_number(&mut self) -> TokenType {
        let mut result = String::new();
        let mut is_float = false;
        
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                result.push(ch);
                self.advance();
            } else if ch == '.' && !is_float {
                // Check if next character is digit
                if let Some(next_ch) = self.peek_ahead(1) {
                    if next_ch.is_ascii_digit() {
                        is_float = true;
                        result.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        
        if is_float {
            TokenType::Float(result.parse().unwrap_or(0.0))
        } else {
            TokenType::Integer(result.parse().unwrap_or(0))
        }
    }
    
    /// Read string literal
    fn read_string(&mut self) -> Result<String, String> {
        let mut result = String::new();
        self.advance(); // Skip opening quote
        
        while let Some(ch) = self.peek() {
            if ch == '"' {
                self.advance(); // Skip closing quote
                return Ok(result);
            } else if ch == '\\' {
                self.advance();
                if let Some(escaped) = self.advance() {
                    match escaped {
                        'n' => result.push('\n'),
                        't' => result.push('\t'),
                        'r' => result.push('\r'),
                        '\\' => result.push('\\'),
                        '"' => result.push('"'),
                        _ => {
                            result.push('\\');
                            result.push(escaped);
                        }
                    }
                }
            } else {
                result.push(ch);
                self.advance();
            }
        }
        
        Err("Unterminated string literal".to_string())
    }

    /// Read interpolated string literal (支持 ${expression} 语法)
    fn read_interpolated_string(&mut self) -> Result<Vec<InterpolationPart>, String> {
        let mut parts = Vec::new();
        let mut current_text = String::new();
        self.advance(); // Skip opening quote

        while let Some(ch) = self.peek() {
            if ch == '"' {
                // 结束字符串
                if !current_text.is_empty() {
                    parts.push(InterpolationPart::Text(current_text));
                }
                self.advance(); // Skip closing quote
                return Ok(parts);
            } else if ch == '$' && self.peek_ahead(1) == Some('{') {
                // 开始插值表达式
                if !current_text.is_empty() {
                    parts.push(InterpolationPart::Text(current_text));
                    current_text = String::new();
                }

                self.advance(); // Skip $
                self.advance(); // Skip {

                // 读取表达式直到 }，正确处理字符串字面量
                let mut expression = String::new();
                let mut brace_count = 1;
                let mut in_string = false;
                let mut escape_next = false;
                let mut in_closure = false;
                let mut closure_pipe_count = 0;

                while let Some(expr_ch) = self.peek() {
                    if escape_next {
                        // 转义字符，直接添加
                        expression.push(expr_ch);
                        self.advance();
                        escape_next = false;
                    } else if expr_ch == '\\' && in_string {
                        // 在字符串中遇到转义字符
                        expression.push(expr_ch);
                        self.advance();
                        escape_next = true;
                    } else if expr_ch == '"' {
                        // 字符串开始或结束
                        in_string = !in_string;
                        expression.push(expr_ch);
                        self.advance();
                    } else if !in_string && expr_ch == '|' {
                        // 处理闭包管道符号
                        if !in_closure {
                            in_closure = true;
                            closure_pipe_count = 1;
                        } else {
                            closure_pipe_count += 1;
                            if closure_pipe_count == 2 {
                                in_closure = false;
                                closure_pipe_count = 0;
                            }
                        }
                        expression.push(expr_ch);
                        self.advance();
                    } else if !in_string && expr_ch == '}' {
                        // 不在字符串中遇到 }，检查是否是结束大括号
                        if !in_closure {
                            brace_count -= 1;
                            if brace_count == 0 {
                                self.advance(); // Skip closing }
                                break;
                            }
                        }
                        expression.push(expr_ch);
                        self.advance();
                    } else if !in_string && expr_ch == '{' {
                        // 不在字符串中遇到 {，增加嵌套计数
                        if !in_closure {
                            brace_count += 1;
                        }
                        expression.push(expr_ch);
                        self.advance();
                    } else {
                        // 其他字符，直接添加
                        expression.push(expr_ch);
                        self.advance();
                    }
                }

                if brace_count > 0 {
                    return Err("Unterminated interpolation expression".to_string());
                }

                if in_string {
                    return Err("Unterminated string literal in interpolation expression".to_string());
                }

                parts.push(InterpolationPart::Expression(expression));
            } else if ch == '\\' {
                // 处理转义字符
                self.advance();
                if let Some(escaped) = self.advance() {
                    match escaped {
                        'n' => current_text.push('\n'),
                        't' => current_text.push('\t'),
                        'r' => current_text.push('\r'),
                        '\\' => current_text.push('\\'),
                        '"' => current_text.push('"'),
                        '$' => current_text.push('$'),
                        _ => {
                            current_text.push('\\');
                            current_text.push(escaped);
                        }
                    }
                }
            } else {
                current_text.push(ch);
                self.advance();
            }
        }

        Err("Unterminated interpolated string literal".to_string())
    }

    /// 检查字符串是否包含插值表达式
    fn contains_interpolation(&self) -> bool {
        let mut pos = self.position + 1; // Skip opening quote
        let mut in_escape = false;

        while pos < self.input.len() {
            let ch = self.input[pos];

            if in_escape {
                in_escape = false;
                pos += 1;
                continue;
            }

            if ch == '\\' {
                in_escape = true;
                pos += 1;
                continue;
            }

            if ch == '"' {
                // 到达字符串结尾
                break;
            }

            if ch == '$' && pos + 1 < self.input.len() && self.input[pos + 1] == '{' {
                return true;
            }

            pos += 1;
        }

        false
    }
    
    /// Convert identifier to keyword or return identifier token
    fn identifier_to_token(&self, identifier: &str) -> TokenType {
        match identifier {
            "var" => TokenType::Var,
            "let" => TokenType::Let,
            "const" => TokenType::Const,
            "fn" => TokenType::Fn,
            "ns" => TokenType::Ns,
            "using" => TokenType::Using,
            "lib" => TokenType::Lib,
            "import" => TokenType::Import,
            "return" => TokenType::Return,
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "while" => TokenType::While,
            "for" => TokenType::For,
            "in" => TokenType::In,
            "do" => TokenType::Do,
            "break" => TokenType::Break,
            "continue" => TokenType::Continue,
            "switch" => TokenType::Switch,
            "case" => TokenType::Case,
            "default" => TokenType::Default,
            "try" => TokenType::Try,
            "catch" => TokenType::Catch,
            "finally" => TokenType::Finally,
            "throw" => TokenType::Throw,
            "true" => TokenType::True,
            "false" => TokenType::False,
            "memo" => TokenType::Memo,
            "enum" => TokenType::Enum,
            "struct" => TokenType::Struct,
            "class" => TokenType::Class,
            "trait" => TokenType::Trait,
            "impl" => TokenType::Impl,
            "match" => TokenType::Match,
            "when" => TokenType::When,
            "public" => TokenType::Public,
            "private" => TokenType::Private,
            "protected" => TokenType::Protected,
            "package" => TokenType::Package,
            "static" => TokenType::Static,
            "virtual" => TokenType::Virtual,
            "override" => TokenType::Override,
            "abstract" => TokenType::Abstract,
            "friend" => TokenType::Friend,
            "extends" => TokenType::Extends,
            "implements" => TokenType::Implements,
            "super" => TokenType::Super,
            "int" => TokenType::Int,
            "long" => TokenType::Long,
            "float" => TokenType::Float32,
            "string" => TokenType::StringType,
            "num" => TokenType::Num,
            "bool" => TokenType::Bool,
            "array" => TokenType::Array,
            "obj" => TokenType::Obj,
            "auto" => TokenType::Auto,
            "null" => TokenType::Null,
            "mut" => TokenType::Mut,
            "const" => TokenType::ConstPtr,
            _ => TokenType::Identifier(identifier.to_string()),
        }
    }
    
    /// Get next token
    pub fn next_token(&mut self) -> Result<Token, String> {
        self.skip_whitespace();
        
        let start_line = self.line;
        let start_column = self.column;
        
        if let Some(ch) = self.peek() {
            let lexeme_start = self.position;
            
            let token_type = match ch {
                '/' => {
                    if self.peek_ahead(1) == Some('/') {
                        self.skip_line_comment();
                        return self.next_token(); // Skip comment and get next token
                    } else {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenType::DivideEqual
                        } else {
                            TokenType::Divide
                        }
                    }
                }
                '+' => {
                    self.advance();
                    if self.peek() == Some('+') {
                        self.advance();
                        TokenType::PlusPlus
                    } else if self.peek() == Some('=') {
                        self.advance();
                        TokenType::PlusEqual
                    } else {
                        TokenType::Plus
                    }
                }
                '-' => {
                    self.advance();
                    if self.peek() == Some('>') {
                        self.advance();
                        TokenType::Arrow
                    } else if self.peek() == Some('-') {
                        self.advance();
                        TokenType::MinusMinus
                    } else if self.peek() == Some('=') {
                        self.advance();
                        TokenType::MinusEqual
                    } else {
                        TokenType::Minus
                    }
                }
                '*' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenType::MultiplyEqual
                    } else {
                        TokenType::Multiply
                    }
                }
                '%' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenType::ModuloEqual
                    } else {
                        TokenType::Modulo
                    }
                }
                '=' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenType::Equal
                    } else if self.peek() == Some('>') {
                        self.advance();
                        TokenType::FatArrow
                    } else {
                        TokenType::Assign
                    }
                }
                '!' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenType::NotEqual
                    } else {
                        TokenType::Not
                    }
                }
                '<' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenType::LessEqual
                    } else if self.peek() == Some('<') {
                        self.advance();
                        TokenType::LeftShift
                    } else {
                        TokenType::Less
                    }
                }
                '>' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenType::GreaterEqual
                    } else if self.peek() == Some('>') {
                        self.advance();
                        TokenType::RightShift
                    } else {
                        TokenType::Greater
                    }
                }
                '&' => {
                    self.advance();
                    if self.peek() == Some('&') {
                        self.advance();
                        TokenType::And
                    } else {
                        TokenType::BitAnd
                    }
                }
                '|' => {
                    self.advance();
                    if self.peek() == Some('|') {
                        self.advance();
                        TokenType::Or
                    } else if self.peek() == Some('>') {
                        self.advance();
                        TokenType::PipeForward
                    } else {
                        TokenType::Pipe
                    }
                }
                '^' => {
                    self.advance();
                    TokenType::BitXor
                }
                '~' => {
                    self.advance();
                    TokenType::BitNot
                }
                '(' => {
                    self.advance();
                    TokenType::LeftParen
                }
                ')' => {
                    self.advance();
                    TokenType::RightParen
                }
                '{' => {
                    self.advance();
                    TokenType::LeftBrace
                }
                '}' => {
                    self.advance();
                    TokenType::RightBrace
                }
                '[' => {
                    self.advance();
                    TokenType::LeftBracket
                }
                ']' => {
                    self.advance();
                    TokenType::RightBracket
                }
                ';' => {
                    self.advance();
                    TokenType::Semicolon
                }
                ',' => {
                    self.advance();
                    TokenType::Comma
                }
                ':' => {
                    self.advance();
                    if self.peek() == Some(':') {
                        self.advance();
                        TokenType::DoubleColon
                    } else if self.peek() == Some('=') {
                        self.advance();
                        TokenType::ColonAssign
                    } else {
                        TokenType::Colon
                    }
                }
                '.' => {
                    self.advance();
                    // Check for range operators
                    if self.peek() == Some('.') {
                        self.advance(); // consume second .
                        // Check for inclusive range ..=
                        if self.peek() == Some('=') {
                            self.advance(); // consume =
                            TokenType::DotDotEqual
                        } else if self.peek() == Some('.') {
                            // Check for ... (three dots)
                            self.advance(); // consume third .
                            TokenType::Range
                        } else {
                            // Just .. (exclusive range)
                            TokenType::DotDot
                        }
                    } else {
                        TokenType::Dot
                    }
                }
                '?' => {
                    self.advance();
                    // Check for null coalescing operator ??
                    if self.peek() == Some('?') {
                        self.advance(); // consume second ?
                        TokenType::DoubleQuestion
                    } else {
                        TokenType::Question
                    }
                }
                '"' => {
                    // 先检查是否包含插值表达式
                    if self.contains_interpolation() {
                        match self.read_interpolated_string() {
                            Ok(parts) => TokenType::InterpolatedString(parts),
                            Err(e) => return Err(e),
                        }
                    } else {
                        match self.read_string() {
                            Ok(s) => TokenType::String(s),
                            Err(e) => return Err(e),
                        }
                    }
                }
                _ if ch.is_ascii_digit() => self.read_number(),
                _ if ch.is_alphabetic() || ch == '_' => {
                    let identifier = self.read_identifier();
                    self.identifier_to_token(&identifier)
                }
                _ => {
                    return Err(format!("Unexpected character '{}' at {}:{}", ch, start_line, start_column));
                }
            };
            
            let lexeme = self.input[lexeme_start..self.position].iter().collect();
            Ok(Token::new(token_type, start_line, start_column, lexeme))
        } else {
            Ok(Token::new(TokenType::Eof, start_line, start_column, String::new()))
        }
    }
    
    /// Tokenize entire input
    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        
        loop {
            let token = self.next_token()?;
            let is_eof = matches!(token.token_type, TokenType::Eof);
            tokens.push(token);
            
            if is_eof {
                break;
            }
        }
        
        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_tokens() {
        let mut lexer = Lexer::new("var x : int = 42;");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens.len(), 8); // var, x, :, int, =, 42, ;, EOF
        assert!(matches!(tokens[0].token_type, TokenType::Var));
        assert!(matches!(tokens[1].token_type, TokenType::Identifier(_)));
        assert!(matches!(tokens[2].token_type, TokenType::Colon));
        assert!(matches!(tokens[3].token_type, TokenType::Int));
        assert!(matches!(tokens[4].token_type, TokenType::Assign));
        assert!(matches!(tokens[5].token_type, TokenType::Integer(42)));
        assert!(matches!(tokens[6].token_type, TokenType::Semicolon));
    }
    
    #[test]
    fn test_function_declaration() {
        let mut lexer = Lexer::new("fn main(x : long, y : long) : long { return x + y; }");
        let tokens = lexer.tokenize().unwrap();
        
        assert!(matches!(tokens[0].token_type, TokenType::Fn));
        assert!(matches!(tokens[1].token_type, TokenType::Identifier(_)));
        assert!(matches!(tokens[2].token_type, TokenType::LeftParen));
    }
}
