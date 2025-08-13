use crate::ast::{Expression, BinaryOperator, CompareOperator, LogicalOperator};
use crate::interpreter::value::Value;
use std::collections::HashMap;
use super::jit;

pub trait Evaluator {
    fn evaluate_expression(&mut self, expr: &Expression) -> Value;
    fn perform_binary_operation(&self, left: &Value, op: &BinaryOperator, right: &Value) -> Value;
    fn get_variable(&self, name: &str) -> Option<Value>;
    fn call_function(&mut self, function_name: &str, args: Vec<Value>) -> Value;
}

pub fn perform_binary_operation(left: &Value, op: &BinaryOperator, right: &Value) -> Value {
    match (left, op, right) {
        // 整数运算（直接计算，避免JIT开销）
        (Value::Int(l), BinaryOperator::Add, Value::Int(r)) => Value::Int(l + r),
        (Value::Int(l), BinaryOperator::Subtract, Value::Int(r)) => Value::Int(l - r),
        (Value::Int(l), BinaryOperator::Multiply, Value::Int(r)) => Value::Int(l * r),
        (Value::Int(l), BinaryOperator::Divide, Value::Int(r)) => {
            if *r == 0 { panic!("除以零错误"); }
            Value::Int(l / r)
        },
        (Value::Int(l), BinaryOperator::Modulo, Value::Int(r)) => {
            if *r == 0 { panic!("除以零错误"); }
            Value::Int(l % r)
        },
        // 浮点数运算（直接计算）
        (Value::Float(l), BinaryOperator::Add, Value::Float(r)) => Value::Float(l + r),
        (Value::Float(l), BinaryOperator::Subtract, Value::Float(r)) => Value::Float(l - r),
        (Value::Float(l), BinaryOperator::Multiply, Value::Float(r)) => Value::Float(l * r),
        (Value::Float(l), BinaryOperator::Divide, Value::Float(r)) => {
            if *r == 0.0 { panic!("除以零错误"); }
            Value::Float(l / r)
        },
        // 长整型运算（直接计算）
        (Value::Long(l), BinaryOperator::Add, Value::Long(r)) => Value::Long(l + r),
        (Value::Long(l), BinaryOperator::Subtract, Value::Long(r)) => Value::Long(l - r),
        (Value::Long(l), BinaryOperator::Multiply, Value::Long(r)) => Value::Long(l * r),
        (Value::Long(l), BinaryOperator::Divide, Value::Long(r)) => {
            if *r == 0 { panic!("除以零错误"); }
            Value::Long(l / r)
        },
        (Value::Long(l), BinaryOperator::Modulo, Value::Long(r)) => Value::Long(jit::jit_mod(*l, *r)),
        
        // 整数和浮点数混合运算
        (Value::Int(l), BinaryOperator::Add, Value::Float(r)) => Value::Float(*l as f64 + r),
        (Value::Float(l), BinaryOperator::Add, Value::Int(r)) => Value::Float(l + *r as f64),
        (Value::Int(l), BinaryOperator::Subtract, Value::Float(r)) => Value::Float(*l as f64 - r),
        (Value::Float(l), BinaryOperator::Subtract, Value::Int(r)) => Value::Float(l - *r as f64),
        (Value::Int(l), BinaryOperator::Multiply, Value::Float(r)) => Value::Float(*l as f64 * r),
        (Value::Float(l), BinaryOperator::Multiply, Value::Int(r)) => Value::Float(l * *r as f64),
        (Value::Int(l), BinaryOperator::Divide, Value::Float(r)) => {
            if *r == 0.0 {
                panic!("除以零错误");
            }
            Value::Float(*l as f64 / r)
        },
        (Value::Float(l), BinaryOperator::Divide, Value::Int(r)) => {
            if *r == 0 {
                panic!("除以零错误");
            }
            Value::Float(l / *r as f64)
        },
        
        // 整数和长整型混合运算
        (Value::Int(l), BinaryOperator::Add, Value::Long(r)) => Value::Long(*l as i64 + r),
        (Value::Long(l), BinaryOperator::Add, Value::Int(r)) => Value::Long(l + *r as i64),
        (Value::Int(l), BinaryOperator::Subtract, Value::Long(r)) => Value::Long(*l as i64 - r),
        (Value::Long(l), BinaryOperator::Subtract, Value::Int(r)) => Value::Long(l - *r as i64),
        (Value::Int(l), BinaryOperator::Multiply, Value::Long(r)) => Value::Long(*l as i64 * r),
        (Value::Long(l), BinaryOperator::Multiply, Value::Int(r)) => Value::Long(l * *r as i64),
        (Value::Int(l), BinaryOperator::Divide, Value::Long(r)) => {
            if *r == 0 {
                panic!("除以零错误");
            }
            Value::Long(*l as i64 / r)
        },
        (Value::Long(l), BinaryOperator::Divide, Value::Int(r)) => {
            if *r == 0 {
                panic!("除以零错误");
            }
            Value::Long(l / *r as i64)
        },
        
        // 字符串连接
        (Value::String(l), BinaryOperator::Add, Value::String(r)) => Value::String(l.clone() + r),
        
        // 字符串和其他类型的连接
        (Value::String(l), BinaryOperator::Add, Value::Int(r)) => Value::String(l.clone() + &r.to_string()),
        (Value::String(l), BinaryOperator::Add, Value::Float(r)) => Value::String(l.clone() + &r.to_string()),
        (Value::String(l), BinaryOperator::Add, Value::Bool(r)) => Value::String(l.clone() + &r.to_string()),
        (Value::String(l), BinaryOperator::Add, Value::Long(r)) => Value::String(l.clone() + &r.to_string()),
        
        // 其他类型和字符串的连接
        (Value::Int(l), BinaryOperator::Add, Value::String(r)) => Value::String(l.to_string() + r),
        (Value::Float(l), BinaryOperator::Add, Value::String(r)) => Value::String(l.to_string() + r),
        (Value::Bool(l), BinaryOperator::Add, Value::String(r)) => Value::String(l.to_string() + r),
        (Value::Long(l), BinaryOperator::Add, Value::String(r)) => Value::String(l.to_string() + r),
        (Value::EnumValue(l), BinaryOperator::Add, Value::String(r)) => {
            let enum_str = if l.fields.is_empty() {
                format!("{}::{}", l.enum_name, l.variant_name)
            } else {
                let field_strs: Vec<String> = l.fields.iter().map(|f| f.to_string()).collect();
                format!("{}::{}({})", l.enum_name, l.variant_name, field_strs.join(", "))
            };
            Value::String(enum_str + r)
        },

        // 字符串和其他类型的连接（包括EnumValue）
        (Value::String(l), BinaryOperator::Add, Value::EnumValue(r)) => {
            let enum_str = if r.fields.is_empty() {
                format!("{}::{}", r.enum_name, r.variant_name)
            } else {
                let field_strs: Vec<String> = r.fields.iter().map(|f| f.to_string()).collect();
                format!("{}::{}({})", r.enum_name, r.variant_name, field_strs.join(", "))
            };
            Value::String(l.clone() + &enum_str)
        },

        // 字符串和指针的连接
        (Value::String(l), BinaryOperator::Add, Value::Pointer(r)) => {
            let ptr_str = if r.is_null {
                "null".to_string()
            } else {
                format!("*{:p}", r.address as *const usize)
            };
            Value::String(l.clone() + &ptr_str)
        },

        // 指针算术运算
        (Value::Pointer(ptr), BinaryOperator::Add, Value::Int(offset)) => {
            use crate::interpreter::expression_evaluator::ExpressionEvaluator;
            // 这里需要调用指针算术逻辑
            let element_size = match ptr.target_type {
                crate::interpreter::value::PointerType::Int => 4,
                crate::interpreter::value::PointerType::Float => 4,
                crate::interpreter::value::PointerType::Bool => 1,
                crate::interpreter::value::PointerType::String => 8,
                crate::interpreter::value::PointerType::Long => 8,
                _ => 8,
            };
            let new_address = ptr.address + (*offset as usize * element_size);

            let new_ptr = crate::interpreter::value::PointerInstance {
                address: new_address,
                target_type: ptr.target_type.clone(),
                is_null: false,
                level: ptr.level,
                tag_id: None, // 算术结果不继承标记
            };

            Value::Pointer(new_ptr)
        },
        (Value::Pointer(ptr), BinaryOperator::Subtract, Value::Int(offset)) => {
            let element_size = match ptr.target_type {
                crate::interpreter::value::PointerType::Int => 4,
                crate::interpreter::value::PointerType::Float => 4,
                crate::interpreter::value::PointerType::Bool => 1,
                crate::interpreter::value::PointerType::String => 8,
                crate::interpreter::value::PointerType::Long => 8,
                _ => 8,
            };
            let new_address = ptr.address - (*offset as usize * element_size);

            let new_ptr = crate::interpreter::value::PointerInstance {
                address: new_address,
                target_type: ptr.target_type.clone(),
                is_null: false,
                level: ptr.level,
                tag_id: None, // 算术结果不继承标记
            };

            Value::Pointer(new_ptr)
        },
        (Value::Pointer(ptr1), BinaryOperator::Subtract, Value::Pointer(ptr2)) => {
            let element_size = match ptr1.target_type {
                crate::interpreter::value::PointerType::Int => 4,
                crate::interpreter::value::PointerType::Float => 4,
                crate::interpreter::value::PointerType::Bool => 1,
                crate::interpreter::value::PointerType::String => 8,
                crate::interpreter::value::PointerType::Long => 8,
                _ => 8,
            };
            let diff = (ptr1.address as isize - ptr2.address as isize) / element_size as isize;
            Value::Int(diff as i32)
        },

        // 指针和字符串的连接
        (Value::Pointer(l), BinaryOperator::Add, Value::String(r)) => {
            let ptr_str = if l.is_null {
                "null".to_string()
            } else {
                let stars = "*".repeat(l.level);
                format!("{}0x{:x}", stars, l.address)
            };
            Value::String(ptr_str + r)
        },

        // 处理None值的字符串拼接
        (Value::String(l), BinaryOperator::Add, Value::None) => {
            eprintln!("警告: 字符串拼接中遇到None值");
            Value::String(l.clone() + "null")
        },
        (Value::None, BinaryOperator::Add, Value::String(r)) => {
            eprintln!("警告: 字符串拼接中遇到None值");
            Value::String("null".to_string() + r)
        },

        // v0.7.2新增：位运算操作符支持
        // 按位与操作
        (Value::Int(l), BinaryOperator::BitwiseAnd, Value::Int(r)) => Value::Int(l & r),
        (Value::Long(l), BinaryOperator::BitwiseAnd, Value::Long(r)) => Value::Long(l & r),
        (Value::Int(l), BinaryOperator::BitwiseAnd, Value::Long(r)) => Value::Long((*l as i64) & r),
        (Value::Long(l), BinaryOperator::BitwiseAnd, Value::Int(r)) => Value::Long(l & (*r as i64)),

        // 按位或操作
        (Value::Int(l), BinaryOperator::BitwiseOr, Value::Int(r)) => Value::Int(l | r),
        (Value::Long(l), BinaryOperator::BitwiseOr, Value::Long(r)) => Value::Long(l | r),
        (Value::Int(l), BinaryOperator::BitwiseOr, Value::Long(r)) => Value::Long((*l as i64) | r),
        (Value::Long(l), BinaryOperator::BitwiseOr, Value::Int(r)) => Value::Long(l | (*r as i64)),

        // 按位异或操作
        (Value::Int(l), BinaryOperator::BitwiseXor, Value::Int(r)) => Value::Int(l ^ r),
        (Value::Long(l), BinaryOperator::BitwiseXor, Value::Long(r)) => Value::Long(l ^ r),
        (Value::Int(l), BinaryOperator::BitwiseXor, Value::Long(r)) => Value::Long((*l as i64) ^ r),
        (Value::Long(l), BinaryOperator::BitwiseXor, Value::Int(r)) => Value::Long(l ^ (*r as i64)),

        // 左移操作
        (Value::Int(l), BinaryOperator::LeftShift, Value::Int(r)) => {
            if *r < 0 || *r >= 32 {
                panic!("移位操作数超出范围: {}", r);
            }
            Value::Int(l << r)
        },
        (Value::Long(l), BinaryOperator::LeftShift, Value::Int(r)) => {
            if *r < 0 || *r >= 64 {
                panic!("移位操作数超出范围: {}", r);
            }
            Value::Long(l << r)
        },
        (Value::Int(l), BinaryOperator::LeftShift, Value::Long(r)) => {
            if *r < 0 || *r >= 32 {
                panic!("移位操作数超出范围: {}", r);
            }
            Value::Int(l << r)
        },

        // 右移操作
        (Value::Int(l), BinaryOperator::RightShift, Value::Int(r)) => {
            if *r < 0 || *r >= 32 {
                panic!("移位操作数超出范围: {}", r);
            }
            Value::Int(l >> r)
        },
        (Value::Long(l), BinaryOperator::RightShift, Value::Int(r)) => {
            if *r < 0 || *r >= 64 {
                panic!("移位操作数超出范围: {}", r);
            }
            Value::Long(l >> r)
        },
        (Value::Int(l), BinaryOperator::RightShift, Value::Long(r)) => {
            if *r < 0 || *r >= 32 {
                panic!("移位操作数超出范围: {}", r);
            }
            Value::Int(l >> r)
        },

        // 不支持的操作
        _ => panic!("不支持的二元操作: {:?} {:?} {:?}", left, op, right),
    }
}

pub fn evaluate_compare_operation(left: &Value, op: &CompareOperator, right: &Value) -> Value {
    match (op, left, right) {
        // 整数比较（JIT）
        (CompareOperator::Equal, Value::Int(l), Value::Int(r)) => Value::Bool(jit::jit_eq_i64((*l).into(), (*r).into())),
        (CompareOperator::NotEqual, Value::Int(l), Value::Int(r)) => Value::Bool(jit::jit_ne_i64((*l).into(), (*r).into())),
        (CompareOperator::Greater, Value::Int(l), Value::Int(r)) => Value::Bool(jit::jit_gt_i64((*l).into(), (*r).into())),
        (CompareOperator::Less, Value::Int(l), Value::Int(r)) => Value::Bool(jit::jit_lt_i64((*l).into(), (*r).into())),
        (CompareOperator::GreaterEqual, Value::Int(l), Value::Int(r)) => Value::Bool(jit::jit_ge_i64((*l).into(), (*r).into())),
        (CompareOperator::LessEqual, Value::Int(l), Value::Int(r)) => Value::Bool(jit::jit_le_i64((*l).into(), (*r).into())),
        // 浮点数比较（JIT）
        (CompareOperator::Equal, Value::Float(l), Value::Float(r)) => Value::Bool(jit::jit_eq_f64(*l, *r)),
        (CompareOperator::NotEqual, Value::Float(l), Value::Float(r)) => Value::Bool(jit::jit_ne_f64(*l, *r)),
        (CompareOperator::Greater, Value::Float(l), Value::Float(r)) => Value::Bool(jit::jit_gt_f64(*l, *r)),
        (CompareOperator::Less, Value::Float(l), Value::Float(r)) => Value::Bool(jit::jit_lt_f64(*l, *r)),
        (CompareOperator::GreaterEqual, Value::Float(l), Value::Float(r)) => Value::Bool(jit::jit_ge_f64(*l, *r)),
        (CompareOperator::LessEqual, Value::Float(l), Value::Float(r)) => Value::Bool(jit::jit_le_f64(*l, *r)),
        // 长整型比较（JIT）
        (CompareOperator::Equal, Value::Long(l), Value::Long(r)) => Value::Bool(jit::jit_eq_i64(*l, *r)),
        (CompareOperator::NotEqual, Value::Long(l), Value::Long(r)) => Value::Bool(jit::jit_ne_i64(*l, *r)),
        (CompareOperator::Greater, Value::Long(l), Value::Long(r)) => Value::Bool(jit::jit_gt_i64(*l, *r)),
        (CompareOperator::Less, Value::Long(l), Value::Long(r)) => Value::Bool(jit::jit_lt_i64(*l, *r)),
        (CompareOperator::GreaterEqual, Value::Long(l), Value::Long(r)) => Value::Bool(jit::jit_ge_i64(*l, *r)),
        (CompareOperator::LessEqual, Value::Long(l), Value::Long(r)) => Value::Bool(jit::jit_le_i64(*l, *r)),
        
        // 字符串比较
        (CompareOperator::Equal, Value::String(l), Value::String(r)) => Value::Bool(l == r),
        (CompareOperator::NotEqual, Value::String(l), Value::String(r)) => Value::Bool(l != r),
        
        // 布尔值比较
        (CompareOperator::Equal, Value::Bool(l), Value::Bool(r)) => Value::Bool(l == r),
        (CompareOperator::NotEqual, Value::Bool(l), Value::Bool(r)) => Value::Bool(l != r),

        // 枚举值比较
        (CompareOperator::Equal, Value::EnumValue(l), Value::EnumValue(r)) => {
            Value::Bool(l.enum_name == r.enum_name && l.variant_name == r.variant_name && l.fields == r.fields)
        },
        (CompareOperator::NotEqual, Value::EnumValue(l), Value::EnumValue(r)) => {
            Value::Bool(!(l.enum_name == r.enum_name && l.variant_name == r.variant_name && l.fields == r.fields))
        },

        // 混合类型比较
        (CompareOperator::Equal, _, _) => Value::Bool(false), // 不同类型永远不相等
        (CompareOperator::NotEqual, _, _) => Value::Bool(true), // 不同类型永远不相等
        
        // 不支持的比较
        _ => panic!("不支持的比较操作: {:?} {:?} {:?}", left, op, right),
    }
} 