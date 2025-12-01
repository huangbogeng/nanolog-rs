//! 基本使用示例
//! 
//! 展示如何使用nanolog-rs进行基本的日志记录

use nanolog_rs::{AsyncLogger, Level, Record, DefaultFormatter, ConsoleSink, FileSink, MemorySink};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== nanolog-rs 基本使用示例 ===\n");
    
    // 1. 创建控制台日志器
    println!("1. 创建控制台日志器...");
    let console_logger = AsyncLogger::new(
        Level::Debug,
        Arc::new(DefaultFormatter::new()),
        Arc::new(ConsoleSink::new()),
        1000,
        10,
        Duration::from_millis(100),
    );
    
    // 记录不同级别的日志
    console_logger.log(Record::new(
        Level::Trace,
        "examples::basic_usage",
        file!(),
        line!(),
        "这是一个跟踪级别的日志".to_string()
    ))?;
    
    console_logger.log(Record::new(
        Level::Debug,
        "examples::basic_usage",
        file!(),
        line!(),
        "这是一个调试级别的日志".to_string()
    ))?;
    
    console_logger.log(Record::new(
        Level::Info,
        "examples::basic_usage",
        file!(),
        line!(),
        "这是一个信息级别的日志".to_string()
    ))?;
    
    console_logger.log(Record::new(
        Level::Warn,
        "examples::basic_usage",
        file!(),
        line!(),
        "这是一个警告级别的日志".to_string()
    ))?;
    
    console_logger.log(Record::new(
        Level::Error,
        "examples::basic_usage",
        file!(),
        line!(),
        "这是一个错误级别的日志".to_string()
    ))?;
    
    // 2. 创建文件日志器
    println!("\n2. 创建文件日志器...");
    let file_logger = AsyncLogger::new(
        Level::Info,
        Arc::new(DefaultFormatter::new()),
        Arc::new(FileSink::new("logs/example.log")?),
        1000,
        50,
        Duration::from_millis(500),
    );
    
    // 批量记录日志到文件
    for i in 0..5 {
        file_logger.log(Record::new(
            Level::Info,
            "examples::basic_usage",
            file!(),
            line!(),
            format!("文件日志记录 #{} - 这是写入文件的日志消息", i + 1)
        ))?;
    }
    
    // 3. 创建内存日志器（用于测试）
    println!("\n3. 创建内存日志器...");
    let memory_sink = Arc::new(MemorySink::new());
    let memory_logger = AsyncLogger::new(
        Level::Debug,
        Arc::new(DefaultFormatter::new()),
        memory_sink.clone(),
        100,
        5,
        Duration::from_millis(50),
    );
    
    // 记录一些测试日志
    for i in 0..3 {
        memory_logger.log(Record::new(
            Level::Info,
            "examples::basic_usage",
            file!(),
            line!(),
            format!("内存日志记录 #{} - 测试消息", i + 1)
        ))?;
    }
    
    // 等待日志处理完成
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // 获取内存中的日志内容
    let memory_content = memory_sink.get_content();
    println!("内存日志器中的内容:");
    println!("{}", String::from_utf8_lossy(&memory_content));
    
    // 4. 演示并发日志记录
    println!("\n4. 演示并发日志记录...");
    let concurrent_logger = Arc::new(AsyncLogger::new(
        Level::Info,
        Arc::new(DefaultFormatter::new()),
        Arc::new(ConsoleSink::new()),
        5000,
        100,
        Duration::from_millis(10),
    ));
    
    let mut handles = vec![];
    
    // 创建多个线程并发记录日志
    for thread_id in 0..3 {
        let logger = concurrent_logger.clone();
        let handle = std::thread::spawn(move || {
            for i in 0..10 {
                let record = Record::new(
                    Level::Info,
                    "examples::concurrent",
                    file!(),
                    line!(),
                    format!("线程 {} - 日志记录 #{}", thread_id, i + 1)
                );
                
                if let Err(e) = logger.log(record) {
                    eprintln!("线程 {} 记录日志失败: {}", thread_id, e);
                }
            }
        });
        handles.push(handle);
    }
    
    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }
    
    // 5. 优雅关闭所有日志器
    println!("\n5. 优雅关闭所有日志器...");
    
    console_logger.shutdown().await?;
    file_logger.shutdown().await?;
    memory_logger.shutdown().await?;
    concurrent_logger.shutdown().await?;
    
    println!("\n=== 示例执行完成 ===");
    
    Ok(())
}