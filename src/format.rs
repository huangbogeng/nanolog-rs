/*!
高性能日志格式化器。

简化设计，专注于零拷贝和低延迟格式化。
*/

use crate::Record;
use std::fmt;

/// 高性能格式化器接口
pub trait Formatter: Send + Sync {
    /// 将日志记录格式化为字节数组（高性能版本）
    fn format(&self, record: &Record) -> Result<Vec<u8>, fmt::Error>;
}

/// 默认高性能格式化器
pub struct DefaultFormatter {
    /// 是否使用彩色输出
    colored: bool,
}

impl DefaultFormatter {
    /// 创建新的默认格式化器
    pub fn new() -> Self {
        Self {
            colored: Self::should_use_color(),
        }
    }

    /// 创建使用彩色输出的格式化器
    pub fn colored() -> Self {
        Self { colored: true }
    }

    /// 创建不使用彩色输出的格式化器
    pub fn plain() -> Self {
        Self { colored: false }
    }

    /// 检查是否应该使用彩色输出
    fn should_use_color() -> bool {
        // 在实际应用中，可以检查终端是否支持颜色
        // 这里简化处理，默认在非Windows系统上使用彩色
        #[cfg(not(windows))]
        return true;
        #[cfg(windows)]
        return false;
    }

    /// 使用预分配缓冲区高效格式化时间戳
    fn format_timestamp(&self, timestamp_ns: u128) -> String {
        // 正确处理纳秒时间戳：将其分为秒和纳秒两部分
        let seconds = timestamp_ns / 1_000_000_000;
        let nanos = (timestamp_ns % 1_000_000_000) as u32;

        // 转换为可读时间格式：HH:MM:SS.NNNNNNNNN
        let seconds_total = seconds as u64;
        let hours = seconds_total / 3600;
        let minutes = (seconds_total % 3600) / 60;
        let seconds_remaining = seconds_total % 60;

        format!(
            "{:02}:{:02}:{:02}.{:09}",
            hours, minutes, seconds_remaining, nanos
        )
    }
}

impl Default for DefaultFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for DefaultFormatter {
    fn format(&self, record: &Record) -> Result<Vec<u8>, fmt::Error> {
        let mut result = Vec::new();

        // 格式化时间戳
        result.extend_from_slice(b"[");
        result.extend_from_slice(self.format_timestamp(record.timestamp()).as_bytes());
        result.extend_from_slice(b"] ");

        // 格式化级别（可选带颜色）
        if self.colored {
            let level_str = format!(
                "\x1b[{}m[{:5}]\x1b[0m ",
                match record.level() {
                    crate::Level::Trace => 90, // 灰色
                    crate::Level::Debug => 36, // 青色
                    crate::Level::Info => 32,  // 绿色
                    crate::Level::Warn => 33,  // 黄色
                    crate::Level::Error => 31, // 红色
                },
                record.level()
            );
            result.extend_from_slice(level_str.as_bytes());
        } else {
            let level_str = format!("[{:5}] ", record.level());
            result.extend_from_slice(level_str.as_bytes());
        }

        // 格式化模块名和行号
        let target_str = format!("[{}:{}] ", record.target(), record.line());
        result.extend_from_slice(target_str.as_bytes());

        // 格式化消息内容
        result.extend_from_slice(record.message().as_bytes());

        // 添加换行符
        result.push(b'\n');

        Ok(result)
    }
}

/// JSON格式化器（高性能版本）
pub struct JsonFormatter {
    /// 是否格式化输出（美化格式）
    pretty: bool,
}

impl JsonFormatter {
    /// 创建新的JSON格式化器
    pub fn new() -> Self {
        Self { pretty: false }
    }

    /// 创建美化格式的JSON格式化器
    pub fn pretty() -> Self {
        Self { pretty: true }
    }
}

impl Default for JsonFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for JsonFormatter {
    fn format(&self, record: &Record) -> Result<Vec<u8>, fmt::Error> {
        let result = if self.pretty {
            // 美化格式
            format!(
                "{{\n  \"timestamp\": {},\n  \"level\": \"{}\",\n  \"target\": \"{}\",\n  \"file\": \"{}\",\n  \"line\": {},\n  \"message\": \"{}\"\n}}\n",
                record.timestamp(),
                record.level().as_str(),
                record.target(),
                record.file(),
                record.line(),
                record.message().replace('"', "\\\"")
            )
        } else {
            // 紧凑格式
            format!(
                "{{\"timestamp\":{},\"level\":\"{}\",\"target\":\"{}\",\"file\":\"{}\",\"line\":{},\"message\":\"{}\"}}\n",
                record.timestamp(),
                record.level().as_str(),
                record.target(),
                record.file(),
                record.line(),
                record.message().replace('"', "\\\"")
            )
        };

        Ok(result.into_bytes())
    }
}

/// 简单格式化器（最高性能）
pub struct SimpleFormatter;

impl SimpleFormatter {
    /// 创建新的简单格式化器
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for SimpleFormatter {
    fn format(&self, record: &Record) -> Result<Vec<u8>, fmt::Error> {
        // 最简单的格式化：级别 + 消息
        let result = format!("[{}] {}\n", record.level(), record.message());
        Ok(result.into_bytes())
    }
}
