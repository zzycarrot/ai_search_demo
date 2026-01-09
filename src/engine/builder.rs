// engine/builder.rs - 搜索引擎构建器
//! 使用 Builder 模式构建 SearchEngine

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use tantivy::{Index, ReloadPolicy, directory::MmapDirectory};

use crate::config::Config;
use crate::schema;
use crate::schema::fields::*;
use crate::ai::KeywordExtractor;
use crate::extract::TextExtractor;

use super::core::{SearchEngine, EngineError, EngineResult};

/// 搜索引擎构建器
pub struct SearchEngineBuilder {
    config: Option<Arc<Config>>,
    storage_path: Option<PathBuf>,
    heap_size: usize,
    enable_ai: bool,
    model_path: Option<PathBuf>,
}

impl Default for SearchEngineBuilder {
    fn default() -> Self {
        Self {
            config: None,
            storage_path: None,
            heap_size: 50_000_000, // 50MB
            enable_ai: true,
            model_path: None,
        }
    }
}

impl SearchEngineBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 设置配置
    pub fn with_config(mut self, config: Arc<Config>) -> Self {
        self.config = Some(config);
        self
    }
    
    /// 设置存储路径
    pub fn with_storage_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.storage_path = Some(path.into());
        self
    }
    
    /// 设置 IndexWriter 堆大小
    pub fn with_heap_size(mut self, size: usize) -> Self {
        self.heap_size = size;
        self
    }
    
    /// 是否启用 AI 功能
    pub fn with_ai(mut self, enable: bool) -> Self {
        self.enable_ai = enable;
        self
    }
    
    /// 设置模型路径
    pub fn with_model_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.model_path = Some(path.into());
        self
    }
    
    /// 构建搜索引擎
    pub fn build(self) -> EngineResult<SearchEngine> {
        // 获取或创建配置
        let config = self.config.unwrap_or_else(|| Arc::new(Config::global().clone()));
        
        // 确定存储路径
        let storage_path = self.storage_path
            .or_else(|| config.index_config.storage_path.as_ref().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("storage"));
        
        // 创建存储目录
        std::fs::create_dir_all(&storage_path)?;
        
        // 构建 Schema
        let schema = schema::build_schema();
        
        // 创建或打开索引
        let dir = MmapDirectory::open(&storage_path)
            .map_err(|e| EngineError::Directory(e.to_string()))?;
        let index = Index::open_or_create(dir, schema.clone())?;
        
        // 注册分词器
        schema::register_tokenizers(&index);
        
        // 创建 Writer
        let writer = index.writer(self.heap_size)?;
        let writer = Arc::new(RwLock::new(writer));
        
        // 创建 Reader
        let reader = index.reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        
        // 创建关键词提取器
        let keyword_extractor = if self.enable_ai {
            let model_path = self.model_path
                .or_else(|| config.ai_config.model_path.as_ref().map(PathBuf::from))
                .unwrap_or_else(|| PathBuf::from("model"));
            
            KeywordExtractor::new(&model_path).ok()
        } else {
            None
        };
        
        // 创建文本提取器
        let text_extractor = TextExtractor::new();
        
        // 获取字段
        let field_title = schema.get_field(FIELD_TITLE)
            .map_err(|_| EngineError::Config("Missing field: title".to_string()))?;
        let field_body = schema.get_field(FIELD_BODY)
            .map_err(|_| EngineError::Config("Missing field: body".to_string()))?;
        let field_tags = schema.get_field(FIELD_TAGS)
            .map_err(|_| EngineError::Config("Missing field: tags".to_string()))?;
        let field_path = schema.get_field(FIELD_PATH)
            .map_err(|_| EngineError::Config("Missing field: path".to_string()))?;
        let field_parent_path = schema.get_field(FIELD_PARENT_PATH)
            .map_err(|_| EngineError::Config("Missing field: parent_path".to_string()))?;
        let field_filename = schema.get_field(FIELD_FILENAME)
            .map_err(|_| EngineError::Config("Missing field: filename".to_string()))?;
        let field_file_type = schema.get_field(FIELD_FILE_TYPE)
            .map_err(|_| EngineError::Config("Missing field: file_type".to_string()))?;
        let field_file_size = schema.get_field(FIELD_FILE_SIZE)
            .map_err(|_| EngineError::Config("Missing field: file_size".to_string()))?;
        let field_modified_time = schema.get_field(FIELD_MODIFIED_TIME)
            .map_err(|_| EngineError::Config("Missing field: modified_time".to_string()))?;
        let field_created_time = schema.get_field(FIELD_CREATED_TIME)
            .map_err(|_| EngineError::Config("Missing field: created_time".to_string()))?;
        let field_indexed_time = schema.get_field(FIELD_INDEXED_TIME)
            .map_err(|_| EngineError::Config("Missing field: indexed_time".to_string()))?;
        
        Ok(SearchEngine {
            index,
            writer,
            reader,
            keyword_extractor,
            text_extractor,
            config,
            field_title,
            field_body,
            field_tags,
            field_path,
            field_parent_path,
            field_filename,
            field_file_type,
            field_file_size,
            field_modified_time,
            field_created_time,
            field_indexed_time,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_builder() {
        let temp_dir = tempdir().unwrap();
        
        let result = SearchEngineBuilder::new()
            .with_storage_path(temp_dir.path())
            .with_ai(false)
            .build();
        
        assert!(result.is_ok());
    }
}
