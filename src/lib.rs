// src/lib.rs
pub mod config;
pub mod models;
pub mod extract;
pub mod search;
pub mod indexer;
pub mod ai;
pub mod cache;    // Embedding 缓存模块
pub mod registry; // 文件注册表，协调扫描和监听

pub use config::*;
pub use models::*;
pub use extract::*;
pub use search::*;
pub use indexer::*;
pub use ai::*;
pub use cache::*;
pub use registry::*;
