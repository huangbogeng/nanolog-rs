use nanolog_rs::{init_global_logger, AsyncLoggerBuilder, Level};
use nanolog_rs::sink::MemorySink;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_macros_with_memory_sink() {
    let mem_sink = Arc::new(MemorySink::new());

    let logger = AsyncLoggerBuilder::new()
        .level(Level::Trace)
        .with_simple_formatting()
        .sink(mem_sink.clone())
        .queue_capacity(1024)
        .batch_size(64)
        .flush_interval(Duration::from_millis(20))
        .build()
        .expect("build logger");

    let logger = Arc::new(logger);
    let _ = init_global_logger(logger.clone());

    nanolog_rs::error!("e1");
    nanolog_rs::warn!("w1");
    nanolog_rs::info!("i1");
    nanolog_rs::debug!("d1");
    nanolog_rs::trace!("t1");

    let _ = logger.flush();

    let content = mem_sink.get_content();
    let s = String::from_utf8_lossy(&content);
    assert!(s.contains("[ERROR] e1"));
    assert!(s.contains("[WARN] w1"));
    assert!(s.contains("[INFO] i1"));
    assert!(s.contains("[DEBUG] d1"));
    assert!(s.contains("[TRACE] t1"));
}
