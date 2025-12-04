/*!
高性能日志格式化器。

简化设计，专注于零拷贝和低延迟格式化。
*/

use crate::Record;
use chrono::{DateTime, FixedOffset, Utc};
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
    /// 时间戳显示风格
    timestamp_style: TimestampStyle,
}

/// 时间戳显示风格
pub enum TimestampStyle {
    /// 使用 UNIX 纳秒整型（十进制）
    NumericNs,
    /// 使用 ISO8601 字符串并带时区偏移（chrono）；`None` 表示使用 UTC
    Iso8601(Option<FixedOffset>),
}

impl DefaultFormatter {
    /// 创建新的默认格式化器
    pub fn new() -> Self {
        Self {
            colored: Self::should_use_color(),
            timestamp_style: TimestampStyle::NumericNs,
        }
    }

    /// 创建使用彩色输出的格式化器
    pub fn colored() -> Self {
        Self {
            colored: true,
            timestamp_style: TimestampStyle::NumericNs,
        }
    }

    /// 创建不使用彩色输出的格式化器
    pub fn plain() -> Self {
        Self {
            colored: false,
            timestamp_style: TimestampStyle::NumericNs,
        }
    }

    /// 使用 ISO8601 上海时区时间戳
    pub fn with_iso8601_shanghai() -> Self {
        let offset = FixedOffset::east_opt(8 * 3600);
        Self {
            colored: Self::should_use_color(),
            timestamp_style: TimestampStyle::Iso8601(offset),
        }
    }

    /// 设置时间戳风格
    pub fn with_timestamp_style(style: TimestampStyle) -> Self {
        Self {
            colored: Self::should_use_color(),
            timestamp_style: style,
        }
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

    /// 时间戳格式化（根据风格）
    fn format_timestamp(&self, timestamp_ns: u128) -> String {
        match self.timestamp_style {
            TimestampStyle::NumericNs => timestamp_ns.to_string(),
            TimestampStyle::Iso8601(offset_opt) => {
                // 将纳秒转换为秒，并在溢出时舍弃精度
                let secs_u128 = timestamp_ns / 1_000_000_000;
                let nanos_u32 = (timestamp_ns % 1_000_000_000) as u32;
                let (secs_i64, nanos_i32) = if secs_u128 > i64::MAX as u128 {
                    (i64::MAX, 0)
                } else {
                    (secs_u128 as i64, nanos_u32)
                };

                let utc_dt = DateTime::<Utc>::from_timestamp(secs_i64, nanos_i32)
                    .unwrap_or(DateTime::<Utc>::UNIX_EPOCH);
                match offset_opt {
                    Some(offset) => utc_dt
                        .with_timezone(&offset)
                        .format("%Y-%m-%dT%H:%M:%S%.9f%:z")
                        .to_string(),
                    None => utc_dt.format("%Y-%m-%dT%H:%M:%S%.9fZ").to_string(),
                }
            }
        }
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

        // 格式化时间戳（可配置：数字或ISO8601）
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
