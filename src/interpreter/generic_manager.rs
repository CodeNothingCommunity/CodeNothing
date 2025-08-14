// 泛型类型管理器 - 处理运行时泛型类型实例化和管理
use crate::ast::*;
use crate::interpreter::value::Value;
use std::collections::HashMap;

/// 泛型类型实例
#[derive(Debug, Clone)]
pub struct GenericTypeInstance {
    pub base_name: String,                    // 基础类型名 (如 "Vec", "Map")
    pub type_arguments: Vec<Type>,            // 类型参数 (如 [int], [String, int])
    pub instantiated_type: Type,              // 实例化后的完整类型
    pub metadata: GenericInstanceMetadata,    // 实例元数据
}

/// 泛型实例元数据
#[derive(Debug, Clone)]
pub struct GenericInstanceMetadata {
    pub creation_time: u64,                   // 创建时间戳
    pub usage_count: usize,                   // 使用次数
    pub is_cached: bool,                      // 是否已缓存
    pub size_hint: Option<usize>,             // 大小提示
}

/// 泛型函数实例
#[derive(Debug, Clone)]
pub struct GenericFunctionInstance {
    pub base_function: Function,              // 基础函数定义
    pub type_arguments: Vec<Type>,            // 类型参数
    pub instantiated_signature: String,      // 实例化后的函数签名
    pub specialized_body: Vec<Statement>,     // 特化后的函数体
}

/// 泛型类实例
#[derive(Debug, Clone)]
pub struct GenericClassInstance {
    pub base_class: Class,                    // 基础类定义
    pub type_arguments: Vec<Type>,            // 类型参数
    pub instantiated_fields: HashMap<String, Type>, // 实例化后的字段类型
    pub instantiated_methods: Vec<GenericFunctionInstance>, // 实例化后的方法
}

/// 泛型类型管理器
#[derive(Debug)]
pub struct GenericTypeManager {
    // 类型实例缓存
    type_instances: HashMap<String, GenericTypeInstance>,
    
    // 函数实例缓存
    function_instances: HashMap<String, GenericFunctionInstance>,
    
    // 类实例缓存
    class_instances: HashMap<String, GenericClassInstance>,
    
    // 类型推导缓存
    inference_cache: HashMap<String, HashMap<String, Type>>,
    
    // 性能统计
    instantiation_count: usize,
    cache_hits: usize,
    cache_misses: usize,
}

impl GenericTypeManager {
    /// 创建新的泛型类型管理器
    pub fn new() -> Self {
        Self {
            type_instances: HashMap::new(),
            function_instances: HashMap::new(),
            class_instances: HashMap::new(),
            inference_cache: HashMap::new(),
            instantiation_count: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    /// 实例化泛型类型
    pub fn instantiate_type(&mut self, base_type: &Type, type_args: &[Type]) -> Result<Type, String> {
        let instance_key = self.generate_type_key(base_type, type_args);
        
        // 检查缓存
        if let Some(cached_instance) = self.type_instances.get_mut(&instance_key) {
            cached_instance.metadata.usage_count += 1;
            self.cache_hits += 1;
            return Ok(cached_instance.instantiated_type.clone());
        }
        
        self.cache_misses += 1;
        self.instantiation_count += 1;
        
        // 执行实际的类型实例化
        let instantiated_type = match base_type {
            Type::Generic(name) => {
                if type_args.is_empty() {
                    return Err(format!("泛型类型 {} 需要类型参数", name));
                }
                type_args[0].clone()
            },
            Type::GenericClass(class_name, _) => {
                Type::GenericClass(class_name.clone(), type_args.to_vec())
            },
            Type::GenericEnum(enum_name, _) => {
                Type::GenericEnum(enum_name.clone(), type_args.to_vec())
            },
            _ => base_type.clone(),
        };
        
        // 创建实例并缓存
        let instance = GenericTypeInstance {
            base_name: self.extract_base_name(base_type),
            type_arguments: type_args.to_vec(),
            instantiated_type: instantiated_type.clone(),
            metadata: GenericInstanceMetadata {
                creation_time: self.get_current_time(),
                usage_count: 1,
                is_cached: true,
                size_hint: self.calculate_size_hint(&instantiated_type),
            },
        };
        
        self.type_instances.insert(instance_key, instance);
        Ok(instantiated_type)
    }

    /// 实例化泛型函数
    pub fn instantiate_function(&mut self, base_function: &Function, type_args: &[Type]) -> Result<GenericFunctionInstance, String> {
        let instance_key = self.generate_function_key(base_function, type_args);
        
        // 检查缓存
        if let Some(cached_instance) = self.function_instances.get(&instance_key) {
            self.cache_hits += 1;
            return Ok(cached_instance.clone());
        }
        
        self.cache_misses += 1;
        self.instantiation_count += 1;
        
        // 创建类型参数映射
        let mut type_mapping = HashMap::new();
        for (i, generic_param) in base_function.generic_parameters.iter().enumerate() {
            if i < type_args.len() {
                type_mapping.insert(generic_param.name.clone(), type_args[i].clone());
            }
        }
        
        // 特化函数体
        let specialized_body = self.specialize_statements(&base_function.body, &type_mapping)?;
        
        // 生成实例化签名
        let instantiated_signature = self.generate_function_signature(base_function, type_args);
        
        let instance = GenericFunctionInstance {
            base_function: base_function.clone(),
            type_arguments: type_args.to_vec(),
            instantiated_signature,
            specialized_body,
        };
        
        self.function_instances.insert(instance_key, instance.clone());
        Ok(instance)
    }

    /// 实例化泛型类
    pub fn instantiate_class(&mut self, base_class: &Class, type_args: &[Type]) -> Result<GenericClassInstance, String> {
        let instance_key = self.generate_class_key(base_class, type_args);
        
        // 检查缓存
        if let Some(cached_instance) = self.class_instances.get(&instance_key) {
            self.cache_hits += 1;
            return Ok(cached_instance.clone());
        }
        
        self.cache_misses += 1;
        self.instantiation_count += 1;
        
        // 创建类型参数映射
        let mut type_mapping = HashMap::new();
        for (i, generic_param) in base_class.generic_parameters.iter().enumerate() {
            if i < type_args.len() {
                type_mapping.insert(generic_param.name.clone(), type_args[i].clone());
            }
        }
        
        // 实例化字段类型
        let mut instantiated_fields = HashMap::new();
        for field in &base_class.fields {
            let instantiated_type = self.substitute_type(&field.field_type, &type_mapping)?;
            instantiated_fields.insert(field.name.clone(), instantiated_type);
        }
        
        // 实例化方法
        let mut instantiated_methods = Vec::new();
        for method in &base_class.methods {
            let method_function = Function {
                name: method.name.clone(),
                generic_parameters: method.generic_parameters.clone(),
                parameters: method.parameters.clone(),
                return_type: method.return_type.clone(),
                body: method.body.clone(),
                where_clause: Vec::new(), // 添加缺失的where_clause字段
            };
            
            let method_instance = self.instantiate_function(&method_function, type_args)?;
            instantiated_methods.push(method_instance);
        }
        
        let instance = GenericClassInstance {
            base_class: base_class.clone(),
            type_arguments: type_args.to_vec(),
            instantiated_fields,
            instantiated_methods,
        };
        
        self.class_instances.insert(instance_key, instance.clone());
        Ok(instance)
    }

    /// 类型替换 - 将泛型参数替换为具体类型
    fn substitute_type(&self, original_type: &Type, type_mapping: &HashMap<String, Type>) -> Result<Type, String> {
        match original_type {
            Type::Generic(name) => {
                if let Some(concrete_type) = type_mapping.get(name) {
                    Ok(concrete_type.clone())
                } else {
                    Err(format!("未找到泛型参数 {} 的具体类型", name))
                }
            },
            Type::GenericClass(class_name, args) => {
                let substituted_args: Result<Vec<Type>, String> = args.iter()
                    .map(|arg| self.substitute_type(arg, type_mapping))
                    .collect();
                Ok(Type::GenericClass(class_name.clone(), substituted_args?))
            },
            Type::GenericEnum(enum_name, args) => {
                let substituted_args: Result<Vec<Type>, String> = args.iter()
                    .map(|arg| self.substitute_type(arg, type_mapping))
                    .collect();
                Ok(Type::GenericEnum(enum_name.clone(), substituted_args?))
            },
            Type::Array(element_type) => {
                let substituted_element = self.substitute_type(element_type, type_mapping)?;
                Ok(Type::Array(Box::new(substituted_element)))
            },
            Type::Pointer(target_type) => {
                let substituted_target = self.substitute_type(target_type, type_mapping)?;
                Ok(Type::Pointer(Box::new(substituted_target)))
            },
            _ => Ok(original_type.clone()),
        }
    }

    /// 特化语句列表
    fn specialize_statements(&self, statements: &[Statement], type_mapping: &HashMap<String, Type>) -> Result<Vec<Statement>, String> {
        // 这里应该递归地处理所有语句中的类型引用
        // 当前版本返回原始语句，后续可以扩展
        Ok(statements.to_vec())
    }

    /// 生成类型实例的唯一键
    fn generate_type_key(&self, base_type: &Type, type_args: &[Type]) -> String {
        let base_name = self.extract_base_name(base_type);
        let args_str: Vec<String> = type_args.iter().map(|t| format!("{:?}", t)).collect();
        format!("{}[{}]", base_name, args_str.join(","))
    }

    /// 生成函数实例的唯一键
    fn generate_function_key(&self, function: &Function, type_args: &[Type]) -> String {
        let args_str: Vec<String> = type_args.iter().map(|t| format!("{:?}", t)).collect();
        format!("{}[{}]", function.name, args_str.join(","))
    }

    /// 生成类实例的唯一键
    fn generate_class_key(&self, class: &Class, type_args: &[Type]) -> String {
        let args_str: Vec<String> = type_args.iter().map(|t| format!("{:?}", t)).collect();
        format!("{}[{}]", class.name, args_str.join(","))
    }

    /// 提取基础类型名
    fn extract_base_name(&self, type_: &Type) -> String {
        match type_ {
            Type::Generic(name) => name.clone(),
            Type::GenericClass(name, _) => name.clone(),
            Type::GenericEnum(name, _) => name.clone(),
            Type::GenericFunction(name, _) => name.clone(),
            _ => format!("{:?}", type_),
        }
    }

    /// 生成函数签名
    fn generate_function_signature(&self, function: &Function, type_args: &[Type]) -> String {
        let args_str: Vec<String> = type_args.iter().map(|t| format!("{:?}", t)).collect();
        format!("{}[{}]", function.name, args_str.join(","))
    }

    /// 计算类型大小提示
    fn calculate_size_hint(&self, type_: &Type) -> Option<usize> {
        match type_ {
            Type::Int => Some(4),
            Type::Long => Some(8),
            Type::Float => Some(8),
            Type::Bool => Some(1),
            Type::String => Some(24), // 估算值
            _ => None,
        }
    }

    /// 获取当前时间戳
    fn get_current_time(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// 获取性能统计
    pub fn get_stats(&self) -> GenericManagerStats {
        GenericManagerStats {
            total_instantiations: self.instantiation_count,
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
            cached_types: self.type_instances.len(),
            cached_functions: self.function_instances.len(),
            cached_classes: self.class_instances.len(),
        }
    }

    /// 清理缓存
    pub fn clear_cache(&mut self) {
        self.type_instances.clear();
        self.function_instances.clear();
        self.class_instances.clear();
        self.inference_cache.clear();
    }
}

/// 泛型管理器性能统计
#[derive(Debug, Clone)]
pub struct GenericManagerStats {
    pub total_instantiations: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub cached_types: usize,
    pub cached_functions: usize,
    pub cached_classes: usize,
}
