use ::std::collections::HashMap;

// 定义库函数类型
pub type LibraryFunction = fn(Vec<String>) -> String;

/// 命名空间构建器，用于简化库函数的命名空间注册
pub struct NamespaceBuilder {
    namespace: String,
    functions: HashMap<String, LibraryFunction>,
}

impl NamespaceBuilder {
    /// 创建一个新的命名空间构建器
    /// 
    /// # 参数
    /// * `namespace` - 命名空间名称
    pub fn new(namespace: &str) -> Self {
        NamespaceBuilder {
            namespace: namespace.to_string(),
            functions: HashMap::new(),
        }
    }
    
    /// 向命名空间中添加函数
    /// 
    /// # 参数
    /// * `name` - 函数名称（不含命名空间前缀）
    /// * `func` - 函数指针
    /// 
    /// # 返回
    /// 返回自身引用，支持链式调用
    pub fn add_function(&mut self, name: &str, func: LibraryFunction) -> &mut Self {
        let full_name = if self.namespace.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.namespace, name)
        };
        self.functions.insert(full_name, func);
        self
    }
    
    /// 将命名空间中的所有函数注册到目标HashMap
    /// 
    /// # 参数
    /// * `target` - 目标函数映射
    pub fn register_all(&self, target: &mut HashMap<String, LibraryFunction>) {
        for (name, func) in &self.functions {
            target.insert(name.clone(), *func);
        }
    }
    
    /// 获取命名空间名称
    pub fn namespace(&self) -> &str {
        &self.namespace
    }
    
    /// 获取已注册函数数量
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }
    
    /// 检查函数是否已注册
    pub fn has_function(&self, name: &str) -> bool {
        let full_name = if self.namespace.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.namespace, name)
        };
        self.functions.contains_key(&full_name)
    }
    
    /// 向命名空间中添加直接调用函数（不带命名空间前缀）
    /// 这允许函数被直接调用，而不需要命名空间前缀
    /// 
    /// # 参数
    /// * `name` - 函数名称
    /// * `func` - 函数指针
    /// 
    /// # 返回
    /// 返回自身引用，支持链式调用
    pub fn add_direct_function(&mut self, name: &str, func: LibraryFunction) -> &mut Self {
        self.functions.insert(name.to_string(), func);
        self
    }
    
    /// 获取函数映射的克隆
    pub fn get_functions(&self) -> HashMap<String, LibraryFunction> {
        self.functions.clone()
    }
}

/// 创建并初始化多个命名空间
/// 
/// # 参数
/// * `namespaces` - 命名空间名称和函数的映射
/// 
/// # 返回
/// 返回合并后的函数映射
pub fn register_namespaces(namespaces: Vec<(&str, Vec<(&str, LibraryFunction)>)>) -> HashMap<String, LibraryFunction> {
    let mut all_functions = HashMap::new();
    
    for (namespace, functions) in namespaces {
        let mut ns_builder = NamespaceBuilder::new(namespace);
        for (name, func) in functions {
            ns_builder.add_function(name, func);
        }
        ns_builder.register_all(&mut all_functions);
    }
    
    all_functions
}

/// 创建指向函数映射的原始指针，用于库初始化
/// 
/// # 参数
/// * `functions` - 函数映射
/// 
/// # 返回
/// 返回函数映射的原始指针
pub fn create_library_pointer(functions: HashMap<String, LibraryFunction>) -> *mut HashMap<String, LibraryFunction> {
    Box::into_raw(Box::new(functions))
}

/// 库函数注册器，用于统一管理库函数的注册
pub struct LibraryRegistry {
    namespaces: HashMap<String, NamespaceBuilder>,
    direct_functions: HashMap<String, LibraryFunction>,
}

impl LibraryRegistry {
    /// 创建一个新的库函数注册器
    pub fn new() -> Self {
        LibraryRegistry {
            namespaces: HashMap::new(),
            direct_functions: HashMap::new(),
        }
    }
    
    /// 获取或创建命名空间构建器
    /// 
    /// # 参数
    /// * `namespace` - 命名空间名称
    /// 
    /// # 返回
    /// 返回命名空间构建器的可变引用
    pub fn namespace(&mut self, namespace: &str) -> &mut NamespaceBuilder {
        if !self.namespaces.contains_key(namespace) {
            self.namespaces.insert(namespace.to_string(), NamespaceBuilder::new(namespace));
        }
        self.namespaces.get_mut(namespace).unwrap()
    }
    
    /// 添加直接调用函数（不带命名空间前缀）
    /// 
    /// # 参数
    /// * `name` - 函数名称
    /// * `func` - 函数指针
    /// 
    /// # 返回
    /// 返回自身引用，支持链式调用
    pub fn add_direct_function(&mut self, name: &str, func: LibraryFunction) -> &mut Self {
        self.direct_functions.insert(name.to_string(), func);
        self
    }
    
    /// 构建最终的函数映射
    /// 
    /// # 返回
    /// 返回合并所有命名空间和直接函数后的函数映射
    pub fn build(&self) -> HashMap<String, LibraryFunction> {
        let mut all_functions = HashMap::new();
        
        // 添加所有命名空间函数
        for (_, ns_builder) in &self.namespaces {
            ns_builder.register_all(&mut all_functions);
        }
        
        // 添加所有直接函数
        for (name, func) in &self.direct_functions {
            all_functions.insert(name.clone(), *func);
        }
        
        all_functions
    }
    
    /// 构建并创建库指针
    /// 
    /// # 返回
    /// 返回函数映射的原始指针，用于库初始化
    pub fn build_library_pointer(&self) -> *mut HashMap<String, LibraryFunction> {
        create_library_pointer(self.build())
    }
} 