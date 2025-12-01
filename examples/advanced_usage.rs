//! 高级使用示例
//! 
//! 展示如何使用自定义格式化器和输出目标

use nanolog_rs::{AsyncLogger, Level, Record, Formatter, Sink};
use std::sync::Arc;
use std::time::Duration;
use std::io;
use std::io::Write;

/// 自定义JSON格式化器
struct JsonFormatter;

impl Formatter for JsonFormatter {
    fn format(&self, record: &Record) -> Result<Vec<u8>, std::fmt::Error> {
        let json_string = format!(
            r#"{{"level":"{:?}","timestamp":{},"target":"{}","file":"{}","line":{},"message":"{}"}}"#,
            record.level(),
            record.timestamp(),
            record.target(),
            record.file(),
            record.line(),
            record.message().replace('"', "\\\"")
        );
        Ok(json_string.into_bytes())
    }
}

/// 自定义彩色控制台输出
struct ColoredConsoleSink;

#[async_trait::async_trait]
impl Sink for ColoredConsoleSink {
    async fn write(&self, data: &[u8]) -> io::Result<()> {
        // 直接输出字节数据到控制台
        io::stdout().write_all(data)?;
        Ok(())
    }
    
    async fn write_batch(&self, data: &[Vec<u8>]) -> io::Result<()> {
        for item in data {
            io::stdout().write_all(item)?;
        }
        Ok(())
    }
    
    async fn flush(&self) -> io::Result<()> {
        io::stdout().flush()?;
        Ok(())
    }
    
    async fn shutdown(&self) -> io::Result<()> {
        Ok(())
    }
}

/// 复合输出目标 - 同时输出到多个目标
struct CompositeSink {
    sinks: Vec<Arc<dyn Sink + Send + Sync>>,
}

impl CompositeSink {
    fn new(sinks: Vec<Arc<dyn Sink + Send + Sync>>) -> Self {
        Self { sinks }
    }
}

#[async_trait::async_trait]
impl Sink for CompositeSink {
    async fn write(&self, data: &[u8]) -> io::Result<()> {
        for sink in &self.sinks {
            sink.write(data).await?;
        }
        Ok(())
    }
    
    async fn write_batch(&self, data: &[Vec<u8>]) -> io::Result<()> {
        for sink in &self.sinks {
            sink.write_batch(data).await?;
        }
        Ok(())
    }
    
    async fn flush(&self) -> io::Result<()> {
        for sink in &self.sinks {
            sink.flush().await?;
        }
        Ok(())
    }
    
    async fn shutdown(&self) -> io::Result<()> {
        for sink in &self.sinks {
            sink.shutdown().await?;
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== nanolog-rs 高级使用示例 ===\n");
    
    // 1. 使用自定义JSON格式化器
    println!("1. 使用自定义JSON格式化器...");
    let json_logger = AsyncLogger::new(
        Level::Info,
        Arc::new(JsonFormatter),
        Arc::new(ColoredConsoleSink),
        1000,
        10,
        Duration::from_millis(100),
    );
    
    json_logger.log(Record::new(
        Level::Info,
        "examples::advanced",
        file!(),
        line!(),
        "使用JSON格式化的日志消息".to_string()
    ))?;
    
    // 2. 使用复合输出目标
    println!("\n2. 使用复合输出目标...");
    
    use nanolog_rs::{ConsoleSink, FileSink};
    
    let composite_sink = Arc::new(CompositeSink::new(vec![
        Arc::new(ConsoleSink::new()),  // 输出到控制台
        Arc::new(FileSink::new("logs/composite.log")?),  // 输出到文件
        Arc::new(ColoredConsoleSink),  // 输出到彩色控制台
    ]));
    
    let composite_logger = AsyncLogger::new(
        Level::Debug,
        Arc::new(JsonFormatter),
        composite_sink,
        2000,
        20,
        Duration::from_millis(200),
    );
    
    // 记录不同级别的日志到多个目标
    for level in [Level::Debug, Level::Info, Level::Warn, Level::Error] {
        composite_logger.log(Record::new(
            level,
            "examples::composite",
            file!(),
            line!(),
            format!("复合输出测试 - {}级别日志", format!("{:?}", level))
        ))?;
    }
    
    // 3. 演示批量处理性能
    println!("\n3. 演示批量处理性能...");
    
    let performance_logger = AsyncLogger::new(
        Level::Info,
        Arc::new(JsonFormatter),
        Arc::new(FileSink::new("logs/performance.log")?),
        10000,
        100,
        Duration::from_millis(10),
    );
    
    // 批量记录大量日志
    let start_time = std::time::Instant::now();
    
    for i in 0..1000 {
        performance_logger.log(Record::new(
            Level::Info,
            "examples::performance",
            file!(),
            line!(),
            format!("性能测试日志 #{} - 这是一个用于测试批量处理性能的日志消息", i + 1)
        ))?;
    }
    
    let elapsed = start_time.elapsed();
    println!("记录1000条日志耗时: {:?}", elapsed);
    println!("平均每条日志耗时: {:?}", elapsed / 1000);
    
    // 4. 演示错误处理
    println!("\n4. 演示错误处理...");
    
    // 测试队列满的情况
    let small_queue_logger = AsyncLogger::new(
        Level::Info,
        Arc::new(JsonFormatter),
        Arc::new(ConsoleSink::new()),
        5,  // 很小的队列容量
        1,
        Duration::from_millis(100),
    );
    
    // 尝试记录超过队列容量的日志
    for i in 0..10 {
        match small_queue_logger.log(Record::new(
            Level::Info,
            "examples::error_handling",
            file!(),
            line!(),
            format!("错误处理测试 #{}", i + 1)
        )) {
            Ok(()) => println!("成功记录日志 #{}", i + 1),
            Err(e) => println!("记录日志 #{} 失败: {}", i + 1, e),
        }
    }
    
    // 5. 优雅关闭
    println!("\n5. 优雅关闭所有日志器...");
    
    json_logger.shutdown().await?;
    composite_logger.shutdown().await?;
    performance_logger.shutdown().await?;
    small_queue_logger.shutdown().await?;
    
    println!("\n=== 高级示例执行完成 ===");
    
    Ok(())
}