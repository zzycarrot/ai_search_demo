// api/request.rs - 搜索请求类型
//! 定义搜索请求的结构化类型

use serde::{Deserialize, Serialize};
use crate::query::{QueryOptions, SortBy};

/// 搜索请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// 原始查询文本（包含过滤语法）
    pub query: String,
    
    /// 返回结果数量限制
    #[serde(default = "default_limit")]
    pub limit: usize,
    
    /// 跳过的结果数量（用于分页）
    #[serde(default)]
    pub offset: usize,
    
    /// 是否返回高亮片段
    #[serde(default = "default_true")]
    pub highlight: bool,
    
    /// 高亮片段的最大长度
    #[serde(default = "default_snippet_length")]
    pub snippet_length: usize,
    
    /// 是否使用 AI 提取关键词
    #[serde(default = "default_true")]
    pub use_ai: bool,
    
    /// 额外的过滤条件（可选，也可以通过 query 语法指定）
    #[serde(default)]
    pub filters: Option<QueryFiltersRequest>,
}

fn default_limit() -> usize { 20 }
fn default_true() -> bool { true }
fn default_snippet_length() -> usize { 200 }

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            limit: default_limit(),
            offset: 0,
            highlight: true,
            snippet_length: default_snippet_length(),
            use_ai: true,
            filters: None,
        }
    }
}

impl SearchRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            ..Default::default()
        }
    }
    
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
    
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }
    
    pub fn with_filters(mut self, filters: QueryFiltersRequest) -> Self {
        self.filters = Some(filters);
        self
    }
    
    /// 转换为内部 QueryOptions
    pub fn to_query_options(&self) -> QueryOptions {
        QueryOptions {
            limit: self.limit,
            offset: self.offset,
            sort_by: self.filters.as_ref()
                .and_then(|f| f.sort_by.clone())
                .unwrap_or_default(),
            highlight: self.highlight,
            preview_length: self.snippet_length,
        }
    }
}

/// 过滤条件请求（JSON 格式）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryFiltersRequest {
    /// 路径过滤（支持 glob 模式）
    #[serde(default)]
    pub paths: Vec<PathFilterRequest>,
    
    /// 文件类型过滤（扩展名，如 "pdf", "txt"）
    #[serde(default)]
    pub file_types: Vec<String>,
    
    /// 时间过滤
    #[serde(default)]
    pub time: Option<TimeFilterRequest>,
    
    /// 文件大小过滤（字节）
    #[serde(default)]
    pub size: Option<SizeFilterRequest>,
    
    /// 标签过滤
    #[serde(default)]
    pub tags: Vec<String>,
    
    /// 排序方式
    #[serde(default)]
    pub sort_by: Option<SortBy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathFilterRequest {
    pub pattern: String,
    #[serde(default)]
    pub exclude: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TimeFilterRequest {
    /// 最近 N 小时
    #[serde(rename = "hours")]
    LastHours { hours: u32 },
    
    /// 最近 N 天
    #[serde(rename = "days")]
    LastDays { days: u32 },
    
    /// 最近 N 周
    #[serde(rename = "weeks")]
    LastWeeks { weeks: u32 },
    
    /// 时间戳范围
    #[serde(rename = "range")]
    Range { 
        #[serde(default)]
        after: Option<u64>,
        #[serde(default)]
        before: Option<u64>,
    },
    
    /// 今天
    #[serde(rename = "today")]
    Today,
    
    /// 本周
    #[serde(rename = "this_week")]
    ThisWeek,
    
    /// 本月
    #[serde(rename = "this_month")]
    ThisMonth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SizeFilterRequest {
    /// 大于
    #[serde(rename = "gt")]
    GreaterThan { bytes: u64 },
    
    /// 小于
    #[serde(rename = "lt")]
    LessThan { bytes: u64 },
    
    /// 范围
    #[serde(rename = "range")]
    Between { min: u64, max: u64 },
}

/// 索引请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexRequest {
    /// 要索引的文件路径
    pub path: String,
    
    /// 是否强制重新索引
    #[serde(default)]
    pub force: bool,
}

/// 删除请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequest {
    /// 要删除的文件路径
    pub path: String,
}

/// 批量索引请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchIndexRequest {
    /// 要索引的文件路径列表
    pub paths: Vec<String>,
    
    /// 是否强制重新索引
    #[serde(default)]
    pub force: bool,
}
