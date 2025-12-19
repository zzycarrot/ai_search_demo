// main.rs
use std::io::{self, Write};
use std::path::Path;
use anyhow::Result;
use std::sync::Arc;

use ai_search_demo::indexer;
use ai_search_demo::search;
use ai_search_demo::config;
use ai_search_demo::ai::BertModel;


fn main() -> Result<()> {

    println!(" [AI] 正在加载 BERT 模型 (首次运行需下载)...");
    // 初始化 BERT，并用 Arc 包裹以便在多线程共享
    let bert = Arc::new(BertModel::new()?); 
    println!(" [AI] 模型加载完毕！");

    let watch_path = Path::new(config::WATCH_PATH);
    let storage_path = Path::new(config::STORAGE_PATH);

    if !watch_path.exists() { std::fs::create_dir_all(watch_path)?; }

    println!("--- 文件搜索系统 ---");
    println!(" [后台] 正在监控: {:?}", watch_path);
    println!(" [前台] 输入关键词进行搜索 (输入 'quit' 退出)");

   // 1. 初始化索引 (schema 里现在有 tags 字段了)
    let (index, schema) = indexer::init_persistent_index(storage_path)?;

    // 2. 扫描现有文件 (传入 bert)
    indexer::scan_existing_files(watch_path, &index, &schema, &bert)?;

    // 3. 启动后台监控 (传入 bert)
    let index_for_watcher = index.clone();
    let schema_for_watcher = schema.clone();
    let bert_for_watcher = bert.clone(); // Arc 克隆，只是引用计数+1
    indexer::start_watcher_thread(watch_path.to_path_buf(), index_for_watcher, schema_for_watcher, bert_for_watcher);

    // 4. 主线程循环：处理用户输入并调用 search 模块
    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "quit" || input == "exit" {
            break;
        }
        if input.is_empty() {
            continue;
        }

        // 调用 lib 里的 search 模块进行搜索
        // 注意：Tantivy 的 Reader 会自动感知 index 的变化，所以这里不需要手动 reload
        if let Err(e) = search::search_index(&index, input) {
            println!("搜索出错: {}", e);
        }
    }

    Ok(())
}