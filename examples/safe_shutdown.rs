use nanolog_rs::{AsyncLoggerBuilder, ConsoleSink, DefaultFormatter, Level, global_logger, init_global_logger, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<(), nanolog_rs::error::Error> {
    let logger = AsyncLoggerBuilder::new()
        .level(Level::Trace)
        .with_file_output("logs/macro.log")
        .with_console_output()
        .build()?;
    init_global_logger(Arc::new(logger))?;

    nanolog_rs::info!(target: "init", "service started");
    if let Some(gl) = global_logger() {
        gl.flush()?;
        gl.shutdown()?;
    }
    Ok(())
}
