use criterion::{Criterion, criterion_group, criterion_main};
use nanolog_rs::buffer::BufferPool;
use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;

fn benchmark_buffer_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("Buffer Pool");
    group.measurement_time(Duration::from_secs(10)); // 增加测量时间以获得更稳定的结果
    group.sample_size(100); // 增加样本数量以减少波动

    group.bench_function("acquire_release", |b| {
        let pool = BufferPool::new(100, 1000);
        b.iter(|| {
            // 获取并归还缓冲区
            let buffer = black_box(pool.acquire());
            pool.release(buffer);
            black_box(());
        });
    });

    group.bench_function("buffer_write", |b| {
        let pool = BufferPool::new(100, 1000);
        b.iter(|| {
            let buffer = black_box(pool.acquire());
            if let Some(buffer) = Arc::get_mut(&mut buffer.clone()) {
                let _ = buffer.write_str(black_box("This is a test log message with some data"));
                let _ = buffer.write_str(&format!(" {}", black_box(42)));
            }
            pool.release(buffer);
            black_box(());
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_buffer_pool);
criterion_main!(benches);
