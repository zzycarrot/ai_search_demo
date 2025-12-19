use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, doc};
use tantivy::TantivyDocument;
use tantivy_jieba::JiebaTokenizer;
use anyhow::Result;

use crate::models::FileDoc;

pub fn run_search_demo(file_doc: FileDoc, search_key: &str) -> Result<()> {
    println!("\n--- 启动搜索引擎演示 (Jieba 分词版) ---");

    // 1. 定义 Schema
    let mut schema_builder = Schema::builder();

    // 【关键修改点 A】：自定义文本索引选项，指定 tokenizer 为 "jieba"
    let text_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("jieba") // <--- 这里指定分词器名字
                .set_index_option(IndexRecordOption::WithFreqsAndPositions)
        )
        .set_stored(); // 允许存储原始内容

    // 使用我们自定义的 options，而不是默认的 TEXT
    let title_field = schema_builder.add_text_field("title", text_options.clone());
    let body_field = schema_builder.add_text_field("body", text_options.clone());

    // 路径不需要分词，还是用 STRING
    let path_field = schema_builder.add_text_field("path", STRING | STORED);

    let schema = schema_builder.build();

    // 2. 创建索引
    let index = Index::create_in_ram(schema.clone());

    // 【关键修改点 B】：注册 Jieba 分词器
    // 这一步必须在 writer 创建之前做，否则 writer 不知道 "jieba" 是什么
    let tokenizer = JiebaTokenizer {};
    index.tokenizers().register("jieba", tokenizer);

    // 3. 创建写入器
    let mut index_writer = index.writer(50_000_000)?;

    println!("正在将文档加入索引...");

    // 4. 添加文档
    index_writer.add_document(doc!(
        title_field => file_doc.title.as_str(),
        body_field => file_doc.content.as_str(),
        path_field => file_doc.path.as_str()
    ))?;

    // 5. 提交 (类似于 git commit，不提交就搜不到)
    index_writer.commit()?;
    println!("索引构建完成！");

    // 6. 创建搜索器
    let reader = index.reader()?;
    let searcher = reader.searcher();

    // 7. 模拟用户搜索
    // 假设我们要搜索文档标题里的某个词，或者内容里的词
    // 为了演示成功，我们直接搜索标题的一部分
    // 注意：tantivy 默认分词器对中文支持一般，它按空格分词。
    // 如果你的标题是英文（如 "Risv-V Guide"），搜 "Guide" 能搜到。
    // 如果是中文（如 "实验报告"），搜 "实验" 可能搜不到（因为它被当成了一整块）。
    // 这里我们先构造一个简单的查询。
    let query_str = search_key; // 使用传入的搜索关键词
    println!("正在执行搜索，关键词: '{}'", query_str);

    // 解析查询语句：告诉引擎去 'title' 和 'body' 两个字段里找
    let query_parser = QueryParser::for_index(&index, vec![title_field, body_field]);
    let query = query_parser.parse_query(query_str)?;

    // 执行搜索：获取评分最高的 10 个结果
    let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;

    println!("搜索结果数量: {}", top_docs.len());

    // 8. 遍历结果
    for (_score, doc_address) in top_docs {
        // 修改点 1: 显式指定变量类型为 TantivyDocument
        let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

        // 修改点 2:
        // Tantivy 0.21+ 的 Value 类型通常使用 .as_str() 获取文本
        // 另外建议用 unwrap_or("默认值") 防止字段为空时崩溃
        let doc_title = retrieved_doc.get_first(title_field)
            .and_then(|v| v.as_str()) // 注意这里改成了 as_str()
            .unwrap_or("[无标题]");

        let doc_path = retrieved_doc.get_first(path_field)
            .and_then(|v| v.as_str()) // 注意这里改成了 as_str()
            .unwrap_or("[无路径]");

        println!(" ✅ 找到匹配文档!");
        println!("    标题: {}", doc_title);
        println!("    路径: {}", doc_path);
        println!("    匹配度(Score): {:.4}", _score);
    }

    Ok(())
}