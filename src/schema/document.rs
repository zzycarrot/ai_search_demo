// schema/document.rs - 文档结构定义
//! 定义索引文档的结构化表示

use std::path::Path;
use std::time::SystemTime;
use serde::{Serialize, Deserialize};

/// 索引文档 - 完整的文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDocument {
    /// 文件标题（通常是文件名，不含扩展名）
    pub title: String,
    /// 文件内容
    pub content: String,
    /// AI 生成的标签
    pub tags: Vec<String>,
    /// 文件元数据
    pub metadata: FileMetadata,
}

/// 文件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// 完整文件路径
    pub path: String,
    /// 父目录路径
    pub parent_path: String,
    /// 文件名（含扩展名）
    pub filename: String,
    /// 文件类型/扩展名 (不含点号)
    pub file_type: String,
    /// 文件大小（字节）
    pub file_size: u64,
    /// 修改时间（Unix 时间戳秒）
    pub modified_time: u64,
    /// 创建时间（Unix 时间戳秒）
    pub created_time: u64,
    /// 索引时间（Unix 时间戳秒）
    pub indexed_time: u64,
}

impl FileMetadata {
    /// 从文件路径提取元数据
    pub fn from_path(path: &Path) -> std::io::Result<Self> {
        let metadata = std::fs::metadata(path)?;
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        
        let path_str = canonical_path.to_string_lossy().to_string();
        let parent_path = canonical_path.parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let filename = canonical_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let file_type = canonical_path.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        
        let file_size = metadata.len();
        
        let modified_time = metadata.modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // macOS/Linux 支持 created()，Windows 也支持
        let created_time = metadata.created()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let indexed_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Ok(Self {
            path: path_str,
            parent_path,
            filename,
            file_type,
            file_size,
            modified_time,
            created_time,
            indexed_time,
        })
    }
}

impl IndexDocument {
    /// 创建新的索引文档
    pub fn new(title: String, content: String, tags: Vec<String>, metadata: FileMetadata) -> Self {
        Self { title, content, tags, metadata }
    }
    
    /// 获取标签字符串（空格分隔）
    pub fn tags_string(&self) -> String {
        self.tags.join(" ")
    }
}

// ============== 兼容旧代码 ==============

/// 旧版文件文档结构（保持向后兼容）
#[derive(Debug)]
pub struct FileDoc {
    pub title: String,
    pub content: String,
    pub path: String,
}

impl From<IndexDocument> for FileDoc {
    fn from(doc: IndexDocument) -> Self {
        Self {
            title: doc.title,
            content: doc.content,
            path: doc.metadata.path,
        }
    }
}

impl From<&IndexDocument> for FileDoc {
    fn from(doc: &IndexDocument) -> Self {
        Self {
            title: doc.title.clone(),
            content: doc.content.clone(),
            path: doc.metadata.path.clone(),
        }
    }
}
