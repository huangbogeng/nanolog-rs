# NanoLog-rs

[![Crates.io](https://img.shields.io/crates/v/nanolog-rs.svg)](https://crates.io/crates/nanolog-rs)
[![Documentation](https://docs.rs/nanolog-rs/badge.svg)](https://docs.rs/nanolog-rs)
[![License](https://img.shields.io/crates/l/nanolog-rs.svg)](https://github.com/huangbogeng/nanolog-rs/blob/master/LICENSE)

High-performance asynchronous logging library designed for high-frequency trading systems, providing ultra-low latency logging capabilities.

[English](README.md) | [中文](README_zh.md)

## Features

- **Ultra-Low Latency**: Sub-microsecond logging latency for critical applications
- **Zero-Copy Design**: Minimizes memory allocation and copying overhead
- **Asynchronous Processing**: Non-blocking logging operations with dedicated worker threads
- **Lock-Free Architecture**: Thread-safe operations without mutex contention
- **Multiple Output Targets**: Support for console, file, and custom sinks
- **Flexible Formatting**: Built-in formatters (Simple, JSON) with custom formatter support
- **Configurable Levels**: Standard log levels (TRACE, DEBUG, INFO, WARN, ERROR, FATAL)
- **Batch Processing**: Efficient batch handling for high-throughput scenarios

## Quick Start

### Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
nanolog-rs = "0.1.0"
tokio = { version = "1.32", features = ["full"] }
```

### Basic Usage

```rust
use nanolog_rs::{AsyncLogger, Level};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    let logger = AsyncLogger::builder()
        .level(Level::Info)
        .build()?;
    
    // Log some messages
    logger.log(nanolog_rs::Record::new(
        Level::Info,
        "app::main",
        file!(),
        line!(),
        "Application started".to_string()
    ))?;
    
    logger.log(nanolog_rs::Record::new(
        Level::Warn,
        "app::main",
        file!(),
        line!(),
        "This is a warning message".to_string()
    ))?;
    
    logger.log(nanolog_rs::Record::new(
        Level::Error,
        "app::main",
        file!(),
        line!(),
        "An error occurred: Something went wrong".to_string()
    ))?;
    
    // Ensure all logs are flushed before shutdown
    logger.shutdown().await?;
    
    Ok(())
}
```

## Core Components

### AsyncLogger

The main entry point for logging operations. Provides asynchronous, non-blocking logging capabilities.

To create an AsyncLogger instance, use the modern Builder pattern:

```rust
use nanolog_rs::AsyncLoggerBuilder;

let logger = AsyncLoggerBuilder::new()
    .level(Level::Debug)
    .batch_size(1000)
    .flush_interval(Duration::from_millis(100))
    .build()?;
```

Or use the convenience methods for common configurations:

```rust
use nanolog_rs::AsyncLoggerBuilder;

let logger = AsyncLoggerBuilder::new()
    .with_debug_level()
    .with_json_formatting()
    .with_console_output()
    .build()?;
```

### Formatters

Built-in formatters for different output formats:

- `SimpleFormatter`: Human-readable format
- `JsonFormatter`: Structured JSON format
- Custom formatters can be implemented via the `Formatter` trait

### Sinks

Output destinations for log messages:

- `ConsoleSink`: Standard output/error
- File sinks (planned)
- Network sinks (planned)
- Custom sinks via the `Sink` trait

## Advanced Usage

### Custom Formatter

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

// Use custom formatter
let logger = AsyncLogger::builder()
    .formatter(Arc::new(CustomFormatter))
    .build()?;
```

### Custom Sink

```rust
use nanolog_rs::{Sink, Record};
use std::sync::Arc;

struct CustomSink;

#[async_trait::async_trait]
impl Sink for CustomSink {
    async fn write(&self, formatted_message: &str, _record: &Record) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Custom write logic (e.g., send to external service)
        println!("Custom sink: {}", formatted_message);
        Ok(())
    }
    
    async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Custom flush logic
        Ok(())
    }
}

// Use custom sink
let logger = AsyncLogger::builder()
    .sink(Arc::new(CustomSink))
    .build()?;
```

## Performance

NanoLog-rs is optimized for high-frequency trading systems with the following performance characteristics:

- Logging latency < 1 microsecond (99.9th percentile)
- Supports millions of log entries per second
- Low CPU and memory footprint
- Non-blocking design that doesn't impact main business logic

### Benchmark Results

Our benchmark suite demonstrates the ultra-low latency capabilities of NanoLog-rs:

| Operation | Average Time | Throughput |
|-----------|-------------|------------|
| Buffer Acquisition/Release | 53.5 ns | ~18 million/sec |
| Buffer Write Operations | 73.1 ns | ~13 million/sec |

These results were obtained using Criterion.rs with 100 samples over 10 seconds of measurement time. The benchmarks measure raw operation performance in isolation, demonstrating the minimal overhead of our core components.

### Running Benchmarks

Run the built-in benchmark suite:

```bash
cargo bench
```

Performance targets:
- Ultra-low latency for critical applications
- High throughput for batch processing
- Minimal resource consumption

## Testing

To run all tests, use:

```bash
cargo test
```

### Test Organization

Our tests are organized into three categories:

1. Unit Tests: Located within each module's `tests` submodule
2. Integration Tests: Located in the `tests/` directory
3. Documentation Tests: Embedded in code documentation

### Builder Pattern Tests

To specifically run tests related to the Builder pattern, use:

```bash
cargo test builder
```

These tests cover:
- Basic builder creation and configuration
- Convenience methods functionality
- Builder construction and error handling
- Complete configuration workflows
- Integration testing with real-world scenarios

We also have dedicated integration tests in `tests/builder_integration_test.rs` that demonstrate real-world usage patterns:

```bash
cargo test builder_integration
```

## Error Handling

The library uses `thiserror` for comprehensive error handling:

```rust
match AsyncLogger::builder().build() {
    Ok(logger) => {
        logger.init();
        // Logging operations...
    },
    Err(e) => {
        eprintln!("Failed to initialize logger: {}", e);
        // Handle initialization failure
    }
}
```

## Future Plans

See [TODO.md](TODO.md) for our development roadmap including:
- Log rotation and archiving
- Configuration system
- Additional output targets
- Enhanced filtering mechanisms

## Project Structure

```
nanolog-rs/
├── benches/                 # Benchmark tests
│   ├── benchmarks.rs
│   └── logger_benchmark.rs
├── examples/                # Usage examples
│   ├── advanced_usage.rs
│   ├── basic_usage.rs
│   ├── builder_example.rs
│   └── modern_builder_example.rs
├── logs/                    # Log files (gitignored)
│   ├── composite.log
│   ├── example.log
│   ├── example_modern.log
│   └── performance.log
├── src/                     # Source code
│   ├── buffer.rs           # Buffer pool implementation
│   ├── builder.rs          # AsyncLoggerBuilder implementation
│   ├── error.rs            # Error handling
│   ├── format.rs           # Message formatting
│   ├── level.rs            # Log levels
│   ├── lib.rs              # Library entry point
│   ├── logger.rs           # Main logger implementation
│   ├── record.rs           # Log record structures
│   ├── sink.rs             # Output sinks
│   └── tests/              # Internal unit tests
├── tests/                   # Integration tests
│   └── builder_integration_test.rs
├── .gitignore              # Git ignore rules
├── Cargo.lock              # Dependency lock file
├── Cargo.toml              # Package manifest
├── LICENSE                 # License file
├── README.md               # English documentation
├── README_zh.md            # Chinese documentation
└── TODO.md                 # Development roadmap
```

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues for bugs and feature requests.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a pull request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by high-performance logging systems used in financial services
- Built with Rust's safety and performance guarantees