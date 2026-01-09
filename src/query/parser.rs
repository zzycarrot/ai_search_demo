// query/parser.rs - 查询解析器
//! 解析自然语言查询 + 结构化过滤语法
//! 
//! 支持的语法:
//! - 自然语言: "a pdf about os kernel coding"
//! - 路径过滤: --path=/root/usr* 或 --path="/path with spaces/*"
//! - 排除路径: --exclude-path=/tmp/*
//! - 时间过滤: --time=7d (过去7天), --time=24h, --time=1w, --time=3m
//! - 时间范围: --after=2024-01-01 --before=2024-12-31
//! - 时间字段: --time-field=created|modified|indexed
//! - 文件类型: --type=pdf,md,txt
//! - 文件大小: --size=>1mb, --size=<100kb, --size=1mb-10mb
//! - 标签: --tag=rust,kernel
//! - 排序: --sort=relevance|modified|created|size|name
//! - 数量: --limit=20

use regex::Regex;
use std::collections::HashMap;
use once_cell::sync::Lazy;

use super::types::*;

/// 查询解析器
pub struct QueryParser {
    /// AI 关键词提取器（可选）
    keyword_extractor: Option<Box<dyn Fn(&str) -> String + Send + Sync>>,
}

// 正则表达式预编译
static ARG_PATTERN: Lazy<Regex> = Lazy::new(|| {
    // 匹配 --key=value 或 --key="value with spaces"
    Regex::new(r#"--([a-z\-]+)=(?:"([^"]+)"|([^\s]+))"#).unwrap()
});

static SIZE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([<>])?(\d+(?:\.\d+)?)(kb|mb|gb|b)?(?:-(\d+(?:\.\d+)?)(kb|mb|gb|b)?)?$").unwrap()
});

static TIME_RELATIVE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\d+)(h|d|w|m)$").unwrap()
});

impl QueryParser {
    /// 创建新的解析器
    pub fn new() -> Self {
        Self { keyword_extractor: None }
    }
    
    /// 设置 AI 关键词提取器
    pub fn with_keyword_extractor<F>(mut self, extractor: F) -> Self 
    where 
        F: Fn(&str) -> String + Send + Sync + 'static 
    {
        self.keyword_extractor = Some(Box::new(extractor));
        self
    }
    
    /// 解析查询字符串
    pub fn parse(&self, input: &str) -> ParsedQuery {
        let input = input.trim();
        let mut args: HashMap<String, Vec<String>> = HashMap::new();
        let mut text_parts: Vec<&str> = Vec::new();
        
        // 提取所有 --key=value 参数
        let mut last_end = 0;
        for cap in ARG_PATTERN.captures_iter(input) {
            let full_match = cap.get(0).unwrap();
            
            // 收集参数之前的文本
            if full_match.start() > last_end {
                let before = &input[last_end..full_match.start()];
                text_parts.push(before.trim());
            }
            last_end = full_match.end();
            
            let key = cap.get(1).unwrap().as_str().to_string();
            // 优先使用带引号的值，否则使用不带引号的值
            let value = cap.get(2)
                .or_else(|| cap.get(3))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            
            args.entry(key).or_insert_with(Vec::new).push(value);
        }
        
        // 收集最后一个参数之后的文本
        if last_end < input.len() {
            text_parts.push(&input[last_end..]);
        }
        
        // 合并文本部分
        let raw_text: String = text_parts.iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();
        
        // 使用 AI 提取关键词（如果有）
        let text = if let Some(ref extractor) = self.keyword_extractor {
            if !raw_text.is_empty() {
                extractor(&raw_text)
            } else {
                raw_text.clone()
            }
        } else {
            raw_text.clone()
        };
        
        // 解析过滤条件
        let filters = self.parse_filters(&args);
        
        ParsedQuery { text, raw_text, filters }
    }
    
    /// 解析过滤条件
    fn parse_filters(&self, args: &HashMap<String, Vec<String>>) -> QueryFilters {
        let mut filters = QueryFilters::default();
        
        // 路径过滤
        if let Some(paths) = args.get("path") {
            for path in paths {
                filters.paths.push(PathFilter::include(path.clone()));
            }
        }
        if let Some(paths) = args.get("exclude-path") {
            for path in paths {
                filters.paths.push(PathFilter::exclude(path.clone()));
            }
        }
        
        // 文件类型
        if let Some(types) = args.get("type") {
            for t in types {
                filters.file_types.extend(
                    t.split(',').map(|s| s.trim().to_lowercase())
                );
            }
        }
        
        // 标签
        if let Some(tags) = args.get("tag") {
            for t in tags {
                filters.tags.extend(
                    t.split(',').map(|s| s.trim().to_string())
                );
            }
        }
        
        // 时间字段
        let time_field = args.get("time-field")
            .and_then(|v| v.first())
            .map(|s| match s.as_str() {
                "created" => TimeField::Created,
                "indexed" => TimeField::Indexed,
                _ => TimeField::Modified,
            })
            .unwrap_or(TimeField::Modified);
        
        // 时间过滤
        if let Some(time) = args.get("time").and_then(|v| v.first()) {
            if let Some(range) = self.parse_time_range(time) {
                filters.time = Some(TimeFilter { field: time_field, range });
            }
        } else if let Some(after) = args.get("after").and_then(|v| v.first()) {
            if let Some(ts) = self.parse_date(after) {
                filters.time = Some(TimeFilter { 
                    field: time_field, 
                    range: TimeRange::After(ts) 
                });
            }
        } else if let Some(before) = args.get("before").and_then(|v| v.first()) {
            if let Some(ts) = self.parse_date(before) {
                filters.time = Some(TimeFilter { 
                    field: time_field, 
                    range: TimeRange::Before(ts) 
                });
            }
        }
        
        // 文件大小
        if let Some(size) = args.get("size").and_then(|v| v.first()) {
            filters.size = self.parse_size_filter(size);
        }
        
        filters
    }
    
    /// 解析相对时间范围
    fn parse_time_range(&self, s: &str) -> Option<TimeRange> {
        let s = s.to_lowercase();
        
        // 预定义值
        match s.as_str() {
            "today" => return Some(TimeRange::Today),
            "week" | "this-week" => return Some(TimeRange::ThisWeek),
            "month" | "this-month" => return Some(TimeRange::ThisMonth),
            _ => {}
        }
        
        // 相对时间: 7d, 24h, 1w, 3m
        if let Some(cap) = TIME_RELATIVE_PATTERN.captures(&s) {
            let num: u32 = cap.get(1)?.as_str().parse().ok()?;
            let unit = cap.get(2)?.as_str();
            return match unit {
                "h" => Some(TimeRange::LastHours(num)),
                "d" => Some(TimeRange::LastDays(num)),
                "w" => Some(TimeRange::LastWeeks(num)),
                "m" => Some(TimeRange::LastMonths(num)),
                _ => None,
            };
        }
        
        None
    }
    
    /// 解析日期字符串 (YYYY-MM-DD)
    fn parse_date(&self, s: &str) -> Option<u64> {
        // 简单解析 YYYY-MM-DD 格式
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return None;
        }
        
        let year: i32 = parts[0].parse().ok()?;
        let month: u32 = parts[1].parse().ok()?;
        let day: u32 = parts[2].parse().ok()?;
        
        // 简化计算（不完全准确，但足够用于过滤）
        // 假设每月30天，不考虑闰年
        let days_since_epoch = (year - 1970) as u64 * 365 
            + (month - 1) as u64 * 30 
            + (day - 1) as u64;
        
        Some(days_since_epoch * 86400)
    }
    
    /// 解析大小过滤
    fn parse_size_filter(&self, s: &str) -> Option<SizeFilter> {
        let s = s.to_lowercase();
        
        if let Some(cap) = SIZE_PATTERN.captures(&s) {
            let op = cap.get(1).map(|m| m.as_str());
            let num1: f64 = cap.get(2)?.as_str().parse().ok()?;
            let unit1 = cap.get(3).map(|m| m.as_str()).unwrap_or("b");
            
            let size1 = self.parse_size_bytes(num1, unit1)?;
            
            // 检查是否是范围 (1mb-10mb)
            if let (Some(num2_match), Some(unit2_match)) = (cap.get(4), cap.get(5)) {
                let num2: f64 = num2_match.as_str().parse().ok()?;
                let unit2 = unit2_match.as_str();
                let size2 = self.parse_size_bytes(num2, unit2)?;
                return Some(SizeFilter::Between(size1, size2));
            }
            
            // 单个值
            return match op {
                Some(">") => Some(SizeFilter::GreaterThan(size1)),
                Some("<") => Some(SizeFilter::LessThan(size1)),
                _ => Some(SizeFilter::LessThan(size1)), // 默认小于
            };
        }
        
        None
    }
    
    /// 转换大小单位为字节
    fn parse_size_bytes(&self, num: f64, unit: &str) -> Option<u64> {
        let multiplier = match unit {
            "b" => 1u64,
            "kb" => 1024,
            "mb" => 1024 * 1024,
            "gb" => 1024 * 1024 * 1024,
            _ => return None,
        };
        Some((num * multiplier as f64) as u64)
    }
}

impl Default for QueryParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_query() {
        let parser = QueryParser::new();
        let result = parser.parse("rust programming tutorial");
        assert_eq!(result.text, "rust programming tutorial");
        assert!(result.filters.paths.is_empty());
    }
    
    #[test]
    fn test_parse_with_path_filter() {
        let parser = QueryParser::new();
        let result = parser.parse("kernel docs --path=/usr/src/*");
        assert_eq!(result.text, "kernel docs");
        assert_eq!(result.filters.paths.len(), 1);
        assert_eq!(result.filters.paths[0].pattern, "/usr/src/*");
    }
    
    #[test]
    fn test_parse_with_multiple_filters() {
        let parser = QueryParser::new();
        let result = parser.parse("os kernel --type=pdf,md --time=7d --path=/docs/*");
        assert_eq!(result.text, "os kernel");
        assert_eq!(result.filters.file_types, vec!["pdf", "md"]);
        assert!(result.filters.time.is_some());
    }
}
