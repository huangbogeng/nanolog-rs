//! 性能基准测试
use criterion::{Criterion, criterion_group, criterion_main};
use nanolog_rs::{
    AsyncLogger, DefaultFormatter, Formatter, Level, MemorySink, Record,
    buffer::{BufferPool, ByteBuffer},
};
use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;

/// 测试ByteBuffer的性能
fn bench_byte_buffer(c: &mut Criterion) {
    let mut group = c.benchmark_group("byte_buffer");
    group.measurement_time(Duration::from_secs(5)); // 设置最大测量时间为5秒
    group.sample_size(10); // 减少样本数量以加快测试

    group.bench_function("new", |b| {
        b.iter(|| {
            let buffer = ByteBuffer::new(1024);
            black_box(buffer);
        });
    });

    group.bench_function("write_bytes", |b| {
        let data = vec![0u8; 100];
        b.iter(|| {
            let mut buffer = ByteBuffer::new(1024);
            let _ = buffer.write_bytes(&data);
            black_box(buffer);
        });
    });

    group.bench_function("reserve", |b| {
        b.iter(|| {
            let mut buffer = ByteBuffer::new(100);
            buffer.reserve(1000);
            black_box(buffer);
        });
    });

    group.finish();
}

/// 测试BufferPool的性能
fn bench_buffer_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_pool");
    group.measurement_time(Duration::from_secs(5)); // 设置最大测量时间为5秒
    group.sample_size(10); // 减少样本数量以加快测试

    group.bench_function("acquire_release", |b| {
        let pool = BufferPool::new(100, 1000);
        b.iter(|| {
            let buffer = pool.acquire();
            pool.release(buffer);
        });
    });

    group.bench_function("concurrent_acquire", |b| {
        let pool = Arc::new(BufferPool::new(100, 1000));
        b.iter(|| {
            let pool1 = pool.clone();
            let pool2 = pool.clone();

            let handle1 = std::thread::spawn(move || {
                let buffer = pool1.acquire();
                pool1.release(buffer);
            });

            let handle2 = std::thread::spawn(move || {
                let buffer = pool2.acquire();
                pool2.release(buffer);
            });

            let _ = handle1.join();
            let _ = handle2.join();
        });
    });

    group.finish();
}

/// 测试日志记录的性能
fn bench_logging(c: &mut Criterion) {
    let mut group = c.benchmark_group("logging");
    group.measurement_time(Duration::from_secs(5)); // 设置最大测量时间为5秒
    group.sample_size(10); // 减少样本数量以加快测试

    group.bench_function("record_creation", |b| {
        b.iter(|| {
            let record = Record::new(
                Level::Info,
                "benchmark",
                file!(),
                line!(),
                "This is a benchmark log message".to_string(),
            );
            black_box(record);
        });
    });

    group.bench_function("single_log", |b| {
        let logger = AsyncLogger::new(
            Level::Info,
            Arc::new(DefaultFormatter::new()),
            Arc::new(MemorySink::new()),
            1000,
            10,
            Duration::from_millis(100),
        );

        b.iter(|| {
            let record = Record::new(
                Level::Info,
                "benchmark",
                file!(),
                line!(),
                "Benchmark log message".to_string(),
            );
            let _ = logger.log(record);
        });

        // 关闭日志器以释放资源
        let logger = logger;
        if let Ok(runtime) = tokio::runtime::Runtime::new() {
            runtime.block_on(async {
                let _ = logger.shutdown().await;
            });
        }
    });

    group.bench_function("batch_logging", |b| {
        let logger = AsyncLogger::new(
            Level::Info,
            Arc::new(DefaultFormatter::new()),
            Arc::new(MemorySink::new()),
            10000,
            100,
            Duration::from_millis(10),
        );

        b.iter(|| {
            for i in 0..100 {
                let record = Record::new(
                    Level::Info,
                    "benchmark",
                    file!(),
                    line!(),
                    format!("Log message {}", i),
                );
                let _ = logger.log(record);
            }
        });

        // 关闭日志器以释放资源
        let logger = logger;
        if let Ok(runtime) = tokio::runtime::Runtime::new() {
            runtime.block_on(async {
                let _ = logger.shutdown().await;
            });
        }
    });

    group.finish();
}

/// 测试格式化性能
fn bench_formatting(c: &mut Criterion) {
    let mut group = c.benchmark_group("formatting");
    group.measurement_time(Duration::from_secs(5)); // 设置最大测量时间为5秒
    group.sample_size(10); // 减少样本数量以加快测试

    let formatter = DefaultFormatter::new();
    let record = Record::new(
        Level::Info,
        "benchmark",
        file!(),
        line!(),
        "This is a test log message for benchmarking".to_string(),
    );

    group.bench_function("format_record", |b| {
        b.iter(|| {
            let formatted = formatter.format(&record);
            let _ = black_box(formatted);
        });
    });

    group.bench_function("format_batch", |b| {
        let records: Vec<Record> = (0..10)
            .map(|i| {
                Record::new(
                    Level::Info,
                    "benchmark",
                    file!(),
                    line!(),
                    format!("Log message {}", i),
                )
            })
            .collect();

        b.iter(|| {
            for record in &records {
                let formatted = formatter.format(record);
                let _ = black_box(formatted);
            }
        });
    });

    group.finish();
}

/// 测试并发性能
fn bench_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent");
    group.measurement_time(Duration::from_secs(5)); // 设置最大测量时间为5秒
    group.sample_size(10); // 减少样本数量以加快测试

    group.bench_function("multi_thread_logging", |b| {
        let logger = Arc::new(AsyncLogger::new(
            Level::Info,
            Arc::new(DefaultFormatter::new()),
            Arc::new(MemorySink::new()),
            10000,
            100,
            Duration::from_millis(10),
        ));

        b.iter(|| {
            let mut handles = vec![];

            for _ in 0..4 {
                let logger = logger.clone();
                let handle = std::thread::spawn(move || {
                    for i in 0..25 {
                        let record = Record::new(
                            Level::Info,
                            "benchmark",
                            file!(),
                            line!(),
                            format!("Thread log {}", i),
                        );
                        let _ = logger.log(record);
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                let _ = handle.join();
            }
        });

        // 关闭日志器以释放资源
        if let Ok(logger) = Arc::try_unwrap(logger) {
            if let Ok(runtime) = tokio::runtime::Runtime::new() {
                runtime.block_on(async {
                    let _ = logger.shutdown().await;
                });
            }
        }
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_byte_buffer,
    bench_buffer_pool,
    bench_logging,
    bench_formatting,
    bench_concurrent
);
criterion_main!(benches);
