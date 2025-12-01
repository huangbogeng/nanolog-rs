//! 日志宏性能示例
//!
//! 展示nanolog-rs日志宏的惰性求值特性如何避免不必要的性能开销

use nanolog_rs::{AsyncLoggerBuilder, Level, init_global_logger};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 使用Builder模式创建日志器，只记录ERROR级别以上的日志
    let logger = AsyncLoggerBuilder::new()
        .level(Level::Error) // 只记录ERROR及以上级别的日志
        .build()?;

    // 初始化全局日志器
    init_global_logger(Arc::new(logger))?;

    println!("开始性能测试...");

    // 测试INFO级别日志（会被过滤掉）
    let start = Instant::now();
    for i in 0..100000 {
        // 这些日志不会被记录，因为当前级别是ERROR
        // 但由于惰性求值，format!宏不会被执行
        nanolog_rs::info!(
            "这是一条不会被记录的信息日志，带有复杂格式化: {}, {}, {}",
            i,
            expensive_function(),
            another_expensive_function()
        );
    }
    let elapsed = start.elapsed();
    println!("执行100,000次被过滤的info!宏调用耗时: {:?}", elapsed);

    // 测试ERROR级别日志（会被记录）
    let start2 = Instant::now();
    for i in 0..1000 {
        // 这些日志会被记录，format!宏会被执行
        nanolog_rs::error!(
            "这是一条会被记录的错误日志，带有复杂格式化: {}, {}, {}",
            i,
            expensive_function(),
            another_expensive_function()
        );
    }
    let elapsed2 = start2.elapsed();
    println!("执行1,000次实际记录的error!宏调用耗时: {:?}", elapsed2);

    println!("性能测试完成！");

    Ok(())
}

/// 模拟一个耗时的函数
fn expensive_function() -> String {
    // 模拟一些耗时操作
    "expensive_result".to_string()
}

/// 模拟另一个耗时的函数
fn another_expensive_function() -> i32 {
    // 模拟一些耗时操作
    42
}
