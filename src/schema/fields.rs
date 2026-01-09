// schema/fields.rs - 字段名常量定义
//! 统一管理所有 Schema 字段名，避免魔法字符串

/// 文件标题
pub const FIELD_TITLE: &str = "title";
/// 文件内容
pub const FIELD_BODY: &str = "body";
/// AI 生成的标签
pub const FIELD_TAGS: &str = "tags";
/// 完整文件路径
pub const FIELD_PATH: &str = "path";
/// 父目录路径
pub const FIELD_PARENT_PATH: &str = "parent_path";
/// 文件名（不含路径）
pub const FIELD_FILENAME: &str = "filename";
/// 文件类型/扩展名
pub const FIELD_FILE_TYPE: &str = "file_type";
/// 文件大小（字节）
pub const FIELD_FILE_SIZE: &str = "file_size";
/// 文件修改时间
pub const FIELD_MODIFIED_TIME: &str = "modified_time";
/// 文件创建时间
pub const FIELD_CREATED_TIME: &str = "created_time";
/// 索引时间
pub const FIELD_INDEXED_TIME: &str = "indexed_time";
