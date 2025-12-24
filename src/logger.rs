/*!
高性能非阻塞日志系统实现。

基于 Disruptor 环形缓冲与零拷贝技术，提供低延迟、高吞吐量的日志记录能力。

## 特性

- 非阻塞发布：调用方快速发布日志记录到环形缓冲，不等待 I/O
- 零拷贝记录：`&'static str` 元数据和高效字节格式化，减少分配
- 批量处理：消费者闭包在批尾统一刷新，支持批量写入接口
- 线程安全：`Arc` 与原子计数统计发送/写入/丢失
- 优雅关闭：等待已发送日志全部写出后关闭输出目标

## 使用示例

```rust
use nanolog_rs::{AsyncLogger, Level, Record, DefaultFormatter, ConsoleSink};
use std::sync::Arc;
use std::time::Duration;

let logger = AsyncLogger::new(
    Level::Info,
    Arc::new(DefaultFormatter::new()),
    Arc::new(ConsoleSink::new()),
    1024,
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
logger.shutdown().unwrap();
```
*/

use disruptor::*;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use crate::Level;
use crate::Record;
use crate::error::Error;
use crate::format::Formatter;
use crate::sink::Sink;

/// 工作线程配置
struct Event {
    record: Record,
}

/// 高性能异步日志器
pub struct AsyncLogger {
    level: Level,
    sink: Arc<dyn Sink>,
    shutdown: Arc<AtomicBool>,
    sent_count: Arc<AtomicUsize>,
    written_count: Arc<AtomicUsize>,
    lost_count: Arc<AtomicUsize>,
    loss_detection_enabled: bool,
    publisher: Arc<dyn Fn(Record) + Send + Sync>,
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
        queue_capacity: usize,
        _batch_size: usize,
        _flush_interval: Duration,
    ) -> Self {
        let shutdown = Arc::new(AtomicBool::new(false));
        let sent_count = Arc::new(AtomicUsize::new(0));
        let written_count = Arc::new(AtomicUsize::new(0));
        let lost_count = Arc::new(AtomicUsize::new(0));

        let formatter_c = formatter.clone();
        let sink_c = sink.clone();
        let written_c = written_count.clone();

        let factory = || Event {
            record: Record::new(Level::Info, "nanolog_rs", "", 0, String::new()),
        };

        let processor = move |e: &Event, _sequence: Sequence, end_of_batch: bool| {
            if let Ok(formatted) = formatter_c.format(&e.record) {
                let _ = sink_c.write(&formatted);
                written_c.fetch_add(1, Ordering::Relaxed);
            }
            if end_of_batch {
                let _ = sink_c.flush();
            }
        };

        let size = queue_capacity.next_power_of_two().max(64);
        let prod = build_multi_producer(size, factory, BusySpin)
            .handle_events_with(processor)
            .build();

        let publisher = {
            let prod_source = prod.clone();
            move |record: Record| {
                let mut p = prod_source.clone();
                p.publish(|e| {
                    e.record = record.clone();
                });
            }
        };

        Self {
            level,
            sink,
            shutdown,
            sent_count,
            written_count,
            lost_count,
            loss_detection_enabled: true,
            publisher: Arc::new(publisher),
        }
    }

    /// 记录日志（非阻塞）
    pub fn log(&self, record: Record) -> Result<(), Error> {
        if !self.should_log(record.level()) {
            return Ok(());
        }

        if self.loss_detection_enabled {
            self.sent_count.fetch_add(1, Ordering::Relaxed);
        }

        (self.publisher)(record.clone());

        Ok(())
    }

    /// 获取日志丢失统计信息
    pub fn get_loss_stats(&self) -> (usize, usize, usize) {
        let sent = self.sent_count.load(Ordering::Relaxed);
        let written = self.written_count.load(Ordering::Relaxed);
        let lost = self.lost_count.load(Ordering::Relaxed);

        // 计算当前丢失的日志数量
        let current_lost = sent.saturating_sub(written);

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

    //

    /// 检查是否应该记录指定级别的日志
    pub fn should_log(&self, level: Level) -> bool {
        level >= self.level
    }

    /// 获取日志级别
    pub fn level(&self) -> Level {
        self.level
    }

    /// 刷新日志（等待所有日志处理完成）
    pub fn flush(&self) -> Result<(), Error> {
        loop {
            let sent = self.sent_count.load(Ordering::Relaxed);
            let written = self.written_count.load(Ordering::Relaxed);
            if written >= sent {
                break;
            }
            std::thread::yield_now();
        }
        let _ = self.sink.flush();
        Ok(())
    }

    /// 优雅关闭日志器
    pub fn shutdown(&self) -> Result<(), Error> {
        self.shutdown.store(true, Ordering::Release);

        loop {
            let sent = self.sent_count.load(Ordering::Relaxed);
            let written = self.written_count.load(Ordering::Relaxed);
            if written >= sent {
                break;
            }
            std::thread::yield_now();
        }
        let _ = self.sink.shutdown();
        Ok(())
    }
}

impl Drop for AsyncLogger {
    fn drop(&mut self) {
        if !self.shutdown.load(Ordering::Acquire) {
            self.shutdown.store(true, Ordering::Release);
            let _ = self.sink.shutdown();
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
        Self {
            logger: Mutex::new(None),
        }
    }

    /// 初始化全局日志器
    pub fn init(&self, logger: Arc<AsyncLogger>) -> Result<(), Error> {
        let mut guard = self
            .logger
            .lock()
            .map_err(|_| Error::Concurrent("global logger lock poisoned"))?;

        // 在测试环境中允许重新初始化
        #[cfg(test)]
        {
            *guard = Some(logger);
            Ok(())
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
        self.logger
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// 记录日志
    pub fn log(&self, record: Record) -> Result<(), Error> {
        if let Some(logger) = self
            .logger
            .lock()
            .map_err(|_| Error::Concurrent("global logger lock poisoned"))?
            .as_ref()
        {
            logger.log(record)
        } else {
            Err(Error::NotInitialized)
        }
    }

    /// 刷新日志
    pub fn flush(&self) -> Result<(), Error> {
        if let Some(logger) = self
            .logger
            .lock()
            .map_err(|_| Error::Concurrent("global logger lock poisoned"))?
            .as_ref()
        {
            logger.flush()
        } else {
            Err(Error::NotInitialized)
        }
    }

    /// 关闭日志器
    pub fn shutdown(&self) -> Result<(), Error> {
        if let Some(logger) = self
            .logger
            .lock()
            .map_err(|_| Error::Concurrent("global logger lock poisoned"))?
            .as_ref()
        {
            logger.shutdown()
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
    let global_logger = GLOBAL_LOGGER.get_or_init(GlobalLogger::new);

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

    #[test]
    fn test_async_logger_basic() {
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
        assert!(logger.flush().is_ok());
        assert!(logger.shutdown().is_ok());
    }

    #[test]
    fn test_global_logger() {
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

    #[test]
    fn test_concurrent_publish_without_mutex() {
        let formatter = Arc::new(DefaultFormatter::new());
        let sink = Arc::new(crate::sink::MemorySink::new());
        let logger = Arc::new(AsyncLogger::new(
            Level::Info,
            formatter,
            sink,
            2048,
            64,
            Duration::from_millis(10),
        ));

        let threads = 4;
        let per_thread = 250;
        let mut handles = Vec::new();
        for _ in 0..threads {
            let lg = logger.clone();
            let handle = std::thread::spawn(move || {
                for i in 0..per_thread {
                    let _ = lg.log(Record::new(
                        Level::Info,
                        "test::concurrent",
                        file!(),
                        line!(),
                        format!("msg {}", i),
                    ));
                }
            });
            handles.push(handle);
        }
        for h in handles {
            let _ = h.join();
        }

        assert!(logger.flush().is_ok());
        let (sent, written, lost) = logger.get_loss_stats();
        assert_eq!(sent, threads * per_thread);
        assert!(written >= sent);
        assert_eq!(lost, 0);
        assert!(logger.shutdown().is_ok());
    }
}
