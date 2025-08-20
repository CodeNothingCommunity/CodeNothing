//! Parser for CodeNothing language
//! 
//! Implements recursive descent parser for the language grammar

use crate::lang::lexer::{Token, TokenType};
use crate::lang::ast::*;
use std::collections::VecDeque;

/// Parser errors
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub position: Position,
}

impl ParseError {
    pub fn new(message: String, position: Position) -> Self {
        Self { message, position }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error at {}:{}: {}", 
               self.position.line, self.position.column, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Recursive descent parser
pub struct Parser {
    tokens: VecDeque<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens: tokens.into(),
            current: 0,
        }
    }
    
    /// Get current token without consuming
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current)
    }
    
    /// Get token at offset without consuming
    fn peek_ahead(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.current + offset)
    }
    
    /// Consume and return current token
    fn advance(&mut self) -> Option<Token> {
        if self.current < self.tokens.len() {
            let token = self.tokens[self.current].clone();
            self.current += 1;
            Some(token)
        } else {
            None
        }
    }
    
    /// Check if current token matches expected type
    fn check(&self, token_type: &TokenType) -> bool {
        if let Some(token) = self.peek() {
            std::mem::discriminant(&token.token_type) == std::mem::discriminant(token_type)
        } else {
            false
        }
    }
    
    /// Consume token if it matches expected type
    fn match_token(&mut self, token_type: TokenType) -> bool {
        if self.check(&token_type) {
            self.advance();
            true
        } else {
            false
        }
    }
    
    /// Expect specific token type and consume it
    fn expect(&mut self, token_type: TokenType, message: &str) -> Result<Token, ParseError> {
        if let Some(token) = self.peek() {
            if std::mem::discriminant(&token.token_type) == std::mem::discriminant(&token_type) {
                Ok(self.advance().unwrap())
            } else {
                Err(ParseError::new(
                    format!("{}, found {:?}", message, token.token_type),
                    Position::new(token.line, token.column)
                ))
            }
        } else {
            Err(ParseError::new(
                format!("{}, found EOF", message),
                Position::new(0, 0)
            ))
        }
    }
    
    /// Skip newlines
    fn skip_newlines(&mut self) {
        while self.match_token(TokenType::Newline) {
            // Skip
        }
    }
    
    /// Parse complete program
    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut program = Program::new();
        
        self.skip_newlines();
        
        while !self.check(&TokenType::Eof) {
            let declaration = self.parse_declaration()?;
            program.add_declaration(declaration);
            self.skip_newlines();
        }
        
        Ok(program)
    }
    
    /// Parse top-level declaration
    fn parse_declaration(&mut self) -> Result<Declaration, ParseError> {
        self.skip_newlines();
        
        if let Some(token) = self.peek() {
            match &token.token_type {
                TokenType::Fn => {
                    let func = self.parse_function()?;
                    Ok(Declaration::Function(func))
                }
                TokenType::Ns => {
                    let ns = self.parse_namespace()?;
                    Ok(Declaration::Namespace(ns))
                }
                TokenType::Using => {
                    let using = self.parse_using()?;
                    Ok(Declaration::Using(using))
                }
                TokenType::Memo => {
                    let memo = self.parse_memo()?;
                    Ok(Declaration::Memo(memo))
                }
                TokenType::Identifier(_) => {
                    // Global variable declaration: name : type = value;
                    let stmt = self.parse_global_variable()?;
                    Ok(Declaration::Variable(stmt))
                }
                _ => {
                    Err(ParseError::new(
                        format!("Expected declaration, found {:?}", token.token_type),
                        Position::new(token.line, token.column)
                    ))
                }
            }
        } else {
            Err(ParseError::new(
                "Expected declaration, found EOF".to_string(),
                Position::new(0, 0)
            ))
        }
    }
    
    /// Parse function declaration
    fn parse_function(&mut self) -> Result<FunctionDeclaration, ParseError> {
        let fn_token = self.expect(TokenType::Fn, "Expected 'fn'")?;
        let position = Position::new(fn_token.line, fn_token.column);
        
        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected function name")?;
        let name = if let TokenType::Identifier(n) = name_token.token_type {
            n
        } else {
            return Err(ParseError::new("Expected function name".to_string(), position));
        };
        
        self.expect(TokenType::LeftParen, "Expected '(' after function name")?;
        
        let mut parameters = Vec::new();
        
        if !self.check(&TokenType::RightParen) {
            loop {
                let param = self.parse_parameter()?;
                parameters.push(param);
                
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        
        self.expect(TokenType::RightParen, "Expected ')' after parameters")?;
        
        let return_type = if self.match_token(TokenType::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        
        let body = self.parse_statement()?;
        
        Ok(FunctionDeclaration {
            name,
            parameters,
            return_type,
            body,
            position,
        })
    }
    
    /// Parse function parameter
    fn parse_parameter(&mut self) -> Result<Parameter, ParseError> {
        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected parameter name")?;
        let name = if let TokenType::Identifier(n) = name_token.token_type {
            n
        } else {
            return Err(ParseError::new(
                "Expected parameter name".to_string(),
                Position::new(name_token.line, name_token.column)
            ));
        };
        
        self.expect(TokenType::Colon, "Expected ':' after parameter name")?;
        let type_annotation = self.parse_type_annotation()?;
        
        Ok(Parameter {
            name,
            type_annotation,
            position: Position::new(name_token.line, name_token.column),
        })
    }
    
    /// Parse type annotation
    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, ParseError> {
        if let Some(token) = self.peek() {
            let position = Position::new(token.line, token.column);
            
            match &token.token_type {
                TokenType::Int => {
                    self.advance();
                    Ok(TypeAnnotation::Int)
                }
                TokenType::Long => {
                    self.advance();
                    Ok(TypeAnnotation::Long)
                }
                TokenType::Float32 => {
                    self.advance();
                    if self.match_token(TokenType::LeftParen) {
                        let size_token = self.expect(TokenType::Integer(0), "Expected number")?;
                        if let TokenType::Integer(size) = size_token.token_type {
                            self.expect(TokenType::RightParen, "Expected ')' after size")?;
                            Ok(TypeAnnotation::Float(Some(size as u32)))
                        } else {
                            Err(ParseError::new("Expected number".to_string(), position))
                        }
                    } else {
                        Ok(TypeAnnotation::Float(None))
                    }
                }
                TokenType::StringType => {
                    self.advance();
                    Ok(TypeAnnotation::String)
                }
                TokenType::Num => {
                    self.advance();
                    if self.match_token(TokenType::LeftParen) {
                        let size_token = self.expect(TokenType::Integer(0), "Expected number")?;
                        if let TokenType::Integer(size) = size_token.token_type {
                            self.expect(TokenType::RightParen, "Expected ')' after size")?;
                            Ok(TypeAnnotation::Num(Some(size as u32)))
                        } else {
                            Err(ParseError::new("Expected number".to_string(), position))
                        }
                    } else {
                        Ok(TypeAnnotation::Num(None))
                    }
                }
                TokenType::Bool => {
                    self.advance();
                    Ok(TypeAnnotation::Bool)
                }
                TokenType::Array => {
                    self.advance();
                    // TODO: Parse array element type
                    Ok(TypeAnnotation::Array(Box::new(TypeAnnotation::Auto)))
                }
                TokenType::Obj => {
                    self.advance();
                    Ok(TypeAnnotation::Obj)
                }
                TokenType::Auto => {
                    self.advance();
                    Ok(TypeAnnotation::Auto)
                }
                TokenType::Null => {
                    self.advance();
                    Ok(TypeAnnotation::Null)
                }
                TokenType::Identifier(name) => {
                    let name = name.clone();
                    self.advance();
                    Ok(TypeAnnotation::Custom(name))
                }
                _ => {
                    Err(ParseError::new(
                        format!("Expected type annotation, found {:?}", token.token_type),
                        position
                    ))
                }
            }
        } else {
            Err(ParseError::new(
                "Expected type annotation, found EOF".to_string(),
                Position::new(0, 0)
            ))
        }
    }
    
    /// Parse namespace declaration
    fn parse_namespace(&mut self) -> Result<NamespaceDeclaration, ParseError> {
        let ns_token = self.expect(TokenType::Ns, "Expected 'ns'")?;
        let position = Position::new(ns_token.line, ns_token.column);
        
        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected namespace name")?;
        let name = if let TokenType::Identifier(n) = name_token.token_type {
            n
        } else {
            return Err(ParseError::new("Expected namespace name".to_string(), position));
        };
        
        self.expect(TokenType::LeftBrace, "Expected '{' after namespace name")?;
        
        let mut items = Vec::new();
        self.skip_newlines();
        
        while !self.check(&TokenType::RightBrace) && !self.check(&TokenType::Eof) {
            if self.check(&TokenType::Fn) {
                let func = self.parse_function()?;
                items.push(NamespaceItem::Function(func));
            } else if self.check(&TokenType::Ns) {
                // Parse nested namespace
                let nested_ns = self.parse_namespace()?;
                items.push(NamespaceItem::Namespace(nested_ns));
            } else {
                // TODO: Parse other namespace items (variables)
                break;
            }
            self.skip_newlines();
        }
        
        self.expect(TokenType::RightBrace, "Expected '}' after namespace body")?;
        
        Ok(NamespaceDeclaration {
            name,
            items,
            position,
        })
    }
    
    /// Parse using declaration
    fn parse_using(&mut self) -> Result<UsingDeclaration, ParseError> {
        let using_token = self.expect(TokenType::Using, "Expected 'using'")?;
        let position = Position::new(using_token.line, using_token.column);
        
        self.expect(TokenType::Ns, "Expected 'ns' after 'using'")?;
        
        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected namespace name")?;
        let namespace = if let TokenType::Identifier(n) = name_token.token_type {
            n
        } else {
            return Err(ParseError::new("Expected namespace name".to_string(), position));
        };
        
        self.expect(TokenType::Semicolon, "Expected ';' after using declaration")?;
        
        Ok(UsingDeclaration {
            namespace,
            position,
        })
    }

    /// Parse memo declaration
    fn parse_memo(&mut self) -> Result<MemoDeclaration, ParseError> {
        let memo_token = self.expect(TokenType::Memo, "Expected 'memo'")?;
        let position = Position::new(memo_token.line, memo_token.column);

        // Parse optional parameters: memo(max_size: 100, ttl: 3600)
        let params = if self.check(&TokenType::LeftParen) {
            self.advance(); // consume '('

            let mut max_size = None;
            let mut ttl = None;

            // Parse parameters
            while !self.check(&TokenType::RightParen) && !self.is_at_end() {
                if let Some(token) = self.peek() {
                    if let TokenType::Identifier(param_name) = &token.token_type {
                        let param_name = param_name.clone();
                        self.advance(); // consume parameter name

                        self.expect(TokenType::Colon, "Expected ':' after parameter name")?;

                        if let Some(value_token) = self.peek() {
                            if let TokenType::Integer(value) = value_token.token_type {
                                self.advance(); // consume value

                                match param_name.as_str() {
                                    "max_size" => max_size = Some(value),
                                    "ttl" => ttl = Some(value),
                                    _ => return Err(ParseError::new(
                                        format!("Unknown memo parameter: {}", param_name),
                                        position
                                    )),
                                }
                            } else {
                                return Err(ParseError::new(
                                    "Expected integer value for memo parameter".to_string(),
                                    position
                                ));
                            }
                        }

                        // Optional comma
                        if self.check(&TokenType::Comma) {
                            self.advance();
                        }
                    } else {
                        return Err(ParseError::new(
                            "Expected parameter name".to_string(),
                            position
                        ));
                    }
                } else {
                    break;
                }
            }

            self.expect(TokenType::RightParen, "Expected ')' after memo parameters")?;

            Some(MemoParams { max_size, ttl })
        } else {
            None
        };

        // Parse memo content block
        self.expect(TokenType::LeftBrace, "Expected '{' after memo declaration")?;

        let mut declarations = Vec::new();

        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            // Parse memo content (functions or namespaces, but not nested memos)
            if let Some(token) = self.peek() {
                match &token.token_type {
                    TokenType::Fn => {
                        let func = self.parse_function()?;
                        declarations.push(Declaration::Function(func));
                    }
                    TokenType::Ns => {
                        let ns = self.parse_namespace()?;
                        declarations.push(Declaration::Namespace(ns));
                    }
                    TokenType::Using => {
                        let using = self.parse_using()?;
                        declarations.push(Declaration::Using(using));
                    }
                    TokenType::Memo => {
                        return Err(ParseError::new(
                            "Nested memo declarations are not allowed".to_string(),
                            Position::new(token.line, token.column)
                        ));
                    }
                    _ => {
                        // Skip unexpected tokens for now
                        self.advance();
                    }
                }
            } else {
                break;
            }
        }

        self.expect(TokenType::RightBrace, "Expected '}' after memo block")?;

        // Determine content type
        let content = if declarations.len() == 1 {
            match &declarations[0] {
                Declaration::Function(func) => MemoContent::Function(func.clone()),
                Declaration::Namespace(ns) => MemoContent::Namespace(ns.clone()),
                _ => MemoContent::Block(declarations),
            }
        } else {
            MemoContent::Block(declarations)
        };

        Ok(MemoDeclaration {
            params,
            content,
            position,
        })
    }
    
    /// Parse global variable declaration
    fn parse_global_variable(&mut self) -> Result<Statement, ParseError> {
        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected variable name")?;
        let name = if let TokenType::Identifier(n) = name_token.token_type {
            n
        } else {
            return Err(ParseError::new(
                "Expected variable name".to_string(),
                Position::new(name_token.line, name_token.column)
            ));
        };
        
        self.expect(TokenType::Colon, "Expected ':' after variable name")?;
        let type_annotation = Some(self.parse_type_annotation()?);
        
        let initializer = if self.match_token(TokenType::Assign) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.expect(TokenType::Semicolon, "Expected ';' after variable declaration")?;
        
        Ok(Statement::VarDeclaration {
            scope: VarScope::Global,
            name,
            type_annotation,
            initializer,
            position: Position::new(name_token.line, name_token.column),
        })
    }
    
    /// Parse statement
    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        self.skip_newlines();
        
        if let Some(token) = self.peek() {
            match &token.token_type {
                TokenType::Var | TokenType::Let | TokenType::Const => {
                    self.parse_variable_declaration()
                }
                TokenType::Return => {
                    self.parse_return_statement()
                }
                TokenType::If => {
                    self.parse_if_statement()
                }
                TokenType::While => {
                    self.parse_while_statement()
                }
                TokenType::For => {
                    self.parse_for_statement()
                }
                TokenType::Switch => {
                    self.parse_switch_statement()
                }
                TokenType::LeftBrace => {
                    self.parse_block_statement()
                }
                TokenType::Break => {
                    let token = self.advance().unwrap();
                    self.expect(TokenType::Semicolon, "Expected ';' after 'break'")?;
                    Ok(Statement::Break(Position::new(token.line, token.column)))
                }
                TokenType::Continue => {
                    let token = self.advance().unwrap();
                    self.expect(TokenType::Semicolon, "Expected ';' after 'continue'")?;
                    Ok(Statement::Continue(Position::new(token.line, token.column)))
                }
                TokenType::Try => {
                    self.parse_try_statement()
                }
                TokenType::Throw => {
                    self.parse_throw_statement()
                }
                _ => {
                    // Expression statement
                    let expr = self.parse_expression()?;
                    self.expect(TokenType::Semicolon, "Expected ';' after expression")?;
                    Ok(Statement::Expression(expr))
                }
            }
        } else {
            Err(ParseError::new(
                "Expected statement, found EOF".to_string(),
                Position::new(0, 0)
            ))
        }
    }
    
    /// Parse variable declaration
    fn parse_variable_declaration(&mut self) -> Result<Statement, ParseError> {
        let scope_token = self.advance().unwrap();
        let scope = match scope_token.token_type {
            TokenType::Var => VarScope::Var,
            TokenType::Let => VarScope::Let,
            TokenType::Const => VarScope::Const,
            _ => unreachable!(),
        };
        
        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected variable name")?;
        let name = if let TokenType::Identifier(n) = name_token.token_type {
            n
        } else {
            return Err(ParseError::new(
                "Expected variable name".to_string(),
                Position::new(scope_token.line, scope_token.column)
            ));
        };
        
        let type_annotation = if self.match_token(TokenType::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        
        let initializer = if self.match_token(TokenType::Assign) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.expect(TokenType::Semicolon, "Expected ';' after variable declaration")?;
        
        Ok(Statement::VarDeclaration {
            scope,
            name,
            type_annotation,
            initializer,
            position: Position::new(scope_token.line, scope_token.column),
        })
    }
    
    /// Parse return statement
    fn parse_return_statement(&mut self) -> Result<Statement, ParseError> {
        let return_token = self.expect(TokenType::Return, "Expected 'return'")?;
        let position = Position::new(return_token.line, return_token.column);
        
        let value = if self.check(&TokenType::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        
        self.expect(TokenType::Semicolon, "Expected ';' after return statement")?;
        
        Ok(Statement::Return { value, position })
    }
    
    /// Parse if statement
    fn parse_if_statement(&mut self) -> Result<Statement, ParseError> {
        let if_token = self.expect(TokenType::If, "Expected 'if'")?;
        let position = Position::new(if_token.line, if_token.column);
        
        self.expect(TokenType::LeftParen, "Expected '(' after 'if'")?;
        let condition = self.parse_expression()?;
        self.expect(TokenType::RightParen, "Expected ')' after if condition")?;
        
        let then_branch = Box::new(self.parse_statement()?);
        
        let else_branch = if self.match_token(TokenType::Else) {
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };
        
        Ok(Statement::If {
            condition,
            then_branch,
            else_branch,
            position,
        })
    }
    
    /// Parse while statement
    fn parse_while_statement(&mut self) -> Result<Statement, ParseError> {
        let while_token = self.expect(TokenType::While, "Expected 'while'")?;
        let position = Position::new(while_token.line, while_token.column);
        
        self.expect(TokenType::LeftParen, "Expected '(' after 'while'")?;
        let condition = self.parse_expression()?;
        self.expect(TokenType::RightParen, "Expected ')' after while condition")?;
        
        let body = Box::new(self.parse_statement()?);
        
        Ok(Statement::While {
            condition,
            body,
            position,
        })
    }
    
    /// Parse for statement
    fn parse_for_statement(&mut self) -> Result<Statement, ParseError> {
        let for_token = self.expect(TokenType::For, "Expected 'for'")?;
        let position = Position::new(for_token.line, for_token.column);

        self.expect(TokenType::LeftParen, "Expected '(' after 'for'")?;

        // Check if this is a range-based for loop: for (variable : range)
        if let Some(token) = self.peek() {
            if let TokenType::Identifier(_) = token.token_type {
                // Look ahead to see if there's a colon after the identifier
                if let Some(next_token) = self.peek_ahead(1) {
                    if next_token.token_type == TokenType::Colon {
                        // This is a range-based for loop
                        let var_token = self.advance().unwrap();
                        let variable = if let TokenType::Identifier(name) = var_token.token_type {
                            name
                        } else {
                            return Err(ParseError::new("Expected variable name".to_string(), position));
                        };

                        self.expect(TokenType::Colon, "Expected ':' after variable")?;
                        let range = self.parse_expression()?;
                        self.expect(TokenType::RightParen, "Expected ')' after range")?;

                        let body = Box::new(self.parse_statement()?);

                        return Ok(Statement::ForRange {
                            variable,
                            range,
                            body,
                            position,
                        });
                    }
                }
            }
        }

        // Traditional for loop: for (init; condition; increment)
        let initializer = if self.check(&TokenType::Semicolon) {
            None
        } else {
            Some(Box::new(self.parse_statement()?))
        };

        let condition = if self.check(&TokenType::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect(TokenType::Semicolon, "Expected ';' after for condition")?;

        let increment = if self.check(&TokenType::RightParen) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect(TokenType::RightParen, "Expected ')' after for clauses")?;

        let body = Box::new(self.parse_statement()?);

        Ok(Statement::For {
            initializer,
            condition,
            increment,
            body,
            position,
        })
    }
    
    /// Parse switch statement
    fn parse_switch_statement(&mut self) -> Result<Statement, ParseError> {
        let switch_token = self.advance().unwrap(); // consume 'switch'
        let position = Position::new(switch_token.line, switch_token.column);

        self.expect(TokenType::LeftParen, "Expected '(' after 'switch'")?;
        let expression = self.parse_expression()?;
        self.expect(TokenType::RightParen, "Expected ')' after switch expression")?;

        self.expect(TokenType::LeftBrace, "Expected '{' after switch expression")?;

        let mut cases = Vec::new();
        let mut default_case = None;

        self.skip_newlines();

        while !self.check(&TokenType::RightBrace) && !self.check(&TokenType::Eof) {
            if self.match_token(TokenType::Case) {
                let case_value = self.parse_expression()?;
                self.expect(TokenType::Colon, "Expected ':' after case value")?;

                // Parse case body (can be multiple statements)
                let mut case_statements = Vec::new();
                self.skip_newlines();

                while !self.check(&TokenType::Case) &&
                      !self.check(&TokenType::Default) &&
                      !self.check(&TokenType::RightBrace) &&
                      !self.check(&TokenType::Eof) {
                    case_statements.push(self.parse_statement()?);
                    self.skip_newlines();
                }

                let case_body = if case_statements.len() == 1 {
                    case_statements.into_iter().next().unwrap()
                } else {
                    Statement::Block {
                        statements: case_statements,
                        position: position.clone(),
                    }
                };

                cases.push(CaseClause {
                    value: case_value,
                    body: case_body,
                    position: position.clone(),
                });

            } else if self.match_token(TokenType::Default) {
                if default_case.is_some() {
                    return Err(ParseError::new(
                        "Multiple default cases in switch statement".to_string(),
                        position
                    ));
                }

                self.expect(TokenType::Colon, "Expected ':' after 'default'")?;

                // Parse default body (can be multiple statements)
                let mut default_statements = Vec::new();
                self.skip_newlines();

                while !self.check(&TokenType::Case) &&
                      !self.check(&TokenType::Default) &&
                      !self.check(&TokenType::RightBrace) &&
                      !self.check(&TokenType::Eof) {
                    default_statements.push(self.parse_statement()?);
                    self.skip_newlines();
                }

                let default_body = if default_statements.len() == 1 {
                    default_statements.into_iter().next().unwrap()
                } else {
                    Statement::Block {
                        statements: default_statements,
                        position: position.clone(),
                    }
                };

                default_case = Some(Box::new(default_body));

            } else {
                return Err(ParseError::new(
                    "Expected 'case' or 'default' in switch statement".to_string(),
                    position
                ));
            }
        }

        self.expect(TokenType::RightBrace, "Expected '}' after switch body")?;

        Ok(Statement::Switch {
            expression,
            cases,
            default_case,
            position,
        })
    }

    /// Parse block statement
    fn parse_block_statement(&mut self) -> Result<Statement, ParseError> {
        let brace_token = self.expect(TokenType::LeftBrace, "Expected '{'")?;
        let position = Position::new(brace_token.line, brace_token.column);
        
        let mut statements = Vec::new();
        self.skip_newlines();
        
        while !self.check(&TokenType::RightBrace) && !self.check(&TokenType::Eof) {
            statements.push(self.parse_statement()?);
            self.skip_newlines();
        }
        
        self.expect(TokenType::RightBrace, "Expected '}' after block")?;
        
        Ok(Statement::Block { statements, position })
    }
    
    /// Parse expression (placeholder - will be expanded)
    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_assignment()
    }
    
    /// Parse assignment expression
    fn parse_assignment(&mut self) -> Result<Expression, ParseError> {
        let expr = self.parse_logical_or()?;

        if self.match_token(TokenType::Assign) {
            let position = expr.position().clone();
            let right = self.parse_assignment()?;
            return Ok(Expression::Binary {
                left: Box::new(expr),
                operator: BinaryOperator::Assign,
                right: Box::new(right),
                position,
            });
        }

        Ok(expr)
    }
    
    /// Parse logical OR expression
    fn parse_logical_or(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_logical_and()?;

        while self.match_token(TokenType::Or) {
            let position = expr.position().clone();
            let right = self.parse_logical_and()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator: BinaryOperator::Or,
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }
    
    /// Parse logical AND expression
    fn parse_logical_and(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_equality()?;

        while self.match_token(TokenType::And) {
            let position = expr.position().clone();
            let right = self.parse_equality()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator: BinaryOperator::And,
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }
    
    /// Parse equality expression
    fn parse_equality(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_comparison()?;

        while let Some(token) = self.peek() {
            let operator = match token.token_type {
                TokenType::Equal => {
                    self.advance();
                    BinaryOperator::Equal
                }
                TokenType::NotEqual => {
                    self.advance();
                    BinaryOperator::NotEqual
                }
                _ => break,
            };

            let position = expr.position().clone();
            let right = self.parse_comparison()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }
    
    /// Parse comparison expression
    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_range()?;

        while let Some(token) = self.peek() {
            let operator = match token.token_type {
                TokenType::Greater => {
                    self.advance();
                    BinaryOperator::Greater
                }
                TokenType::GreaterEqual => {
                    self.advance();
                    BinaryOperator::GreaterEqual
                }
                TokenType::Less => {
                    self.advance();
                    BinaryOperator::Less
                }
                TokenType::LessEqual => {
                    self.advance();
                    BinaryOperator::LessEqual
                }
                _ => break,
            };

            let position = expr.position().clone();
            let right = self.parse_term()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }

    /// Parse range expression (start...end)
    fn parse_range(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_term()?;

        if self.match_token(TokenType::Range) {
            let position = expr.position().clone();
            let end = self.parse_term()?;

            expr = Expression::Range {
                start: Box::new(expr),
                end: Box::new(end),
                position,
            };
        }

        Ok(expr)
    }

    /// Parse term expression (+ -)
    fn parse_term(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_factor()?;

        while let Some(token) = self.peek() {
            let operator = match token.token_type {
                TokenType::Plus => {
                    self.advance();
                    BinaryOperator::Add
                }
                TokenType::Minus => {
                    self.advance();
                    BinaryOperator::Subtract
                }
                _ => break,
            };

            let position = expr.position().clone();
            let right = self.parse_factor()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }
    
    /// Parse factor expression (* / %)
    fn parse_factor(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_unary()?;

        while let Some(token) = self.peek() {
            let operator = match token.token_type {
                TokenType::Multiply => {
                    self.advance();
                    BinaryOperator::Multiply
                }
                TokenType::Divide => {
                    self.advance();
                    BinaryOperator::Divide
                }
                TokenType::Modulo => {
                    self.advance();
                    BinaryOperator::Modulo
                }
                _ => break,
            };

            let position = expr.position().clone();
            let right = self.parse_unary()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }
    
    /// Parse unary expression
    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        if let Some(token) = self.peek() {
            let position = Position::new(token.line, token.column);
            
            match token.token_type {
                TokenType::Minus => {
                    self.advance();
                    let operand = self.parse_unary()?;
                    Ok(Expression::Unary {
                        operator: UnaryOperator::Minus,
                        operand: Box::new(operand),
                        position,
                    })
                }
                TokenType::Not => {
                    self.advance();
                    let operand = self.parse_unary()?;
                    Ok(Expression::Unary {
                        operator: UnaryOperator::Not,
                        operand: Box::new(operand),
                        position,
                    })
                }
                _ => self.parse_call(),
            }
        } else {
            Err(ParseError::new(
                "Expected expression, found EOF".to_string(),
                Position::new(0, 0)
            ))
        }
    }
    
    /// Parse call expression
    fn parse_call(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_token(TokenType::LeftParen) {
                // Function call
                let mut arguments = Vec::new();

                if !self.check(&TokenType::RightParen) {
                    loop {
                        arguments.push(self.parse_expression()?);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                }

                self.expect(TokenType::RightParen, "Expected ')' after arguments")?;

                let position = expr.position().clone();
                expr = Expression::Call {
                    function: Box::new(expr),
                    arguments,
                    position,
                };
            } else if self.match_token(TokenType::LeftBracket) {
                // Array access
                let index = self.parse_expression()?;
                self.expect(TokenType::RightBracket, "Expected ']' after array index")?;

                let position = expr.position().clone();
                expr = Expression::ArrayAccess {
                    array: Box::new(expr),
                    index: Box::new(index),
                    position,
                };
            } else if self.match_token(TokenType::Dot) {
                // Field access
                let field_token = self.peek().ok_or_else(|| ParseError::new("Expected field name after '.'".to_string(),
                    Position::new(0, 0)))?;
                let field_name = if let TokenType::Identifier(name) = &field_token.token_type {
                    let name = name.clone();
                    self.advance();
                    name
                } else {
                    return Err(ParseError::new("Expected identifier after '.'".to_string(),
                        Position::new(field_token.line, field_token.column)));
                };

                let position = expr.position().clone();
                expr = Expression::FieldAccess {
                    object: Box::new(expr),
                    field: field_name,
                    position,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }
    
    /// Parse primary expression
    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        if let Some(token) = self.peek() {
            let position = Position::new(token.line, token.column);
            
            match &token.token_type {
                TokenType::Integer(value) => {
                    let value = *value;
                    self.advance();
                    Ok(Expression::Integer(value, position))
                }
                TokenType::Float(value) => {
                    let value = *value;
                    self.advance();
                    Ok(Expression::Float(value, position))
                }
                TokenType::String(value) => {
                    let value = value.clone();
                    self.advance();
                    Ok(Expression::String(value, position))
                }
                TokenType::True => {
                    self.advance();
                    Ok(Expression::Boolean(true, position))
                }
                TokenType::False => {
                    self.advance();
                    Ok(Expression::Boolean(false, position))
                }
                TokenType::Null => {
                    self.advance();
                    Ok(Expression::Null(position))
                }
                TokenType::Identifier(name) => {
                    let name = name.clone();
                    self.advance();
                    
                    // Check for namespace access (support multiple levels)
                    if self.match_token(TokenType::DoubleColon) {
                        let mut namespace_path = vec![name];

                        // Parse the rest of the namespace path
                        loop {
                            let member_token = self.expect(TokenType::Identifier(String::new()), "Expected member name after '::'")?;
                            if let TokenType::Identifier(member) = member_token.token_type {
                                // Check if there's another :: (more nesting)
                                if self.match_token(TokenType::DoubleColon) {
                                    namespace_path.push(member);
                                } else {
                                    // This is the final member name
                                    return Ok(Expression::NamespaceAccess {
                                        namespace_path,
                                        name: member,
                                        position,
                                    });
                                }
                            } else {
                                return Err(ParseError::new("Expected member name".to_string(), position));
                            }
                        }
                    } else {
                        Ok(Expression::Identifier(name, position))
                    }
                }
                TokenType::LeftParen => {
                    self.advance();
                    let expr = self.parse_expression()?;
                    self.expect(TokenType::RightParen, "Expected ')' after expression")?;
                    Ok(expr)
                }
                TokenType::LeftBracket => {
                    self.advance();
                    let mut elements = Vec::new();

                    // Handle empty array
                    if self.match_token(TokenType::RightBracket) {
                        return Ok(Expression::Array(elements, position));
                    }

                    // Parse array elements
                    loop {
                        elements.push(self.parse_expression()?);

                        if self.match_token(TokenType::Comma) {
                            continue;
                        } else if self.match_token(TokenType::RightBracket) {
                            break;
                        } else {
                            return Err(ParseError::new("Expected ',' or ']' in array literal".to_string(), position));
                        }
                    }

                    Ok(Expression::Array(elements, position))
                }
                TokenType::LeftBrace => {
                    self.advance();
                    let mut fields = Vec::new();

                    // Handle empty object
                    if self.match_token(TokenType::RightBrace) {
                        return Ok(Expression::Object(fields, position));
                    }

                    // Parse object fields
                    loop {
                        // Parse key (must be identifier or string)
                        let key = match &self.peek().unwrap().token_type {
                            TokenType::Identifier(name) => {
                                let key = name.clone();
                                self.advance();
                                key
                            }
                            TokenType::String(s) => {
                                let key = s.clone();
                                self.advance();
                                key
                            }
                            _ => return Err(ParseError::new("Expected identifier or string as object key".to_string(), position)),
                        };

                        // Expect colon
                        self.expect(TokenType::Colon, "Expected ':' after object key")?;

                        // Parse value
                        let value = self.parse_expression()?;
                        fields.push((key, value));

                        if self.match_token(TokenType::Comma) {
                            continue;
                        } else if self.match_token(TokenType::RightBrace) {
                            break;
                        } else {
                            return Err(ParseError::new("Expected ',' or '}' in object literal".to_string(), position));
                        }
                    }

                    Ok(Expression::Object(fields, position))
                }
                _ => {
                    Err(ParseError::new(
                        format!("Unexpected token {:?}", token.token_type),
                        position
                    ))
                }
            }
        } else {
            Err(ParseError::new(
                "Expected expression, found EOF".to_string(),
                Position::new(0, 0)
            ))
        }
    }

    /// Parse try statement
    fn parse_try_statement(&mut self) -> Result<Statement, ParseError> {
        let try_token = self.advance().unwrap(); // consume 'try'
        let position = Position::new(try_token.line, try_token.column);

        // Parse try block
        let try_block = Box::new(self.parse_statement()?);

        // Parse catch clauses
        let mut catch_clauses = Vec::new();
        while self.match_token(TokenType::Catch) {
            let catch_token = self.peek().unwrap();
            let catch_position = Position::new(catch_token.line, catch_token.column);

            // Parse optional exception variable and type
            let (exception_variable, exception_type) = if self.match_token(TokenType::LeftParen) {
                let var_name = if let Some(token) = self.peek() {
                    if let TokenType::Identifier(name) = &token.token_type {
                        let name = name.clone();
                        self.advance();
                        Some(name)
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Optional type annotation
                let exception_type = if self.match_token(TokenType::Colon) {
                    if let Some(token) = self.peek() {
                        if let TokenType::Identifier(type_name) = &token.token_type {
                            let type_name = type_name.clone();
                            self.advance();
                            Some(type_name)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                self.expect(TokenType::RightParen, "Expected ')' after catch clause")?;
                (var_name, exception_type)
            } else {
                (None, None)
            };

            // Parse catch body
            let body = Box::new(self.parse_statement()?);

            catch_clauses.push(CatchClause {
                exception_variable,
                exception_type,
                body,
                position: catch_position,
            });
        }

        // Parse optional finally block
        let finally_block = if self.match_token(TokenType::Finally) {
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        // Must have at least one catch or finally
        if catch_clauses.is_empty() && finally_block.is_none() {
            return Err(ParseError::new(
                "Try statement must have at least one catch or finally clause".to_string(),
                position
            ));
        }

        Ok(Statement::Try {
            try_block,
            catch_clauses,
            finally_block,
            position,
        })
    }

    /// Parse throw statement
    fn parse_throw_statement(&mut self) -> Result<Statement, ParseError> {
        let throw_token = self.advance().unwrap(); // consume 'throw'
        let position = Position::new(throw_token.line, throw_token.column);

        let value = self.parse_expression()?;
        self.expect(TokenType::Semicolon, "Expected ';' after throw statement")?;

        Ok(Statement::Throw {
            value,
            position,
        })
    }

    /// Check if we're at the end of tokens
    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::lexer::Lexer;
    
    #[test]
    fn test_parse_simple_function() {
        let source = "fn main() : int { return 42; }";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let program = parser.parse_program().unwrap();
        assert_eq!(program.declarations.len(), 1);
        
        if let Declaration::Function(func) = &program.declarations[0] {
            assert_eq!(func.name, "main");
            assert_eq!(func.parameters.len(), 0);
            assert!(matches!(func.return_type, Some(TypeAnnotation::Int)));
        } else {
            panic!("Expected function declaration");
        }
    }
    
    #[test]
    fn test_parse_variable_declaration() {
        let source = "var x : int = 42;";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let stmt = parser.parse_statement().unwrap();
        if let Statement::VarDeclaration { scope, name, type_annotation, .. } = stmt {
            assert!(matches!(scope, VarScope::Var));
            assert_eq!(name, "x");
            assert!(matches!(type_annotation, Some(TypeAnnotation::Int)));
        } else {
            panic!("Expected variable declaration");
        }
    }
}
