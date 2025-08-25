use ::std::collections::HashMap;
use ::std::fs;
use ::std::path::Path;
use ::std::io::Write;

// 导入通用库
use cn_common::namespace::{LibraryFunction, create_library_pointer, register_namespaces};

// 根命名空间函数
// 判断路径是否存在
fn cn_exists(args: Vec<String>) -> String {
    if args.is_empty() {
        return "false".to_string();
    }
    
    let path = &args[0];
    Path::new(path).exists().to_string()
}

// 判断是否为文件
fn cn_is_file(args: Vec<String>) -> String {
    if args.is_empty() {
        return "false".to_string();
    }
    
    let path = &args[0];
    Path::new(path).is_file().to_string()
}

// 判断是否为目录
fn cn_is_dir(args: Vec<String>) -> String {
    if args.is_empty() {
        return "false".to_string();
    }
    
    let path = &args[0];
    Path::new(path).is_dir().to_string()
}

// 文件操作命名空间
mod file {
    use super::*;
    
    // 读取文件内容
    pub fn cn_read(args: Vec<String>) -> String {
        if args.is_empty() {
            return "".to_string();
        }
        
        let path = &args[0];
        match fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => format!("ERROR: {}", err)
        }
    }
    
    // 读取文件内容为二进制
    pub fn cn_read_bytes(args: Vec<String>) -> String {
        if args.is_empty() {
            return "".to_string();
        }
        
        let path = &args[0];
        match fs::read(path) {
            Ok(bytes) => {
                // 将二进制数据转换为16进制字符串
                bytes.iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<String>>()
                    .join("")
            },
            Err(err) => format!("ERROR: {}", err)
        }
    }
    
    // 写入文件
    pub fn cn_write(args: Vec<String>) -> String {
        if args.len() < 2 {
            return "ERROR: 需要两个参数: 文件路径和内容".to_string();
        }
        
        let path = &args[0];
        let content = &args[1];
        
        match fs::write(path, content) {
            Ok(_) => "true".to_string(),
            Err(err) => format!("ERROR: {}", err)
        }
    }
    
    // 追加内容到文件
    pub fn cn_append(args: Vec<String>) -> String {
        if args.len() < 2 {
            return "ERROR: 需要两个参数: 文件路径和内容".to_string();
        }
        
        let path = &args[0];
        let content = &args[1];
        
        let mut file = match fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(path) {
                Ok(file) => file,
                Err(err) => return format!("ERROR: {}", err)
            };
            
        match file.write_all(content.as_bytes()) {
            Ok(_) => "true".to_string(),
            Err(err) => format!("ERROR: {}", err)
        }
    }
    
    // 删除文件
    pub fn cn_delete(args: Vec<String>) -> String {
        if args.is_empty() {
            return "ERROR: 需要文件路径参数".to_string();
        }
        
        let path = &args[0];
        match fs::remove_file(path) {
            Ok(_) => "true".to_string(),
            Err(err) => format!("ERROR: {}", err)
        }
    }
    
    // 复制文件
    pub fn cn_copy(args: Vec<String>) -> String {
        if args.len() < 2 {
            return "ERROR: 需要两个参数: 源文件路径和目标文件路径".to_string();
        }
        
        let src = &args[0];
        let dst = &args[1];
        
        match fs::copy(src, dst) {
            Ok(_) => "true".to_string(),
            Err(err) => format!("ERROR: {}", err)
        }
    }
    
    // 重命名文件
    pub fn cn_rename(args: Vec<String>) -> String {
        if args.len() < 2 {
            return "ERROR: 需要两个参数: 原文件路径和新文件路径".to_string();
        }
        
        let old_path = &args[0];
        let new_path = &args[1];
        
        match fs::rename(old_path, new_path) {
            Ok(_) => "true".to_string(),
            Err(err) => format!("ERROR: {}", err)
        }
    }
    
    // 获取文件大小
    pub fn cn_size(args: Vec<String>) -> String {
        if args.is_empty() {
            return "ERROR: 需要文件路径参数".to_string();
        }
        
        let path = &args[0];
        match fs::metadata(path) {
            Ok(metadata) => metadata.len().to_string(),
            Err(err) => format!("ERROR: {}", err)
        }
    }
}

// 目录操作命名空间
mod dir {
    use super::*;
    
    // 创建目录
    pub fn cn_create(args: Vec<String>) -> String {
        if args.is_empty() {
            return "ERROR: 需要目录路径参数".to_string();
        }
        
        let path = &args[0];
        match fs::create_dir_all(path) {
            Ok(_) => "true".to_string(),
            Err(err) => format!("ERROR: {}", err)
        }
    }
    
    // 删除目录
    pub fn cn_delete(args: Vec<String>) -> String {
        if args.is_empty() {
            return "ERROR: 需要目录路径参数".to_string();
        }
        
        let path = &args[0];
        match fs::remove_dir(path) {
            Ok(_) => "true".to_string(),
            Err(err) => format!("ERROR: {}", err)
        }
    }
    
    // 递归删除目录
    pub fn cn_delete_all(args: Vec<String>) -> String {
        if args.is_empty() {
            return "ERROR: 需要目录路径参数".to_string();
        }
        
        let path = &args[0];
        match fs::remove_dir_all(path) {
            Ok(_) => "true".to_string(),
            Err(err) => format!("ERROR: {}", err)
        }
    }
    
    // 列出目录内容
    pub fn cn_list(args: Vec<String>) -> String {
        if args.is_empty() {
            return "ERROR: 需要目录路径参数".to_string();
        }
        
        let path = &args[0];
        match fs::read_dir(path) {
            Ok(entries) => {
                let mut result = Vec::new();
                for entry in entries {
                    if let Ok(entry) = entry {
                        result.push(entry.path().to_string_lossy().to_string());
                    }
                }
                result.join("\n")
            },
            Err(err) => format!("ERROR: {}", err)
        }
    }
    
    // 获取当前工作目录
    pub fn cn_current(_args: Vec<String>) -> String {
        match ::std::env::current_dir() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(err) => format!("ERROR: {}", err)
        }
    }
}

// 路径操作命名空间
mod path {
    use super::*;
    use ::std::path::PathBuf;
    
    // 连接路径
    pub fn cn_join(args: Vec<String>) -> String {
        if args.is_empty() {
            return "".to_string();
        }
        
        let mut path_buf = PathBuf::new();
        for part in args {
            path_buf.push(part);
        }
        
        path_buf.to_string_lossy().to_string()
    }
    
    // 获取父目录
    pub fn cn_parent(args: Vec<String>) -> String {
        if args.is_empty() {
            return "".to_string();
        }
        
        let path = Path::new(&args[0]);
        match path.parent() {
            Some(parent) => parent.to_string_lossy().to_string(),
            None => "".to_string()
        }
    }
    
    // 获取文件名
    pub fn cn_filename(args: Vec<String>) -> String {
        if args.is_empty() {
            return "".to_string();
        }
        
        let path = Path::new(&args[0]);
        match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => "".to_string()
        }
    }
    
    // 获取文件扩展名
    pub fn cn_extension(args: Vec<String>) -> String {
        if args.is_empty() {
            return "".to_string();
        }
        
        let path = Path::new(&args[0]);
        match path.extension() {
            Some(ext) => ext.to_string_lossy().to_string(),
            None => "".to_string()
        }
    }
    
    // 获取不带扩展名的文件名
    pub fn cn_stem(args: Vec<String>) -> String {
        if args.is_empty() {
            return "".to_string();
        }
        
        let path = Path::new(&args[0]);
        match path.file_stem() {
            Some(stem) => stem.to_string_lossy().to_string(),
            None => "".to_string()
        }
    }
    
    // 判断路径是否为绝对路径
    pub fn cn_is_absolute(args: Vec<String>) -> String {
        if args.is_empty() {
            return "false".to_string();
        }
        
        let path = Path::new(&args[0]);
        path.is_absolute().to_string()
    }
}

// 初始化函数，返回函数映射
#[no_mangle]
pub extern "C" fn cn_init() -> *mut HashMap<String, LibraryFunction> {
    // 使用register_namespaces函数一次性注册多个命名空间
    let functions = register_namespaces(vec![
        // 根命名空间函数
        ("", vec![
            ("exists", cn_exists),
            ("is_file", cn_is_file),
            ("is_dir", cn_is_dir),
        ]),
        // 文件操作命名空间
        ("file", vec![
            ("read", file::cn_read),
            ("read_bytes", file::cn_read_bytes),
            ("write", file::cn_write),
            ("append", file::cn_append),
            ("delete", file::cn_delete),
            ("copy", file::cn_copy),
            ("rename", file::cn_rename),
            ("size", file::cn_size),
        ]),
        // 目录操作命名空间
        ("dir", vec![
            ("create", dir::cn_create),
            ("delete", dir::cn_delete),
            ("delete_all", dir::cn_delete_all),
            ("list", dir::cn_list),
            ("current", dir::cn_current),
        ]),
        // 路径操作命名空间
        ("path", vec![
            ("join", path::cn_join),
            ("parent", path::cn_parent),
            ("filename", path::cn_filename),
            ("extension", path::cn_extension),
            ("stem", path::cn_stem),
            ("is_absolute", path::cn_is_absolute),
        ]),
    ]);
    
    // 将HashMap装箱并转换为原始指针
    create_library_pointer(functions)
} 