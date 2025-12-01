/*!
高性能日志记录结构。

简化设计，专注于核心功能，确保零拷贝和低内存开销。
*/

use crate::Level;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// 日志记录结构体
///
/// 包含日志的所有元数据和内容信息，使用零拷贝技术优化性能。
#[derive(Clone, Debug)]
pub struct Record {
    /// 日志级别
    level: Level,
    /// 时间戳（纳秒精度）
    timestamp: u128,
    /// 目标/模块名称（使用 &'static str 避免分配）
    target: &'static str,
    /// 文件路径（使用 &'static str 避免分配）
    file: &'static str,
    /// 行号
    line: u32,
    /// 消息内容（使用 String 但支持零拷贝优化）
    message: String,
}

impl Record {
    /// 创建新的日志记录（高性能版本）
    #[inline]
    pub fn new(
        level: Level,
        target: &'static str,
        file: &'static str,
        line: u32,
        message: String,
    ) -> Self {
        Self {
            level,
            timestamp: Self::current_timestamp(),
            target,
            file,
            line,
            message,
        }
    }

    /// 获取当前时间戳（纳秒精度）
    #[inline]
    fn current_timestamp() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default() // 如果系统时间在UNIX EPOCH之前，则使用默认值
            .as_nanos()
    }

    /// 获取日志级别
    #[inline]
    pub fn level(&self) -> Level {
        self.level
    }

    /// 获取时间戳（纳秒）
    #[inline]
    pub fn timestamp(&self) -> u128 {
        self.timestamp
    }

    /// 获取目标/模块名称
    #[inline]
    pub fn target(&self) -> &'static str {
        self.target
    }

    /// 获取文件路径
    #[inline]
    pub fn file(&self) -> &'static str {
        self.file
    }

    /// 获取行号
    #[inline]
    pub fn line(&self) -> u32 {
        self.line
    }

    /// 获取消息内容
    #[inline]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// 消费记录并返回消息内容（零拷贝优化）
    #[inline]
    pub fn into_message(self) -> String {
        self.message
    }
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 高性能格式化：避免不必要的字符串分配
        write!(
            f,
            "[{}] [{:?}] [{}:{}] {}",
            self.timestamp, self.level, self.target, self.line, self.message
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_creation() {
        let record = Record::new(
            Level::Info,
            "test_module",
            "test_file.rs",
            42,
            "Test message".to_string(),
        );

        assert_eq!(record.level(), Level::Info);
        assert_eq!(record.target(), "test_module");
        assert_eq!(record.file(), "test_file.rs");
        assert_eq!(record.line(), 42);
        assert_eq!(record.message(), "Test message");
        assert!(record.timestamp() > 0);
    }

    #[test]
    fn test_record_into_message() {
        let record = Record::new(
            Level::Error,
            "test",
            "test.rs",
            10,
            "Error message".to_string(),
        );

        let message = record.into_message();
        assert_eq!(message, "Error message");
    }
}
