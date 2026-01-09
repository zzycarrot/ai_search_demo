// api/response.rs - 搜索响应类型
//! 定义搜索结果的结构化响应类型

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 搜索响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// 查询信息
    pub query: QueryInfo,
    
    /// 搜索结果
    pub results: Vec<SearchResult>,
    
    /// 总匹配数（不考虑分页）
    pub total: usize,
    
    /// 分页信息
    pub pagination: Pagination,
    
    /// 聚合统计（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregations: Option<Aggregations>,
    
    /// 搜索耗时（毫秒）
    pub took_ms: u64,
}

impl SearchResponse {
    pub fn new(query: QueryInfo, results: Vec<SearchResult>, total: usize) -> Self {
        Self {
            pagination: Pagination {
                offset: 0,
                limit: results.len(),
                has_more: total > results.len(),
            },
            query,
            results,
            total,
            aggregations: None,
            took_ms: 0,
        }
    }
    
    pub fn with_pagination(mut self, offset: usize, limit: usize) -> Self {
        self.pagination = Pagination {
            offset,
            limit,
            has_more: self.total > offset + limit,
        };
        self
    }
    
    pub fn with_took(mut self, took_ms: u64) -> Self {
        self.took_ms = took_ms;
        self
    }
    
    pub fn with_aggregations(mut self, aggregations: Aggregations) -> Self {
        self.aggregations = Some(aggregations);
        self
    }
}

/// 查询信息（用于调试和展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryInfo {
    /// 原始查询文本
    pub raw_query: String,
    
    /// 解析后的搜索文本（去除过滤语法后）
    pub search_text: String,
    
    /// AI 提取的关键词
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    
    /// 应用的过滤条件摘要
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub applied_filters: Vec<String>,
}

impl QueryInfo {
    pub fn new(raw_query: impl Into<String>, search_text: impl Into<String>) -> Self {
        Self {
            raw_query: raw_query.into(),
            search_text: search_text.into(),
            keywords: Vec::new(),
            applied_filters: Vec::new(),
        }
    }
    
    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = keywords;
        self
    }
    
    pub fn add_filter(mut self, filter: impl Into<String>) -> Self {
        self.applied_filters.push(filter.into());
        self
    }
}

/// 单个搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 文件路径
    pub path: String,
    
    /// 文件名
    pub filename: String,
    
    /// 父目录路径
    pub parent_path: String,
    
    /// 相关性得分
    pub score: f32,
    
    /// 标题
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    
    /// 高亮片段
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub highlights: Vec<Highlight>,
    
    /// 文件元数据
    pub metadata: FileMetadataResponse,
    
    /// 提取的标签/关键词
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl SearchResult {
    pub fn new(path: impl Into<String>, score: f32) -> Self {
        let path_str = path.into();
        let path_obj = std::path::Path::new(&path_str);
        
        Self {
            filename: path_obj.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string(),
            parent_path: path_obj.parent()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string(),
            path: path_str,
            score,
            title: None,
            highlights: Vec::new(),
            metadata: FileMetadataResponse::default(),
            tags: Vec::new(),
        }
    }
    
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
    
    pub fn with_highlights(mut self, highlights: Vec<Highlight>) -> Self {
        self.highlights = highlights;
        self
    }
    
    pub fn with_metadata(mut self, metadata: FileMetadataResponse) -> Self {
        self.metadata = metadata;
        self
    }
    
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

/// 高亮片段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight {
    /// 字段名称
    pub field: String,
    
    /// 高亮文本（包含标记）
    pub text: String,
    
    /// 匹配位置（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<HighlightPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightPosition {
    pub start: usize,
    pub end: usize,
}

/// 文件元数据响应
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileMetadataResponse {
    /// 文件类型/扩展名
    pub file_type: String,
    
    /// 文件大小（字节）
    pub file_size: u64,
    
    /// 文件大小（人类可读格式）
    pub file_size_display: String,
    
    /// 创建时间（Unix 时间戳）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_time: Option<u64>,
    
    /// 修改时间（Unix 时间戳）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_time: Option<u64>,
    
    /// 索引时间（Unix 时间戳）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_time: Option<u64>,
    
    /// 格式化的创建时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_time_display: Option<String>,
    
    /// 格式化的修改时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_time_display: Option<String>,
}

impl FileMetadataResponse {
    pub fn new(file_type: impl Into<String>, file_size: u64) -> Self {
        Self {
            file_type: file_type.into(),
            file_size,
            file_size_display: format_file_size(file_size),
            ..Default::default()
        }
    }
    
    pub fn with_times(mut self, created: Option<u64>, modified: Option<u64>, indexed: Option<u64>) -> Self {
        self.created_time = created;
        self.modified_time = modified;
        self.indexed_time = indexed;
        self.created_time_display = created.map(format_timestamp);
        self.modified_time_display = modified.map(format_timestamp);
        self
    }
}

/// 分页信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}

/// 聚合统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Aggregations {
    /// 按文件类型统计
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub by_type: HashMap<String, usize>,
    
    /// 按目录统计
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub by_directory: HashMap<String, usize>,
    
    /// 按时间范围统计（如：今天、本周、本月）
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub by_time: HashMap<String, usize>,
}

/// 索引响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResponse {
    /// 是否成功
    pub success: bool,
    
    /// 消息
    pub message: String,
    
    /// 索引的文件路径
    pub path: String,
    
    /// 耗时（毫秒）
    pub took_ms: u64,
}

/// 批量索引响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchIndexResponse {
    /// 成功数量
    pub success_count: usize,
    
    /// 失败数量
    pub failed_count: usize,
    
    /// 失败详情
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<IndexFailure>,
    
    /// 总耗时（毫秒）
    pub took_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexFailure {
    pub path: String,
    pub error: String,
}

/// 错误响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// 错误代码
    pub code: String,
    
    /// 错误消息
    pub message: String,
    
    /// 详细信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl ErrorResponse {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }
    
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

// 辅助函数
fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn format_timestamp(ts: u64) -> String {
    // 简单的时间格式化，实际可以使用 chrono 库
    use std::time::{Duration, UNIX_EPOCH};
    
    let datetime = UNIX_EPOCH + Duration::from_secs(ts);
    let now = std::time::SystemTime::now();
    
    if let Ok(elapsed) = now.duration_since(datetime) {
        let secs = elapsed.as_secs();
        
        if secs < 60 {
            "刚刚".to_string()
        } else if secs < 3600 {
            format!("{} 分钟前", secs / 60)
        } else if secs < 86400 {
            format!("{} 小时前", secs / 3600)
        } else if secs < 7 * 86400 {
            format!("{} 天前", secs / 86400)
        } else {
            // 简单返回时间戳，实际应使用 chrono 格式化
            format!("{}天前", secs / 86400)
        }
    } else {
        "未来".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(500), "500 B");
        assert_eq!(format_file_size(1024), "1.00 KB");
        assert_eq!(format_file_size(1536), "1.50 KB");
        assert_eq!(format_file_size(1048576), "1.00 MB");
        assert_eq!(format_file_size(1073741824), "1.00 GB");
    }
    
    #[test]
    fn test_search_result() {
        let result = SearchResult::new("/home/user/docs/report.pdf", 0.95)
            .with_title("Annual Report 2024")
            .with_metadata(FileMetadataResponse::new("pdf", 1024 * 1024));
        
        assert_eq!(result.filename, "report.pdf");
        assert_eq!(result.parent_path, "/home/user/docs");
        assert_eq!(result.title, Some("Annual Report 2024".to_string()));
    }
}
