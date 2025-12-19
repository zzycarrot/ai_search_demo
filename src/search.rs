// search.rs
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::{Index, TantivyDocument};
use tantivy::schema::*;
use anyhow::Result;

// 这个函数现在只负责搜索，不负责建索引
pub fn search_index(index: &Index, query_str: &str) -> Result<()> {
    let reader = index.reader()?;
    let searcher = reader.searcher();
    
    // 获取 Schema 用于字段解析
    let schema = index.schema();
    let title_field = schema.get_field("title").unwrap();
    let body_field = schema.get_field("body").unwrap();
    let path_field = schema.get_field("path").unwrap();

    let query_parser = QueryParser::for_index(index, vec![title_field, body_field]);
    
    // 解析查询
    let query = match query_parser.parse_query(query_str) {
        Ok(q) => q,
        Err(_) => {
            println!("   ❌ 查询语法错误，请重试 (例如: 'Rust AND Linux')");
            return Ok(());
        }
    };

    let top_docs = searcher.search(&query, &TopDocs::with_limit(5))?;

    if top_docs.is_empty() {
        println!("   📭 没有找到相关文档");
    }

    for (_score, doc_address) in top_docs {
        let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

        let title = retrieved_doc.get_first(title_field).and_then(|v| v.as_str()).unwrap_or("无标题");
        let path = retrieved_doc.get_first(path_field).and_then(|v| v.as_str()).unwrap_or("无路径");
        
        // 可选：在这里调用 extract::format_content_preview 来显示摘要
        // 但为了性能，这里只显示标题和路径
        println!("   📄 [{}] (Score: {:.2}) \n       路径: {}", title, _score, path);
    }

    Ok(())
}