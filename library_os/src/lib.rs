use ::std::collections::HashMap;
use ::std::env;
use ::std::process::Command;
use sysinfo::{System, SystemExt, DiskExt, ProcessExt, CpuExt};

// 导入通用库
use cn_common::namespace::{LibraryFunction, LibraryRegistry};

// 命名空间函数
mod std {
    use super::*;
    
    // 获取操作系统名称
    pub fn cn_os_name(_args: Vec<String>) -> String {
        env::consts::OS.to_string()
    }
    
    // 获取操作系统家族
    pub fn cn_os_family(_args: Vec<String>) -> String {
        env::consts::FAMILY.to_string()
    }
    
    // 获取系统架构
    pub fn cn_os_arch(_args: Vec<String>) -> String {
        env::consts::ARCH.to_string()
    }
    
    // 获取环境变量
    // 参数: env_var_name
    pub fn cn_env(args: Vec<String>) -> String {
        if args.is_empty() {
            return "错误: 缺少环境变量名参数".to_string();
        }
        
        match env::var(&args[0]) {
            Ok(val) => val,
            Err(_) => "".to_string(),
        }
    }
    
    // 获取所有环境变量
    pub fn cn_env_all(_args: Vec<String>) -> String {
        let mut result = String::new();
        for (key, value) in env::vars() {
            result.push_str(&format!("{}={}\n", key, value));
        }
        result
    }
    
    // 获取当前工作目录
    pub fn cn_cwd(_args: Vec<String>) -> String {
        match env::current_dir() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => "错误: 无法获取当前工作目录".to_string(),
        }
    }
    
    // 获取用户主目录
    pub fn cn_home_dir(_args: Vec<String>) -> String {
        match dirs::home_dir() {
            Some(path) => path.to_string_lossy().to_string(),
            None => "错误: 无法获取用户主目录".to_string(),
        }
    }
    
    // 获取临时目录
    pub fn cn_temp_dir(_args: Vec<String>) -> String {
        env::temp_dir().to_string_lossy().to_string()
    }
    
    // 获取主机名
    pub fn cn_hostname(_args: Vec<String>) -> String {
        match hostname::get() {
            Ok(name) => name.to_string_lossy().to_string(),
            Err(_) => "错误: 无法获取主机名".to_string(),
        }
    }
    
    // 获取内存信息
    pub fn cn_memory(_args: Vec<String>) -> String {
        let mut system = System::new_all();
        system.refresh_all();
        
        let total_mem = system.total_memory();
        let used_mem = system.used_memory();
        let total_swap = system.total_swap();
        let used_swap = system.used_swap();
        
        format!("内存总量: {} KB\n已用内存: {} KB\n交换空间总量: {} KB\n已用交换空间: {} KB",
            total_mem, used_mem, total_swap, used_swap)
    }
    
    // 获取CPU信息
    pub fn cn_cpu_info(_args: Vec<String>) -> String {
        let mut system = System::new_all();
        system.refresh_all();
        
        let mut result = String::new();
        result.push_str(&format!("CPU核心数: {}\n", system.cpus().len()));
        
        for (i, cpu) in system.cpus().iter().enumerate() {
            result.push_str(&format!("CPU {}: {}%, {}\n", 
                i, cpu.cpu_usage(), cpu.brand()));
        }
        
        result
    }
    
    // 获取磁盘信息
    pub fn cn_disk_info(_args: Vec<String>) -> String {
        let mut system = System::new_all();
        system.refresh_disks_list();
        
        let mut result = String::new();
        
        for disk in system.disks() {
            result.push_str(&format!("磁盘: {}\n总空间: {} KB\n可用空间: {} KB\n挂载点: {}\n\n",
                disk.name().to_string_lossy(),
                disk.total_space() / 1024,
                disk.available_space() / 1024,
                disk.mount_point().to_string_lossy()));
        }
        
        result
    }
    
    // 执行系统命令
    // 参数: command, [arg1, arg2, ...]
    pub fn cn_exec(args: Vec<String>) -> String {
        if args.is_empty() {
            return "错误: 缺少命令参数".to_string();
        }
        
        let command = &args[0];
        let command_args = &args[1..];
        
        match Command::new(command).args(command_args).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                
                if !stderr.is_empty() {
                    format!("{}错误输出:\n{}", stdout, stderr)
                } else {
                    stdout
                }
            },
            Err(e) => format!("执行命令失败: {}", e),
        }
    }
    
    // 获取进程列表
    pub fn cn_processes(_args: Vec<String>) -> String {
        let mut system = System::new_all();
        system.refresh_all();
        
        let mut result = String::new();
        result.push_str("PID\t名称\t内存使用(KB)\tCPU使用(%)\n");
        
        for (pid, process) in system.processes() {
            result.push_str(&format!("{}\t{}\t{}\t{}\n",
                pid,
                process.name(),
                process.memory(),
                process.cpu_usage()));
        }
        
        result
    }
    
    // 获取系统启动时间（秒）
    pub fn cn_uptime(_args: Vec<String>) -> String {
        let mut system = System::new_all();
        system.refresh_all();
        
        system.uptime().to_string()
    }
    
    // 获取当前用户名
    pub fn cn_username(_args: Vec<String>) -> String {
        match env::var("USERNAME").or_else(|_| env::var("USER")) {
            Ok(name) => name,
            Err(_) => "未知用户".to_string(),
        }
    }
    
    // 检查是否是Windows系统
    pub fn cn_is_windows(_args: Vec<String>) -> String {
        if cfg!(target_os = "windows") {
            "true".to_string()
        } else {
            "false".to_string()
        }
    }
    
    // 检查是否是Linux系统
    pub fn cn_is_linux(_args: Vec<String>) -> String {
        if cfg!(target_os = "linux") {
            "true".to_string()
        } else {
            "false".to_string()
        }
    }
    
    // 检查是否是macOS系统
    pub fn cn_is_macos(_args: Vec<String>) -> String {
        if cfg!(target_os = "macos") {
            "true".to_string()
        } else {
            "false".to_string()
        }
    }
}

// 初始化函数，返回函数映射
#[no_mangle]
pub extern "C" fn cn_init() -> *mut HashMap<String, LibraryFunction> {
    // 创建库函数注册器
    let mut registry = LibraryRegistry::new();
    
    // 注册std命名空间下的函数
    let std_ns = registry.namespace("std");
    std_ns.add_function("os_name", std::cn_os_name)
         .add_function("os_family", std::cn_os_family)
         .add_function("os_arch", std::cn_os_arch)
         .add_function("env", std::cn_env)
         .add_function("env_all", std::cn_env_all)
         .add_function("cwd", std::cn_cwd)
         .add_function("home_dir", std::cn_home_dir)
         .add_function("temp_dir", std::cn_temp_dir)
         .add_function("hostname", std::cn_hostname)
         .add_function("memory", std::cn_memory)
         .add_function("cpu_info", std::cn_cpu_info)
         .add_function("disk_info", std::cn_disk_info)
         .add_function("exec", std::cn_exec)
         .add_function("processes", std::cn_processes)
         .add_function("uptime", std::cn_uptime)
         .add_function("username", std::cn_username)
         .add_function("is_windows", std::cn_is_windows)
         .add_function("is_linux", std::cn_is_linux)
         .add_function("is_macos", std::cn_is_macos);
    
    // 同时注册为直接函数，不需要命名空间前缀
    registry.add_direct_function("os_name", std::cn_os_name)
            .add_direct_function("username", std::cn_username)
            .add_direct_function("hostname", std::cn_hostname)
            .add_direct_function("exec", std::cn_exec);
    
    // 构建并返回库指针
    registry.build_library_pointer()
} 