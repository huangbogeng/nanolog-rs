/*!
高性能日志库的错误处理模块。

简化错误类型，专注于核心错误场景，避免不必要的性能开销。
*/

use std::fmt;
use std::io;

/// 高性能日志库的错误类型
#[derive(Debug)]
pub enum Error {
    /// 初始化错误，日志器已经被初始化
    AlreadyInitialized,

    /// 未初始化错误，日志器尚未初始化
    NotInitialized,

    /// I/O错误，如文件写入失败
    Io(io::Error),

    /// 队列错误，如队列已满或为空
    Queue(&'static str),

    /// 配置错误，如无效的配置值
    Config(&'static str),

    /// 内存错误，如缓冲区溢出
    Memory(&'static str),

    /// 格式化错误，如无效的格式化字符串
    Formatting(&'static str),

    /// 关闭错误，如关闭过程中发生错误
    Shutdown(&'static str),

    /// 轮转错误，如日志轮转过程中发生错误
    Rotation(&'static str),

    /// 并发错误，如线程同步问题
    Concurrent(&'static str),

    /// 其他错误
    Other(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::AlreadyInitialized => write!(f, "logger already initialized"),
            Error::NotInitialized => write!(f, "logger not initialized"),
            Error::Io(err) => write!(f, "I/O error: {}", err),
            Error::Queue(msg) => write!(f, "queue error: {}", msg),
            Error::Config(msg) => write!(f, "configuration error: {}", msg),
            Error::Memory(msg) => write!(f, "memory error: {}", msg),
            Error::Formatting(msg) => write!(f, "formatting error: {}", msg),
            Error::Shutdown(msg) => write!(f, "shutdown error: {}", msg),
            Error::Rotation(msg) => write!(f, "rotation error: {}", msg),
            Error::Concurrent(msg) => write!(f, "concurrent error: {}", msg),
            Error::Other(msg) => write!(f, "other error: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

/// 结果类型别名，简化错误处理
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::AlreadyInitialized;
        assert_eq!(err.to_string(), "logger already initialized");

        let err = Error::NotInitialized;
        assert_eq!(err.to_string(), "logger not initialized");

        let err = Error::Config("invalid level");
        assert_eq!(err.to_string(), "configuration error: invalid level");

        let err = Error::Memory("buffer overflow");
        assert_eq!(err.to_string(), "memory error: buffer overflow");

        let err = Error::Formatting("invalid format string");
        assert_eq!(err.to_string(), "formatting error: invalid format string");

        let err = Error::Shutdown("failed to shutdown");
        assert_eq!(err.to_string(), "shutdown error: failed to shutdown");

        let err = Error::Rotation("failed to rotate log");
        assert_eq!(err.to_string(), "rotation error: failed to rotate log");

        let err = Error::Concurrent("thread synchronization issue");
        assert_eq!(
            err.to_string(),
            "concurrent error: thread synchronization issue"
        );

        let err = Error::Other("unknown error");
        assert_eq!(err.to_string(), "other error: unknown error");
    }
}
