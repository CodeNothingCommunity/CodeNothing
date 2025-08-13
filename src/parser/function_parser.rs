// 函数解析模块
// 包含函数解析相关函数

use crate::ast::{Function, Parameter, GenericParameter, TypeConstraint};
use crate::parser::parser_base::ParserBase;
use crate::parser::parser_utils::skip_to_next_statement_or_end;
use crate::parser::statement_parser::StatementParser;
use crate::parser::expression_parser::ExpressionParser;

/// 解析函数
pub fn parse_function(parser: &mut ParserBase) -> Result<Function, String> {
    parser.expect("fn")?;

    let name = match parser.consume() {
        Some(name) => name,
        None => return Err("期望函数名".to_string()),
    };

    // 解析泛型参数 (可选) - 使用试探性解析
    let generic_parameters = if parser.peek() == Some(&"<".to_string()) {
        // 保存当前位置
        let saved_position = parser.position;

        // 尝试解析泛型参数
        match parser.parse_generic_parameters() {
            Ok(params) => params,
            Err(_) => {
                // 泛型解析失败，恢复位置
                parser.position = saved_position;
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    parser.expect("(")?;
    
    // 解析函数参数
    let mut parameters = Vec::new();
    if parser.peek() != Some(&")".to_string()) {
        // 至少有一个参数
        let param_name = parser.consume().ok_or_else(|| "期望参数名".to_string())?;
        parser.expect(":")?;
        let param_type = parser.parse_type()?;
        
        // 检查是否有默认值
        let default_value = if parser.peek() == Some(&"=".to_string()) {
            parser.consume(); // 消费等号
            Some(parser.parse_expression()?)
        } else {
            None
        };
        
        parameters.push(Parameter {
            name: param_name,
            param_type,
            default_value,
        });
        
        // 解析剩余参数
        while parser.peek() == Some(&",".to_string()) {
            parser.consume(); // 消费逗号
            let param_name = parser.consume().ok_or_else(|| "期望参数名".to_string())?;
            parser.expect(":")?;
            let param_type = parser.parse_type()?;
            
            // 检查是否有默认值
            let default_value = if parser.peek() == Some(&"=".to_string()) {
                parser.consume(); // 消费等号
                Some(parser.parse_expression()?)
            } else {
                None
            };
            
            parameters.push(Parameter {
                name: param_name,
                param_type,
                default_value,
            });
        }
    }
    
    parser.expect(")")?;
    
    parser.expect(":")?;
    let return_type = parser.parse_type()?;

    // 解析 where 子句 (可选)
    let where_clause = parser.parse_where_clause()?;

    parser.expect("{")?;
    
    let mut body = Vec::new();
    while let Some(token) = parser.peek() {
        if token == "}" {
            break;
        }
        body.push(parser.parse_statement()?);
    }
    
    if parser.peek() != Some(&"}".to_string()) {
        return Err(format!("期望 '}}', 但得到了 {:?}", parser.peek()));
    }
    parser.consume(); // 消费 "}"
    
    if parser.peek() != Some(&";".to_string()) {
        return Err(format!("在函数 '{}' 定义末尾期望 ';', 但得到了 {:?}", name, parser.peek()));
    }
    parser.consume(); // 消费 ";"
    
    Ok(Function {
        name,
        generic_parameters,
        parameters,
        return_type,
        body,
        where_clause,
    })
}

/// 收集函数解析错误
pub fn parse_function_collect_errors(parser: &mut ParserBase, errors: &mut Vec<String>) -> Result<Function, ()> {
    if let Err(e) = parser.expect("fn") {
        errors.push(e);
        return Err(());
    }
    
    let name = match parser.consume() {
        Some(name) => name,
        None => {
            errors.push("期望函数名".to_string());
            return Err(());
        }
    };
    
    if let Err(e) = parser.expect("(") {
        errors.push(e);
        return Err(());
    }
    
    // 解析函数参数
    let mut parameters = Vec::new();
    if parser.peek() != Some(&")".to_string()) {
        // 至少有一个参数
        let param_name = match parser.consume() {
            Some(name) => name,
            None => {
                errors.push("期望参数名".to_string());
                return Err(());
            }
        };
        
        if let Err(e) = parser.expect(":") {
            errors.push(e);
            return Err(());
        }
        
        let param_type = match parser.parse_type() {
            Ok(t) => t,
            Err(e) => {
                errors.push(e);
                return Err(());
            }
        };
        
        // 检查是否有默认值
        let default_value = if parser.peek() == Some(&"=".to_string()) {
            parser.consume(); // 消费等号
            match parser.parse_expression() {
                Ok(expr) => Some(expr),
                Err(e) => {
                    errors.push(e);
                    return Err(());
                }
            }
        } else {
            None
        };
        
        parameters.push(Parameter {
            name: param_name,
            param_type,
            default_value,
        });
        
        // 解析剩余参数
        while parser.peek() == Some(&",".to_string()) {
            parser.consume(); // 消费逗号
            
            let param_name = match parser.consume() {
                Some(name) => name,
                None => {
                    errors.push("期望参数名".to_string());
                    return Err(());
                }
            };
            
            if let Err(e) = parser.expect(":") {
                errors.push(e);
                return Err(());
            }
            
            let param_type = match parser.parse_type() {
                Ok(t) => t,
                Err(e) => {
                    errors.push(e);
                    return Err(());
                }
            };
            
            // 检查是否有默认值
            let default_value = if parser.peek() == Some(&"=".to_string()) {
                parser.consume(); // 消费等号
                match parser.parse_expression() {
                    Ok(expr) => Some(expr),
                    Err(e) => {
                        errors.push(e);
                        return Err(());
                    }
                }
            } else {
                None
            };
            
            parameters.push(Parameter {
                name: param_name,
                param_type,
                default_value,
            });
        }
    }
    
    if let Err(e) = parser.expect(")") {
        errors.push(e);
        return Err(());
    }
    
    if let Err(e) = parser.expect(":") {
        errors.push(e);
        return Err(());
    }
    
    let return_type = match parser.parse_type() {
        Ok(t) => t,
        Err(e) => {
            errors.push(e);
            return Err(());
        }
    };
    
    if let Err(e) = parser.expect("{") {
        errors.push(e);
        return Err(());
    }
    
    let mut body = Vec::new();
    let mut brace_count = 1; // 我们已经消费了开括号
    
    while parser.position < parser.tokens.len() {
        if parser.peek() == Some(&"}".to_string()) {
            brace_count -= 1;
            if brace_count == 0 {
                // 找到了匹配的右括号
                break;
            }
            parser.consume();
        } else if parser.peek() == Some(&"{".to_string()) {
            brace_count += 1;
            parser.consume();
        } else {
            match parser.parse_statement() {
                Ok(stmt) => body.push(stmt),
                Err(e) => {
                    errors.push(e);
                    // 跳过到下一个语句的开始，或者函数结束
                    skip_to_next_statement_or_end(parser);
                    if parser.peek() == Some(&"}".to_string()) && brace_count == 1 {
                        break;
                    }
                }
            }
        }
    }
    
    if parser.peek() != Some(&"}".to_string()) {
        errors.push(format!("期望 '}}', 但得到了 {:?}", parser.peek()));
        return Err(());
    }
    parser.consume(); // 消费 "}"
    
    if parser.peek() != Some(&";".to_string()) {
        errors.push(format!("在函数 '{}' 定义末尾期望 ';', 但得到了 {:?}", name, parser.peek()));
        return Err(());
    }
    parser.consume(); // 消费 ";"
    
    Ok(Function {
        name,
        generic_parameters: Vec::new(), // 暂时不支持泛型
        parameters,
        return_type,
        body,
        where_clause: Vec::new(),
    })
} 