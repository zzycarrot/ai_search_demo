// schema/mod.rs - Schema 定义模块
//! 定义索引的 Schema 结构和文档类型

mod document;
pub mod fields;

pub use document::*;
pub use fields::*;

use tantivy::schema::{Schema, SchemaBuilder, TextOptions, TextFieldIndexing, IndexRecordOption, FAST, STORED, STRING};
use tantivy_jieba::JiebaTokenizer;
use tantivy::Index;

/// 创建增强版 Schema
/// 支持更丰富的元数据字段，便于过滤查询
pub fn build_schema() -> Schema {
    let mut schema_builder = SchemaBuilder::default();
    
    // 文本索引选项（带分词）
    let text_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("jieba")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions)
        )
        .set_stored();
    
    // === 核心字段 ===
    // 文件标题（可搜索）
    schema_builder.add_text_field(FIELD_TITLE, text_options.clone());
    // 文件内容（可搜索）
    schema_builder.add_text_field(FIELD_BODY, text_options.clone());
    // AI 生成的标签（可搜索）
    schema_builder.add_text_field(FIELD_TAGS, text_options.clone());
    
    // === 路径字段 ===
    // 完整路径（精确匹配 + 存储）
    schema_builder.add_text_field(FIELD_PATH, STRING | STORED);
    // 父目录路径（用于目录过滤，支持前缀匹配）
    schema_builder.add_text_field(FIELD_PARENT_PATH, STRING | STORED);
    // 文件名（不含路径）
    schema_builder.add_text_field(FIELD_FILENAME, STRING | STORED);
    
    // === 元数据字段 ===
    // 文件类型/扩展名 (pdf, txt, md)
    schema_builder.add_text_field(FIELD_FILE_TYPE, STRING | STORED);
    // 文件大小（字节）
    schema_builder.add_u64_field(FIELD_FILE_SIZE, FAST | STORED);
    // 修改时间（Unix 时间戳）
    schema_builder.add_u64_field(FIELD_MODIFIED_TIME, FAST | STORED);
    // 创建时间（Unix 时间戳）
    schema_builder.add_u64_field(FIELD_CREATED_TIME, FAST | STORED);
    // 索引时间（Unix 时间戳）
    schema_builder.add_u64_field(FIELD_INDEXED_TIME, FAST | STORED);
    
    schema_builder.build()
}

/// 注册分词器到索引
pub fn register_tokenizers(index: &Index) {
    let tokenizer = JiebaTokenizer {};
    index.tokenizers().register("jieba", tokenizer);
}
