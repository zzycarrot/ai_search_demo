// query/types.rs - 查询相关类型定义
//! 定义查询请求和过滤条件的结构

use serde::{Serialize, Deserialize};

/// 解析后的查询请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedQuery {
    /// 自然语言查询文本（AI 处理后的核心关键词）
    pub text: String,
    /// 原始查询文本
    pub raw_text: String,
    /// 过滤条件
    pub filters: QueryFilters,
}

/// 查询过滤条件
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryFilters {
    /// 路径过滤（支持通配符）
    /// 例如: ["/root/usr*", "/bin/*"]
    pub paths: Vec<PathFilter>,
    
    /// 时间过滤
    pub time: Option<TimeFilter>,
    
    /// 文件类型过滤
    /// 例如: ["pdf", "md"]
    pub file_types: Vec<String>,
    
    /// 文件大小过滤
    pub size: Option<SizeFilter>,
    
    /// 标签过滤
    pub tags: Vec<String>,
}

/// 路径过滤条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathFilter {
    /// 路径模式（支持 glob 通配符）
    pub pattern: String,
    /// 是否排除（true 表示排除匹配的路径）
    pub exclude: bool,
}

impl PathFilter {
    pub fn include(pattern: impl Into<String>) -> Self {
        Self { pattern: pattern.into(), exclude: false }
    }
    
    pub fn exclude(pattern: impl Into<String>) -> Self {
        Self { pattern: pattern.into(), exclude: true }
    }
}

/// 时间过滤条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeFilter {
    /// 时间字段类型
    pub field: TimeField,
    /// 时间范围
    pub range: TimeRange,
}

/// 时间字段类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TimeField {
    /// 创建时间
    Created,
    /// 修改时间
    Modified,
    /// 索引时间
    Indexed,
}

impl Default for TimeField {
    fn default() -> Self {
        Self::Modified
    }
}

/// 时间范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeRange {
    /// 过去 N 天
    LastDays(u32),
    /// 过去 N 小时
    LastHours(u32),
    /// 过去 N 周
    LastWeeks(u32),
    /// 过去 N 月
    LastMonths(u32),
    /// 精确范围 (开始时间戳, 结束时间戳)
    Between(u64, u64),
    /// 某天之后
    After(u64),
    /// 某天之前
    Before(u64),
    /// 今天
    Today,
    /// 本周
    ThisWeek,
    /// 本月
    ThisMonth,
}

/// 文件大小过滤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SizeFilter {
    /// 大于 N 字节
    GreaterThan(u64),
    /// 小于 N 字节
    LessThan(u64),
    /// 范围 (min, max)
    Between(u64, u64),
}

/// 排序方式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SortBy {
    /// 按相关度（默认）
    Relevance,
    /// 按修改时间（最新优先）
    ModifiedDesc,
    /// 按修改时间（最旧优先）
    ModifiedAsc,
    /// 按创建时间（最新优先）
    CreatedDesc,
    /// 按文件大小（最大优先）
    SizeDesc,
    /// 按文件名
    Name,
}

impl Default for SortBy {
    fn default() -> Self {
        Self::Relevance
    }
}

/// 查询选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOptions {
    /// 排序方式
    pub sort_by: SortBy,
    /// 返回数量限制
    pub limit: usize,
    /// 偏移量（分页）
    pub offset: usize,
    /// 是否高亮匹配内容
    pub highlight: bool,
    /// 内容预览长度
    pub preview_length: usize,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            sort_by: SortBy::Relevance,
            limit: 10,
            offset: 0,
            highlight: true,
            preview_length: 200,
        }
    }
}
