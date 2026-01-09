// engine/mod.rs - 搜索引擎抽象层
//! 提供统一的搜索引擎接口

pub mod core;
pub mod builder;

pub use core::*;
pub use builder::*;
