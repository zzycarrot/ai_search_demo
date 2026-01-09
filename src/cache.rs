// cache.rs - Embedding 缓存模块
// 使用 sled 存储 embedding 向量，避免重复计算

use anyhow::Result;
use sled::Db;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::SystemTime;

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

/// 文件元数据缓存条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileMetaEntry {
    /// 文件大小（字节）
    pub file_size: u64,
    /// 最后修改时间（Unix 时间戳秒）
    pub mtime: u64,
    /// 是否已索引
    pub indexed: bool,
}

impl FileMetaEntry {
    /// 从文件系统读取元数据创建条目
    pub fn from_path(path: &Path) -> Result<Self> {
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len();
        let mtime = metadata.modified()
            .unwrap_or(SystemTime::now())
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Ok(Self {
            file_size,
            mtime,
            indexed: false,
        })
    }
    
    /// 检查文件是否需要重新索引
    /// 比对当前文件元数据与缓存的元数据
    pub fn needs_reindex(&self, current: &FileMetaEntry) -> bool {
        // 比对1: 文件大小变化 -> 内容肯定变了
        if self.file_size != current.file_size {
            return true;
        }
        // 比对2: 修改时间变新 -> 内容大概率变了
        if current.mtime > self.mtime {
            return true;
        }
        false
    }
}

/// 文件状态检查结果
#[derive(Debug)]
pub enum FileStatus {
    /// 新文件，需要索引
    New,
    /// 文件已修改，需要重新索引
    Modified,
    /// 文件未变化，跳过
    Unchanged,
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
    
    // ============== 文件元数据缓存 ==============
    
    /// 元数据缓存的 key 前缀
    const META_PREFIX: &'static str = "meta:";
    
    /// 生成元数据缓存的 key
    fn meta_key(file_path: &str) -> Vec<u8> {
        format!("{}{}", Self::META_PREFIX, file_path).into_bytes()
    }
    
    /// 检查文件状态（基于元数据，不读取文件内容）
    /// 
    /// 比对规则：
    /// 1. 文件大小变化 -> 内容肯定变了 -> 重新索引
    /// 2. 修改时间变新 -> 内容大概率变了 -> 重新索引
    pub fn check_file_status(&self, file_path: &str, path: &Path) -> FileStatus {
        // 读取当前文件元数据
        let current = match FileMetaEntry::from_path(path) {
            Ok(meta) => meta,
            Err(_) => return FileStatus::New, // 无法读取元数据，当作新文件
        };
        
        // 查找缓存中的元数据
        let key = Self::meta_key(file_path);
        match self.db.get(&key) {
            Ok(Some(data)) => {
                match bincode::deserialize::<FileMetaEntry>(&data) {
                    Ok(cached) => {
                        if cached.needs_reindex(&current) {
                            FileStatus::Modified
                        } else {
                            FileStatus::Unchanged
                        }
                    }
                    Err(_) => FileStatus::New, // 反序列化失败，当作新文件
                }
            }
            Ok(None) => FileStatus::New, // 缓存中不存在
            Err(_) => FileStatus::New,   // 读取失败，当作新文件
        }
    }
    
    /// 保存文件元数据到缓存
    pub fn save_file_meta(&self, file_path: &str, path: &Path) -> Result<()> {
        let mut meta = FileMetaEntry::from_path(path)?;
        meta.indexed = true;
        
        let key = Self::meta_key(file_path);
        let data = bincode::serialize(&meta)?;
        self.db.insert(key, data)?;
        self.db.flush()?;
        Ok(())
    }
    
    /// 获取缓存的文件元数据
    pub fn get_file_meta(&self, file_path: &str) -> Option<FileMetaEntry> {
        let key = Self::meta_key(file_path);
        self.db.get(&key).ok()?.and_then(|data| {
            bincode::deserialize::<FileMetaEntry>(&data).ok()
        })
    }
    
    /// 删除文件元数据缓存
    pub fn remove_file_meta(&self, file_path: &str) -> Result<()> {
        let key = Self::meta_key(file_path);
        self.db.remove(key)?;
        self.db.flush()?;
        Ok(())
    }
    
    /// 获取所有已缓存的文件路径
    pub fn get_all_cached_paths(&self) -> Vec<String> {
        let prefix = Self::META_PREFIX.as_bytes();
        self.db.scan_prefix(prefix)
            .filter_map(|result| {
                result.ok().and_then(|(key, _)| {
                    String::from_utf8(key.to_vec()).ok()
                        .map(|s| s.strip_prefix(Self::META_PREFIX).unwrap_or(&s).to_string())
                })
            })
            .collect()
    }
    
    /// 获取元数据缓存统计
    pub fn meta_stats(&self) -> usize {
        let prefix = Self::META_PREFIX.as_bytes();
        self.db.scan_prefix(prefix).count()
    }
}
