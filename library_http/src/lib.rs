use ::std::collections::HashMap;
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::str::FromStr;
use std::time::Duration;

// 导入通用库
use cn_common::namespace::{LibraryFunction, NamespaceBuilder, create_library_pointer, LibraryRegistry};

// HTTP命名空间
mod http {
    use super::*;

    // 执行GET请求
    pub fn cn_get(args: Vec<String>) -> String {
        if args.is_empty() {
            return "错误: 未提供URL".to_string();
        }
        
        let url = &args[0];
        let client = Client::new();
        
        match client.get(url).send() {
            Ok(response) => format_response(response),
            Err(err) => format!("错误: {}", err)
        }
    }
    
    // 执行POST请求
    pub fn cn_post(args: Vec<String>) -> String {
        if args.len() < 2 {
            return "错误: 请提供URL和请求体".to_string();
        }
        
        let url = &args[0];
        let body = &args[1];
        let client = Client::new();
        
        match client.post(url).body(body.clone()).send() {
            Ok(response) => format_response(response),
            Err(err) => format!("错误: {}", err)
        }
    }
    
    // 执行PUT请求
    pub fn cn_put(args: Vec<String>) -> String {
        if args.len() < 2 {
            return "错误: 请提供URL和请求体".to_string();
        }
        
        let url = &args[0];
        let body = &args[1];
        let client = Client::new();
        
        match client.put(url).body(body.clone()).send() {
            Ok(response) => format_response(response),
            Err(err) => format!("错误: {}", err)
        }
    }
    
    // 执行DELETE请求
    pub fn cn_delete(args: Vec<String>) -> String {
        if args.is_empty() {
            return "错误: 未提供URL".to_string();
        }
        
        let url = &args[0];
        let client = Client::new();
        
        match client.delete(url).send() {
            Ok(response) => format_response(response),
            Err(err) => format!("错误: {}", err)
        }
    }
    
    // 带自定义头的请求
    pub fn cn_request(args: Vec<String>) -> String {
        if args.len() < 3 {
            return "错误: 请提供方法、URL和头信息".to_string();
        }
        
        let method = &args[0];
        let url = &args[1];
        let headers_str = &args[2];
        let body = args.get(3).cloned().unwrap_or_default();
        
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();
            
        // 解析头信息 (格式: "key1:value1;key2:value2")
        let mut headers = HeaderMap::new();
        for header_pair in headers_str.split(';') {
            if let Some((key, value)) = header_pair.split_once(':') {
                if let (Ok(name), Ok(val)) = (
                    HeaderName::from_str(key.trim()),
                    HeaderValue::from_str(value.trim())
                ) {
                    headers.insert(name, val);
                }
            }
        }
        
        let request_builder = match method.to_uppercase().as_str() {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
            "HEAD" => client.head(url),
            "PATCH" => client.patch(url),
            _ => return format!("错误: 不支持的HTTP方法 '{}'", method)
        };
        
        let request_with_headers = request_builder.headers(headers);
        
        // 添加请求体（如果有）
        let request_with_body = if !body.is_empty() && method != "GET" && method != "HEAD" {
            request_with_headers.body(body)
        } else {
            request_with_headers
        };
        
        match request_with_body.send() {
            Ok(response) => format_response(response),
            Err(err) => format!("错误: {}", err)
        }
    }
    
    // 编码URL
    pub fn cn_encode_url(args: Vec<String>) -> String {
        if args.is_empty() {
            return String::new();
        }
        
        url::form_urlencoded::byte_serialize(args[0].as_bytes())
            .collect::<String>()
    }
    
    // 解码URL
    pub fn cn_decode_url(args: Vec<String>) -> String {
        if args.is_empty() {
            return String::new();
        }
        
        match url::form_urlencoded::parse(args[0].as_bytes())
            .map(|(key, val)| format!("{}{}", key, val))
            .collect::<String>() {
                s if s.is_empty() => args[0].clone(),
                s => s
            }
    }
}

// 格式化HTTP响应
fn format_response(response: Response) -> String {
    let status = response.status();
    let headers = response.headers().clone();
    
    match response.text() {
        Ok(body) => {
            let mut result = format!("状态码: {}\n", status);
            
            // 添加头信息
            result.push_str("头信息:\n");
            for (name, value) in headers.iter() {
                if let Ok(val_str) = value.to_str() {
                    result.push_str(&format!("{}: {}\n", name, val_str));
                }
            }
            
            // 添加响应体
            result.push_str("\n响应体:\n");
            result.push_str(&body);
            
            result
        },
        Err(err) => format!("状态码: {}\n读取响应体时出错: {}", status, err)
    }
}

// 初始化函数，返回函数映射
#[no_mangle]
pub extern "C" fn cn_init() -> *mut HashMap<String, LibraryFunction> {
    // 创建库函数注册器
    let mut registry = LibraryRegistry::new();
    
    // 注册HTTP命名空间下的函数
    let http_ns = registry.namespace("http");
    http_ns.add_function("get", http::cn_get)
           .add_function("post", http::cn_post)
           .add_function("put", http::cn_put)
           .add_function("delete", http::cn_delete)
           .add_function("request", http::cn_request)
           .add_function("encode_url", http::cn_encode_url)
           .add_function("decode_url", http::cn_decode_url);
           
    // 构建并返回库指针
    registry.build_library_pointer()
} 