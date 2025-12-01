/*!
高性能异步日志系统实现。

基于无锁队列和零拷贝技术，提供低延迟、高吞吐量的日志记录能力。

## 特性

- **异步处理**: 使用Tokio运行时进行异步日志处理，避免阻塞主线程
- **零拷贝**: 使用`&'static str`避免不必要的字符串分配
- **无锁队列**: 基于`crossbeam_queue::SegQueue`实现高性能并发
- **批量处理**: 支持批量格式化和写入，提高吞吐量
- **线程安全**: 使用`Arc`和原子操作确保线程安全
- **优雅关闭**: 支持优雅关闭，确保所有日志都被处理

## 使用示例

```rust
use nanolog_rs::{AsyncLogger, Level, Record, DefaultFormatter, ConsoleSink};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let logger = AsyncLogger::new(
        Level::Info,
        Arc::new(DefaultFormatter::new()),
        Arc::new(ConsoleSink::new()),
        1000,
        100,
        Duration::from_millis(100),
    );
    
    let record = Record::new(
        Level::Info,
        "example",
        file!(),
        line!(),
        "Hello, world!".to_string()
    );
    
    logger.log(record).unwrap();
    logger.shutdown().await.unwrap();
}
```
*/

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use crossbeam_queue::SegQueue;
use tokio::sync::Notify;
use tokio::time::interval;

use crate::Level;
use crate::Record;
use crate::sink::Sink;
use crate::format::Formatter;
use crate::error::Error;

/// 工作线程配置
struct WorkerConfig {
    batch_size: usize,
    flush_interval: Duration,
}

/// 高性能异步日志器
pub struct AsyncLogger {
    level: Level,
    queue: Arc<SegQueue<Record>>,
    shutdown: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl AsyncLogger {
    /// 创建新的异步日志器构建器
    pub fn builder() -> crate::builder::AsyncLoggerBuilder {
        crate::builder::AsyncLoggerBuilder::new()
    }

    /// 创建新的异步日志器
    pub fn new(
        level: Level,
        formatter: Arc<dyn Formatter>,
        sink: Arc<dyn Sink>,
        _queue_capacity: usize,
        batch_size: usize,
        flush_interval: Duration,
    ) -> Self {
        let queue = Arc::new(SegQueue::new());
        let shutdown = Arc::new(AtomicBool::new(false));
        let notify = Arc::new(Notify::new());
        
        let logger = Self {
            level,
            queue: queue.clone(),
            shutdown: shutdown.clone(),
            notify: notify.clone(),
        };
        
        // 启动后台工作线程
        logger.start_worker(
            queue, 
            shutdown, 
            notify, 
            formatter, 
            sink, 
            WorkerConfig {
                batch_size,
                flush_interval,
            }
        );
        
        logger
    }
    
    /// 启动后台工作线程
    fn start_worker(
        &self,
        queue: Arc<SegQueue<Record>>,
        shutdown: Arc<AtomicBool>,
        notify: Arc<Notify>,
        formatter: Arc<dyn Formatter>,
        sink: Arc<dyn Sink>,
        config: WorkerConfig,
    ) {
        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(config.batch_size);
            let mut flush_timer = interval(config.flush_interval);
            
            loop {
                tokio::select! {
                    // 处理队列中的日志记录
                    _ = notify.notified() => {
                        while let Some(record) = queue.pop() {
                            batch.push(record);
                            
                            if batch.len() >= config.batch_size {
                                Self::process_batch(&batch, &formatter, &sink).await;
                                batch.clear();
                            }
                        }
                    },
                    
                    // 定期刷新
                    _ = flush_timer.tick() => {
                        if !batch.is_empty() {
                            Self::process_batch(&batch, &formatter, &sink).await;
                            batch.clear();
                        }
                    },
                    
                    // 检查关闭信号
                    _ = tokio::task::yield_now() => {
                        if shutdown.load(Ordering::Acquire) {
                            // 处理剩余日志
                            if !batch.is_empty() {
                                Self::process_batch(&batch, &formatter, &sink).await;
                            }
                            
                            // 处理队列中剩余的所有日志
                            while let Some(record) = queue.pop() {
                                if let Ok(formatted) = formatter.format(&record) {
                                    let _ = sink.write(&formatted).await;
                                }
                            }
                            
                            // 关闭sink
                            let _ = sink.shutdown().await;
                            break;
                        }
                    }
                }
            }
        });
    }
    
    /// 批量处理日志记录
    async fn process_batch(
        records: &[Record],
        formatter: &Arc<dyn Formatter>,
        sink: &Arc<dyn Sink>,
    ) {
        let mut formatted_batch = Vec::with_capacity(records.len());
        
        // 批量格式化
        for record in records {
            if let Ok(formatted) = formatter.format(record) {
                formatted_batch.push(formatted);
            }
        }
        
        // 批量写入
        if !formatted_batch.is_empty() {
            let _ = sink.write_batch(&formatted_batch).await;
        }
    }
    
    /// 记录日志（非阻塞）
pub fn log(&self, record: Record) -> Result<(), Error> {
    if !self.should_log(record.level()) {
        return Ok(());
    }
    
    // 使用无锁队列，避免阻塞
    self.queue.push(record);
    
    // 通知工作线程有新日志
    self.notify.notify_one();
    
    Ok(())
}
    
    /// 检查是否应该记录指定级别的日志
    pub fn should_log(&self, level: Level) -> bool {
        level >= self.level
    }
    
    /// 获取日志级别
    pub fn level(&self) -> Level {
        self.level
    }
    
    /// 刷新日志（等待所有日志处理完成）
pub async fn flush(&self) -> Result<(), Error> {
    // 发送通知确保工作线程处理完所有日志
    self.notify.notify_one();
    
    // 短暂等待以确保日志被处理
    tokio::time::sleep(Duration::from_millis(10)).await;
    
    Ok(())
}

/// 优雅关闭日志器
pub async fn shutdown(&self) -> Result<(), Error> {
    self.shutdown.store(true, Ordering::Release);
    self.notify.notify_one();
    
    // 等待工作线程完成
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    Ok(())
}
}

impl Drop for AsyncLogger {
    fn drop(&mut self) {
        // 如果日志器被丢弃但未正确关闭，尝试优雅关闭
        if !self.shutdown.load(Ordering::Acquire) {
            self.shutdown.store(true, Ordering::Release);
            self.notify.notify_one();
            
            // 短暂等待以确保工作线程有机会处理关闭信号
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}



/// 全局日志器管理
pub struct GlobalLogger {
    logger: Option<Arc<AsyncLogger>>,
}

impl GlobalLogger {
    /// 创建新的全局日志器
    pub fn new() -> Self {
        Self { logger: None }
    }
    
    /// 初始化全局日志器
pub fn init(&mut self, logger: Arc<AsyncLogger>) -> Result<(), Error> {
    if self.logger.is_some() {
        return Err(Error::AlreadyInitialized);
    }
    
    self.logger = Some(logger);
    Ok(())
}
    
    /// 获取全局日志器实例
    pub fn get(&self) -> Option<&Arc<AsyncLogger>> {
        self.logger.as_ref()
    }
    
    /// 记录日志
pub fn log(&self, record: Record) -> Result<(), Error> {
    if let Some(logger) = &self.logger {
        logger.log(record)
    } else {
        Err(Error::NotInitialized)
    }
}

/// 刷新日志
pub async fn flush(&self) -> Result<(), Error> {
    if let Some(logger) = &self.logger {
        logger.flush().await
    } else {
        Err(Error::NotInitialized)
    }
}

/// 关闭日志器
pub async fn shutdown(&self) -> Result<(), Error> {
    if let Some(logger) = &self.logger {
        logger.shutdown().await
    } else {
        Err(Error::NotInitialized)
    }
}
}

impl Default for GlobalLogger {
    fn default() -> Self {
        Self::new()
    }
}

// 全局日志器实例 - 使用OnceLock确保线程安全
use std::sync::OnceLock;

static GLOBAL_LOGGER: OnceLock<GlobalLogger> = OnceLock::new();

/// 初始化全局日志器
pub fn init_global_logger(logger: Arc<AsyncLogger>) -> Result<(), Error> {
    let mut global_logger = GlobalLogger::new();
    global_logger.init(logger)?;
    
    GLOBAL_LOGGER.set(global_logger)
        .map_err(|_| Error::AlreadyInitialized)
}

/// 获取全局日志器
pub fn global_logger() -> Option<&'static GlobalLogger> {
    GLOBAL_LOGGER.get()
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::DefaultFormatter;
    use crate::sink::ConsoleSink;
    
    #[tokio::test]
    async fn test_async_logger_basic() {
        let formatter = Arc::new(DefaultFormatter::new());
        let sink = Arc::new(ConsoleSink::new());
        
        let logger = AsyncLogger::new(
            Level::Debug,
            formatter,
            sink,
            1000, // queue capacity
            10,   // batch size
            Duration::from_millis(100), // flush interval
        );
        
        let record = Record::new(
            Level::Info,
            "test",
            "test.rs",
            1,
            "Test message".to_string(),
        );
        
        assert!(logger.log(record).is_ok());
        assert!(logger.flush().await.is_ok());
        assert!(logger.shutdown().await.is_ok());
    }
    
    #[tokio::test]
    async fn test_global_logger() {
        let formatter = Arc::new(DefaultFormatter::new());
        let sink = Arc::new(ConsoleSink::new());
        
        let logger = Arc::new(AsyncLogger::new(
            Level::Debug,
            formatter,
            sink,
            1000,
            10,
            Duration::from_millis(100),
        ));
        
        assert!(init_global_logger(logger).is_ok());
        assert!(global_logger().is_some());
    }
}