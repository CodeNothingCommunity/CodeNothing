//! Lexical analyzer for CodeNothing language
//! 
//! Tokenizes source code into a stream of tokens for parsing

use std::fmt;

/// Token types in CodeNothing language
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Literals
    Integer(i64),
    Float(f64),
    String(String),
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
    Return,     // return
    If,         // if
    Else,       // else
    While,      // while
    For,        // for
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
    
    // Operators
    Plus,       // +
    Minus,      // -
    Multiply,   // *
    Divide,     // /
    Modulo,     // %
    Assign,     // =
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
    
    /// Skip whitespace (except newlines)
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() && ch != '\n' {
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
    
    /// Convert identifier to keyword or return identifier token
    fn identifier_to_token(&self, identifier: &str) -> TokenType {
        match identifier {
            "var" => TokenType::Var,
            "let" => TokenType::Let,
            "const" => TokenType::Const,
            "fn" => TokenType::Fn,
            "ns" => TokenType::Ns,
            "using" => TokenType::Using,
            "return" => TokenType::Return,
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "while" => TokenType::While,
            "for" => TokenType::For,
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
                '\n' => {
                    self.advance();
                    TokenType::Newline
                }
                '/' => {
                    if self.peek_ahead(1) == Some('/') {
                        self.skip_line_comment();
                        return self.next_token(); // Skip comment and get next token
                    } else {
                        self.advance();
                        TokenType::Divide
                    }
                }
                '+' => {
                    self.advance();
                    TokenType::Plus
                }
                '-' => {
                    self.advance();
                    TokenType::Minus
                }
                '*' => {
                    self.advance();
                    TokenType::Multiply
                }
                '%' => {
                    self.advance();
                    TokenType::Modulo
                }
                '=' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenType::Equal
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
                    } else {
                        TokenType::BitOr
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
                    } else {
                        TokenType::Colon
                    }
                }
                '.' => {
                    self.advance();
                    // Check for range operator ...
                    if self.peek() == Some('.') && self.peek_ahead(1) == Some('.') {
                        self.advance(); // consume second .
                        self.advance(); // consume third .
                        TokenType::Range
                    } else {
                        TokenType::Dot
                    }
                }
                '?' => {
                    self.advance();
                    TokenType::Question
                }
                '"' => {
                    match self.read_string() {
                        Ok(s) => TokenType::String(s),
                        Err(e) => return Err(e),
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
