/*!
独立的Builder模块，用于构建AsyncLogger实例。

该模块提供了现代化的Builder模式，使用户能够以流畅的方式配置和创建日志器实例。
*/

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::Level;

use crate::error::Error;
use crate::format::Formatter;
use crate::format::TimestampStyle;
use crate::logger::AsyncLogger;
use crate::sink::Sink;

/// 构建器模式配置
#[derive(Clone)]
pub struct AsyncLoggerBuilder {
    level: Level,
    formatter: Option<Arc<dyn Formatter>>,
    sink: Option<Arc<dyn Sink>>,
    queue_capacity: usize,
    batch_size: usize,
    flush_interval: Duration,
}

impl Default for AsyncLoggerBuilder {
    fn default() -> Self {
        Self {
            level: Level::Info,
            formatter: None,
            sink: None,
            queue_capacity: 1000,
            batch_size: 100,
            flush_interval: Duration::from_millis(100),
        }
    }
}

impl AsyncLoggerBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置日志级别
    pub fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// 设置格式化器
    pub fn formatter(mut self, formatter: Arc<dyn Formatter>) -> Self {
        self.formatter = Some(formatter);
        self
    }

    /// 设置输出目标
    pub fn sink(mut self, sink: Arc<dyn Sink>) -> Self {
        self.sink = Some(sink);
        self
    }

    /// 设置队列容量
    pub fn queue_capacity(mut self, capacity: usize) -> Self {
        self.queue_capacity = capacity;
        self
    }

    /// 设置批处理大小
    pub fn batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// 设置刷新间隔
    pub fn flush_interval(mut self, interval: Duration) -> Self {
        self.flush_interval = interval;
        self
    }

    /// 设置为调试级别 (便捷方法)
    pub fn with_debug_level(mut self) -> Self {
        self.level = Level::Debug;
        self
    }

    /// 设置为跟踪级别 (便捷方法)
    pub fn with_trace_level(mut self) -> Self {
        self.level = Level::Trace;
        self
    }

    /// 使用JSON格式化器 (便捷方法)
    pub fn with_json_formatting(mut self) -> Self {
        self.formatter = Some(Arc::new(crate::format::JsonFormatter::new()));
        self
    }

    /// 使用简单格式化器 (便捷方法)
    pub fn with_simple_formatting(mut self) -> Self {
        self.formatter = Some(Arc::new(crate::format::SimpleFormatter::new()));
        self
    }

    /// 使用默认格式化器并配置为上海时区 ISO8601 (便捷方法)
    pub fn with_iso8601_shanghai_formatting(mut self) -> Self {
        self.formatter = Some(Arc::new(
            crate::format::DefaultFormatter::with_iso8601_shanghai(),
        ));
        self
    }

    /// 使用默认格式化器并指定时间戳风格 (便捷方法)
    pub fn with_default_timestamp_style(mut self, style: TimestampStyle) -> Self {
        self.formatter = Some(Arc::new(
            crate::format::DefaultFormatter::with_timestamp_style(style),
        ));
        self
    }

    /// 使用控制台输出 (便捷方法)
    pub fn with_console_output(mut self) -> Self {
        self.sink = Some(Arc::new(crate::sink::ConsoleSink::new()));
        self
    }

    /// 使用文件输出 (便捷方法)
    pub fn with_file_output<P: AsRef<Path>>(mut self, path: P) -> Self {
        match crate::sink::FileSink::new(path) {
            Ok(sink) => self.sink = Some(Arc::new(sink)),
            Err(_) => {
                // 如果文件创建失败，则回退到控制台输出
                self.sink = Some(Arc::new(crate::sink::ConsoleSink::new()));
            }
        }
        self
    }

    /// 构建AsyncLogger实例
    pub fn build(self) -> Result<AsyncLogger, Error> {
        let formatter = self
            .formatter
            .unwrap_or_else(|| Arc::new(crate::format::DefaultFormatter::new()));
        let sink = self
            .sink
            .unwrap_or_else(|| Arc::new(crate::sink::ConsoleSink::new()));

        Ok(AsyncLogger::new(
            self.level,
            formatter,
            sink,
            self.queue_capacity,
            self.batch_size,
            self.flush_interval,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_builder_creation() {
        let builder = AsyncLoggerBuilder::new();
        assert_eq!(builder.level, Level::Info);
    }

    #[test]
    fn test_builder_with_level() {
        let builder = AsyncLoggerBuilder::new().level(Level::Debug);
        assert_eq!(builder.level, Level::Debug);
    }

    #[test]
    fn test_builder_with_convenience_methods() {
        let builder = AsyncLoggerBuilder::new()
            .with_debug_level()
            .with_console_output()
            .with_simple_formatting();

        assert_eq!(builder.level, Level::Debug);
        assert!(builder.formatter.is_some());
        assert!(builder.sink.is_some());
    }

    #[test]
    fn test_builder_build() {
        let result = AsyncLoggerBuilder::new().level(Level::Info).build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_all_convenience_methods() {
        let builder = AsyncLoggerBuilder::new()
            .with_debug_level()
            .with_trace_level() // This should override debug level
            .with_json_formatting()
            .with_simple_formatting() // This should override json formatting
            .with_console_output();

        assert_eq!(builder.level, Level::Trace);
        assert!(builder.formatter.is_some());
        assert!(builder.sink.is_some());
    }

    #[test]
    fn test_builder_configuration_methods() {
        let builder = AsyncLoggerBuilder::new()
            .queue_capacity(2000)
            .batch_size(50)
            .flush_interval(Duration::from_millis(200));

        // We can't directly access the fields, but we can verify the builder was configured
        assert_eq!(builder.queue_capacity, 2000);
        assert_eq!(builder.batch_size, 50);
        assert_eq!(builder.flush_interval, Duration::from_millis(200));
    }

    #[test]
    fn test_builder_with_all_configurations() {
        let result = AsyncLoggerBuilder::new()
            .level(Level::Trace)
            .with_json_formatting()
            .with_console_output()
            .queue_capacity(2000)
            .batch_size(50)
            .flush_interval(Duration::from_millis(200))
            .build();

        assert!(result.is_ok());
    }
}
