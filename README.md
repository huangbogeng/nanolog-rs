# NanoLog-rs

高性能非阻塞日志库，基于 Disruptor 环形缓冲实现超低延迟与高吞吐。

## 特性

- 非阻塞发布：调用方快速发布日志记录到环形缓冲，不等待 I/O
- 零拷贝记录：使用高效字节格式化与预分配事件，减少分配
- 批量处理：消费者在批尾统一刷新，支持 `write_batch` 降低系统调用
- 线程安全：`Arc` 与原子计数统计发送/写入/丢失
- 优雅关闭：等待已发布日志全部写出后关闭输出目标

## 安装

```toml
[dependencies]
nanolog-rs = "0.2"
```

## 快速开始

```rust
use nanolog_rs::{AsyncLogger, Level, Record, DefaultFormatter, ConsoleSink};
use std::sync::Arc;
use std::time::Duration;

fn main() {
    let logger = AsyncLogger::new(
        Level::Info,
        Arc::new(DefaultFormatter::new()),
        Arc::new(ConsoleSink::new()),
        1024,
        100,
        Duration::from_millis(100),
    );

    let record = Record::new(
        Level::Info,
        "example",
        file!(),
        line!(),
        "Hello, world!".to_string()
    );

    logger.log(record).unwrap();
    logger.shutdown().unwrap();
}
```

## 文件输出

```rust
use nanolog_rs::{AsyncLogger, Level, Record, SimpleFormatter, FileSink};
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 便捷方法
    let logger = AsyncLogger::builder()
        .level(Level::Info)
        .with_file_output("logs/app.log")
        .build()?;

    // 自定义缓冲
    let buffered_sink = Arc::new(FileSink::with_buffer_size("logs/app_buffered.log", 1 << 20)?);
    let buffered = AsyncLogger::builder()
        .level(Level::Info)
        .formatter(Arc::new(SimpleFormatter::new()))
        .sink(buffered_sink)
        .queue_capacity(2048)
        .flush_interval(Duration::from_millis(20))
        .build()?;

    logger.log(Record::new(Level::Info, "app", file!(), line!(), "file logger".to_string()))?;
    buffered.log(Record::new(Level::Info, "app", file!(), line!(), "buffered file".to_string()))?;

    logger.shutdown()?;
    buffered.shutdown()?;
    Ok(())
}
```

## 宏用法

```rust
use nanolog_rs::{AsyncLoggerBuilder, Level, init_global_logger};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = AsyncLoggerBuilder::new()
        .level(Level::Trace)
        .with_file_output("logs/macro.log")
        .build()?;
    init_global_logger(Arc::new(logger))?;

    nanolog_rs::info!(target: "init", "service started");
    Ok(())
}
```

## 运行

- 构建示例：`cargo build --examples`
- 运行示例：`cargo run --example builder_example`
- 运行测试：`cargo test`
- 运行基准：`cargo bench`

## 许可

MIT License

