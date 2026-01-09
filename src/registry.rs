// registry.rs - 文件注册表，用于协调扫描和监听之间的同步
// 解决"先监听后扫描"带来的重复处理问题

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// 文件状态
#[derive(Debug, Clone)]
pub struct FileState {
    /// 文件最后修改时间
    pub modified_time: SystemTime,
    /// 最后处理时间（用于判断是否需要重新处理）
    pub processed_time: SystemTime,
    /// 是否正在处理中（防止并发处理）
    pub processing: bool,
}

/// 文件注册表 - 线程安全的文件状态管理
#[derive(Clone)]
pub struct FileRegistry {
    inner: Arc<RwLock<RegistryInner>>,
}

struct RegistryInner {
    /// 文件路径 -> 文件状态
    files: HashMap<PathBuf, FileState>,
    /// 扫描是否完成
    scan_completed: bool,
    /// 扫描期间收集的待处理事件
    pending_events: Vec<PendingEvent>,
}

#[derive(Debug, Clone)]
pub struct PendingEvent {
    pub path: PathBuf,
    pub event_type: EventType,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    Create,
    Modify,
    Delete,
}

impl FileRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RegistryInner {
                files: HashMap::new(),
                scan_completed: false,
                pending_events: Vec::new(),
            })),
        }
    }

    /// 尝试开始处理文件（原子操作）
    /// 返回 true 表示可以处理，false 表示应该跳过
    pub fn try_start_processing(&self, path: &PathBuf, file_mod_time: SystemTime) -> bool {
        let mut inner = self.inner.write().unwrap();
        
        if let Some(state) = inner.files.get_mut(path) {
            // 如果正在处理中，跳过
            if state.processing {
                return false;
            }
            // 如果文件修改时间没变，跳过
            if state.modified_time >= file_mod_time {
                return false;
            }
            // 标记为正在处理
            state.processing = true;
            state.modified_time = file_mod_time;
            true
        } else {
            // 新文件，添加到注册表并标记为处理中
            inner.files.insert(path.clone(), FileState {
                modified_time: file_mod_time,
                processed_time: SystemTime::now(),
                processing: true,
            });
            true
        }
    }

    /// 完成处理文件
    pub fn finish_processing(&self, path: &PathBuf) {
        let mut inner = self.inner.write().unwrap();
        if let Some(state) = inner.files.get_mut(path) {
            state.processing = false;
            state.processed_time = SystemTime::now();
        }
    }

    /// 标记文件已删除
    pub fn mark_deleted(&self, path: &PathBuf) {
        let mut inner = self.inner.write().unwrap();
        inner.files.remove(path);
    }

    /// 添加待处理事件（扫描期间使用）
    pub fn add_pending_event(&self, path: PathBuf, event_type: EventType) {
        let mut inner = self.inner.write().unwrap();
        // 只在扫描未完成时收集事件
        if !inner.scan_completed {
            inner.pending_events.push(PendingEvent {
                path,
                event_type,
                timestamp: SystemTime::now(),
            });
        }
    }

    /// 标记扫描完成，返回待处理的事件
    pub fn complete_scan(&self) -> Vec<PendingEvent> {
        let mut inner = self.inner.write().unwrap();
        inner.scan_completed = true;
        std::mem::take(&mut inner.pending_events)
    }

    /// 检查扫描是否完成
    pub fn is_scan_completed(&self) -> bool {
        let inner = self.inner.read().unwrap();
        inner.scan_completed
    }

    /// 检查文件是否已被处理（用于去重）
    pub fn is_file_processed(&self, path: &PathBuf, file_mod_time: SystemTime) -> bool {
        let inner = self.inner.read().unwrap();
        if let Some(state) = inner.files.get(path) {
            // 如果注册表中的修改时间 >= 当前文件修改时间，说明已处理
            state.modified_time >= file_mod_time
        } else {
            false
        }
    }

    /// 获取统计信息
    pub fn stats(&self) -> (usize, usize) {
        let inner = self.inner.read().unwrap();
        let processing_count = inner.files.values().filter(|s| s.processing).count();
        (inner.files.len(), processing_count)
    }
}

impl Default for FileRegistry {
    fn default() -> Self {
        Self::new()
    }
}
