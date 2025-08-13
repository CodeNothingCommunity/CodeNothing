// 类解析模块
use crate::ast::{Class, Field, Method, Constructor, Parameter, Type, Visibility, GenericParameter, TypeConstraint};
use crate::parser::parser_base::ParserBase;
use crate::parser::statement_parser::StatementParser;
use crate::parser::expression_parser::ExpressionParser;

pub trait ClassParser {
    fn parse_class(&mut self) -> Result<Class, String>;
    fn parse_visibility(&mut self) -> (Visibility, bool, bool, bool, bool);
    fn parse_field(&mut self) -> Result<Field, String>;
    fn parse_method(&mut self) -> Result<Method, String>;
    fn parse_constructor(&mut self) -> Result<Constructor, String>;
    fn is_field_declaration(&mut self) -> bool;
    fn try_parse_field(&mut self) -> Result<Field, String>;
}

impl<'a> ClassParser for ParserBase<'a> {
    fn parse_class(&mut self) -> Result<Class, String> {
        // 检查是否为抽象类
        let is_abstract = if self.peek() == Some(&"abstract".to_string()) {
            self.consume(); // 消费 "abstract"
            true
        } else {
            false
        };
        
        // 消费 "class" 关键字
        self.consume(); // class
        
        // 获取类名
        let class_name = self.consume().ok_or_else(|| "期望类名".to_string())?;

        // 解析泛型参数 (可选) - 使用试探性解析
        let generic_parameters = if self.peek() == Some(&"<".to_string()) {
            // 保存当前位置
            let saved_position = self.position;

            // 尝试解析泛型参数
            match self.parse_generic_parameters() {
                Ok(params) => params,
                Err(_) => {
                    // 泛型解析失败，恢复位置
                    self.position = saved_position;
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };
        
        // 检查是否有继承
        let super_class = if self.peek() == Some(&"extends".to_string()) {
            self.consume(); // 消费 "extends"
            Some(self.consume().ok_or_else(|| "期望父类名".to_string())?)
        } else {
            None
        };
        
        // 检查是否实现接口 (implements Interface1, Interface2)
        let mut implements = Vec::new();
        if self.peek() == Some(&"implements".to_string()) {
            self.consume(); // 消费 "implements"
            
            // 解析实现的接口列表
            loop {
                let interface_name = self.consume().ok_or_else(|| "期望接口名".to_string())?;
                implements.push(interface_name);
                
                if self.peek() != Some(&",".to_string()) {
                    break;
                }
                self.consume(); // 消费 ","
            }
        }
        
        // 解析类体
        self.expect("{")?;
        
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut constructors = Vec::new();
        
        while self.peek() != Some(&"}".to_string()) {
            // 解析访问修饰符和其他修饰符
            let (visibility, is_static, is_virtual, is_override, is_abstract) = self.parse_visibility();
            
            // 获取下一个token
            let next_token = self.peek().cloned();
            
            match next_token.as_deref() {
                Some("constructor") => {
                    // 解析构造函数
                    let constructor = self.parse_constructor()?;
                    constructors.push(constructor);
                },
                Some("fn") => {
                    // 解析方法
                    let mut method = self.parse_method()?;
                    method.visibility = visibility;
                    method.is_static = is_static;
                    method.is_virtual = is_virtual;
                    method.is_override = is_override;
                    method.is_abstract = is_abstract;
                    methods.push(method);
                },
                Some(_) => {
                    // 尝试解析字段
                    match self.try_parse_field() {
                        Ok(mut field) => {
                            field.visibility = visibility;
                            field.is_static = is_static;
                            fields.push(field);
                        },
                        Err(e) => {
                            return Err(format!("解析类成员失败: {}", e));
                        }
                    }
                },
                None => {
                    return Err("期望字段、方法或构造函数".to_string());
                }
            }
        }

        self.expect("}")?;

        // 解析 where 子句 (可选)
        let where_clause = self.parse_where_clause()?;

        self.expect(";")?;

        Ok(Class {
            name: class_name,
            generic_parameters,
            super_class,
            implements,
            fields,
            methods,
            constructors,
            is_abstract,
            friends: Vec::new(), // v0.7.2新增：暂时为空，后续实现友元解析
            where_clause,
        })
    }
    
    fn parse_visibility(&mut self) -> (Visibility, bool, bool, bool, bool) {
        let mut visibility = Visibility::Public;
        let mut is_static = false;
        let mut is_virtual = false;
        let mut is_override = false;
        let mut is_abstract = false;
        
        // 解析修饰符
        while let Some(token) = self.peek() {
            match token.as_str() {
                "private" => {
                    self.consume();
                    visibility = Visibility::Private;
                },
                "protected" => {
                    self.consume();
                    visibility = Visibility::Protected;
                },
                "public" => {
                    self.consume();
                    visibility = Visibility::Public;
                },
                "static" => {
                    self.consume();
                    is_static = true;
                },
                "virtual" => {
                    self.consume();
                    is_virtual = true;
                },
                "override" => {
                    self.consume();
                    is_override = true;
                },
                "abstract" => {
                    self.consume();
                    is_abstract = true;
                },
                _ => break,
            }
        }
        
        (visibility, is_static, is_virtual, is_override, is_abstract)
    }
    
    fn is_field_declaration(&mut self) -> bool {
        let current_pos = self.position;
        
        // 跳过可能的标识符
        if self.peek().is_some() {
            self.consume();
            // 检查是否有冒号
            let has_colon = self.peek() == Some(&":".to_string());
            // 恢复位置
            self.position = current_pos;
            has_colon
        } else {
            false
        }
    }
    
    fn try_parse_field(&mut self) -> Result<Field, String> {
        // 保存当前位置以便回退
        let start_pos = self.position;
        
        // 尝试解析字段名
        let field_name = match self.consume() {
            Some(name) => name,
            None => {
                self.position = start_pos;
                return Err("期望字段名".to_string());
            }
        };
        
        // 检查是否有冒号
        if self.peek() != Some(&":".to_string()) {
            self.position = start_pos;
            return Err(format!("期望字段声明，但在 '{}' 后没有找到 ':'", field_name));
        }
        
        // 继续解析字段
        self.expect(":")?;
        
        // 字段类型
        let field_type = self.parse_type()?;
        
        // 可选的初始值
        let initial_value = if self.peek() == Some(&"=".to_string()) {
            self.consume(); // 消费 "="
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.expect(";")?;
        
        Ok(Field {
            name: field_name,
            field_type,
            visibility: Visibility::Public, // 将在调用处设置
            initial_value,
            is_static: false, // 将在调用处设置
        })
    }
    
    fn parse_field(&mut self) -> Result<Field, String> {
        // 字段名
        let field_name = self.consume().ok_or_else(|| "期望字段名".to_string())?;
        
        self.expect(":")?;
        
        // 字段类型
        let field_type = self.parse_type()?;
        
        // 可选的初始值
        let initial_value = if self.peek() == Some(&"=".to_string()) {
            self.consume(); // 消费 "="
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.expect(";")?;
        
        Ok(Field {
            name: field_name,
            field_type,
            visibility: Visibility::Public, // 将在调用处设置
            initial_value,
            is_static: false, // 将在调用处设置
        })
    }
    
    fn parse_method(&mut self) -> Result<Method, String> {
        self.consume(); // 消费 "fn"
        
        // 方法名
        let method_name = self.consume().ok_or_else(|| "期望方法名".to_string())?;

        // 解析泛型参数 (可选) - 使用试探性解析
        let generic_parameters = if self.peek() == Some(&"<".to_string()) {
            // 保存当前位置
            let saved_position = self.position;

            // 尝试解析泛型参数
            match self.parse_generic_parameters() {
                Ok(params) => params,
                Err(_) => {
                    // 泛型解析失败，恢复位置
                    self.position = saved_position;
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };
        
        // 解析参数列表
        self.expect("(")?;
        let mut parameters = Vec::new();
        
        if self.peek() != Some(&")".to_string()) {
            loop {
                let param_name = self.consume().ok_or_else(|| "期望参数名".to_string())?;
                self.expect(":")?;
                let param_type = self.parse_type()?;
                
                parameters.push(Parameter {
                    name: param_name.clone(),
                    param_type,
                    default_value: None,
                });
                
                if self.peek() != Some(&",".to_string()) {
                    break;
                }
                self.consume(); // 消费 ","
            }
        }
        
        self.expect(")")?;
        self.expect(":")?;
        
        // 返回类型
        let return_type = self.parse_type()?;

        // 解析 where 子句 (可选)
        let where_clause = self.parse_where_clause()?;

        // 方法体（抽象方法可能没有方法体）
        let body = if self.peek() == Some(&";".to_string()) {
            // 抽象方法，没有方法体
            self.consume(); // 消费 ";"
            Vec::new()
        } else {
            // 普通方法，有方法体
            self.expect("{")?;
            let mut body = Vec::new();
            
            while self.peek() != Some(&"}".to_string()) {
                let stmt = self.parse_statement()?;
                body.push(stmt);
            }
            
            self.expect("}")?;
            self.expect(";")?;
            body
        };
        
        Ok(Method {
            name: method_name,
            generic_parameters,
            parameters,
            return_type,
            body,
            visibility: Visibility::Public, // 将在调用处设置
            is_static: false, // 将在调用处设置
            is_virtual: false, // 将在调用处设置
            is_override: false, // 将在调用处设置
            is_abstract: false, // 将在调用处设置
            where_clause,
        })
    }
    
    fn parse_constructor(&mut self) -> Result<Constructor, String> {
        self.consume(); // 消费 "constructor"

        // 解析泛型参数 (可选) - 使用试探性解析
        let generic_parameters = if self.peek() == Some(&"<".to_string()) {
            // 保存当前位置
            let saved_position = self.position;

            // 尝试解析泛型参数
            match self.parse_generic_parameters() {
                Ok(params) => params,
                Err(_) => {
                    // 泛型解析失败，恢复位置
                    self.position = saved_position;
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        // 解析参数列表
        self.expect("(")?;
        let mut parameters = Vec::new();
        
        if self.peek() != Some(&")".to_string()) {
            loop {
                let param_name = self.consume().ok_or_else(|| "期望参数名".to_string())?;
                self.expect(":")?;
                let param_type = self.parse_type()?;
                
                parameters.push(Parameter {
                    name: param_name,
                    param_type,
                    default_value: None,
                });
                
                if self.peek() != Some(&",".to_string()) {
                    break;
                }
                self.consume(); // 消费 ","
            }
        }
        
        self.expect(")")?;
        
        // 构造函数体
        self.expect("{")?;
        let mut body = Vec::new();
        
        while self.peek() != Some(&"}".to_string()) {
            let stmt = self.parse_statement()?;
            body.push(stmt);
        }
        
        self.expect("}")?;
        self.expect(";")?;
        
        Ok(Constructor {
            generic_parameters,
            parameters,
            body,
        })
    }
}