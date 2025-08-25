use ::std::collections::HashMap;

// 导入通用库
use cn_common::namespace::{LibraryFunction, LibraryRegistry};

// 根命名空间数学函数
// 绝对值函数
fn cn_abs(args: Vec<String>) -> String {
    if args.is_empty() {
        return "0".to_string();
    }

    if let Ok(num) = args[0].parse::<f64>() {
        num.abs().to_string()
    } else {
        "0".to_string()
    }
}

// 最大值函数
fn cn_max(args: Vec<String>) -> String {
    if args.len() < 2 {
        return "0".to_string();
    }

    let a = args[0].parse::<f64>().unwrap_or(0.0);
    let b = args[1].parse::<f64>().unwrap_or(0.0);
    a.max(b).to_string()
}

// 最小值函数
fn cn_min(args: Vec<String>) -> String {
    if args.len() < 2 {
        return "0".to_string();
    }

    let a = args[0].parse::<f64>().unwrap_or(0.0);
    let b = args[1].parse::<f64>().unwrap_or(0.0);
    a.min(b).to_string()
}

// 幂函数
fn cn_pow(args: Vec<String>) -> String {
    if args.len() < 2 {
        return "0".to_string();
    }

    let base = args[0].parse::<f64>().unwrap_or(0.0);
    let exp = args[1].parse::<f64>().unwrap_or(0.0);
    base.powf(exp).to_string()
}

// 平方根函数
fn cn_sqrt(args: Vec<String>) -> String {
    if args.is_empty() {
        return "0".to_string();
    }

    if let Ok(num) = args[0].parse::<f64>() {
        if num >= 0.0 {
            num.sqrt().to_string()
        } else {
            "NaN".to_string()
        }
    } else {
        "0".to_string()
    }
}

// 立方根函数
fn cn_cbrt(args: Vec<String>) -> String {
    if args.is_empty() {
        return "0".to_string();
    }

    if let Ok(num) = args[0].parse::<f64>() {
        num.cbrt().to_string()
    } else {
        "0".to_string()
    }
}

// 向上取整
fn cn_ceil(args: Vec<String>) -> String {
    if args.is_empty() {
        return "0".to_string();
    }

    if let Ok(num) = args[0].parse::<f64>() {
        num.ceil().to_string()
    } else {
        "0".to_string()
    }
}

// 向下取整
fn cn_floor(args: Vec<String>) -> String {
    if args.is_empty() {
        return "0".to_string();
    }

    if let Ok(num) = args[0].parse::<f64>() {
        num.floor().to_string()
    } else {
        "0".to_string()
    }
}

// 四舍五入
fn cn_round(args: Vec<String>) -> String {
    if args.is_empty() {
        return "0".to_string();
    }

    if let Ok(num) = args[0].parse::<f64>() {
        num.round().to_string()
    } else {
        "0".to_string()
    }
}

// 截断小数部分
fn cn_trunc(args: Vec<String>) -> String {
    if args.is_empty() {
        return "0".to_string();
    }

    if let Ok(num) = args[0].parse::<f64>() {
        num.trunc().to_string()
    } else {
        "0".to_string()
    }
}

// 符号函数
fn cn_sign(args: Vec<String>) -> String {
    if args.is_empty() {
        return "0".to_string();
    }

    if let Ok(num) = args[0].parse::<f64>() {
        if num > 0.0 {
            "1".to_string()
        } else if num < 0.0 {
            "-1".to_string()
        } else {
            "0".to_string()
        }
    } else {
        "0".to_string()
    }
}

// 三角函数命名空间
mod trig {

    // 正弦函数
    pub fn cn_sin(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        if let Ok(num) = args[0].parse::<f64>() {
            num.sin().to_string()
        } else {
            "0".to_string()
        }
    }

    // 余弦函数
    pub fn cn_cos(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        if let Ok(num) = args[0].parse::<f64>() {
            num.cos().to_string()
        } else {
            "0".to_string()
        }
    }

    // 正切函数
    pub fn cn_tan(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        if let Ok(num) = args[0].parse::<f64>() {
            num.tan().to_string()
        } else {
            "0".to_string()
        }
    }

    // 反正弦函数
    pub fn cn_asin(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        if let Ok(num) = args[0].parse::<f64>() {
            if num >= -1.0 && num <= 1.0 {
                num.asin().to_string()
            } else {
                "NaN".to_string()
            }
        } else {
            "0".to_string()
        }
    }

    // 反余弦函数
    pub fn cn_acos(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        if let Ok(num) = args[0].parse::<f64>() {
            if num >= -1.0 && num <= 1.0 {
                num.acos().to_string()
            } else {
                "NaN".to_string()
            }
        } else {
            "0".to_string()
        }
    }

    // 反正切函数
    pub fn cn_atan(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        if let Ok(num) = args[0].parse::<f64>() {
            num.atan().to_string()
        } else {
            "0".to_string()
        }
    }

    // 角度转弧度
    pub fn cn_to_radians(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        if let Ok(degrees) = args[0].parse::<f64>() {
            degrees.to_radians().to_string()
        } else {
            "0".to_string()
        }
    }

    // 弧度转角度
    pub fn cn_to_degrees(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        if let Ok(radians) = args[0].parse::<f64>() {
            radians.to_degrees().to_string()
        } else {
            "0".to_string()
        }
    }
}

// 对数函数命名空间
mod log {

    // 自然对数
    pub fn cn_ln(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        if let Ok(num) = args[0].parse::<f64>() {
            if num > 0.0 {
                num.ln().to_string()
            } else {
                "NaN".to_string()
            }
        } else {
            "0".to_string()
        }
    }

    // 以10为底的对数
    pub fn cn_log10(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        if let Ok(num) = args[0].parse::<f64>() {
            if num > 0.0 {
                num.log10().to_string()
            } else {
                "NaN".to_string()
            }
        } else {
            "0".to_string()
        }
    }

    // 以2为底的对数
    pub fn cn_log2(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        if let Ok(num) = args[0].parse::<f64>() {
            if num > 0.0 {
                num.log2().to_string()
            } else {
                "NaN".to_string()
            }
        } else {
            "0".to_string()
        }
    }

    // 指定底数的对数
    pub fn cn_log(args: Vec<String>) -> String {
        if args.len() < 2 {
            return "0".to_string();
        }

        let num = args[0].parse::<f64>().unwrap_or(0.0);
        let base = args[1].parse::<f64>().unwrap_or(0.0);

        if num > 0.0 && base > 0.0 && base != 1.0 {
            (num.ln() / base.ln()).to_string()
        } else {
            "NaN".to_string()
        }
    }
}

// 双曲函数命名空间
mod hyperbolic {
    // 双曲正弦函数
    pub fn cn_sinh(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        if let Ok(num) = args[0].parse::<f64>() {
            num.sinh().to_string()
        } else {
            "0".to_string()
        }
    }

    // 双曲余弦函数
    pub fn cn_cosh(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        if let Ok(num) = args[0].parse::<f64>() {
            num.cosh().to_string()
        } else {
            "0".to_string()
        }
    }

    // 双曲正切函数
    pub fn cn_tanh(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        if let Ok(num) = args[0].parse::<f64>() {
            num.tanh().to_string()
        } else {
            "0".to_string()
        }
    }

    // 反双曲正弦函数
    pub fn cn_asinh(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        if let Ok(num) = args[0].parse::<f64>() {
            num.asinh().to_string()
        } else {
            "0".to_string()
        }
    }

    // 反双曲余弦函数
    pub fn cn_acosh(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        if let Ok(num) = args[0].parse::<f64>() {
            if num >= 1.0 {
                num.acosh().to_string()
            } else {
                "NaN".to_string()
            }
        } else {
            "0".to_string()
        }
    }

    // 反双曲正切函数
    pub fn cn_atanh(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        if let Ok(num) = args[0].parse::<f64>() {
            if num > -1.0 && num < 1.0 {
                num.atanh().to_string()
            } else {
                "NaN".to_string()
            }
        } else {
            "0".to_string()
        }
    }
}

// 统计函数命名空间
mod stats {
    // 计算平均值
    pub fn cn_mean(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        let mut sum = 0.0;
        let mut count = 0;

        for arg in args {
            if let Ok(num) = arg.parse::<f64>() {
                sum += num;
                count += 1;
            }
        }

        if count > 0 {
            (sum / count as f64).to_string()
        } else {
            "0".to_string()
        }
    }

    // 计算中位数
    pub fn cn_median(args: Vec<String>) -> String {
        if args.is_empty() {
            return "0".to_string();
        }

        let mut numbers: Vec<f64> = Vec::new();
        for arg in args {
            if let Ok(num) = arg.parse::<f64>() {
                numbers.push(num);
            }
        }

        if numbers.is_empty() {
            return "0".to_string();
        }

        numbers.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let len = numbers.len();

        if len % 2 == 0 {
            ((numbers[len / 2 - 1] + numbers[len / 2]) / 2.0).to_string()
        } else {
            numbers[len / 2].to_string()
        }
    }

    // 计算标准差
    pub fn cn_stddev(args: Vec<String>) -> String {
        if args.len() < 2 {
            return "0".to_string();
        }

        let mut numbers: Vec<f64> = Vec::new();
        for arg in args {
            if let Ok(num) = arg.parse::<f64>() {
                numbers.push(num);
            }
        }

        if numbers.len() < 2 {
            return "0".to_string();
        }

        let mean = numbers.iter().sum::<f64>() / numbers.len() as f64;
        let variance = numbers.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / (numbers.len() - 1) as f64;

        variance.sqrt().to_string()
    }

    // 计算方差
    pub fn cn_variance(args: Vec<String>) -> String {
        if args.len() < 2 {
            return "0".to_string();
        }

        let mut numbers: Vec<f64> = Vec::new();
        for arg in args {
            if let Ok(num) = arg.parse::<f64>() {
                numbers.push(num);
            }
        }

        if numbers.len() < 2 {
            return "0".to_string();
        }

        let mean = numbers.iter().sum::<f64>() / numbers.len() as f64;
        let variance = numbers.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / (numbers.len() - 1) as f64;

        variance.to_string()
    }
}

// 常数命名空间
mod constants {

    // 圆周率π
    pub fn cn_pi(_args: Vec<String>) -> String {
        std::f64::consts::PI.to_string()
    }

    // 自然常数e
    pub fn cn_e(_args: Vec<String>) -> String {
        std::f64::consts::E.to_string()
    }

    // 黄金比例φ
    pub fn cn_phi(_args: Vec<String>) -> String {
        ((1.0 + 5.0_f64.sqrt()) / 2.0).to_string()
    }

    // 2的平方根
    pub fn cn_sqrt2(_args: Vec<String>) -> String {
        std::f64::consts::SQRT_2.to_string()
    }

    // 欧拉常数（Euler-Mascheroni常数）
    pub fn cn_euler_gamma(_args: Vec<String>) -> String {
        "0.5772156649015329".to_string() // 欧拉常数的近似值
    }

    // 1/π
    pub fn cn_frac_1_pi(_args: Vec<String>) -> String {
        std::f64::consts::FRAC_1_PI.to_string()
    }

    // 2/π
    pub fn cn_frac_2_pi(_args: Vec<String>) -> String {
        std::f64::consts::FRAC_2_PI.to_string()
    }

    // ln(2)
    pub fn cn_ln_2(_args: Vec<String>) -> String {
        std::f64::consts::LN_2.to_string()
    }

    // ln(10)
    pub fn cn_ln_10(_args: Vec<String>) -> String {
        std::f64::consts::LN_10.to_string()
    }
}

// 随机数生成命名空间
mod random {
    use std::time::{SystemTime, UNIX_EPOCH};

    // 简单的线性同余生成器状态
    static mut RNG_STATE: u64 = 1;

    // 设置随机数种子
    pub fn cn_seed(args: Vec<String>) -> String {
        unsafe {
            if args.is_empty() {
                // 使用当前时间作为种子
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;
                RNG_STATE = now;
            } else if let Ok(seed) = args[0].parse::<u64>() {
                RNG_STATE = seed;
            }
            RNG_STATE.to_string()
        }
    }

    // 生成0到1之间的随机浮点数
    pub fn cn_random(_args: Vec<String>) -> String {
        unsafe {
            // 线性同余生成器: (a * x + c) mod m
            RNG_STATE = RNG_STATE.wrapping_mul(1103515245).wrapping_add(12345);
            let normalized = (RNG_STATE as f64) / (u64::MAX as f64);
            normalized.to_string()
        }
    }

    // 生成指定范围内的随机整数
    pub fn cn_randint(args: Vec<String>) -> String {
        if args.len() < 2 {
            return "0".to_string();
        }

        let min = args[0].parse::<i32>().unwrap_or(0);
        let max = args[1].parse::<i32>().unwrap_or(1);

        if min >= max {
            return min.to_string();
        }

        unsafe {
            RNG_STATE = RNG_STATE.wrapping_mul(1103515245).wrapping_add(12345);
            let range = (max - min) as u64;
            let result = min + ((RNG_STATE % range) as i32);
            result.to_string()
        }
    }

    // 生成指定范围内的随机浮点数
    pub fn cn_uniform(args: Vec<String>) -> String {
        if args.len() < 2 {
            return "0".to_string();
        }

        let min = args[0].parse::<f64>().unwrap_or(0.0);
        let max = args[1].parse::<f64>().unwrap_or(1.0);

        unsafe {
            RNG_STATE = RNG_STATE.wrapping_mul(1103515245).wrapping_add(12345);
            let normalized = (RNG_STATE as f64) / (u64::MAX as f64);
            let result = min + normalized * (max - min);
            result.to_string()
        }
    }
}

// 数值分析命名空间
mod numeric {
    // 计算阶乘
    pub fn cn_factorial(args: Vec<String>) -> String {
        if args.is_empty() {
            return "1".to_string();
        }

        if let Ok(n) = args[0].parse::<u32>() {
            if n > 20 {
                return "Infinity".to_string(); // 防止溢出
            }

            let mut result = 1u64;
            for i in 1..=n {
                result *= i as u64;
            }
            result.to_string()
        } else {
            "1".to_string()
        }
    }

    // 计算组合数 C(n, k)
    pub fn cn_combination(args: Vec<String>) -> String {
        if args.len() < 2 {
            return "0".to_string();
        }

        let n = args[0].parse::<u32>().unwrap_or(0);
        let k = args[1].parse::<u32>().unwrap_or(0);

        if k > n {
            return "0".to_string();
        }

        if k == 0 || k == n {
            return "1".to_string();
        }

        // 使用更稳定的计算方法
        let k = k.min(n - k); // 利用对称性
        let mut result = 1u64;

        for i in 0..k {
            result = result * (n - i) as u64 / (i + 1) as u64;
        }

        result.to_string()
    }

    // 计算排列数 P(n, k)
    pub fn cn_permutation(args: Vec<String>) -> String {
        if args.len() < 2 {
            return "0".to_string();
        }

        let n = args[0].parse::<u32>().unwrap_or(0);
        let k = args[1].parse::<u32>().unwrap_or(0);

        if k > n {
            return "0".to_string();
        }

        let mut result = 1u64;
        for i in 0..k {
            result *= (n - i) as u64;
        }

        result.to_string()
    }

    // 计算最大公约数
    pub fn cn_gcd(args: Vec<String>) -> String {
        if args.len() < 2 {
            return "0".to_string();
        }

        let mut a = args[0].parse::<u64>().unwrap_or(0);
        let mut b = args[1].parse::<u64>().unwrap_or(0);

        while b != 0 {
            let temp = b;
            b = a % b;
            a = temp;
        }

        a.to_string()
    }

    // 计算最小公倍数
    pub fn cn_lcm(args: Vec<String>) -> String {
        if args.len() < 2 {
            return "0".to_string();
        }

        let a = args[0].parse::<u64>().unwrap_or(0);
        let b = args[1].parse::<u64>().unwrap_or(0);

        if a == 0 || b == 0 {
            return "0".to_string();
        }

        // 使用 lcm(a,b) = |a*b| / gcd(a,b)
        let mut gcd_a = a;
        let mut gcd_b = b;

        while gcd_b != 0 {
            let temp = gcd_b;
            gcd_b = gcd_a % gcd_b;
            gcd_a = temp;
        }

        let result = (a * b) / gcd_a;
        result.to_string()
    }
}

// 初始化函数，返回函数映射
#[no_mangle]
pub extern "C" fn cn_init() -> *mut HashMap<String, LibraryFunction> {
    // 创建库函数注册器
    let mut registry = LibraryRegistry::new();

    // 注册根命名空间的基础数学函数
    registry.add_direct_function("abs", cn_abs)
            .add_direct_function("max", cn_max)
            .add_direct_function("min", cn_min)
            .add_direct_function("pow", cn_pow)
            .add_direct_function("sqrt", cn_sqrt)
            .add_direct_function("cbrt", cn_cbrt)
            .add_direct_function("ceil", cn_ceil)
            .add_direct_function("floor", cn_floor)
            .add_direct_function("round", cn_round)
            .add_direct_function("trunc", cn_trunc)
            .add_direct_function("sign", cn_sign);

    // 注册三角函数命名空间
    let trig_ns = registry.namespace("trig");
    trig_ns.add_function("sin", trig::cn_sin)
           .add_function("cos", trig::cn_cos)
           .add_function("tan", trig::cn_tan)
           .add_function("asin", trig::cn_asin)
           .add_function("acos", trig::cn_acos)
           .add_function("atan", trig::cn_atan)
           .add_function("to_radians", trig::cn_to_radians)
           .add_function("to_degrees", trig::cn_to_degrees);

    // 注册对数函数命名空间
    let log_ns = registry.namespace("log");
    log_ns.add_function("ln", log::cn_ln)
          .add_function("log10", log::cn_log10)
          .add_function("log2", log::cn_log2)
          .add_function("log", log::cn_log);

    // 注册双曲函数命名空间
    let hyp_ns = registry.namespace("hyperbolic");
    hyp_ns.add_function("sinh", hyperbolic::cn_sinh)
          .add_function("cosh", hyperbolic::cn_cosh)
          .add_function("tanh", hyperbolic::cn_tanh)
          .add_function("asinh", hyperbolic::cn_asinh)
          .add_function("acosh", hyperbolic::cn_acosh)
          .add_function("atanh", hyperbolic::cn_atanh);

    // 注册统计函数命名空间
    let stats_ns = registry.namespace("stats");
    stats_ns.add_function("mean", stats::cn_mean)
            .add_function("median", stats::cn_median)
            .add_function("stddev", stats::cn_stddev)
            .add_function("variance", stats::cn_variance);

    // 注册随机数生成命名空间
    let random_ns = registry.namespace("random");
    random_ns.add_function("seed", random::cn_seed)
             .add_function("random", random::cn_random)
             .add_function("randint", random::cn_randint)
             .add_function("uniform", random::cn_uniform);

    // 注册数值分析命名空间
    let numeric_ns = registry.namespace("numeric");
    numeric_ns.add_function("factorial", numeric::cn_factorial)
              .add_function("combination", numeric::cn_combination)
              .add_function("permutation", numeric::cn_permutation)
              .add_function("gcd", numeric::cn_gcd)
              .add_function("lcm", numeric::cn_lcm);

    // 注册常数命名空间
    let const_ns = registry.namespace("constants");
    const_ns.add_function("pi", constants::cn_pi)
            .add_function("e", constants::cn_e)
            .add_function("phi", constants::cn_phi)
            .add_function("sqrt2", constants::cn_sqrt2)
            .add_function("euler_gamma", constants::cn_euler_gamma)
            .add_function("frac_1_pi", constants::cn_frac_1_pi)
            .add_function("frac_2_pi", constants::cn_frac_2_pi)
            .add_function("ln_2", constants::cn_ln_2)
            .add_function("ln_10", constants::cn_ln_10);

    // 构建并返回库指针
    registry.build_library_pointer()
}

/*
 * CodeNothing 扩展数学库 (Extended Math Library)
 *
 * 提供全面的数学计算功能，包括：
 * - 基础数学函数
 * - 三角函数和双曲函数
 * - 对数函数
 * - 统计函数
 * - 随机数生成
 * - 数值分析函数
 * - 数学常数
 *
 * 使用方法：
 *
 * 1. 在 CodeNothing 代码中导入库：
 *    using lib <math>;
 *
 * 2. 基础数学函数（根命名空间）：
 *    result : float = abs("-5.5");          // 绝对值
 *    result : float = max("3.14", "2.71");  // 最大值
 *    result : float = sqrt("16");           // 平方根
 *    result : float = cbrt("8");            // 立方根
 *    result : float = ceil("3.2");          // 向上取整
 *    result : float = floor("3.8");         // 向下取整
 *    result : float = round("3.6");         // 四舍五入
 *    result : int = sign("-5");             // 符号函数
 *
 * 3. 三角函数：
 *    using ns trig;
 *    result : float = sin("1.57");          // 正弦值
 *    result : float = cos("0");             // 余弦值
 *    result : float = to_radians("90");     // 角度转弧度
 *
 * 4. 双曲函数：
 *    using ns hyperbolic;
 *    result : float = sinh("1");            // 双曲正弦
 *    result : float = cosh("0");            // 双曲余弦
 *    result : float = tanh("1");            // 双曲正切
 *
 * 5. 统计函数：
 *    using ns stats;
 *    result : float = mean("1", "2", "3", "4", "5");     // 平均值
 *    result : float = median("1", "3", "2", "5", "4");   // 中位数
 *    result : float = stddev("1", "2", "3", "4", "5");   // 标准差
 *
 * 6. 随机数生成：
 *    using ns random;
 *    seed("12345");                         // 设置随机种子
 *    result : float = random();             // 0-1随机数
 *    result : int = randint("1", "10");     // 1-10随机整数
 *    result : float = uniform("0", "100");  // 0-100随机浮点数
 *
 * 7. 数值分析：
 *    using ns numeric;
 *    result : int = factorial("5");         // 阶乘: 120
 *    result : int = combination("5", "2");  // 组合数: 10
 *    result : int = gcd("12", "8");         // 最大公约数: 4
 *
 * 8. 数学常数：
 *    using ns constants;
 *    pi_val : float = pi();                 // 圆周率π
 *    e_val : float = e();                   // 自然常数e
 *    gamma_val : float = euler_gamma();     // 欧拉常数
 *
 * 注意：
 * - 函数返回值会自动转换为适当的数值类型
 * - 无效输入会返回 "0" 或 "NaN"
 * - 三角函数和双曲函数使用弧度制
 * - 统计函数可接受多个参数
 */