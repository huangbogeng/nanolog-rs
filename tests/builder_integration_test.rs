//! Builder模式集成测试
//!
//! 这个测试文件演示了如何在实际应用中使用AsyncLoggerBuilder

use nanolog_rs::{AsyncLoggerBuilder, Level, Record};
use std::time::Duration;
use std::fs;
use std::path::PathBuf;
use std::env;

#[test]
fn test_builder_integration() {
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
    std::thread::sleep(Duration::from_millis(200));

    // 关闭日志器
    logger.shutdown().expect("Failed to shutdown logger");
    full_config_logger
        .shutdown()
        .expect("Failed to shutdown full config logger");
}

#[test]
fn test_builder_convenience_methods() {
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

    logger.shutdown().expect("Failed to shutdown logger");
}

#[test]
fn test_builder_console_and_file_composed_order_insensitive() {
    let path1 = "logs/builder_combo1.log";
    let path2 = "logs/builder_combo2.log";

    let logger1 = AsyncLoggerBuilder::new()
        .with_console_output()
        .with_file_output(path1)
        .build()
        .expect("build logger1");

    logger1
        .log(Record::new(
            Level::Info,
            "test::combo",
            file!(),
            line!(),
            "combo1".to_string(),
        ))
        .expect("log");
    let _ = logger1.flush();
    let _ = logger1.shutdown();

    let logger2 = AsyncLoggerBuilder::new()
        .with_file_output(path2)
        .with_console_output()
        .build()
        .expect("build logger2");

    logger2
        .log(Record::new(
            Level::Info,
            "test::combo",
            file!(),
            line!(),
            "combo2".to_string(),
        ))
        .expect("log");
    let _ = logger2.flush();
    let _ = logger2.shutdown();

    // 验证文件均已写入
    let c1 = fs::read(path1).expect("read combo1 file");
    let s1 = String::from_utf8_lossy(&c1);
    assert!(s1.contains("combo1"));

    let c2 = fs::read(path2).expect("read combo2 file");
    let s2 = String::from_utf8_lossy(&c2);
    assert!(s2.contains("combo2"));
}

#[test]
fn test_default_home_file_output() {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let base = PathBuf::from(home).join(".nanolog");
    let now = time::OffsetDateTime::now_utc().date();
    let filename = format!(
        "{:04}-{:02}-{:02}.log",
        now.year(),
        now.month() as u8,
        now.day()
    );
    let path = base.join(filename);

    let logger = AsyncLoggerBuilder::new()
        .with_default_home_file_output()
        .build()
        .expect("build default home file logger");

    logger
        .log(Record::new(
            Level::Info,
            "test::default_home",
            file!(),
            line!(),
            "home default".to_string(),
        ))
        .expect("log");
    let _ = logger.flush();
    let _ = logger.shutdown();

    let content = fs::read(path).expect("read default home file");
    let s = String::from_utf8_lossy(&content);
    assert!(s.contains("home default"));
}
