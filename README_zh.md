# NanoLog-rs

[![Crates.io](https://img.shields.io/crates/v/nanolog-rs.svg)](https://crates.io/crates/nanolog-rs)
[![Documentation](https://docs.rs/nanolog-rs/badge.svg)](https://docs.rs/nanolog-rs)
[![License](https://img.shields.io/crates/l/nanolog-rs.svg)](https://github.com/huangbogeng/nanolog-rs/blob/master/LICENSE)

专为高频交易系统设计的高性能异步日志库，提供超低延迟的日志记录能力。

[English](README.md) | 中文

## 特性

- **超低延迟**：关键应用的亚微秒级日志延迟
- **零拷贝设计**：最小化内存分配和拷贝开销
- **异步处理**：非阻塞的日志操作，配备专用工作线程
- **无锁架构**：线程安全操作，无互斥锁争用
- **多输出目标**：支持控制台、文件和自定义接收器
- **灵活格式化**：内置格式化器（简单、JSON）支持自定义格式化器
- **可配置级别**：标准日志级别（TRACE、DEBUG、INFO、WARN、ERROR、FATAL）
- **批处理**：高效批量处理高吞吐量场景

## 快速开始

### 安装

将以下内容添加到您的 `Cargo.toml`：

```toml
[dependencies]
nanolog-rs = "0.1.0"
tokio = { version = "1.32", features = ["full"] }
```

### 基本用法

```rust
use nanolog_rs::{AsyncLogger, Level};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志器
    let logger = AsyncLogger::builder()
        .level(Level::Info)
        .build()?;
    
    // 记录一些消息
    logger.log(nanolog_rs::Record::new(
        Level::Info,
        "app::main",
        file!(),
        line!(),
        "应用程序启动".to_string()
    ))?;
    
    logger.log(nanolog_rs::Record::new(
        Level::Warn,
        "app::main",
        file!(),
        line!(),
        "这是一个警告消息".to_string()
    ))?;
    
    logger.log(nanolog_rs::Record::new(
        Level::Error,
        "app::main",
        file!(),
        line!(),
        "发生错误：出现问题".to_string()
    ))?;
    
    // 关闭前确保所有日志都被刷新
    logger.shutdown().await?;
    
    Ok(())
}
```

## 核心组件

### AsyncLogger

日志操作的主要入口点。提供异步、非阻塞的日志记录功能。

要创建AsyncLogger实例，请使用现代化的Builder模式：

```rust
use nanolog_rs::AsyncLoggerBuilder;

let logger = AsyncLoggerBuilder::new()
    .level(Level::Debug)
    .batch_size(1000)
    .flush_interval(Duration::from_millis(100))
    .build()?;
```

或者使用便捷方法进行常见配置：

```rust
use nanolog_rs::AsyncLoggerBuilder;

let logger = AsyncLoggerBuilder::new()
    .with_debug_level()
    .with_json_formatting()
    .with_console_output()
    .build()?;
```

### 格式化器

内置格式化器用于不同的输出格式：

- `SimpleFormatter`：人类可读格式
- `JsonFormatter`：结构化JSON格式
- 可通过 `Formatter` trait 实现自定义格式化器

### 接收器

日志消息的输出目的地：

- `ConsoleSink`：标准输出/错误
- 文件接收器（计划中）
- 网络接收器（计划中）
- 通过 `Sink` trait 的自定义接收器

## 高级用法

### 自定义格式化器

```rust
use nanolog_rs::{Formatter, Record};
use std::sync::Arc;

struct CustomFormatter;

impl Formatter for CustomFormatter {
    fn format(&self, record: &Record) -> String {
        format!("[{}] {} - {}", 
                record.timestamp(),
                record.level(),
                record.message())
    }
}

// 使用自定义格式化器
let logger = AsyncLogger::builder()
    .formatter(Arc::new(CustomFormatter))
    .build()?;
```

### 自定义接收器

```rust
use nanolog_rs::{Sink, Record};
use std::sync::Arc;

struct CustomSink;

#[async_trait::async_trait]
impl Sink for CustomSink {
    async fn write(&self, formatted_message: &str, _record: &Record) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 自定义写入逻辑（例如，发送到外部服务）
        println!("自定义接收器: {}", formatted_message);
        Ok(())
    }
    
    async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 自定义刷新逻辑
        Ok(())
    }
}

// 使用自定义接收器
let logger = AsyncLogger::builder()
    .sink(Arc::new(CustomSink))
    .build()?;
```

## 性能

NanoLog-rs 为高频交易系统进行了优化，具有以下性能特征：

- 日志延迟 < 1 微秒（99.9百分位）
- 支持每秒数百万条日志条目
- 低CPU和内存占用
- 不影响主业务逻辑的非阻塞设计

### 基准测试结果

我们的基准测试套件展示了NanoLog-rs的超低延迟能力：

| 操作 | 平均时间 | 吞吐量 |
|------|---------|--------|
| 缓冲区获取/释放 | 53.5 ns | ~18 百万次/秒 |
| 缓冲区写入操作 | 73.1 ns | ~13 百万次/秒 |

这些结果是使用Criterion.rs获得的，采样100次，测量时间10秒。基准测试独立测量原始操作性能，展示了我们核心组件的最小开销。

### 运行基准测试

运行内置基准测试套件：

```bash
cargo bench
```

性能目标：
- 关键应用的超低延迟
- 批处理的高吞吐量
- 最小资源消耗

## 测试

要运行所有测试，请使用：

```bash
cargo test
```

### 测试组织

我们的测试分为三类：

1. 单元测试：位于每个模块的`tests`子模块内
2. 集成测试：位于`tests/`目录中
3. 文档测试：嵌入在代码文档中

### Builder模式测试

要专门运行与Builder模式相关的测试，请使用：

```bash
cargo test builder
```

这些测试涵盖了：
- 基本构建器创建和配置
- 便捷方法功能
- 构建器构造和错误处理
- 完整配置工作流
- 真实场景下的集成测试

我们还在`tests/builder_integration_test.rs`中有专门的集成测试，展示了真实世界的使用模式：

```bash
cargo test builder_integration
```

## 错误处理

该库使用 `thiserror` 进行全面的错误处理：

```rust
match AsyncLogger::builder().build() {
    Ok(logger) => {
        logger.init();
        // 日志操作...
    },
    Err(e) => {
        eprintln!("初始化日志器失败: {}", e);
        // 处理初始化失败
    }
}
```

## 未来计划

参见 [TODO.md](TODO.md) 了解我们的开发路线图，包括：
- 日志轮转和归档
- 配置系统
- 其他输出目标
- 增强过滤机制

## 项目结构

```
nanolog-rs/
├── benches/                 # 基准测试
│   ├── benchmarks.rs
│   └── logger_benchmark.rs
├── examples/                # 使用示例
│   ├── advanced_usage.rs
│   ├── basic_usage.rs
│   ├── builder_example.rs
│   └── modern_builder_example.rs
├── logs/                    # 日志文件 (git忽略)
│   ├── composite.log
│   ├── example.log
│   ├── example_modern.log
│   └── performance.log
├── src/                     # 源代码
│   ├── buffer.rs           # 缓冲池实现
│   ├── builder.rs          # AsyncLoggerBuilder实现
│   ├── error.rs            # 错误处理
│   ├── format.rs           # 消息格式化
│   ├── level.rs            # 日志级别
│   ├── lib.rs              # 库入口点
│   ├── logger.rs           # 主日志器实现
│   ├── record.rs           # 日志记录结构
│   ├── sink.rs             # 输出接收器
│   └── tests/              # 内部单元测试
├── tests/                   # 集成测试
│   └── builder_integration_test.rs
├── .gitignore              # Git忽略规则
├── Cargo.lock              # 依赖锁定文件
├── Cargo.toml              # 包清单
├── LICENSE                 # 许可证文件
├── README.md               # 英文文档
├── README_zh.md            # 中文文档
└── TODO.md                 # 开发路线图
```

## 贡献

欢迎贡献！请随时提交拉取请求或开启问题报告bug和功能请求。

1. Fork 仓库
2. 创建您的功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交您的更改 (`git commit -m '添加令人惊叹的功能'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 开启拉取请求

## 许可证

该项目基于MIT许可证 - 详情请见 [LICENSE](LICENSE) 文件。

## 致谢

- 受金融服务业使用的高性能日志系统的启发
- 基于Rust的安全性和性能保证构建