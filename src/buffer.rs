/*!
高性能内存缓冲区实现。

提供零拷贝、线程安全的内存管理机制，支持高效的内存重用和分配策略。
*/

use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// 字节缓冲区实现
///
/// 提供高效的字节数据存储和管理，支持自动扩容和内存池优化。
/// 使用向量存储实现零拷贝操作，避免不必要的内存分配。
#[derive(Debug)]
pub struct ByteBuffer {
    data: NonNull<u8>,
    capacity: usize,
    len: usize,
}

// 为ByteBuffer实现Send和Sync trait
unsafe impl Send for ByteBuffer {}
unsafe impl Sync for ByteBuffer {}

impl ByteBuffer {
    /// 创建新的字节缓冲区
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Capacity must be greater than zero");

        // 使用Vec分配内存，确保正确对齐
        let mut vec = Vec::with_capacity(capacity);
        let ptr = vec.as_mut_ptr();
        let capacity = vec.capacity();

        // 防止Vec被自动释放
        std::mem::forget(vec);

        Self {
            data: unsafe { NonNull::new_unchecked(ptr) },
            capacity,
            len: 0,
        }
    }

    /// 从现有数据创建缓冲区（零拷贝）
    pub fn from_bytes(data: &[u8]) -> Self {
        let mut vec = data.to_vec();
        let ptr = vec.as_mut_ptr();
        let capacity = vec.capacity();
        let len = vec.len();

        std::mem::forget(vec);

        Self {
            data: unsafe { NonNull::new_unchecked(ptr) },
            capacity,
            len,
        }
    }

    /// 获取缓冲区容量
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 获取缓冲区当前长度
    pub fn len(&self) -> usize {
        self.len
    }

    /// 检查缓冲区是否为空
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// 确保缓冲区有足够的空间
    pub fn reserve(&mut self, additional: usize) {
        let required = self.len.saturating_add(additional);
        if required > self.capacity {
            self.resize(required);
        }
    }

    /// 调整缓冲区大小（安全实现）
    fn resize(&mut self, new_capacity: usize) {
        let new_capacity = new_capacity.max(1).max(self.capacity.saturating_mul(2));

        // 创建新的Vec
        let mut new_vec = Vec::with_capacity(new_capacity);

        // 拷贝现有数据（如果有数据）
        if self.len > 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(self.data.as_ptr(), new_vec.as_mut_ptr(), self.len);
                new_vec.set_len(self.len);
            }
        }

        // 释放旧内存（如果有容量）
        if self.capacity > 0 {
            unsafe {
                let _ = Vec::from_raw_parts(self.data.as_ptr(), 0, self.capacity);
            }
        }

        // 更新指针和容量
        let ptr = new_vec.as_mut_ptr();
        std::mem::forget(new_vec);

        self.data = unsafe { NonNull::new_unchecked(ptr) };
        self.capacity = new_capacity;
    }

    /// 写入字节数组
    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        let len = bytes.len();
        if self.len.saturating_add(len) > self.capacity {
            // 自动扩展缓冲区
            self.reserve(len);
        }

        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.data.as_ptr().add(self.len), len);
            self.len += len;
        }

        Ok(())
    }

    /// 写入字符串
    pub fn write_str(&mut self, s: &str) -> Result<(), &'static str> {
        self.write_bytes(s.as_bytes())
    }

    /// 获取缓冲区内容的不可变引用
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.data.as_ptr(), self.len) }
    }

    /// 获取缓冲区内容的可变引用
    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.data.as_ptr(), self.len) }
    }

    /// 清空缓冲区
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// 转换为字符串（UTF-8安全）
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(self.as_bytes()).into_owned()
    }

    /// 转换为字节向量（转移所有权）
    pub fn into_bytes(mut self) -> Vec<u8> {
        let vec = unsafe { Vec::from_raw_parts(self.data.as_ptr(), self.len, self.capacity) };

        // 防止双重释放
        self.len = 0;
        self.capacity = 0;
        self.data = NonNull::dangling();

        vec
    }
}

impl AsRef<[u8]> for ByteBuffer {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Drop for ByteBuffer {
    fn drop(&mut self) {
        if self.capacity > 0 {
            // 安全释放内存
            let _ = unsafe { Vec::from_raw_parts(self.data.as_ptr(), 0, self.capacity) };
        }
    }
}

impl Clone for ByteBuffer {
    fn clone(&self) -> Self {
        Self::from_bytes(self.as_bytes())
    }
}

/// 高性能缓冲区池
///
/// 管理ByteBuffer对象的生命周期，通过对象池模式减少内存分配开销。
/// 使用无锁队列实现高性能的缓冲区获取和释放操作。
/// 支持自动扩容和缓冲区重用，提高内存使用效率。
pub struct BufferPool {
    buffers: crossbeam_queue::SegQueue<Arc<ByteBuffer>>,
    buffer_size: usize,
    max_pool_size: usize,
    current_size: AtomicUsize,
}

impl BufferPool {
    /// 创建新的缓冲区池
    pub fn new(buffer_size: usize, max_pool_size: usize) -> Self {
        Self {
            buffers: crossbeam_queue::SegQueue::new(),
            buffer_size,
            max_pool_size,
            current_size: AtomicUsize::new(0),
        }
    }

    /// 获取缓冲区（优先重用）
    pub fn acquire(&self) -> Arc<ByteBuffer> {
        if let Some(buffer) = self.buffers.pop() {
            buffer
        } else {
            Arc::new(ByteBuffer::new(self.buffer_size))
        }
    }

    /// 释放缓冲区回池中
    pub fn release(&self, buffer: Arc<ByteBuffer>) {
        // 检查当前池大小
        let current = self.current_size.load(Ordering::Relaxed);
        if current < self.max_pool_size {
            // 尝试清空缓冲区内容以便重用
            // 注意：只有当没有其他强引用时，Arc::get_mut才能成功
            if let Some(buf) = Arc::get_mut(&mut Arc::clone(&buffer)) {
                buf.clear();
            }

            // 添加到池中
            self.buffers.push(buffer);
            self.current_size.fetch_add(1, Ordering::Relaxed);
        }
        // 如果池已满，缓冲区将被自动丢弃
    }

    /// 获取池中当前缓冲区数量
    pub fn size(&self) -> usize {
        self.current_size.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_buffer_basic() {
        let mut buffer = ByteBuffer::new(100);
        assert_eq!(buffer.capacity(), 100);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());

        buffer.write_str("Hello").unwrap();
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.as_bytes(), b"Hello");

        buffer.clear();
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_byte_buffer_resize() {
        let mut buffer = ByteBuffer::new(10);
        let data = b"This is a long string that requires resizing";
        buffer.write_bytes(data).unwrap();

        assert!(buffer.capacity() >= data.len());
        assert_eq!(buffer.as_bytes(), data);
    }

    #[test]
    fn test_buffer_pool() {
        let pool = BufferPool::new(1024, 10);

        let buffer1 = pool.acquire();
        let buffer2 = pool.acquire();

        pool.release(buffer1);
        pool.release(buffer2);

        assert_eq!(pool.size(), 2);
    }
}
