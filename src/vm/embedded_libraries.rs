// 跨平台库抽象层 - 借鉴Java JVM的思路
// 字节码文件只包含字节码，库函数通过运行时环境提供

use crate::interpreter::value::Value;
use std::collections::HashMap;
use std::path::Path;

/// 运行时库环境 - 类似Java的JVM运行时
/// 管理库的发现、加载和调用
pub struct RuntimeLibraryEnvironment {
    /// 库搜索路径
    library_paths: Vec<String>,
    /// 已加载的库
    loaded_libraries: HashMap<String, Box<dyn LibraryInterface>>,
    /// 库加载器
    library_loader: Box<dyn LibraryLoader>,
}

/// 库接口 - 所有库都必须实现这个接口
pub trait LibraryInterface {
    /// 获取库名称
    fn get_name(&self) -> &str;

    /// 获取库版本
    fn get_version(&self) -> &str;

    /// 获取可用函数列表
    fn get_functions(&self) -> Vec<String>;

    /// 调用库函数
    fn call_function(&self, func_name: &str, args: Vec<Value>) -> Result<Value, String>;

    /// 检查函数是否存在
    fn has_function(&self, func_name: &str) -> bool;
}

/// 库加载器接口 - 负责从文件系统加载库
pub trait LibraryLoader {
    /// 从路径加载库
    fn load_from_path(&self, lib_path: &str) -> Result<Box<dyn LibraryInterface>, String>;

    /// 检查路径是否为有效的库文件
    fn is_valid_library(&self, lib_path: &str) -> bool;

    /// 获取支持的库文件扩展名
    fn get_supported_extensions(&self) -> Vec<String>;
}

impl RuntimeLibraryEnvironment {
    pub fn new(library_loader: Box<dyn LibraryLoader>) -> Self {
        let mut env = RuntimeLibraryEnvironment {
            library_paths: Vec::new(),
            loaded_libraries: HashMap::new(),
            library_loader,
        };

        // 添加默认库搜索路径
        env.add_default_library_paths();

        env
    }

    /// 添加默认库搜索路径 - 与解释器版本保持一致
    fn add_default_library_paths(&mut self) {
        use std::env;
        use std::path::PathBuf;

        // 1. 解释器目录/library（与解释器版本一致）
        let exe_library_path = match env::current_exe() {
            Ok(exe_path) => {
                match exe_path.parent() {
                    Some(parent) => {
                        let mut path = parent.to_path_buf();
                        path.push("library");
                        path.to_string_lossy().to_string()
                    },
                    None => "./library".to_string(),
                }
            },
            Err(_) => "./library".to_string(),
        };
        self.library_paths.push(exe_library_path);

        // 2. 当前目录/library（与解释器版本一致）
        self.library_paths.push("./library".to_string());
    }

    /// 添加库搜索路径
    pub fn add_library_path(&mut self, path: String) {
        if !self.library_paths.contains(&path) {
            self.library_paths.push(path);
        }
    }

    /// 检查库是否在运行时环境中可用
    pub fn is_library_available(&self, lib_name: &str) -> bool {
        // 首先检查是否已加载
        if self.loaded_libraries.contains_key(lib_name) {
            return true;
        }

        // 在搜索路径中查找库文件
        self.find_library_file(lib_name).is_some()
    }

    /// 在搜索路径中查找库文件
    fn find_library_file(&self, lib_name: &str) -> Option<String> {
        let extensions = self.library_loader.get_supported_extensions();

        for path in &self.library_paths {
            for ext in &extensions {
                let lib_file = format!("{}/{}.{}", path, lib_name, ext);
                if Path::new(&lib_file).exists() && self.library_loader.is_valid_library(&lib_file) {
                    return Some(lib_file);
                }
            }
        }

        None
    }

    /// 加载库
    pub fn load_library(&mut self, lib_name: &str) -> Result<(), String> {
        // 如果已经加载，直接返回
        if self.loaded_libraries.contains_key(lib_name) {
            return Ok(());
        }

        // 查找库文件
        let lib_file = self.find_library_file(lib_name)
            .ok_or_else(|| format!("未找到库 '{}' 在搜索路径中", lib_name))?;

        // 加载库
        let library = self.library_loader.load_from_path(&lib_file)?;

        // 验证库名称
        if library.get_name() != lib_name {
            return Err(format!("库文件 '{}' 的名称 '{}' 与请求的名称 '{}' 不匹配",
                lib_file, library.get_name(), lib_name));
        }

        // 存储已加载的库
        self.loaded_libraries.insert(lib_name.to_string(), library);

        Ok(())
    }

    /// 调用库函数
    pub fn call_library_function(&self, lib_name: &str, func_name: &str, args: Vec<Value>) -> Result<Value, String> {
        let library = self.loaded_libraries.get(lib_name)
            .ok_or_else(|| format!("库 '{}' 未加载", lib_name))?;

        library.call_function(func_name, args)
    }

    /// 获取库函数列表
    pub fn get_library_functions(&self, lib_name: &str) -> Result<Vec<String>, String> {
        let library = self.loaded_libraries.get(lib_name)
            .ok_or_else(|| format!("库 '{}' 未加载", lib_name))?;

        Ok(library.get_functions())
    }

    /// 获取所有可用的库
    pub fn get_available_libraries(&self) -> Vec<String> {
        let mut libraries = Vec::new();
        let extensions = self.library_loader.get_supported_extensions();

        for path in &self.library_paths {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    if let Some(file_name) = entry.file_name().to_str() {
                        for ext in &extensions {
                            if file_name.ends_with(&format!(".{}", ext)) {
                                let lib_name = file_name.strip_suffix(&format!(".{}", ext)).unwrap();
                                if !libraries.contains(&lib_name.to_string()) {
                                    libraries.push(lib_name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        libraries
    }

    /// 获取库搜索路径
    pub fn get_library_paths(&self) -> &Vec<String> {
        &self.library_paths
    }
}
