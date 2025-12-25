# NanoLog-rs

高性能非阻塞日志库，基于 Disruptor 环形缓冲实现超低延迟与高吞吐。

## 特性

- 非阻塞发布：调用方快速发布日志记录到环形缓冲，不等待 I/O
- 零拷贝记录：使用高效字节格式化与预分配事件，减少分配
- 批量处理：消费者在批尾统一刷新，支持 `write_batch` 降低系统调用
- 线程安全：`Arc` 与原子计数统计发送/写入/丢失
- 优雅关闭：等待已发布日志全部写出后关闭输出目标

## 行为说明

- 非阻塞发布：`log()` 只发布到环形缓冲并立即返回，不等待 I/O（见 `src/logger.rs:142-154`）
- 刷新与关闭：`flush()/shutdown()` 会等待已发送计数追上已写入计数，并触发 `sink.flush()/sink.shutdown()`（见 `src/logger.rs:200-228`）
- 容量规则：环形缓冲大小取 `queue_capacity.next_power_of_two().max(64)`，保证 Disruptor 的最小槽位要求（见 `src/logger.rs:112-116`）

## 安装

```toml
[dependencies]
nanolog-rs = "0.2.1"
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

fn main() -> Result<(), nanolog_rs::error::Error> {
    let logger = AsyncLoggerBuilder::new()
        .level(Level::Trace)
        .with_file_output("logs/macro.log")
        .with_console_output()
        .build()?;
    init_global_logger(Arc::new(logger))?;

    nanolog_rs::info!(target: "init", "service started");
    if let Some(gl) = nanolog_rs::global_logger() {
        gl.flush()?;
        gl.shutdown()?;
    }
    Ok(())
}
```

## 安全退出

- 短生命周期程序：发布日志后显式调用 `flush()` 与 `shutdown()`，确保缓冲区写出并关闭输出目标
- 长生命周期服务：在退出路径（如 SIGINT/SIGTERM 信号处理器）调用 `shutdown()`，避免在业务线程中阻塞
- panic 钩子：库在初始化全局日志器时已注册 panic 钩子，异常退出时尽力 `flush + shutdown`
- 注意：`SIGKILL` 无法保证清理；这是所有进程的通用限制

示例（最小安全退出）：`examples/safe_shutdown.rs`

```bash
cargo run --example safe_shutdown
```

## 时间戳配置

- 默认行为：全局使用数值时间戳（UNIX 纳秒整型），写入紧凑、解析高效（JSON 格式保持数值时间戳）
- 可读显示：可为某个 `logger` 配置文本显示为 ISO8601 并带时区（如亚洲上海 `+08:00`）

示例：为当前 `logger` 启用上海时区的易读时间戳显示

```rust
use nanolog_rs::{AsyncLogger, Level};
use nanolog_rs::format::DefaultFormatter;
use std::sync::Arc;
use std::time::Duration;

let formatter = Arc::new(DefaultFormatter::with_iso8601_shanghai());
let logger = AsyncLogger::new(
    Level::Info,
    formatter,
    Arc::new(nanolog_rs::ConsoleSink::new()),
    1024,
    100,
    Duration::from_millis(100),
);
```

通过 Builder 的便捷方法：

```rust
use nanolog_rs::{AsyncLogger, Level};

let logger = AsyncLogger::builder()
    .level(Level::Info)
    .with_iso8601_shanghai_formatting()
    .build()?;
```

或自定义时区偏移：

```rust
use nanolog_rs::{AsyncLogger, Level};
use nanolog_rs::format::TimestampStyle;

let logger = AsyncLogger::builder()
    .level(Level::Info)
    .with_default_timestamp_style(
        TimestampStyle::Iso8601(time::UtcOffset::from_hms(9, 0, 0).unwrap())
    )
    .build()?;
```

注意：`JsonFormatter` 为机器友好，始终写入数值时间戳以保持体积小、解析高效（见 `src/format.rs:146-173`）。

## 运行

- 构建示例：`cargo build --examples`
- 运行示例：`cargo run --example builder_example`
- 运行安全退出示例：`cargo run --example safe_shutdown`
- 运行测试：`cargo test`
- 运行基准：`cargo bench`

## 发布前检查

- 格式化：`cargo fmt --all --check`
- 静态检查：`cargo clippy --all-targets --all-features`
- 单元测试：`cargo test`
- 构建发布包：`cargo build --release`

## 许可

MIT License
