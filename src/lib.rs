/*!
高性能异步日志库。

专注于零拷贝、低延迟和高并发性能的日志系统。
*/

#![warn(missing_docs)]

use std::sync::Arc;

pub mod buffer;
pub mod builder;
pub mod error;
pub mod format;
pub mod level;
pub mod logger;
pub mod macros;
pub mod record;
pub mod sink;

// 公共API导出
pub use crate::builder::AsyncLoggerBuilder;
pub use crate::format::{DefaultFormatter, Formatter, JsonFormatter, SimpleFormatter};
pub use crate::level::Level;
pub use crate::logger::{AsyncLogger, GlobalLogger, global_logger, init_global_logger};
// 注意：宏通过#[macro_export]自动导出，无需在此处重新导出
// pub use crate::macros::*;
pub use crate::record::Record;
pub use crate::sink::{CompositeSink, ConsoleSink, FileSink, MemorySink, NullSink, Sink};

/// 初始化全局日志器
///
/// # 示例
/// ```no_run
/// use nanolog_rs::{init_global_logger, Level, AsyncLogger, DefaultFormatter, ConsoleSink};
/// use std::sync::Arc;
/// use std::time::Duration;
///
/// let formatter = Arc::new(DefaultFormatter::new());
/// let sink = Arc::new(ConsoleSink::new());
/// let logger = Arc::new(AsyncLogger::new(
///     Level::Debug,
///     formatter,
///     sink,
///     1000,
///     10,
///     Duration::from_millis(100),
/// ));
///
/// init_global_logger(logger).unwrap();
/// ```
pub fn init(logger: Arc<AsyncLogger>) -> Result<(), crate::error::Error> {
    init_global_logger(logger)
}

/// 获取全局日志器实例
///
/// # 示例
/// ```
/// use nanolog_rs::global_logger;
///
/// if let Some(logger) = global_logger() {
///     // 使用logger
/// }
/// ```
pub fn get_logger() -> Option<&'static GlobalLogger> {
    global_logger()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_api_compilation() {
        // 测试API是否能正常编译
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

        // 验证基本类型可以正常编译
        let record = Record::new(
            Level::Info,
            "test",
            "test.rs",
            1,
            "Test message".to_string(),
        );

        // 测试日志记录功能
        assert!(logger.log(record).is_ok());
        assert!(logger.flush().is_ok());
        assert!(logger.shutdown().is_ok());
    }
}
