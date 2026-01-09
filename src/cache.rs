// cache.rs - Embedding 缓存模块
// 使用 sled 存储 embedding 向量，避免重复计算

use anyhow::Result;
use sled::Db;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

/// Embedding 缓存管理器
pub struct EmbeddingCache {
    db: Db,
}

/// 缓存条目：包含内容哈希和 embedding 向量
#[derive(serde::Serialize, serde::Deserialize)]
struct CacheEntry {
    content_hash: u64,      // 内容的哈希值，用于检测内容变化
    keywords: Vec<String>,  // 缓存的关键词结果
}

impl EmbeddingCache {
    /// 创建或打开缓存数据库
    pub fn new(cache_path: &Path) -> Result<Self> {
        let db = sled::open(cache_path)?;
        println!(" [Cache] Embedding 缓存已加载: {:?}", cache_path);
        Ok(Self { db })
    }

    /// 计算内容的哈希值
    fn hash_content(content: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// 尝试从缓存获取关键词
    /// 如果内容没变（哈希匹配），返回缓存的关键词
    /// 如果内容变了或不存在，返回 None
    pub fn get_keywords(&self, file_path: &str, content: &str) -> Option<Vec<String>> {
        let current_hash = Self::hash_content(content);
        
        if let Ok(Some(data)) = self.db.get(file_path.as_bytes()) {
            if let Ok(entry) = bincode::deserialize::<CacheEntry>(&data) {
                if entry.content_hash == current_hash {
                    return Some(entry.keywords);
                }
            }
        }
        None
    }

    /// 存储关键词到缓存
    pub fn set_keywords(&self, file_path: &str, content: &str, keywords: Vec<String>) -> Result<()> {
        let entry = CacheEntry {
            content_hash: Self::hash_content(content),
            keywords,
        };
        
        let data = bincode::serialize(&entry)?;
        self.db.insert(file_path.as_bytes(), data)?;
        self.db.flush()?;
        Ok(())
    }

    /// 从缓存中删除指定文件的条目
    pub fn remove(&self, file_path: &str) -> Result<()> {
        self.db.remove(file_path.as_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> (usize, u64) {
        let count = self.db.len();
        let size = self.db.size_on_disk().unwrap_or(0);
        (count, size)
    }
}
