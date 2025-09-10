# Rust 基础语法指南

本文档介绍 Rust 语言的核心概念和基础语法，帮助初学者快速上手。

## 📋 目录

1. [变量和数据类型](#变量和数据类型)
2. [函数](#函数)
3. [控制流](#控制流)
4. [所有权系统](#所有权系统)
5. [结构体和枚举](#结构体和枚举)
6. [错误处理](#错误处理)
7. [模块系统](#模块系统)
8. [常用集合类型](#常用集合类型)

## 变量和数据类型

### 变量声明

```rust
// 不可变变量（默认）
let x = 5;

// 可变变量
let mut y = 10;
y = 15; // 可以修改

// 常量
const MAX_POINTS: u32 = 100_000;

// 变量遮蔽（Shadowing）
let x = x + 1; // 创建新变量，遮蔽之前的 x
```

### 基本数据类型

```rust
// 整数类型
let a: i32 = 42;        // 32位有符号整数
let b: u64 = 100;       // 64位无符号整数
let c = 255u8;          // 8位无符号整数，类型后缀

// 浮点类型
let x: f64 = 3.14;      // 64位浮点数（默认）
let y: f32 = 2.0;       // 32位浮点数

// 布尔类型
let is_active: bool = true;
let is_greater: bool = 10 > 5;

// 字符类型
let c: char = 'z';
let emoji: char = '😻';

// 字符串类型
let s1: &str = "Hello";           // 字符串切片
let s2: String = String::from("World"); // 拥有所有权的字符串
```

### 复合类型

```rust
// 元组
let tup: (i32, f64, u8) = (500, 6.4, 1);
let (x, y, z) = tup; // 解构
let first = tup.0;   // 索引访问

// 数组
let arr: [i32; 5] = [1, 2, 3, 4, 5];
let first = arr[0];
let slice = &arr[1..3]; // 切片：[2, 3]
```

## 函数

### 函数定义

```rust
// 基本函数
fn greet() {
    println!("Hello, world!");
}

// 带参数的函数
fn add(x: i32, y: i32) -> i32 {
    x + y // 表达式，无分号
}

// 带返回值的函数
fn multiply(x: i32, y: i32) -> i32 {
    return x * y; // 显式返回
}

// 多个返回值（元组）
fn divide(x: f64, y: f64) -> (f64, f64) {
    (x / y, x % y)
}
```

### 闭包

```rust
// 闭包语法
let add = |x, y| x + y;
let result = add(5, 3);

// 捕获环境变量
let multiplier = 2;
let multiply_by_two = |x| x * multiplier;

// 闭包作为参数
fn apply_operation<F>(x: i32, f: F) -> i32
where
    F: Fn(i32) -> i32,
{
    f(x)
}
```

## 控制流

### 条件语句

```rust
// if 表达式
let number = 6;
if number % 4 == 0 {
    println!("number is divisible by 4");
} else if number % 3 == 0 {
    println!("number is divisible by 3");
} else {
    println!("number is not divisible by 4 or 3");
}

// if 作为表达式
let condition = true;
let number = if condition { 5 } else { 6 };
```

### 循环

```rust
// loop 无限循环
let mut counter = 0;
let result = loop {
    counter += 1;
    if counter == 10 {
        break counter * 2; // 返回值
    }
};

// while 循环
let mut number = 3;
while number != 0 {
    println!("{}!", number);
    number -= 1;
}

// for 循环
let arr = [10, 20, 30, 40, 50];
for element in arr {
    println!("the value is: {}", element);
}

// 范围循环
for number in 1..4 {
    println!("{}!", number);
}
```

### 模式匹配

```rust
// match 表达式
let number = 2;
match number {
    1 => println!("One"),
    2 | 3 => println!("Two or Three"),
    4..=9 => println!("Four to Nine"),
    _ => println!("Something else"),
}

// 匹配 Option
let some_number = Some(5);
match some_number {
    Some(x) => println!("Got a value: {}", x),
    None => println!("Got nothing"),
}

// if let 语法糖
if let Some(x) = some_number {
    println!("Got: {}", x);
}
```

## 所有权系统

### 所有权规则

1. Rust 中的每一个值都有一个被称为其所有者（owner）的变量
2. 值在任一时刻有且只有一个所有者
3. 当所有者（变量）离开作用域，这个值将被丢弃

```rust
// 移动（Move）
let s1 = String::from("hello");
let s2 = s1; // s1 的所有权移动到 s2，s1 不再有效
// println!("{}", s1); // 编译错误！

// 克隆（Clone）
let s1 = String::from("hello");
let s2 = s1.clone(); // 深拷贝
println!("{} {}", s1, s2); // 都有效

// Copy trait（栈上数据）
let x = 5;
let y = x; // 复制，x 仍然有效
println!("{} {}", x, y);
```

### 借用和引用

```rust
// 不可变引用
fn calculate_length(s: &String) -> usize {
    s.len()
} // s 离开作用域，但因为它不拥有所有权，所以什么也不会发生

let s1 = String::from("hello");
let len = calculate_length(&s1);
println!("The length of '{}' is {}.", s1, len);

// 可变引用
fn change(some_string: &mut String) {
    some_string.push_str(", world");
}

let mut s = String::from("hello");
change(&mut s);

// 引用规则
// 1. 在任意给定时间，要么只能有一个可变引用，要么只能有多个不可变引用
// 2. 引用必须总是有效的
```

### 生命周期

```rust
// 生命周期注解
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}

// 结构体中的生命周期
struct ImportantExcerpt<'a> {
    part: &'a str,
}

impl<'a> ImportantExcerpt<'a> {
    fn level(&self) -> i32 {
        3
    }
}
```

## 结构体和枚举

### 结构体

```rust
// 定义结构体
struct User {
    active: bool,
    username: String,
    email: String,
    sign_in_count: u64,
}

// 创建实例
let user1 = User {
    email: String::from("someone@example.com"),
    username: String::from("someusername123"),
    active: true,
    sign_in_count: 1,
};

// 结构体更新语法
let user2 = User {
    email: String::from("another@example.com"),
    ..user1 // 其余字段从 user1 获取
};

// 元组结构体
struct Color(i32, i32, i32);
struct Point(i32, i32, i32);

let black = Color(0, 0, 0);
let origin = Point(0, 0, 0);
```

### 方法和关联函数

```rust
#[derive(Debug)]
struct Rectangle {
    width: u32,
    height: u32,
}

impl Rectangle {
    // 方法
    fn area(&self) -> u32 {
        self.width * self.height
    }
    
    fn can_hold(&self, other: &Rectangle) -> bool {
        self.width > other.width && self.height > other.height
    }
    
    // 关联函数（类似静态方法）
    fn square(size: u32) -> Rectangle {
        Rectangle {
            width: size,
            height: size,
        }
    }
}

// 使用
let rect1 = Rectangle { width: 30, height: 50 };
let rect2 = Rectangle { width: 10, height: 40 };
let square = Rectangle::square(25);

println!("Area: {}", rect1.area());
println!("Can hold: {}", rect1.can_hold(&rect2));
```

### 枚举

```rust
// 基本枚举
enum IpAddrKind {
    V4,
    V6,
}

// 带数据的枚举
enum IpAddr {
    V4(u8, u8, u8, u8),
    V6(String),
}

let home = IpAddr::V4(127, 0, 0, 1);
let loopback = IpAddr::V6(String::from("::1"));

// 复杂枚举
enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
    ChangeColor(i32, i32, i32),
}

impl Message {
    fn call(&self) {
        match self {
            Message::Quit => println!("Quit"),
            Message::Move { x, y } => println!("Move to ({}, {})", x, y),
            Message::Write(text) => println!("Text: {}", text),
            Message::ChangeColor(r, g, b) => println!("Color: ({}, {}, {})", r, g, b),
        }
    }
}
```

## 错误处理

### Option 枚举

```rust
// Option 用于可能为空的值
let some_number = Some(5);
let some_string = Some("a string");
let absent_number: Option<i32> = None;

// 处理 Option
match some_number {
    Some(x) => println!("Got: {}", x),
    None => println!("Got nothing"),
}

// 使用方法
let x = some_number.unwrap_or(0); // 如果是 None，返回默认值
let y = some_number.map(|x| x * 2); // 如果是 Some，应用函数
```

### Result 枚举

```rust
// Result 用于可能失败的操作
use std::fs::File;
use std::io::ErrorKind;

// 处理 Result
let f = File::open("hello.txt");
let f = match f {
    Ok(file) => file,
    Err(error) => match error.kind() {
        ErrorKind::NotFound => match File::create("hello.txt") {
            Ok(fc) => fc,
            Err(e) => panic!("Problem creating the file: {:?}", e),
        },
        other_error => {
            panic!("Problem opening the file: {:?}", other_error)
        }
    },
};

// 使用 ? 操作符
fn read_username_from_file() -> Result<String, std::io::Error> {
    let mut f = File::open("hello.txt")?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(s)
}
```

## 模块系统

### 模块定义

```rust
// 在同一文件中定义模块
mod front_of_house {
    pub mod hosting {
        pub fn add_to_waitlist() {}
        
        fn seat_at_table() {}
    }
    
    mod serving {
        fn take_order() {}
        fn serve_order() {}
        fn take_payment() {}
    }
}

// 使用模块
pub fn eat_at_restaurant() {
    // 绝对路径
    crate::front_of_house::hosting::add_to_waitlist();
    
    // 相对路径
    front_of_house::hosting::add_to_waitlist();
}

// use 关键字
use crate::front_of_house::hosting;

pub fn eat_at_restaurant2() {
    hosting::add_to_waitlist();
}
```

### 外部包

```rust
// Cargo.toml 中添加依赖
// [dependencies]
// rand = "0.8"

// 使用外部包
use rand::Rng;

fn main() {
    let secret_number = rand::thread_rng().gen_range(1..101);
    println!("Secret number: {}", secret_number);
}
```

## 常用集合类型

### Vector

```rust
// 创建 vector
let mut v: Vec<i32> = Vec::new();
let v2 = vec![1, 2, 3]; // 宏创建

// 添加元素
v.push(5);
v.push(6);

// 访问元素
let third: &i32 = &v[2]; // 可能 panic
let third: Option<&i32> = v.get(2); // 安全访问

// 遍历
for i in &v {
    println!("{}", i);
}

// 可变遍历
for i in &mut v {
    *i += 50;
}
```

### HashMap

```rust
use std::collections::HashMap;

// 创建 HashMap
let mut scores = HashMap::new();
scores.insert(String::from("Blue"), 10);
scores.insert(String::from("Yellow"), 50);

// 访问值
let team_name = String::from("Blue");
let score = scores.get(&team_name); // 返回 Option<&V>

// 遍历
for (key, value) in &scores {
    println!("{}: {}", key, value);
}

// 更新值
scores.entry(String::from("Yellow")).or_insert(50); // 只在键不存在时插入
```

### String

```rust
// 创建字符串
let mut s = String::new();
let s2 = String::from("initial contents");
let s3 = "initial contents".to_string();

// 更新字符串
s.push_str("bar"); // 添加字符串切片
s.push('l'); // 添加单个字符

// 拼接字符串
let s1 = String::from("Hello, ");
let s2 = String::from("world!");
let s3 = s1 + &s2; // s1 被移动，不能再使用

// 使用 format! 宏
let s1 = String::from("tic");
let s2 = String::from("tac");
let s3 = String::from("toe");
let s = format!("{}-{}-{}", s1, s2, s3); // 不获取所有权
```

## 🎯 实践建议

1. **从所有权开始**：理解所有权是掌握 Rust 的关键
2. **多使用编译器**：Rust 编译器的错误信息非常有帮助
3. **先写能编译的代码**：不要过早优化，先让代码能跑起来
4. **善用 `match`**：模式匹配是 Rust 的强大特性
5. **拥抱 `Result` 和 `Option`**：它们让错误处理更安全
6. **阅读标准库文档**：Rust 标准库文档质量很高

## 📚 进阶学习

- **Trait 和泛型**：代码复用和抽象
- **智能指针**：`Box<T>`、`Rc<T>`、`RefCell<T>`
- **并发编程**：线程、消息传递、共享状态
- **异步编程**：`async`/`await`、`Future`
- **宏系统**：声明式宏和过程宏
- **不安全 Rust**：原始指针、不安全函数

---

这份指南涵盖了 Rust 的核心概念。建议结合实际代码练习，逐步掌握这些概念。记住，Rust 的学习曲线可能比较陡峭，但一旦掌握，你会发现它的强大和优雅！