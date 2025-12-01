//! 高级日志宏使用示例
//!
//! 展示如何使用nanolog-rs提供的日志宏的各种功能

use nanolog_rs::{AsyncLoggerBuilder, Level, init_global_logger};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 使用Builder模式创建日志器
    let logger = AsyncLoggerBuilder::new().level(Level::Trace).build()?;

    // 初始化全局日志器
    init_global_logger(Arc::new(logger))?;

    // 基本日志宏使用
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

    // 条件日志记录
    let user_id = 12345;
    if user_id > 0 {
        nanolog_rs::debug!("处理用户请求: user_id = {}", user_id);
    }

    // 在循环中使用日志宏
    for i in 1..=3 {
        nanolog_rs::trace!("循环迭代: {}", i);
    }

    // 错误处理场景
    match perform_operation() {
        Ok(result) => nanolog_rs::info!("操作成功完成: {}", result),
        Err(e) => nanolog_rs::error!("操作失败: {}", e),
    }

    // 性能关键区域的日志
    let start_time = std::time::Instant::now();
    perform_critical_task();
    let duration = start_time.elapsed();
    nanolog_rs::debug!("关键任务执行时间: {:?}", duration);

    // 等待一段时间确保所有日志都被处理
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok(())
}

fn perform_operation() -> Result<String, &'static str> {
    // 模拟一个可能失败的操作
    let success = true;
    if success {
        Ok("操作成功".to_string())
    } else {
        Err("操作失败")
    }
}

fn perform_critical_task() {
    // 模拟一个性能关键的任务
    nanolog_rs::trace!("开始执行关键任务");

    // 模拟一些工作
    for i in 0..1000 {
        // 只在特定条件下记录详细信息以避免日志泛滥
        if i % 100 == 0 {
            nanolog_rs::trace!("任务进度: {}%", i / 10);
        }
    }

    nanolog_rs::trace!("关键任务完成");
}
