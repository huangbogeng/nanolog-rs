//! 现代化Builder模式使用示例
//!
//! 展示如何使用改进后的AsyncLogger Builder模式进行配置

use nanolog_rs::{AsyncLogger, Level, Record, AsyncLoggerBuilder};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== nanolog-rs 现代化Builder模式使用示例 ===\n");
    
    // 1. 使用默认配置创建日志器
    println!("1. 使用默认配置创建日志器...");
    let logger = AsyncLogger::builder()
        .build()?;
    
    logger.log(Record::new(
        Level::Info,
        "examples::modern_builder",
        file!(),
        line!(),
        "这是使用默认配置创建的日志器".to_string()
    ))?;
    
    // 2. 使用便捷方法创建JSON格式的日志器
    println!("\n2. 使用便捷方法创建JSON格式的日志器...");
    let json_logger = AsyncLogger::builder()
        .with_debug_level()
        .with_json_formatting()
        .with_console_output()
        .build()?;
    
    json_logger.log(Record::new(
        Level::Debug,
        "examples::modern_builder",
        file!(),
        line!(),
        "这是一个JSON格式的调试日志".to_string()
    ))?;
    
    // 3. 使用文件输出的日志器
    println!("\n3. 使用文件输出的日志器...");
    let file_logger = AsyncLogger::builder()
        .level(Level::Info)
        .with_file_output("logs/example_modern.log")
        .batch_size(10)
        .flush_interval(Duration::from_millis(50))
        .build()?;
    
    for i in 1..=5 {
        file_logger.log(Record::new(
            Level::Info,
            "examples::modern_builder",
            file!(),
            line!(),
            format!("文件日志记录 #{}", i)
        ))?;
    }
    
    // 4. 高级配置示例
    println!("\n4. 高级配置示例...");
    let advanced_logger = AsyncLogger::builder()
        .with_trace_level()
        .with_simple_formatting()
        .queue_capacity(2000)
        .batch_size(25)
        .flush_interval(Duration::from_millis(25))
        .build()?;
    
    advanced_logger.log(Record::new(
        Level::Trace,
        "examples::modern_builder",
        file!(),
        line!(),
        "高级配置示例 - Trace级别".to_string()
    ))?;
    
    advanced_logger.log(Record::new(
        Level::Info,
        "examples::modern_builder",
        file!(),
        line!(),
        "高级配置示例 - Info级别".to_string()
    ))?;
    
    // 5. 链式调用组合示例
    println!("\n5. 链式调用组合示例...");
    let chained_logger = AsyncLogger::builder()
        .level(Level::Warn)
        .with_json_formatting()
        .with_console_output()
        .queue_capacity(500)
        .batch_size(5)
        .flush_interval(Duration::from_millis(10))
        .build()?;
    
    for i in 1..=3 {
        chained_logger.log(Record::new(
            Level::Warn,
            "examples::modern_builder",
            file!(),
            line!(),
            format!("链式调用警告 #{}", i)
        ))?;
    }
    
    // 等待日志处理完成
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // 6. 优雅关闭所有日志器
    println!("\n6. 优雅关闭所有日志器...");
    logger.shutdown().await?;
    json_logger.shutdown().await?;
    file_logger.shutdown().await?;
    advanced_logger.shutdown().await?;
    chained_logger.shutdown().await?;
    
    println!("\n=== 现代化Builder模式示例执行完成 ===");
    
    Ok(())
}