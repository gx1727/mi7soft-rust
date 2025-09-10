//! 基础的 Hello World Rust 程序
//! 这是学习 Rust 的第一步

fn main() {
    println!("Hello, World!");
    println!("欢迎来到 Rust 编程世界！");
    
    // 变量和数据类型示例
    let name = "Rust 学习者";
    let age = 25;
    let is_learning = true;
    
    println!("姓名: {}", name);
    println!("年龄: {}", age);
    println!("正在学习: {}", is_learning);
    
    // 函数调用示例
    greet_user(name);
    
    // 基础数学运算
    let result = add_numbers(10, 20);
    println!("10 + 20 = {}", result);
    
    // 控制流示例
    demonstrate_control_flow();
}

/// 问候用户的函数
fn greet_user(name: &str) {
    println!("你好, {}! 准备开始学习 Rust 和 Axum 吧！", name);
}

/// 加法函数
fn add_numbers(a: i32, b: i32) -> i32 {
    a + b
}

/// 演示控制流
fn demonstrate_control_flow() {
    println!("\n=== 控制流示例 ===");
    
    // if-else 语句
    let number = 42;
    if number > 0 {
        println!("{} 是正数", number);
    } else if number < 0 {
        println!("{} 是负数", number);
    } else {
        println!("{} 是零", number);
    }
    
    // 循环示例
    println!("\n数字 1 到 5:");
    for i in 1..=5 {
        println!("数字: {}", i);
    }
    
    // 向量和迭代器
    let fruits = vec!["苹果", "香蕉", "橙子"];
    println!("\n水果列表:");
    for (index, fruit) in fruits.iter().enumerate() {
        println!("{}: {}", index + 1, fruit);
    }
}