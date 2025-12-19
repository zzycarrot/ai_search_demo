// src/lib.rs
pub mod config;
pub mod models;
pub mod extract;
pub mod search;
pub mod indexer;
pub mod ai; // <--- 新增这一行

pub use config::*;
pub use models::*;
pub use extract::*;
pub use search::*;
pub use indexer::*;
pub use ai::*; // <--- 新增这一行
