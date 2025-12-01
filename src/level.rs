//! 日志级别定义

use std::fmt;
use std::str::FromStr;

/// 日志级别枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Level {
    /// 跟踪级别 - 最详细的日志信息，用于调试
    Trace = 0,
    /// 调试级别 - 调试信息，用于开发阶段
    Debug = 1,
    /// 信息级别 - 常规信息，用于生产环境
    #[default]
    Info = 2,
    /// 警告级别 - 警告信息，需要关注但不会影响程序运行
    Warn = 3,
    /// 错误级别 - 错误信息，需要立即处理
    Error = 4,
}

impl Level {
    /// 获取级别的字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

impl FromStr for Level {
    type Err = ();

    /// 从字符串解析级别
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "TRACE" => Ok(Level::Trace),
            "DEBUG" => Ok(Level::Debug),
            "INFO" => Ok(Level::Info),
            "WARN" => Ok(Level::Warn),
            "ERROR" => Ok(Level::Error),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
