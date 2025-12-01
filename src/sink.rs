/*!
高性能日志输出目标。

简化设计，专注于零拷贝和低延迟输出。
*/

use std::fs::{File, OpenOptions};
use std::io::{self, Write, BufWriter};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// 高性能输出目标接口
#[async_trait::async_trait]
pub trait Sink: Send + Sync {
    /// 写入日志数据（高性能版本）
    async fn write(&self, data: &[u8]) -> io::Result<()>;
    
    /// 批量写入日志数据
    async fn write_batch(&self, data: &[Vec<u8>]) -> io::Result<()>;
    
    /// 刷新输出缓冲区
    async fn flush(&self) -> io::Result<()>;
    
    /// 关闭输出目标
    async fn shutdown(&self) -> io::Result<()>;
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

#[async_trait::async_trait]
impl Sink for ConsoleSink {
    async fn write(&self, data: &[u8]) -> io::Result<()> {
        if self.stderr {
            io::stderr().write_all(data)?;
        } else {
            io::stdout().write_all(data)?;
        }
        Ok(())
    }
    
    async fn write_batch(&self, data: &[Vec<u8>]) -> io::Result<()> {
        for item in data {
            if self.stderr {
                io::stderr().write_all(item)?;
            } else {
                io::stdout().write_all(item)?;
            }
        }
        Ok(())
    }
    
    async fn flush(&self) -> io::Result<()> {
        if self.stderr {
            io::stderr().flush()?;
        } else {
            io::stdout().flush()?;
        }
        Ok(())
    }
    
    async fn shutdown(&self) -> io::Result<()> {
        Ok(())
    }
}

/// 文件输出目标（高性能版本）
pub struct FileSink {
    /// 文件写入器（使用缓冲写入器提高性能）
    writer: Arc<Mutex<BufWriter<File>>>,
}

impl FileSink {
    /// 创建新的文件输出目标
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path = path.as_ref();
        
        // 确保父目录存在
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)?;
        }
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        
        Ok(Self {
            writer: Arc::new(Mutex::new(BufWriter::new(file))),
        })
    }
    
    /// 创建带缓冲区大小的文件输出目标
    pub fn with_buffer_size<P: AsRef<Path>>(path: P, buffer_size: usize) -> io::Result<Self> {
        let path = path.as_ref();
        
        // 确保父目录存在
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)?;
        }
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        
        Ok(Self {
            writer: Arc::new(Mutex::new(BufWriter::with_capacity(buffer_size, file))),
        })
    }
}

#[async_trait::async_trait]
impl Sink for FileSink {
    async fn write(&self, data: &[u8]) -> io::Result<()> {
        let mut writer = self.writer.lock().map_err(|_| io::Error::other("lock poisoned"))?;
        writer.write_all(data)?;
        Ok(())
    }
    
    async fn write_batch(&self, data: &[Vec<u8>]) -> io::Result<()> {
        let mut writer = self.writer.lock().map_err(|_| io::Error::other("lock poisoned"))?;
        for item in data {
            writer.write_all(item)?;
        }
        Ok(())
    }
    
    async fn flush(&self) -> io::Result<()> {
        let mut writer = self.writer.lock().map_err(|_| io::Error::other("lock poisoned"))?;
        writer.flush()?;
        Ok(())
    }
    
    async fn shutdown(&self) -> io::Result<()> {
        let mut writer = self.writer.lock().map_err(|_| io::Error::other("lock poisoned"))?;
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
        self.buffer.lock().unwrap_or_else(|e| e.into_inner()).clone()
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

#[async_trait::async_trait]
impl Sink for MemorySink {
    async fn write(&self, data: &[u8]) -> io::Result<()> {
        let mut buffer = self.buffer.lock().map_err(|_| io::Error::other("lock poisoned"))?;
        buffer.extend_from_slice(data);
        Ok(())
    }
    
    async fn write_batch(&self, data: &[Vec<u8>]) -> io::Result<()> {
        let mut buffer = self.buffer.lock().map_err(|_| io::Error::other("lock poisoned"))?;
        for item in data {
            buffer.extend_from_slice(item);
        }
        Ok(())
    }
    
    async fn flush(&self) -> io::Result<()> {
        // 内存输出目标不需要刷新
        Ok(())
    }
    
    async fn shutdown(&self) -> io::Result<()> {
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

#[async_trait::async_trait]
impl Sink for NullSink {
    async fn write(&self, _data: &[u8]) -> io::Result<()> {
        // 不执行任何操作
        Ok(())
    }
    
    async fn write_batch(&self, _data: &[Vec<u8>]) -> io::Result<()> {
        // 不执行任何操作
        Ok(())
    }
    
    async fn flush(&self) -> io::Result<()> {
        // 不执行任何操作
        Ok(())
    }
    
    async fn shutdown(&self) -> io::Result<()> {
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

#[async_trait::async_trait]
impl Sink for CompositeSink {
    async fn write(&self, data: &[u8]) -> io::Result<()> {
        for sink in &self.sinks {
            sink.write(data).await?;
        }
        Ok(())
    }
    
    async fn write_batch(&self, data: &[Vec<u8>]) -> io::Result<()> {
        for sink in &self.sinks {
            sink.write_batch(data).await?;
        }
        Ok(())
    }
    
    async fn flush(&self) -> io::Result<()> {
        for sink in &self.sinks {
            sink.flush().await?;
        }
        Ok(())
    }
    
    async fn shutdown(&self) -> io::Result<()> {
        for sink in &self.sinks {
            sink.shutdown().await?;
        }
        Ok(())
    }
}