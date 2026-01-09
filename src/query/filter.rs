// query/filter.rs - 过滤条件构建器
//! 将解析后的过滤条件转换为 Tantivy 查询

use glob::Pattern;
use std::ops::Bound;
use std::time::{SystemTime, UNIX_EPOCH};
use tantivy::query::{Query, BooleanQuery, RangeQuery, TermQuery, Occur};
use tantivy::schema::{Schema, IndexRecordOption};
use tantivy::Term;

use super::types::*;
use crate::schema::fields::*;

/// 过滤器构建器
pub struct FilterBuilder<'a> {
    schema: &'a Schema,
}

impl<'a> FilterBuilder<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self { schema }
    }
    
    /// 构建完整的过滤查询
    pub fn build_filter_query(&self, filters: &QueryFilters) -> Option<Box<dyn Query>> {
        let mut clauses: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        
        // 时间过滤
        if let Some(ref time_filter) = filters.time {
            if let Some(query) = self.build_time_query(time_filter) {
                clauses.push((Occur::Must, query));
            }
        }
        
        // 文件类型过滤
        if !filters.file_types.is_empty() {
            if let Some(query) = self.build_type_query(&filters.file_types) {
                clauses.push((Occur::Must, query));
            }
        }
        
        // 文件大小过滤
        if let Some(ref size_filter) = filters.size {
            if let Some(query) = self.build_size_query(size_filter) {
                clauses.push((Occur::Must, query));
            }
        }
        
        // 标签过滤（使用 Should，匹配任一标签）
        if !filters.tags.is_empty() {
            if let Some(query) = self.build_tags_query(&filters.tags) {
                clauses.push((Occur::Must, query));
            }
        }
        
        if clauses.is_empty() {
            None
        } else {
            Some(Box::new(BooleanQuery::new(clauses)))
        }
    }
    
    /// 构建时间范围查询
    fn build_time_query(&self, filter: &TimeFilter) -> Option<Box<dyn Query>> {
        let field_name = match filter.field {
            TimeField::Created => FIELD_CREATED_TIME,
            TimeField::Modified => FIELD_MODIFIED_TIME,
            TimeField::Indexed => FIELD_INDEXED_TIME,
        };
        
        let field = self.schema.get_field(field_name).ok()?;
        let (start, end) = self.calculate_time_range(&filter.range)?;
        
        // 使用 Term 构建范围查询
        let lower = Bound::Included(Term::from_field_u64(field, start));
        let upper = Bound::Excluded(Term::from_field_u64(field, end));
        Some(Box::new(RangeQuery::new(lower, upper)))
    }
    
    /// 计算时间范围的实际时间戳
    fn calculate_time_range(&self, range: &TimeRange) -> Option<(u64, u64)> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_secs();
        
        match range {
            TimeRange::LastHours(n) => {
                let start = now.saturating_sub(*n as u64 * 3600);
                Some((start, now))
            }
            TimeRange::LastDays(n) => {
                let start = now.saturating_sub(*n as u64 * 86400);
                Some((start, now))
            }
            TimeRange::LastWeeks(n) => {
                let start = now.saturating_sub(*n as u64 * 7 * 86400);
                Some((start, now))
            }
            TimeRange::LastMonths(n) => {
                let start = now.saturating_sub(*n as u64 * 30 * 86400);
                Some((start, now))
            }
            TimeRange::Between(start, end) => Some((*start, *end)),
            TimeRange::After(ts) => Some((*ts, u64::MAX)),
            TimeRange::Before(ts) => Some((0, *ts)),
            TimeRange::Today => {
                // 计算今天 00:00 的时间戳
                let today_start = (now / 86400) * 86400;
                Some((today_start, now))
            }
            TimeRange::ThisWeek => {
                // 简化：过去7天
                let start = now.saturating_sub(7 * 86400);
                Some((start, now))
            }
            TimeRange::ThisMonth => {
                // 简化：过去30天
                let start = now.saturating_sub(30 * 86400);
                Some((start, now))
            }
        }
    }
    
    /// 构建文件类型查询
    fn build_type_query(&self, types: &[String]) -> Option<Box<dyn Query>> {
        let field = self.schema.get_field(FIELD_FILE_TYPE).ok()?;
        
        let clauses: Vec<(Occur, Box<dyn Query>)> = types.iter()
            .map(|t| {
                let term = Term::from_field_text(field, t);
                let query: Box<dyn Query> = Box::new(TermQuery::new(term, IndexRecordOption::Basic));
                (Occur::Should, query)
            })
            .collect();
        
        if clauses.is_empty() {
            None
        } else {
            Some(Box::new(BooleanQuery::new(clauses)))
        }
    }
    
    /// 构建文件大小查询
    fn build_size_query(&self, filter: &SizeFilter) -> Option<Box<dyn Query>> {
        let field = self.schema.get_field(FIELD_FILE_SIZE).ok()?;
        
        let (lower, upper) = match filter {
            SizeFilter::GreaterThan(size) => (
                Bound::Excluded(Term::from_field_u64(field, *size)),
                Bound::Unbounded,
            ),
            SizeFilter::LessThan(size) => (
                Bound::Unbounded,
                Bound::Excluded(Term::from_field_u64(field, *size)),
            ),
            SizeFilter::Between(min, max) => (
                Bound::Included(Term::from_field_u64(field, *min)),
                Bound::Excluded(Term::from_field_u64(field, *max)),
            ),
        };
        
        Some(Box::new(RangeQuery::new(lower, upper)))
    }
    
    /// 构建标签查询
    fn build_tags_query(&self, tags: &[String]) -> Option<Box<dyn Query>> {
        let field = self.schema.get_field(FIELD_TAGS).ok()?;
        
        let clauses: Vec<(Occur, Box<dyn Query>)> = tags.iter()
            .map(|tag| {
                let term = Term::from_field_text(field, tag);
                let query: Box<dyn Query> = Box::new(TermQuery::new(term, IndexRecordOption::Basic));
                (Occur::Should, query)
            })
            .collect();
        
        if clauses.is_empty() {
            None
        } else {
            Some(Box::new(BooleanQuery::new(clauses)))
        }
    }
}

/// 路径匹配器（使用 glob 模式）
pub struct PathMatcher {
    patterns: Vec<(Pattern, bool)>, // (pattern, is_exclude)
}

impl PathMatcher {
    pub fn new(filters: &[PathFilter]) -> Self {
        let patterns = filters.iter()
            .filter_map(|f| {
                Pattern::new(&f.pattern).ok().map(|p| (p, f.exclude))
            })
            .collect();
        
        Self { patterns }
    }
    
    /// 检查路径是否匹配过滤条件
    /// 返回 true 表示路径应该被包含在结果中
    pub fn matches(&self, path: &str) -> bool {
        if self.patterns.is_empty() {
            return true; // 没有过滤条件，全部包含
        }
        
        let mut included = false;
        let mut has_include_patterns = false;
        
        for (pattern, is_exclude) in &self.patterns {
            if *is_exclude {
                // 排除模式：如果匹配则排除
                if pattern.matches(path) {
                    return false;
                }
            } else {
                // 包含模式
                has_include_patterns = true;
                if pattern.matches(path) {
                    included = true;
                }
            }
        }
        
        // 如果有包含模式，必须至少匹配一个
        // 如果只有排除模式，默认包含
        !has_include_patterns || included
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_path_matcher() {
        let filters = vec![
            PathFilter::include("/usr/src/*"),
            PathFilter::exclude("/usr/src/test/*"),
        ];
        
        let matcher = PathMatcher::new(&filters);
        
        assert!(matcher.matches("/usr/src/kernel.c"));
        assert!(!matcher.matches("/usr/src/test/foo.c"));
        assert!(!matcher.matches("/home/user/doc.txt"));
    }
}
