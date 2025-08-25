// 导出命名空间模块
pub mod namespace;

// 通用字符串处理函数
pub mod string {
    /// 处理转义字符，将\n, \t等转换为对应的字符
    pub fn process_escape_chars(input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c == '\\' && chars.peek().is_some() {
                match chars.next().unwrap() {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    c => {
                        result.push('\\');
                        result.push(c);
                    }
                }
            } else {
                result.push(c);
            }
        }
        
        result
    }
}

// 用于测试库是否正常工作的函数
#[no_mangle]
pub extern "C" fn cn_test() -> i32 {
    ::std::println!("CodeNothing通用库测试成功");
    1
} 