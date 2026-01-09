// engine/core.rs - 搜索引擎核心
//! 统一的搜索引擎接口和实现

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use tantivy::{
    Index, IndexWriter, IndexReader,
    TantivyDocument, Term,
    query::{Query, BooleanQuery, QueryParser as TantivyQueryParser, Occur},
    collector::TopDocs,
    schema::{Field, Value},
};

use crate::config::Config;
use crate::schema::{IndexDocument, FileMetadata};
use crate::query::{QueryParser, ParsedQuery};
use crate::query::filter::{FilterBuilder, PathMatcher};
use crate::api::{
    SearchRequest, SearchResponse, SearchResult, QueryInfo,
    FileMetadataResponse, Highlight, IndexResponse,
};
use crate::ai::KeywordExtractor;
use crate::extract::TextExtractor;

/// 搜索引擎错误类型
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("Tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Directory error: {0}")]
    Directory(String),
    
    #[error("Query parse error: {0}")]
    QueryParse(String),
    
    #[error("Index error: {0}")]
    Index(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
}

pub type EngineResult<T> = Result<T, EngineError>;

/// 搜索引擎
pub struct SearchEngine {
    pub(crate) index: Index,
    pub(crate) writer: Arc<RwLock<IndexWriter>>,
    pub(crate) reader: IndexReader,
    pub(crate) keyword_extractor: Option<KeywordExtractor>,
    pub(crate) text_extractor: TextExtractor,
    pub(crate) config: Arc<Config>,
    
    // 字段缓存
    pub(crate) field_title: Field,
    pub(crate) field_body: Field,
    pub(crate) field_tags: Field,
    pub(crate) field_path: Field,
    pub(crate) field_parent_path: Field,
    pub(crate) field_filename: Field,
    pub(crate) field_file_type: Field,
    pub(crate) field_file_size: Field,
    pub(crate) field_modified_time: Field,
    pub(crate) field_created_time: Field,
    pub(crate) field_indexed_time: Field,
}

impl SearchEngine {
    /// 获取 Index 引用
    pub fn index(&self) -> &Index {
        &self.index
    }
    
    /// 获取 IndexWriter 的共享引用
    pub fn writer(&self) -> &Arc<RwLock<IndexWriter>> {
        &self.writer
    }
    
    /// 获取 IndexReader 引用
    pub fn reader(&self) -> &IndexReader {
        &self.reader
    }
    
    /// 执行搜索
    pub fn search(&self, request: &SearchRequest) -> EngineResult<SearchResponse> {
        let start = Instant::now();
        
        // 解析查询
        let parsed = QueryParser::new()
            .parse(&request.query);
        
        // 构建查询信息
        let mut query_info = QueryInfo::new(&request.query, &parsed.text);
        
        // 提取关键词
        let keywords = if request.use_ai {
            if let Some(ref extractor) = self.keyword_extractor {
                extractor.extract(&parsed.text).unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        query_info = query_info.with_keywords(keywords.clone());
        
        // 构建 Tantivy 查询
        let searcher = self.reader.searcher();
        let schema = self.index.schema();
        
        // 文本搜索查询
        let text_query = self.build_text_query(&parsed, &keywords)?;
        
        // 过滤条件查询
        let filter_builder = FilterBuilder::new(&schema);
        let filter_query = filter_builder.build_filter_query(&parsed.filters);
        
        // 合并查询
        let final_query: Box<dyn Query> = if let Some(fq) = filter_query {
            let clauses = vec![
                (Occur::Must, text_query),
                (Occur::Must, fq),
            ];
            Box::new(BooleanQuery::new(clauses))
        } else {
            text_query
        };
        
        // 执行搜索
        let options = request.to_query_options();
        let top_docs = TopDocs::with_limit(options.limit + options.offset);
        
        let search_results = searcher.search(&final_query, &top_docs)?;
        
        // 路径过滤（后处理）
        let path_matcher = PathMatcher::new(&parsed.filters.paths);
        
        // 转换结果
        let mut results = Vec::new();
        for (score, doc_address) in search_results.iter().skip(options.offset) {
            let doc: tantivy::TantivyDocument = searcher.doc(*doc_address)?;
            
            // 获取路径并检查过滤
            let path = doc.get_first(self.field_path)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            if !path_matcher.matches(&path) {
                continue;
            }
            
            let result = self.doc_to_result(&doc, *score, &options)?;
            results.push(result);
            
            if results.len() >= options.limit {
                break;
            }
        }
        
        // 记录应用的过滤条件
        for filter in &parsed.filters.paths {
            let prefix = if filter.exclude { "排除: " } else { "路径: " };
            query_info = query_info.add_filter(format!("{}{}", prefix, filter.pattern));
        }
        if let Some(ref time) = parsed.filters.time {
            query_info = query_info.add_filter(format!("时间: {:?}", time.range));
        }
        if !parsed.filters.file_types.is_empty() {
            query_info = query_info.add_filter(format!("类型: {:?}", parsed.filters.file_types));
        }
        
        let took_ms = start.elapsed().as_millis() as u64;
        
        Ok(SearchResponse::new(query_info, results, search_results.len())
            .with_pagination(options.offset, options.limit)
            .with_took(took_ms))
    }
    
    /// 索引文件
    pub fn index_file(&self, path: &Path) -> EngineResult<IndexResponse> {
        let start = Instant::now();
        
        if !path.exists() {
            return Err(EngineError::FileNotFound(path.display().to_string()));
        }
        
        // 提取文本
        let content = self.text_extractor.extract(path)
            .map_err(|e| EngineError::Index(e.to_string()))?;
        
        // 创建文件元数据
        let metadata = FileMetadata::from_path(path)?;
        
        // 提取关键词作为标签
        let tags = if let Some(ref extractor) = self.keyword_extractor {
            extractor.extract(&content).unwrap_or_default()
        } else {
            Vec::new()
        };
        
        // 创建索引文档
        let title = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let index_doc = IndexDocument::new(title, content, tags, metadata);
        
        // 转换为 Tantivy 文档
        let doc = self.build_tantivy_doc(&index_doc);
        
        // 写入索引
        {
            let mut writer = self.writer.write().unwrap();
            
            // 先删除旧文档
            let term = Term::from_field_text(self.field_path, &index_doc.metadata.path);
            writer.delete_term(term);
            
            // 添加新文档
            writer.add_document(doc)?;
            writer.commit()?;
        }
        
        let took_ms = start.elapsed().as_millis() as u64;
        
        Ok(IndexResponse {
            success: true,
            message: "文件已索引".to_string(),
            path: index_doc.metadata.path,
            took_ms,
        })
    }
    
    /// 删除文件索引
    pub fn delete_file(&self, path: &Path) -> EngineResult<()> {
        let path_str = path.to_string_lossy().to_string();
        let term = Term::from_field_text(self.field_path, &path_str);
        
        let mut writer = self.writer.write().unwrap();
        writer.delete_term(term);
        writer.commit()?;
        
        Ok(())
    }
    
    /// 检查文件是否已索引
    pub fn is_indexed(&self, path: &Path) -> EngineResult<bool> {
        let path_str = path.to_string_lossy().to_string();
        let term = Term::from_field_text(self.field_path, &path_str);
        
        let searcher = self.reader.searcher();
        let query = tantivy::query::TermQuery::new(
            term,
            tantivy::schema::IndexRecordOption::Basic,
        );
        
        let count = searcher.search(&query, &tantivy::collector::Count)?;
        Ok(count > 0)
    }
    
    /// 获取索引统计
    pub fn stats(&self) -> EngineResult<EngineStats> {
        let searcher = self.reader.searcher();
        let num_docs = searcher.num_docs() as usize;
        
        Ok(EngineStats {
            total_documents: num_docs,
            index_size_bytes: self.estimate_index_size()?,
        })
    }
    
    // === 私有方法 ===
    
    fn build_text_query(&self, parsed: &ParsedQuery, keywords: &[String]) -> EngineResult<Box<dyn Query>> {
        // 创建查询解析器
        let query_parser = TantivyQueryParser::for_index(
            &self.index,
            vec![self.field_title, self.field_body, self.field_tags],
        );
        
        // 构建查询文本
        let query_text = if keywords.is_empty() {
            parsed.text.clone()
        } else {
            // 合并原始文本和关键词
            let kw_text = keywords.join(" ");
            if parsed.text.is_empty() {
                kw_text
            } else {
                format!("{} {}", parsed.text, kw_text)
            }
        };
        
        if query_text.is_empty() {
            // 返回匹配所有文档的查询
            return Ok(Box::new(tantivy::query::AllQuery));
        }
        
        query_parser.parse_query(&query_text)
            .map_err(|e| EngineError::QueryParse(e.to_string()))
    }
    
    fn doc_to_result(
        &self,
        doc: &tantivy::TantivyDocument,
        score: f32,
        options: &crate::query::types::QueryOptions,
    ) -> EngineResult<SearchResult> {
        let path = doc.get_first(self.field_path)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let title = doc.get_first(self.field_title)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let file_type = doc.get_first(self.field_file_type)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let file_size = doc.get_first(self.field_file_size)
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        
        let modified_time = doc.get_first(self.field_modified_time)
            .and_then(|v| v.as_u64());
        
        let created_time = doc.get_first(self.field_created_time)
            .and_then(|v| v.as_u64());
        
        let indexed_time = doc.get_first(self.field_indexed_time)
            .and_then(|v| v.as_u64());
        
        // 获取标签
        let tags: Vec<String> = doc.get_all(self.field_tags)
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        
        // 获取高亮（简化版本）
        let highlights = if options.highlight {
            self.extract_highlights(doc, options.preview_length)
        } else {
            Vec::new()
        };
        
        let metadata = FileMetadataResponse::new(&file_type, file_size)
            .with_times(created_time, modified_time, indexed_time);
        
        let mut result = SearchResult::new(path, score)
            .with_metadata(metadata)
            .with_tags(tags)
            .with_highlights(highlights);
        
        if let Some(t) = title {
            result = result.with_title(t);
        }
        
        Ok(result)
    }
    
    fn extract_highlights(&self, doc: &tantivy::TantivyDocument, max_len: usize) -> Vec<Highlight> {
        let mut highlights = Vec::new();
        
        // 简单提取 body 的前 N 个字符作为片段
        if let Some(body) = doc.get_first(self.field_body).and_then(|v| v.as_str()) {
            let snippet: String = body.chars().take(max_len).collect();
            highlights.push(Highlight {
                field: "body".to_string(),
                text: snippet,
                position: None,
            });
        }
        
        highlights
    }
    
    fn build_tantivy_doc(&self, doc: &IndexDocument) -> TantivyDocument {
        let mut tantivy_doc = TantivyDocument::new();
        
        // 标题
        tantivy_doc.add_text(self.field_title, &doc.title);
        
        // 内容
        tantivy_doc.add_text(self.field_body, &doc.content);
        
        // 标签
        for tag in &doc.tags {
            tantivy_doc.add_text(self.field_tags, tag);
        }
        
        // 路径相关
        tantivy_doc.add_text(self.field_path, &doc.metadata.path);
        tantivy_doc.add_text(self.field_parent_path, &doc.metadata.parent_path);
        tantivy_doc.add_text(self.field_filename, &doc.metadata.filename);
        tantivy_doc.add_text(self.field_file_type, &doc.metadata.file_type);
        
        // 数值字段
        tantivy_doc.add_u64(self.field_file_size, doc.metadata.file_size);
        tantivy_doc.add_u64(self.field_modified_time, doc.metadata.modified_time);
        tantivy_doc.add_u64(self.field_created_time, doc.metadata.created_time);
        tantivy_doc.add_u64(self.field_indexed_time, doc.metadata.indexed_time);
        
        tantivy_doc
    }
    
    fn estimate_index_size(&self) -> EngineResult<u64> {
        let storage_path = self.config.index_config.storage_path.as_ref()
            .map(|p| PathBuf::from(p))
            .unwrap_or_else(|| PathBuf::from("storage"));
        
        let mut total_size = 0u64;
        
        if storage_path.exists() {
            for entry in std::fs::read_dir(&storage_path)? {
                let entry = entry?;
                if entry.path().is_file() {
                    total_size += entry.metadata()?.len();
                }
            }
        }
        
        Ok(total_size)
    }
}

/// 引擎统计信息
#[derive(Debug, Clone)]
pub struct EngineStats {
    pub total_documents: usize,
    pub index_size_bytes: u64,
}
