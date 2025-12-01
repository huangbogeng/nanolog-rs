//! 日志宏使用示例
//!
//! 展示如何使用nanolog-rs提供的日志宏

use nanolog_rs::{AsyncLoggerBuilder, Level, init_global_logger};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 使用Builder模式创建日志器
    let logger = AsyncLoggerBuilder::new().level(Level::Trace).build()?;

    // 初始化全局日志器
    init_global_logger(Arc::new(logger))?;

    // 使用宏记录不同级别的日志
    nanolog_rs::error!("这是一个错误消息");
    nanolog_rs::warn!("这是一个警告消息");
    nanolog_rs::info!("这是一个信息消息");
    nanolog_rs::debug!("这是一个调试消息");
    nanolog_rs::trace!("这是一个跟踪消息");

    // 带参数的日志记录
    let x = 42;
    let y = "Rust";
    nanolog_rs::info!("计算结果: x = {}, language = {}", x, y);

    // 带目标的日志记录
    nanolog_rs::info!(target: "network", "网络连接已建立");
    nanolog_rs::error!(target: "database", "数据库连接失败: {}", "连接超时");

    // 等待一段时间确保所有日志都被处理
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok(())
}
