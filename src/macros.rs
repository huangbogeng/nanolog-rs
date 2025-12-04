//! 日志宏模块
//!
//! 提供类似标准log crate的宏，但与nanolog-rs集成

/// 记录日志的宏实现
///
/// 该宏具有惰性求值特性：只有当日志级别启用时，才会执行格式化操作，
/// 避免了不必要的字符串格式化开销。
#[macro_export]
macro_rules! log {
    (target: $target:expr, $lvl:expr, $($arg:tt)+) => ({
        let lvl = $lvl;
        if let Some(logger) = $crate::global_logger() {
            if logger.get().map_or(false, |l| l.should_log(lvl)) {
                let record = $crate::Record::new(
                    lvl,
                    $target,
                    file!(),
                    line!(),
                    format!($($arg)+),
                );
                let _ = logger.log(record);
            }
        }
    });
    ($lvl:expr, $($arg:tt)+) => (
        $crate::log!(target: module_path!(), $lvl, $($arg)+)
    );
}

/// 记录错误级别日志
#[macro_export]
macro_rules! error {
    (target: $target:expr, $($arg:tt)+) => (
        $crate::log!(target: $target, $crate::Level::Error, $($arg)+)
    );
    ($($arg:tt)+) => (
        $crate::log!($crate::Level::Error, $($arg)+)
    );
}

/// 记录警告级别日志
#[macro_export]
macro_rules! warn {
    (target: $target:expr, $($arg:tt)+) => (
        $crate::log!(target: $target, $crate::Level::Warn, $($arg)+)
    );
    ($($arg:tt)+) => (
        $crate::log!($crate::Level::Warn, $($arg)+)
    );
}

/// 记录信息级别日志
#[macro_export]
macro_rules! info {
    (target: $target:expr, $($arg:tt)+) => (
        $crate::log!(target: $target, $crate::Level::Info, $($arg)+)
    );
    ($($arg:tt)+) => (
        $crate::log!($crate::Level::Info, $($arg)+)
    );
}

/// 记录调试级别日志
#[macro_export]
macro_rules! debug {
    (target: $target:expr, $($arg:tt)+) => (
        $crate::log!(target: $target, $crate::Level::Debug, $($arg)+)
    );
    ($($arg:tt)+) => (
        $crate::log!($crate::Level::Debug, $($arg)+)
    );
}

/// 记录跟踪级别日志
#[macro_export]
macro_rules! trace {
    (target: $target:expr, $($arg:tt)+) => (
        $crate::log!(target: $target, $crate::Level::Trace, $($arg)+)
    );
    ($($arg:tt)+) => (
        $crate::log!($crate::Level::Trace, $($arg)+)
    );
}

#[cfg(test)]
mod tests {
    use crate::{AsyncLogger, ConsoleSink, DefaultFormatter, Level, init_global_logger};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_macro_compilation() {
        // 创建测试日志器
        let formatter = Arc::new(DefaultFormatter::new());
        let sink = Arc::new(ConsoleSink::new());
        let logger = Arc::new(AsyncLogger::new(
            Level::Trace,
            formatter,
            sink,
            1024,
            64,
            Duration::from_millis(50),
        ));

        let _ = init_global_logger(logger);

        // 测试宏是否能正常编译
        error!("This is an error message");
        warn!("This is a warning message");
        info!("This is an info message");
        debug!("This is a debug message");
        trace!("This is a trace message");

        // 带参数的宏测试
        let x = 42;
        info!("The answer is {}", x);
        error!("Error occurred with value: {}", x);
    }
}
