use crate::interpreter::debug_println;
use crate::ast::{StringInterpolationSegment, Expression};

// 词法分析器：负责将源代码转换为词法单元（tokens）

// 移除注释
pub fn remove_comments(source: &str) -> String {
    let mut result = String::new();
    let mut in_single_line_comment = false;
    let mut multi_line_comment_depth = 0; // 使用计数器跟踪多行注释的嵌套深度
    let mut in_string = false; // 标记是否在字符串内
    let mut in_backtick_string = false; // 标记是否在反引号字符串内
    let mut escape = false; // 标记是否是转义字符
    let mut i = 0;

    let chars: Vec<char> = source.chars().collect();

    while i < chars.len() {
        // 处理双引号字符串
        if in_string {
            result.push(chars[i]);
            if escape {
                // 转义字符后的字符直接添加
                escape = false;
            } else if chars[i] == '\\' {
                // 转义字符标记
                escape = true;
            } else if chars[i] == '"' {
                // 字符串结束
                in_string = false;
            }
            i += 1;
            continue;
        } else if chars[i] == '"' && !in_backtick_string && !in_single_line_comment && multi_line_comment_depth == 0 {
            // 字符串开始（只有在非注释状态下才处理）
            in_string = true;
            result.push(chars[i]);
            i += 1;
            continue;
        }

        // 处理反引号字符串
        if in_backtick_string {
            result.push(chars[i]);
            if chars[i] == '`' {
                // 反引号字符串结束
                in_backtick_string = false;
            }
            i += 1;
            continue;
        } else if chars[i] == '`' && !in_single_line_comment && multi_line_comment_depth == 0 {
            // 反引号字符串开始（只有在非注释状态下才处理）
            in_backtick_string = true;
            result.push(chars[i]);
            i += 1;
            continue;
        }

        // 处理注释（只有在非字符串状态下才处理）
        if !in_string && !in_backtick_string {
            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' && multi_line_comment_depth == 0 {
                // 单行注释开始（仅当不在多行注释中时）
                in_single_line_comment = true;
                i += 2;
                continue;
            } else if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '!' && !in_single_line_comment {
                // 多行注释开始 /!
                multi_line_comment_depth += 1;
                i += 2;
                continue;
            } else if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' && !in_single_line_comment {
                // 多行注释开始 /* (作为 /! 的别名)
                multi_line_comment_depth += 1;
                i += 2;
                continue;
            } else if in_single_line_comment && chars[i] == '\n' {
                // 单行注释结束
                in_single_line_comment = false;
                result.push(chars[i]);
                i += 1;
                continue;
            } else if i + 1 < chars.len() && chars[i] == '!' && chars[i + 1] == '/' && !in_single_line_comment {
                // 多行注释结束 !/
                if multi_line_comment_depth > 0 {
                    multi_line_comment_depth -= 1;
                }
                i += 2;
                continue;
            } else if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '/' && !in_single_line_comment {
                // 多行注释结束 */ (作为 !/ 的别名)
                if multi_line_comment_depth > 0 {
                    multi_line_comment_depth -= 1;
                }
                i += 2;
                continue;
            }
        }

        if !in_single_line_comment && multi_line_comment_depth == 0 {
            // 非注释内容
            result.push(chars[i]);
        }
        // 在注释内，跳过（不添加到result）
        i += 1;
    }

    result
}

// 用于表示不同类型的词法单元
#[derive(Debug, Clone)]
pub enum Token {
    StringLiteral(String),
    StringInterpolation(Vec<StringInterpolationSegment>),
    Symbol(String),
}

// 词法分析：将源代码转换为词法单元
pub fn tokenize(source: &str, debug: bool) -> Vec<String> {
    // 1. 移除注释
    let source_without_comments = remove_comments(source);
    
    // 2. 处理字符串字面量和字符串插值
    let mut tokens = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = source_without_comments.chars().collect();
    
    while i < chars.len() {
        let c = chars[i];
        
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        
        // 处理双引号字符串
        if c == '"' {
            i += 1;
            let mut string_content = String::new();
            let mut escape = false;
            
            while i < chars.len() {
                if escape {
                    match chars[i] {
                        'n' => string_content.push('\n'),
                        't' => string_content.push('\t'),
                        'r' => string_content.push('\r'),
                        '\\' => string_content.push('\\'),
                        '"' => string_content.push('"'),
                        _ => string_content.push(chars[i]),
                    }
                    escape = false;
                } else if chars[i] == '\\' {
                    escape = true;
                } else if chars[i] == '"' {
                    break;
                } else {
                    string_content.push(chars[i]);
                }
                i += 1;
            }
            
            if i < chars.len() && chars[i] == '"' {
                i += 1;
            }
            
            tokens.push(format!("\"{}\"", string_content));
            continue;
        }
        
        // 处理反引号字符串（字符串插值）
        if c == '`' {
            i += 1;
            let mut string_parts = Vec::new();
            let mut current_text = String::new();
            
            while i < chars.len() && chars[i] != '`' {
                if i + 1 < chars.len() && chars[i] == '$' && chars[i + 1] == '{' {
                    // 如果当前有文本，添加为文本片段
                    if !current_text.is_empty() {
                        string_parts.push(format!("INTERP_TEXT:{}", current_text));
                        current_text = String::new();
                    }
                    
                    i += 2; // 跳过 ${ 
                    
                    // 记录插值表达式的开始位置
                    let expr_start = i;
                    let mut brace_count = 1;
                    
                    // 寻找匹配的右花括号
                    while i < chars.len() && brace_count > 0 {
                        if chars[i] == '{' {
                            brace_count += 1;
                        } else if chars[i] == '}' {
                            brace_count -= 1;
                        }
                        i += 1;
                    }
                    
                    // 提取表达式部分
                    if brace_count == 0 {
                        i -= 1; // 回退一步，刚好在 } 上
                        let expr_text = chars[expr_start..i].iter().collect::<String>();
                        string_parts.push(format!("INTERP_EXPR:{}", expr_text));
                        i += 1; // 跳过 }
                    } else {
                        // 错误：未闭合的花括号
                        tokens.push("ERROR_UNCLOSED_BRACE".to_string());
                        return tokens;
                    }
                } else {
                    current_text.push(chars[i]);
                    i += 1;
                }
            }
            
            // 添加最后的文本片段（如果有）
            if !current_text.is_empty() {
                string_parts.push(format!("INTERP_TEXT:{}", current_text));
            }
            
            // 跳过结束的反引号
            if i < chars.len() && chars[i] == '`' {
                i += 1;
            }
            
            // 添加特殊的字符串插值标记
            tokens.push("INTERP_START".to_string());
            for part in string_parts {
                tokens.push(part);
            }
            tokens.push("INTERP_END".to_string());
            continue;
        }
        
        // 检查多字符运算符
        if i + 1 < chars.len() {
            let two_char_op = format!("{}{}", chars[i], chars[i + 1]);
            // v0.7.2新增：添加位运算符 << 和 >>
            if ["==", "!=", ">=", "<=", "&&", "||", "::", "..", "++", "--", "+=", "-=", "*=", "/=", "%=", "=>", "->", "<<", ">>"].contains(&two_char_op.as_str()) {
                tokens.push(two_char_op);
                i += 2;
                continue;
            }
        }
        
        // 检查标识符或关键字
        if c.is_alphabetic() || c == '_' {
            let mut identifier = String::new();
            
            // 检查是否是原始字符串(r"...")
            if c == 'r' && i + 1 < chars.len() && chars[i + 1] == '"' {
                i += 2; // 跳过 r"
                let mut string_content = String::new();
                
                while i < chars.len() && chars[i] != '"' {
                    string_content.push(chars[i]);
                    i += 1;
                }
                
                if i < chars.len() && chars[i] == '"' {
                    i += 1;
                }
                
                tokens.push(format!("r\"{}\"", string_content));
                continue;
            }
            
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                identifier.push(chars[i]);
                i += 1;
            }
            tokens.push(identifier);
            continue;
        }
        
        // 检查数字（包括科学计数法）
        if c.is_digit(10) || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_digit(10)) || (c == 'e' || c == 'E') {
            let mut number = String::new();
            let mut has_dot = c == '.';

            if has_dot {
                number.push('.');
                i += 1;
            }

            // 解析整数部分和小数部分
            while i < chars.len() && (chars[i].is_digit(10) || (chars[i] == '.' && !has_dot)) {
                if chars[i] == '.' {
                    // 检查是否是范围操作符
                    if i + 1 < chars.len() && chars[i + 1] == '.' {
                        break;
                    }
                    has_dot = true;
                }
                number.push(chars[i]);
                i += 1;
            }

            // 检查科学计数法 (e 或 E)
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                number.push(chars[i]);
                i += 1;

                // 检查可选的正负号
                if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                    number.push(chars[i]);
                    i += 1;
                }

                // 解析指数部分
                while i < chars.len() && chars[i].is_digit(10) {
                    number.push(chars[i]);
                    i += 1;
                }
            }

            tokens.push(number);
            continue;
        }
        
        // 单个字符
        tokens.push(chars[i].to_string());
        i += 1;
    }
    
    if debug {
        debug_println(&format!("词法分析结果: {:?}", tokens));
    }
    
    tokens
} 