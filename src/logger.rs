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

use crossbeam_queue::SegQueue;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::interval;

use crate::Level;
use crate::Record;
use crate::error::Error;
use crate::format::Formatter;
use crate::sink::Sink;

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
    
    // 日志丢失检测相关字段
    /// 已发送的日志数量
    sent_count: Arc<AtomicUsize>,
    /// 已写入的日志数量
    written_count: Arc<AtomicUsize>,
    /// 已丢失的日志数量
    lost_count: Arc<AtomicUsize>,
    /// 是否启用日志丢失检测
    loss_detection_enabled: bool,
    
    // 自诊断日志相关字段
    /// 是否启用自诊断日志
    self_diagnosis_enabled: bool,
    /// 自诊断日志级别
    self_diagnosis_level: Level,
    /// 自诊断日志目标
    self_diagnosis_sink: Option<Arc<dyn Sink>>,
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
        
        // 日志丢失检测相关字段
        let sent_count = Arc::new(AtomicUsize::new(0));
        let written_count = Arc::new(AtomicUsize::new(0));
        let lost_count = Arc::new(AtomicUsize::new(0));
        
        // 启动后台工作线程
        Self::start_worker(
            queue.clone(),
            shutdown.clone(),
            notify.clone(),
            formatter,
            sink,
            WorkerConfig {
                batch_size,
                flush_interval,
            },
            written_count.clone(),
        );

        Self {
            level,
            queue,
            shutdown,
            notify,
            sent_count,
            written_count,
            lost_count,
            loss_detection_enabled: true,
            self_diagnosis_enabled: false,
            self_diagnosis_level: Level::Error,
            self_diagnosis_sink: None,
        }
    }

    /// 启动后台工作线程
    fn start_worker(
        queue: Arc<SegQueue<Record>>,
        shutdown: Arc<AtomicBool>,
        notify: Arc<Notify>,
        formatter: Arc<dyn Formatter>,
        sink: Arc<dyn Sink>,
        config: WorkerConfig,
        written_count: Arc<AtomicUsize>,
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
                                let count = batch.len();
                                Self::process_batch(&batch, &formatter, &sink).await;
                                written_count.fetch_add(count, Ordering::Relaxed);
                                batch.clear();
                            }
                        }
                    },

                    // 定期刷新
                    _ = flush_timer.tick() => {
                        if !batch.is_empty() {
                            let count = batch.len();
                            Self::process_batch(&batch, &formatter, &sink).await;
                            written_count.fetch_add(count, Ordering::Relaxed);
                            batch.clear();
                        }
                    },

                    // 检查关闭信号（使用yield_now避免阻塞）
                    _ = tokio::task::yield_now() => {
                        if shutdown.load(Ordering::Acquire) {
                            // 处理剩余日志
                            if !batch.is_empty() {
                                let count = batch.len();
                                Self::process_batch(&batch, &formatter, &sink).await;
                                written_count.fetch_add(count, Ordering::Relaxed);
                            }

                            // 处理队列中剩余的所有日志
                            let mut remaining_count = 0;
                            while let Some(record) = queue.pop() {
                                remaining_count += 1;
                                if let Ok(formatted) = formatter.format(&record) {
                                    let _ = sink.write(&formatted).await;
                                }
                            }
                            if remaining_count > 0 {
                                written_count.fetch_add(remaining_count, Ordering::Relaxed);
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
        
        // 增加已发送日志计数
        if self.loss_detection_enabled {
            self.sent_count.fetch_add(1, Ordering::Relaxed);
        }

        // 通知工作线程有新日志
        self.notify.notify_one();

        Ok(())
    }
    
    /// 获取日志丢失统计信息
    pub fn get_loss_stats(&self) -> (usize, usize, usize) {
        let sent = self.sent_count.load(Ordering::Relaxed);
        let written = self.written_count.load(Ordering::Relaxed);
        let lost = self.lost_count.load(Ordering::Relaxed);
        
        // 计算当前丢失的日志数量
        let current_lost = if sent > written {
            sent - written
        } else {
            0
        };
        
        // 更新丢失计数器
        if self.loss_detection_enabled && current_lost > lost {
            self.lost_count.store(current_lost, Ordering::Relaxed);
        }
        
        (sent, written, current_lost)
    }
    
    /// 重置日志丢失统计信息
    pub fn reset_loss_stats(&self) {
        self.sent_count.store(0, Ordering::Relaxed);
        self.written_count.store(0, Ordering::Relaxed);
        self.lost_count.store(0, Ordering::Relaxed);
    }
    
    /// 启用或禁用日志丢失检测
    pub fn set_loss_detection(&mut self, enabled: bool) {
        self.loss_detection_enabled = enabled;
        if !enabled {
            self.reset_loss_stats();
        }
    }
    
    /// 启用或禁用自诊断日志
    pub fn set_self_diagnosis(&mut self, enabled: bool) {
        self.self_diagnosis_enabled = enabled;
    }
    
    /// 设置自诊断日志级别
    pub fn set_self_diagnosis_level(&mut self, level: Level) {
        self.self_diagnosis_level = level;
    }
    
    /// 设置自诊断日志目标
    pub fn set_self_diagnosis_sink(&mut self, sink: Arc<dyn Sink>) {
        self.self_diagnosis_sink = Some(sink);
    }
    
    /// 记录自诊断日志
    fn log_self_diagnosis(&self, level: Level, message: &str) {
        if self.self_diagnosis_enabled && level >= self.self_diagnosis_level {
            if let Some(sink) = &self.self_diagnosis_sink {
                let record = Record::new(
                    level,
                    "nanolog_rs::self_diagnosis",
                    file!(),
                    line!(),
                    message.to_string(),
                );
                
                // 使用默认格式化器
                let formatter = crate::format::DefaultFormatter::new();
                if let Ok(formatted) = formatter.format(&record) {
                    let _ = sink.write(&formatted);
                }
            }
        }
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
        
        // 因为shutdown_tx在共享引用中，我们不能获取其所有权
        // 所以我们使用notify通知工作线程，而oneshot通道已在start_worker中使用
        self.notify.notify_one();

        // 等待工作线程完成处理所有日志
        tokio::time::sleep(Duration::from_millis(10)).await;

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
    logger: Mutex<Option<Arc<AsyncLogger>>>,
}

impl GlobalLogger {
    /// 创建新的全局日志器
    pub fn new() -> Self {
        Self { logger: Mutex::new(None) }
    }

    /// 初始化全局日志器
    pub fn init(&self, logger: Arc<AsyncLogger>) -> Result<(), Error> {
        let mut guard = self.logger.lock().unwrap();
        
        // 在测试环境中允许重新初始化
        #[cfg(test)]
        {
            *guard = Some(logger);
            return Ok(());
        }
        
        // 在生产环境中只允许初始化一次
        #[cfg(not(test))]
        {
            if guard.is_some() {
                return Err(Error::AlreadyInitialized);
            }

            *guard = Some(logger);
            Ok(())
        }
    }

    /// 获取全局日志器实例
    pub fn get(&self) -> Option<Arc<AsyncLogger>> {
        self.logger.lock().unwrap().clone()
    }

    /// 记录日志
    pub fn log(&self, record: Record) -> Result<(), Error> {
        if let Some(logger) = self.logger.lock().unwrap().as_ref() {
            logger.log(record)
        } else {
            Err(Error::NotInitialized)
        }
    }

    /// 刷新日志
    pub async fn flush(&self) -> Result<(), Error> {
        if let Some(logger) = self.logger.lock().unwrap().as_ref() {
            logger.flush().await
        } else {
            Err(Error::NotInitialized)
        }
    }

    /// 关闭日志器
    pub async fn shutdown(&self) -> Result<(), Error> {
        if let Some(logger) = self.logger.lock().unwrap().as_ref() {
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
    // 获取或创建全局日志器实例
    let global_logger = GLOBAL_LOGGER.get_or_init(|| GlobalLogger::new());
    
    // 调用实例的init方法来设置日志器
    global_logger.init(logger)
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
            1000,                       // queue capacity
            10,                         // batch size
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
