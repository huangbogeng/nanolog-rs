//! Builder模式使用示例
//!
//! 展示如何使用AsyncLogger的Builder模式进行配置

use nanolog_rs::{AsyncLogger, Level, Record, JsonFormatter, ConsoleSink, AsyncLoggerBuilder};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== nanolog-rs Builder模式使用示例 ===\n");
    
    // 1. 使用Builder模式创建日志器（默认配置）
    println!("1. 使用Builder模式创建日志器（默认配置）...");
    let logger = AsyncLogger::builder()
        .build()?;
    
    logger.log(Record::new(
        Level::Info,
        "examples::builder",
        file!(),
        line!(),
        "这是使用默认配置创建的日志器".to_string()
    ))?;
    
    // 2. 使用Builder模式创建自定义配置的日志器
    println!("\n2. 使用Builder模式创建自定义配置的日志器...");
    let custom_logger = AsyncLogger::builder()
        .level(Level::Debug)
        .formatter(Arc::new(JsonFormatter::new()))
        .sink(Arc::new(ConsoleSink::new()))
        .queue_capacity(2000)
        .batch_size(50)
        .flush_interval(Duration::from_millis(50))
        .build()?;
    
    // 记录不同级别的日志
    custom_logger.log(Record::new(
        Level::Debug,
        "examples::builder",
        file!(),
        line!(),
        "这是一个调试级别的日志".to_string()
    ))?;
    
    custom_logger.log(Record::new(
        Level::Info,
        "examples::builder",
        file!(),
        line!(),
        "这是一个信息级别的日志".to_string()
    ))?;
    
    custom_logger.log(Record::new(
        Level::Warn,
        "examples::builder",
        file!(),
        line!(),
        "这是一个警告级别的日志".to_string()
    ))?;
    
    // 3. 演示链式调用
    println!("\n3. 演示链式调用...");
    let chained_logger = AsyncLogger::builder()
        .level(Level::Trace)
        .batch_size(5)
        .flush_interval(Duration::from_millis(10))
        .build()?;
    
    for i in 0..3 {
        chained_logger.log(Record::new(
            Level::Trace,
            "examples::builder",
            file!(),
            line!(),
            format!("链式调用示例日志 #{}", i + 1)
        ))?;
    }
    
    // 等待日志处理完成
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // 4. 优雅关闭所有日志器
    println!("\n4. 优雅关闭所有日志器...");
    logger.shutdown().await?;
    custom_logger.shutdown().await?;
    chained_logger.shutdown().await?;
    
    println!("\n=== Builder模式示例执行完成 ===");
    
    Ok(())
}