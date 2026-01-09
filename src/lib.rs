// src/lib.rs
//! 本地 AI 文件搜索系统
//!
//! 提供基于 Tantivy 的全文搜索引擎，支持：
//! - 自然语言查询 + 结构化过滤语法
//! - AI 关键词提取
//! - 文件元数据索引（路径、大小、时间等）
//! - 实时文件监控

// 核心模块
pub mod config;
pub mod models;
pub mod extract;
pub mod search;
pub mod indexer;
pub mod ai;
pub mod cache;    // Embedding 缓存模块
pub mod registry; // 文件注册表，协调扫描和监听

// 新架构模块
pub mod schema;   // Schema 定义和文档结构
pub mod query;    // 查询解析和过滤
pub mod api;      // API 请求和响应类型
pub mod engine;   // 搜索引擎抽象

// 重导出核心类型
pub use config::*;
pub use models::*;
pub use extract::*;
pub use search::*;
pub use indexer::*;
pub use ai::*;
pub use cache::*;
pub use registry::*;

// 重导出新架构类型
pub use schema::{IndexDocument, FileMetadata};
pub use query::{QueryParser, ParsedQuery, QueryFilters, PathFilter, TimeFilter, SizeFilter};
pub use api::{SearchRequest, SearchResponse, SearchResult};
pub use engine::{SearchEngine, SearchEngineBuilder, EngineError, EngineResult};
