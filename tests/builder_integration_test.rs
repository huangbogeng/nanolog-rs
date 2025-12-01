//! Builder模式集成测试
//!
//! 这个测试文件演示了如何在实际应用中使用AsyncLoggerBuilder

use nanolog_rs::{AsyncLoggerBuilder, Level, Record};
use std::time::Duration;

#[tokio::test]
async fn test_builder_integration() {
    // 测试默认配置
    let logger = AsyncLoggerBuilder::new()
        .build()
        .expect("Failed to create logger with default configuration");

    logger
        .log(Record::new(
            Level::Info,
            "test::integration",
            file!(),
            line!(),
            "Integration test with default config".to_string(),
        ))
        .expect("Failed to log message");

    // 测试完整配置
    let full_config_logger = AsyncLoggerBuilder::new()
        .level(Level::Trace)
        .with_json_formatting()
        .with_console_output()
        .queue_capacity(2000)
        .batch_size(50)
        .flush_interval(Duration::from_millis(100))
        .build()
        .expect("Failed to create logger with full configuration");

    full_config_logger
        .log(Record::new(
            Level::Debug,
            "test::integration",
            file!(),
            line!(),
            "Integration test with full config".to_string(),
        ))
        .expect("Failed to log message");

    full_config_logger
        .log(Record::new(
            Level::Trace,
            "test::integration",
            file!(),
            line!(),
            "Trace level message".to_string(),
        ))
        .expect("Failed to log trace message");

    // 确保所有日志都被处理
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 关闭日志器
    logger.shutdown().await.expect("Failed to shutdown logger");
    full_config_logger
        .shutdown()
        .await
        .expect("Failed to shutdown full config logger");
}

#[tokio::test]
async fn test_builder_convenience_methods() {
    // 测试便捷方法组合
    let logger = AsyncLoggerBuilder::new()
        .with_debug_level()
        .with_json_formatting()
        .with_console_output()
        .build()
        .expect("Failed to create logger with convenience methods");

    logger
        .log(Record::new(
            Level::Debug,
            "test::convenience",
            file!(),
            line!(),
            "Convenience methods test".to_string(),
        ))
        .expect("Failed to log message");

    logger.shutdown().await.expect("Failed to shutdown logger");
}
