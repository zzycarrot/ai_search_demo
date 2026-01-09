// main.rs
use std::io::{self, Write};
use std::path::Path;
use anyhow::Result;
use std::sync::Arc;

use ai_search_demo::indexer;
use ai_search_demo::search;
use ai_search_demo::config;
use ai_search_demo::ai::BertModel;
use ai_search_demo::cache::EmbeddingCache;
use ai_search_demo::registry::FileRegistry;


fn main() -> Result<()> {

    println!(" [AI] 正在加载 BERT 模型 (首次运行需下载)...");
    // 初始化 BERT，并用 Arc 包裹以便在多线程共享
    let bert = Arc::new(BertModel::new()?); 
    println!(" [AI] 模型加载完毕！");

    // 初始化 Embedding 缓存
    let cache_path = Path::new(config::CACHE_PATH);
    let cache = Arc::new(EmbeddingCache::new(cache_path)?);
    let (cache_count, cache_size) = cache.stats();
    println!(" [Cache] 缓存统计: {} 条记录, {} 字节", cache_count, cache_size);

    let watch_path = Path::new(config::WATCH_PATH);
    let storage_path = Path::new(config::STORAGE_PATH);

    if !watch_path.exists() { std::fs::create_dir_all(watch_path)?; }

    println!("--- 文件搜索系统 ---");
    println!(" [后台] 正在监控: {:?}", watch_path);
    println!(" [前台] 输入关键词进行搜索 (输入 'quit' 退出)");

    // 1. 初始化索引
    let (index, schema, reader) = indexer::init_persistent_index(storage_path)?;

    // 2. 创建文件注册表（用于协调扫描和监听）
    let registry = FileRegistry::new();

    // 3. **先启动监控线程**（会等待扫描完成信号）
    let index_for_watcher = index.clone();
    let schema_for_watcher = schema.clone();
    let bert_for_watcher = bert.clone();
    let cache_for_watcher = cache.clone();
    let registry_for_watcher = registry.clone();
    let scan_complete_signal = indexer::start_watcher_thread(
        watch_path.to_path_buf(), 
        index_for_watcher, 
        schema_for_watcher, 
        bert_for_watcher, 
        cache_for_watcher,
        registry_for_watcher,
    );

    // 4. **后执行扫描**（扫描期间的事件会被 registry 记录）
    indexer::scan_existing_files(watch_path, &index, &schema, &bert, &cache, &registry)?;

    // 5. 通知监控线程扫描完成
    let _ = scan_complete_signal.send(());
    println!(" [同步] 扫描完成，监控已激活");

    // 6. 主线程循环：处理用户输入并调用 search 模块
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
        //bert 来优化查询
        let search_query = bert.refine_query(input);

        // 调用 lib 里的 search 模块进行搜索
        // 使用带自动刷新策略的 reader
        if let Err(e) = search::search_index(&reader, &index, &search_query) {
            println!("搜索出错: {}", e);
        }
    }

    Ok(())
}