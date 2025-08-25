use ::std::collections::HashMap;
use serde_json::{Value as JsonValue, json, Map};

// 导入通用库
use cn_common::namespace::{LibraryFunction, NamespaceBuilder, create_library_pointer, LibraryRegistry};

// JSON命名空间
mod json {
    use super::*;

    // 解析JSON字符串
    pub fn cn_parse(args: Vec<String>) -> String {
        if args.is_empty() {
            return "错误: 未提供JSON字符串".to_string();
        }
        
        let json_str = &args[0];
        
        // 尝试处理可能的转义问题
        let processed_str = preprocess_json_string(json_str);
        
        // 尝试从HTTP响应中提取JSON部分
        let json_content = extract_json_from_http_response(&processed_str);
        
        // 尝试解析JSON
        match serde_json::from_str::<JsonValue>(&json_content) {
            Ok(value) => {
                // 解析成功，返回格式化的JSON
                match serde_json::to_string_pretty(&value) {
                    Ok(pretty) => pretty,
                    Err(e) => format!("错误: 格式化JSON失败: {}", e)
                }
            },
            Err(e) => {
                // 尝试修复常见的JSON格式问题
                let fixed_json = fix_json_string(&json_content);
                match serde_json::from_str::<JsonValue>(&fixed_json) {
                    Ok(value) => {
                        // 修复后解析成功
                        match serde_json::to_string_pretty(&value) {
                            Ok(pretty) => pretty,
                            Err(e) => format!("错误: 格式化JSON失败: {}", e)
                        }
                    },
                    Err(_) => format!("错误: 解析JSON失败: {}", e)
                }
            }
        }
    }
    
    // 格式化JSON字符串
    pub fn cn_format(args: Vec<String>) -> String {
        if args.is_empty() {
            return "错误: 未提供JSON字符串".to_string();
        }
        
        let json_str = &args[0];
        
        // 尝试处理可能的转义问题
        let processed_str = preprocess_json_string(json_str);
        
        // 尝试从HTTP响应中提取JSON部分
        let json_content = extract_json_from_http_response(&processed_str);
        
        // 尝试解析JSON
        match serde_json::from_str::<JsonValue>(&json_content) {
            Ok(value) => {
                // 解析成功，返回格式化的JSON
                match serde_json::to_string_pretty(&value) {
                    Ok(pretty) => pretty,
                    Err(e) => format!("错误: 格式化JSON失败: {}", e)
                }
            },
            Err(e) => {
                // 尝试修复常见的JSON格式问题
                let fixed_json = fix_json_string(&json_content);
                match serde_json::from_str::<JsonValue>(&fixed_json) {
                    Ok(value) => {
                        // 修复后解析成功
                        match serde_json::to_string_pretty(&value) {
                            Ok(pretty) => pretty,
                            Err(e) => format!("错误: 格式化JSON失败: {}", e)
                        }
                    },
                    Err(_) => format!("错误: 无效的JSON字符串: {}", e)
                }
            }
        }
    }
    
    // 创建JSON对象
    pub fn cn_create_object(args: Vec<String>) -> String {
        let mut map = Map::new();
        
        // 解析键值对参数
        for i in (0..args.len()).step_by(2) {
            if i + 1 < args.len() {
                let key = &args[i];
                let value = &args[i + 1];
                
                // 尝试将值解析为JSON值，如果失败则作为字符串处理
                let json_value = match serde_json::from_str::<JsonValue>(value) {
                    Ok(v) => v,
                    Err(_) => {
                        // 尝试将数字字符串转换为数字
                        if let Ok(num) = value.parse::<i64>() {
                            JsonValue::Number(serde_json::Number::from(num))
                        } else if let Ok(float) = value.parse::<f64>() {
                            // 创建浮点数JSON值
                            match serde_json::Number::from_f64(float) {
                                Some(n) => JsonValue::Number(n),
                                None => JsonValue::String(value.clone())
                            }
                        } else {
                            JsonValue::String(value.clone())
                        }
                    }
                };
                
                map.insert(key.clone(), json_value);
            }
        }
        
        // 创建JSON对象并返回
        let obj = JsonValue::Object(map);
        match serde_json::to_string(&obj) {
            Ok(json_str) => json_str,
            Err(e) => format!("错误: 创建JSON对象失败: {}", e)
        }
    }
    
    // 创建JSON数组
    pub fn cn_create_array(args: Vec<String>) -> String {
        let mut array = Vec::new();
        
        // 将所有参数添加到数组中
        for value in args {
            // 尝试将值解析为JSON值，如果失败则作为字符串处理
            let json_value = match serde_json::from_str::<JsonValue>(&value) {
                Ok(v) => v,
                Err(_) => {
                    // 尝试将数字字符串转换为数字
                    if let Ok(num) = value.parse::<i64>() {
                        JsonValue::Number(serde_json::Number::from(num))
                    } else if let Ok(float) = value.parse::<f64>() {
                        // 创建浮点数JSON值
                        match serde_json::Number::from_f64(float) {
                            Some(n) => JsonValue::Number(n),
                            None => JsonValue::String(value)
                        }
                    } else {
                        JsonValue::String(value)
                    }
                }
            };
            
            array.push(json_value);
        }
        
        // 创建JSON数组并返回
        let arr = JsonValue::Array(array);
        match serde_json::to_string(&arr) {
            Ok(json_str) => json_str,
            Err(e) => format!("错误: 创建JSON数组失败: {}", e)
        }
    }
    
    // 从JSON中获取值
    pub fn cn_get_value(args: Vec<String>) -> String {
        if args.len() < 2 {
            return "错误: 请提供JSON字符串和路径".to_string();
        }
        
        let json_str = &args[0];
        let path = &args[1];
        
        // 尝试处理可能的转义问题
        let processed_str = preprocess_json_string(json_str);
        
        // 尝试从HTTP响应中提取JSON部分
        let json_content = extract_json_from_http_response(&processed_str);
        
        // 尝试解析JSON
        match serde_json::from_str::<JsonValue>(&json_content) {
            Ok(value) => {
                // 解析路径
                let path_parts: Vec<&str> = path.split('.').collect();
                let mut current_value = &value;
                
                // 遍历路径
                for part in path_parts {
                    if let Some(index) = part.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                        // 数组索引访问
                        if let Ok(idx) = index.parse::<usize>() {
                            if let Some(arr_value) = current_value.as_array().and_then(|arr| arr.get(idx)) {
                                current_value = arr_value;
                            } else {
                                return format!("错误: 无效的数组索引: {}", part);
                            }
                        } else {
                            return format!("错误: 无效的数组索引格式: {}", part);
                        }
                    } else {
                        // 对象属性访问
                        if let Some(obj_value) = current_value.as_object().and_then(|obj| obj.get(part)) {
                            current_value = obj_value;
                        } else {
                            return format!("错误: 属性不存在: {}", part);
                        }
                    }
                }
                
                // 返回找到的值
                match serde_json::to_string(current_value) {
                    Ok(result) => result,
                    Err(e) => format!("错误: 序列化结果失败: {}", e)
                }
            },
            Err(e) => {
                // 尝试修复常见的JSON格式问题
                let fixed_json = fix_json_string(&json_content);
                match serde_json::from_str::<JsonValue>(&fixed_json) {
                    Ok(value) => {
                        // 修复后解析成功，继续处理路径
                        let path_parts: Vec<&str> = path.split('.').collect();
                        let mut current_value = &value;
                        
                        // 遍历路径
                        for part in path_parts {
                            if let Some(index) = part.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                                // 数组索引访问
                                if let Ok(idx) = index.parse::<usize>() {
                                    if let Some(arr_value) = current_value.as_array().and_then(|arr| arr.get(idx)) {
                                        current_value = arr_value;
                                    } else {
                                        return format!("错误: 无效的数组索引: {}", part);
                                    }
                                } else {
                                    return format!("错误: 无效的数组索引格式: {}", part);
                                }
                            } else {
                                // 对象属性访问
                                if let Some(obj_value) = current_value.as_object().and_then(|obj| obj.get(part)) {
                                    current_value = obj_value;
                                } else {
                                    return format!("错误: 属性不存在: {}", part);
                                }
                            }
                        }
                        
                        // 返回找到的值
                        match serde_json::to_string(current_value) {
                            Ok(result) => result,
                            Err(e) => format!("错误: 序列化结果失败: {}", e)
                        }
                    },
                    Err(_) => format!("错误: 解析JSON失败: {}", e)
                }
            }
        }
    }
    
    // 检查JSON是否有效
    pub fn cn_is_valid(args: Vec<String>) -> String {
        if args.is_empty() {
            return "false".to_string();
        }
        
        let json_str = &args[0];
        
        // 尝试处理可能的转义问题
        let processed_str = preprocess_json_string(json_str);
        
        // 尝试从HTTP响应中提取JSON部分
        let json_content = extract_json_from_http_response(&processed_str);
        
        // 尝试解析JSON
        match serde_json::from_str::<JsonValue>(&json_content) {
            Ok(_) => "true".to_string(),
            Err(_) => {
                // 尝试修复常见的JSON格式问题
                let fixed_json = fix_json_string(&json_content);
                match serde_json::from_str::<JsonValue>(&fixed_json) {
                    Ok(_) => "true".to_string(),
                    Err(_) => "false".to_string()
                }
            }
        }
    }
    
    // 合并两个JSON对象
    pub fn cn_merge(args: Vec<String>) -> String {
        if args.len() < 2 {
            return "错误: 请提供两个JSON对象".to_string();
        }
        
        let json1 = &args[0];
        let json2 = &args[1];
        
        // 尝试处理可能的转义问题
        let processed_json1 = preprocess_json_string(json1);
        let processed_json2 = preprocess_json_string(json2);
        
        // 尝试从HTTP响应中提取JSON部分
        let json_content1 = extract_json_from_http_response(&processed_json1);
        let json_content2 = extract_json_from_http_response(&processed_json2);
        
        // 解析两个JSON对象
        let parse_result1 = serde_json::from_str::<JsonValue>(&json_content1)
            .or_else(|_| serde_json::from_str::<JsonValue>(&fix_json_string(&json_content1)));
            
        let parse_result2 = serde_json::from_str::<JsonValue>(&json_content2)
            .or_else(|_| serde_json::from_str::<JsonValue>(&fix_json_string(&json_content2)));
        
        match (parse_result1, parse_result2) {
            (Ok(mut value1), Ok(value2)) => {
                if let (Some(obj1), Some(obj2)) = (value1.as_object_mut(), value2.as_object()) {
                    // 合并对象
                    for (key, value) in obj2 {
                        obj1.insert(key.clone(), value.clone());
                    }
                    
                    // 返回合并后的对象
                    match serde_json::to_string(&value1) {
                        Ok(result) => result,
                        Err(e) => format!("错误: 序列化合并结果失败: {}", e)
                    }
                } else {
                    "错误: 输入必须是JSON对象".to_string()
                }
            },
            (Err(e), _) => format!("错误: 解析第一个JSON对象失败: {}", e),
            (_, Err(e)) => format!("错误: 解析第二个JSON对象失败: {}", e)
        }
    }
    
    // 预处理JSON字符串，处理可能的转义问题
    fn preprocess_json_string(input: &str) -> String {
        // 如果输入已经是有效的JSON，直接返回
        if serde_json::from_str::<JsonValue>(input).is_ok() {
            return input.to_string();
        }
        
        // 处理可能的转义问题
        let mut processed = input.to_string();
        
        // 处理双引号转义问题
        if processed.contains("\\\"") {
            processed = processed.replace("\\\"", "\"");
        }
        
        // 处理可能的双重转义问题
        if processed.contains("\\\\") {
            processed = processed.replace("\\\\", "\\");
        }
        
        processed
    }
    
    // 从HTTP响应中提取JSON部分
    fn extract_json_from_http_response(response: &str) -> String {
        // 检查是否是HTTP响应
        if response.contains("状态码:") && response.contains("头信息:") && response.contains("响应体:") {
            // 尝试提取响应体部分
            if let Some(body_start) = response.find("响应体:") {
                let body_content = &response[body_start + "响应体:".len()..].trim();
                return body_content.to_string();
            }
        }
        
        // 如果不是HTTP响应或无法提取响应体，返回原始字符串
        response.to_string()
    }
    
    // 修复常见的JSON格式问题
    fn fix_json_string(input: &str) -> String {
        // 如果输入已经是有效的JSON，直接返回
        if serde_json::from_str::<JsonValue>(input).is_ok() {
            return input.to_string();
        }
        
        let mut result = input.to_string();
        
        // 检查是否需要添加引号来修复键
        if result.starts_with('{') {
            // 尝试修复常见的JSON格式问题
            
            // 替换没有引号的键
            // 注意：这是一个简单的修复尝试，不能处理所有复杂情况
            let mut i = 1; // 跳过开头的 '{'
            while i < result.len() {
                if let Some(pos) = result[i..].find(':') {
                    let key_end = i + pos;
                    let mut key_start = i;
                    
                    // 向后查找键的开始位置
                    while key_start < key_end {
                        let c = result.chars().nth(key_start).unwrap_or(' ');
                        if c != ' ' && c != '\t' && c != '\n' && c != '\r' && c != ',' && c != '{' {
                            break;
                        }
                        key_start += 1;
                    }
                    
                    // 检查键是否已经有引号
                    let already_quoted = result.chars().nth(key_start) == Some('"');
                    
                    if !already_quoted {
                        // 在键的前后添加引号
                        let key = result[key_start..key_end].trim();
                        let new_key = format!("\"{}\"", key);
                        result.replace_range(key_start..key_end, &new_key);
                        
                        // 更新索引位置
                        i = key_start + new_key.len() + 1;
                    } else {
                        i = key_end + 1;
                    }
                } else {
                    break;
                }
            }
        }
        
        // 处理可能的值引号问题
        // 这里简单处理，实际情况可能需要更复杂的逻辑
        let mut i = 0;
        while i < result.len() {
            if let Some(pos) = result[i..].find(':') {
                let value_start = i + pos + 1;
                if value_start < result.len() {
                    // 跳过空白字符
                    let mut j = value_start;
                    while j < result.len() {
                        let c = result.chars().nth(j).unwrap_or(' ');
                        if c != ' ' && c != '\t' && c != '\n' && c != '\r' {
                            break;
                        }
                        j += 1;
                    }
                    
                    // 检查值是否需要引号
                    if j < result.len() {
                        let c = result.chars().nth(j).unwrap_or(' ');
                        if c != '"' && c != '{' && c != '[' && !c.is_numeric() && c != 't' && c != 'f' && c != 'n' {
                            // 可能需要添加引号的值
                            let mut value_end = j;
                            while value_end < result.len() {
                                let c = result.chars().nth(value_end).unwrap_or(' ');
                                if c == ',' || c == '}' {
                                    break;
                                }
                                value_end += 1;
                            }
                            
                            // 添加引号
                            let value = result[j..value_end].trim();
                            let new_value = format!("\"{}\"", value);
                            result.replace_range(j..value_end, &new_value);
                            
                            // 更新索引位置
                            i = j + new_value.len();
                        } else {
                            i = j + 1;
                        }
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
        
        result
    }
}

// 初始化函数，返回函数映射
#[no_mangle]
pub extern "C" fn cn_init() -> *mut HashMap<String, LibraryFunction> {
    // 创建库函数注册器
    let mut registry = LibraryRegistry::new();
    
    // 注册JSON命名空间下的函数
    let json_ns = registry.namespace("json");
    json_ns.add_function("parse", json::cn_parse)
           .add_function("format", json::cn_format)
           .add_function("create_object", json::cn_create_object)
           .add_function("create_array", json::cn_create_array)
           .add_function("get_value", json::cn_get_value)
           .add_function("is_valid", json::cn_is_valid)
           .add_function("merge", json::cn_merge);
           
    // 构建并返回库指针
    registry.build_library_pointer()
} 