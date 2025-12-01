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
    }
}
