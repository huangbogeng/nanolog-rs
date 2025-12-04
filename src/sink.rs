/*!
高性能日志输出目标。

简化设计，专注于零拷贝和低延迟输出。
*/

use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// 高性能输出目标接口
pub trait Sink: Send + Sync {
    /// 写入日志数据（高性能版本）
    fn write(&self, data: &[u8]) -> io::Result<()>;

    /// 批量写入日志数据
    fn write_batch(&self, data: &[Vec<u8>]) -> io::Result<()>;

    /// 刷新输出缓冲区
    fn flush(&self) -> io::Result<()>;

    /// 关闭输出目标
    fn shutdown(&self) -> io::Result<()>;
}

/// 控制台输出目标
pub struct ConsoleSink {
    /// 是否使用标准错误输出
    stderr: bool,
}

impl ConsoleSink {
    /// 创建新的控制台输出目标
    pub fn new() -> Self {
        Self { stderr: false }
    }

    /// 创建使用标准错误输出的控制台输出目标
    pub fn stderr() -> Self {
        Self { stderr: true }
    }
}

impl Default for ConsoleSink {
    fn default() -> Self {
        Self::new()
    }
}

impl Sink for ConsoleSink {
    fn write(&self, data: &[u8]) -> io::Result<()> {
        if self.stderr {
            io::stderr().write_all(data)?;
        } else {
            io::stdout().write_all(data)?;
        }
        Ok(())
    }

    fn write_batch(&self, data: &[Vec<u8>]) -> io::Result<()> {
        for item in data {
            if self.stderr {
                io::stderr().write_all(item)?;
            } else {
                io::stdout().write_all(item)?;
            }
        }
        Ok(())
    }

    fn flush(&self) -> io::Result<()> {
        if self.stderr {
            io::stderr().flush()?;
        } else {
            io::stdout().flush()?;
        }
        Ok(())
    }

    fn shutdown(&self) -> io::Result<()> {
        Ok(())
    }
}

/// 文件输出目标（高性能版本）
pub struct FileSink {
    /// 文件路径
    path: std::path::PathBuf,
    /// 文件写入器（使用缓冲写入器提高性能）
    writer: Arc<Mutex<BufWriter<File>>>,
    /// 当前文件大小
    current_size: Arc<std::sync::atomic::AtomicUsize>,
    /// 最大文件大小（字节）
    max_size: Option<usize>,
    /// 轮转时间间隔（秒）
    rotate_interval: Option<u64>,
    /// 最后轮转时间
    last_rotate: Arc<std::sync::atomic::AtomicU64>,
    /// 保留的日志文件数量
    max_files: Option<usize>,
}

impl FileSink {
    /// 创建新的文件输出目标
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path = path.as_ref().to_owned();

        // 确保父目录存在
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let file_size = file.metadata()?.len() as usize;

        Ok(Self {
            path,
            writer: Arc::new(Mutex::new(BufWriter::new(file))),
            current_size: Arc::new(std::sync::atomic::AtomicUsize::new(file_size)),
            max_size: None,
            rotate_interval: None,
            last_rotate: Arc::new(std::sync::atomic::AtomicU64::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            )),
            max_files: None,
        })
    }

    /// 创建带缓冲区大小的文件输出目标
    pub fn with_buffer_size<P: AsRef<Path>>(path: P, buffer_size: usize) -> io::Result<Self> {
        let path = path.as_ref().to_owned();

        // 确保父目录存在
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let file_size = file.metadata()?.len() as usize;

        Ok(Self {
            path,
            writer: Arc::new(Mutex::new(BufWriter::with_capacity(buffer_size, file))),
            current_size: Arc::new(std::sync::atomic::AtomicUsize::new(file_size)),
            max_size: None,
            rotate_interval: None,
            last_rotate: Arc::new(std::sync::atomic::AtomicU64::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            )),
            max_files: None,
        })
    }

    /// 设置最大文件大小（字节）
    pub fn with_max_size(mut self, max_size: usize) -> Self {
        self.max_size = Some(max_size);
        self
    }

    /// 设置轮转时间间隔（秒）
    pub fn with_rotate_interval(mut self, interval: u64) -> Self {
        self.rotate_interval = Some(interval);
        self
    }

    /// 设置保留的日志文件数量
    pub fn with_max_files(mut self, max_files: usize) -> Self {
        self.max_files = Some(max_files);
        self
    }

    /// 检查是否需要轮转
    fn should_rotate(&self) -> bool {
        // 检查文件大小
        if let Some(max_size) = self.max_size {
            if self.current_size.load(std::sync::atomic::Ordering::Relaxed) >= max_size {
                return true;
            }
        }

        // 检查时间间隔
        if let Some(interval) = self.rotate_interval {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let last = self.last_rotate.load(std::sync::atomic::Ordering::Relaxed);
            if now - last >= interval {
                return true;
            }
        }

        false
    }

    /// 执行日志轮转
    fn rotate(&self) -> io::Result<()> {
        let mut writer_guard = self
            .writer
            .lock()
            .map_err(|_| io::Error::other("lock poisoned"))?;

        // 刷新并关闭当前文件
        writer_guard.flush()?;
        drop(writer_guard);

        // 生成轮转文件名
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let rotated_path = format!("{}.{}", self.path.to_string_lossy(), timestamp);

        // 重命名当前文件
        std::fs::rename(&self.path, &rotated_path)?;

        // 创建新的日志文件
        let new_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        // 更新写入器
        let mut writer_guard = self
            .writer
            .lock()
            .map_err(|_| io::Error::other("lock poisoned"))?;
        *writer_guard = BufWriter::new(new_file);

        // 重置文件大小
        self.current_size
            .store(0, std::sync::atomic::Ordering::Relaxed);

        // 更新最后轮转时间
        self.last_rotate
            .store(timestamp, std::sync::atomic::Ordering::Relaxed);

        // 清理旧日志文件
        self.cleanup_old_files()?;

        Ok(())
    }

    /// 清理旧日志文件
    fn cleanup_old_files(&self) -> io::Result<()> {
        if let Some(max_files) = self.max_files {
            // 获取所有日志文件
            let mut files = Vec::new();
            let file_name = self.path.file_name().unwrap_or_default().to_string_lossy();

            if let Some(parent) = self.path.parent() {
                for entry in std::fs::read_dir(parent)? {
                    let entry = entry?;
                    let path = entry.path();

                    if path.is_file() {
                        let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
                        if file_stem.starts_with(&*file_name) {
                            let metadata = path.metadata()?;
                            let mtime = metadata.modified()?;
                            files.push((mtime, path));
                        }
                    }
                }
            }

            // 按修改时间排序
            files.sort_by(|a, b| b.0.cmp(&a.0));

            // 删除多余的文件
            for file in files.into_iter().skip(max_files) {
                let _ = std::fs::remove_file(file.1);
            }
        }

        Ok(())
    }
}

impl Sink for FileSink {
    fn write(&self, data: &[u8]) -> io::Result<()> {
        // 检查是否需要轮转
        if self.should_rotate() {
            self.rotate()?;
        }

        let mut writer = self
            .writer
            .lock()
            .map_err(|_| io::Error::other("lock poisoned"))?;
        writer.write_all(data)?;

        // 更新文件大小
        self.current_size
            .fetch_add(data.len(), std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }

    fn write_batch(&self, data: &[Vec<u8>]) -> io::Result<()> {
        // 检查是否需要轮转
        if self.should_rotate() {
            self.rotate()?;
        }

        let mut writer = self
            .writer
            .lock()
            .map_err(|_| io::Error::other("lock poisoned"))?;

        let mut total_size = 0;
        for item in data {
            writer.write_all(item)?;
            total_size += item.len();
        }

        // 更新文件大小
        self.current_size
            .fetch_add(total_size, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }

    fn flush(&self) -> io::Result<()> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| io::Error::other("lock poisoned"))?;
        writer.flush()?;
        Ok(())
    }

    fn shutdown(&self) -> io::Result<()> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| io::Error::other("lock poisoned"))?;
        writer.flush()?;
        Ok(())
    }
}

/// 内存输出目标（用于测试和调试）
pub struct MemorySink {
    /// 内存缓冲区
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl MemorySink {
    /// 创建新的内存输出目标
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 获取缓冲区内容
    pub fn get_content(&self) -> Vec<u8> {
        self.buffer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// 清空缓冲区
    pub fn clear(&self) {
        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.clear();
        }
    }
}

impl Default for MemorySink {
    fn default() -> Self {
        Self::new()
    }
}

impl Sink for MemorySink {
    fn write(&self, data: &[u8]) -> io::Result<()> {
        let mut buffer = self
            .buffer
            .lock()
            .map_err(|_| io::Error::other("lock poisoned"))?;
        buffer.extend_from_slice(data);
        Ok(())
    }

    fn write_batch(&self, data: &[Vec<u8>]) -> io::Result<()> {
        let mut buffer = self
            .buffer
            .lock()
            .map_err(|_| io::Error::other("lock poisoned"))?;
        for item in data {
            buffer.extend_from_slice(item);
        }
        Ok(())
    }

    fn flush(&self) -> io::Result<()> {
        // 内存输出目标不需要刷新
        Ok(())
    }

    fn shutdown(&self) -> io::Result<()> {
        Ok(())
    }
}

/// 空输出目标（用于性能测试）
#[derive(Default)]
pub struct NullSink;

impl NullSink {
    /// 创建新的空输出目标
    pub fn new() -> Self {
        Self
    }
}

impl Sink for NullSink {
    fn write(&self, _data: &[u8]) -> io::Result<()> {
        // 不执行任何操作
        Ok(())
    }

    fn write_batch(&self, _data: &[Vec<u8>]) -> io::Result<()> {
        // 不执行任何操作
        Ok(())
    }

    fn flush(&self) -> io::Result<()> {
        // 不执行任何操作
        Ok(())
    }

    fn shutdown(&self) -> io::Result<()> {
        // 不执行任何操作
        Ok(())
    }
}

/// 复合输出目标（支持多个输出目标）
#[derive(Default)]
pub struct CompositeSink {
    /// 输出目标列表
    sinks: Vec<Arc<dyn Sink>>,
}

impl CompositeSink {
    /// 创建新的复合输出目标
    pub fn new() -> Self {
        Self { sinks: Vec::new() }
    }

    /// 添加输出目标
    pub fn add_sink(&mut self, sink: Arc<dyn Sink>) {
        self.sinks.push(sink);
    }
}

impl Sink for CompositeSink {
    fn write(&self, data: &[u8]) -> io::Result<()> {
        for sink in &self.sinks {
            sink.write(data)?;
        }
        Ok(())
    }

    fn write_batch(&self, data: &[Vec<u8>]) -> io::Result<()> {
        for sink in &self.sinks {
            sink.write_batch(data)?;
        }
        Ok(())
    }

    fn flush(&self) -> io::Result<()> {
        for sink in &self.sinks {
            sink.flush()?;
        }
        Ok(())
    }

    fn shutdown(&self) -> io::Result<()> {
        for sink in &self.sinks {
            sink.shutdown()?;
        }
        Ok(())
    }
}
