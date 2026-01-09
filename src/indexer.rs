// indexer.rs
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::time::{Duration, SystemTime};
use anyhow::Result;
use std::sync::Arc;

use ignore::WalkBuilder;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher, EventKind};
use tantivy::schema::*;
use tantivy::{Index, doc, IndexWriter, Term, IndexReader, ReloadPolicy};
use tantivy_jieba::JiebaTokenizer;

use crate::ai::BertModel;
use crate::cache::EmbeddingCache;
use crate::config::CONFIG;
use crate::extract::extract_text;
use crate::registry::{FileRegistry, EventType};

// 初始化持久化索引
pub fn init_persistent_index(index_path: &Path) -> Result<(Index, Schema, IndexReader)> {
    let mut schema_builder = Schema::builder();

    let text_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("jieba")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions)
        )
        .set_stored();

    schema_builder.add_text_field("title", text_options.clone());
    schema_builder.add_text_field("body", text_options.clone());
    schema_builder.add_text_field("path", STRING | STORED);
    schema_builder.add_text_field("tags", text_options.clone());
    schema_builder.add_u64_field("timestamp", FAST | STORED);

    let schema = schema_builder.build();

    if !index_path.exists() {
        fs::create_dir_all(index_path)?;
    }

    let index = Index::open_or_create(tantivy::directory::MmapDirectory::open(index_path)?, schema.clone())?;

    let tokenizer = JiebaTokenizer {};
    index.tokenizers().register("jieba", tokenizer);

    // 创建带自动刷新策略的 Reader
    // OnCommitWithDelay 会在 commit 后自动刷新，延迟 500ms
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()?;

    Ok((index, schema, reader))
}

// 检查文件是否需要索引
// 返回 true 表示：数据库里没这个文件，或者文件变新了，需要重新搞
fn should_index_file(path: &Path, index: &Index, schema: &Schema) -> bool {
    let path_str = path.to_string_lossy();
    let reader = match index.reader() {
        Ok(r) => r,
        Err(_) => return true, // 读不出索引就默认重建
    };
    let searcher = reader.searcher();
    
    let path_field = schema.get_field("path").unwrap();
    let timestamp_field = schema.get_field("timestamp").unwrap();

    // 1. 在索引里查这个路径
    let query = Term::from_field_text(path_field, &path_str);
    let term_query = tantivy::query::TermQuery::new(query, IndexRecordOption::Basic);
    
    // 找匹配的文档
    let top_docs = match searcher.search(&term_query, &tantivy::collector::TopDocs::with_limit(1)) {
        Ok(docs) => docs,
        Err(_) => return true,
    };

    if top_docs.is_empty() {
        return true; // 数据库里没这个文件 -> 必须索引
    }

    // 2. 如果找到了，读取它存的时间戳
    let (_score, doc_address) = top_docs[0];
    let doc: TantivyDocument = match searcher.doc(doc_address) {
        Ok(d) => d,
        Err(_) => return true,
    };
    
    // 获取数据库里的旧时间
    let stored_ts = doc.get_first(timestamp_field)
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // 3. 获取硬盘文件当前的时间戳
    let current_ts = fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::now())
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // 4. 比对：如果硬盘文件时间 > 数据库存的时间，说明文件被修改过 -> 需要索引
    current_ts > stored_ts
}

// 从索引中删除文件
pub fn delete_from_index(file_path: &Path, index: &Index, schema: &Schema, cache: Option<&EmbeddingCache>) -> Result<bool> {
    // 尝试规范化路径，如果文件已删除则使用原始路径
    let path_str = file_path.canonicalize()
        .unwrap_or_else(|_| file_path.to_path_buf())
        .to_string_lossy()
        .to_string();
    
    let path_field = schema.get_field("path").unwrap();
    
    let mut index_writer: IndexWriter = index.writer(50_000_000)?;
    
    // 删除规范化路径
    let path_term = Term::from_field_text(path_field, &path_str);
    index_writer.delete_term(path_term);
    
    // 也尝试删除原始路径格式（兼容旧数据）
    let original_path_str = file_path.to_string_lossy();
    if original_path_str != path_str {
        let original_term = Term::from_field_text(path_field, &original_path_str);
        index_writer.delete_term(original_term);
    }
    
    index_writer.commit()?;
    
    // 同时从缓存中删除（两种路径格式都尝试）
    if let Some(c) = cache {
        let _ = c.remove(&path_str);
        let _ = c.remove(&original_path_str);
    }
    
    Ok(true)
}

// 处理单个文件 (改为 pub 供 watcher 使用)
pub fn process_and_index(file_path: &Path, index: &Index, schema: &Schema, bert: &BertModel, cache: &EmbeddingCache) -> Result<()> {
    // 调用 extract 模块的功能
    let doc_data = extract_text(file_path)?;

    //获取文件当前时间戳
    let file_timestamp = fs::metadata(file_path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::now())
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // --- AI 核心步骤：生成关键词 (优先使用缓存) ---
    let keywords = if let Some(cached_keywords) = cache.get_keywords(&doc_data.path, &doc_data.content) {
        println!("   [Cache] 命中缓存: {:?}", cached_keywords);
        cached_keywords
    } else {
        println!("   [AI] 正在分析文档语义...");
        let new_keywords = bert.extract_keywords(&doc_data.content, 3)?; // 提取 3 个关键词
        // 存入缓存
        let _ = cache.set_keywords(&doc_data.path, &doc_data.content, new_keywords.clone());
        println!("   [AI] 生成标签: {:?} (已缓存)", new_keywords);
        new_keywords
    };
    let tags_str = keywords.join(" "); // 变成 "Rust 编程 教程" 这样的字符串存入
    // ---------------------------

    let title_field = schema.get_field("title").unwrap();
    let body_field = schema.get_field("body").unwrap();
    let path_field = schema.get_field("path").unwrap();
    let tags_field = schema.get_field("tags").unwrap();
    let timestamp_field = schema.get_field("timestamp").unwrap();
    // 每次创建 writer 开销较大，但在 Watcher 这种低频场景下是可以接受的
    let mut index_writer: IndexWriter = index.writer(50_000_000)?;

    // 先删除旧的
    let path_term = Term::from_field_text(path_field, &doc_data.path);
    index_writer.delete_term(path_term);

    // 写入新的
    index_writer.add_document(doc!(
        title_field => doc_data.title.as_str(),
        body_field => doc_data.content.as_str(),
        path_field => doc_data.path.as_str(),
        tags_field => tags_str, // <--- 存入 AI 生成的标签
        timestamp_field => file_timestamp // 写入时间戳
    ))?;

    index_writer.commit()?;

    println!("\n[Done] [后台] 新文件已索引: {} (输入搜索词继续)", doc_data.title);
    print!("> ");
    io::stdout().flush()?;

    Ok(())
}

// 清理孤儿索引（文件已删除但索引还在）
pub fn cleanup_orphan_indexes(index: &Index, schema: &Schema, cache: &EmbeddingCache) -> Result<usize> {
    let reader = index.reader()?;
    let searcher = reader.searcher();
    let path_field = schema.get_field("path").unwrap();
    
    let mut orphan_paths: Vec<String> = Vec::new();
    
    // 遍历所有文档
    for segment_reader in searcher.segment_readers() {
        let store_reader = segment_reader.get_store_reader(1)?;
        for doc_id in 0..segment_reader.num_docs() {
            if let Ok(doc) = store_reader.get::<TantivyDocument>(doc_id) {
                if let Some(path_value) = doc.get_first(path_field) {
                    if let Some(path_str) = path_value.as_str() {
                        let path = Path::new(path_str);
                        if !path.exists() {
                            println!("[清理] 发现孤儿索引: {}", path_str);
                            orphan_paths.push(path_str.to_string());
                        }
                    }
                }
            }
        }
    }
    
    let orphan_count = orphan_paths.len();
    
    if orphan_count > 0 {
        let mut index_writer: IndexWriter = index.writer(50_000_000)?;
        for path_str in &orphan_paths {
            let path_term = Term::from_field_text(path_field, path_str);
            index_writer.delete_term(path_term);
            // 同时清理缓存
            let _ = cache.remove(path_str);
        }
        index_writer.commit()?;
        println!("[清理] 已清理 {} 个孤儿索引", orphan_count);
    }
    
    Ok(orphan_count)
}

// 扫描现有文件（使用 registry 协调）
pub fn scan_existing_files(
    watch_path: &Path, 
    index: &Index, 
    schema: &Schema, 
    bert: &BertModel, 
    cache: &EmbeddingCache,
    registry: &FileRegistry,
) -> Result<()> {
    // 先清理孤儿索引
    let _ = cleanup_orphan_indexes(index, schema, cache);
    
    println!(" [后台] 正在扫描现有文件...");
    let mut file_count = 0;

    // 根据配置选择遍历方式
    if CONFIG.walker.use_ripgrep_walker {
        // 使用 ripgrep 风格遍历 (ignore crate)
        scan_with_ripgrep_walker(watch_path, index, schema, bert, cache, registry, &mut file_count)?;
    } else {
        // 使用标准递归遍历
        scan_with_std_walker(watch_path, index, schema, bert, cache, registry, &mut file_count)?;
    }
    
    println!(" [后台] 初始索引完成，共处理 {} 个文件", file_count);
    Ok(())
}

/// 使用 ripgrep 风格遍历 (基于 ignore crate)
/// 自动尊重 .gitignore, .ignore 等忽略规则
fn scan_with_ripgrep_walker(
    watch_path: &Path,
    index: &Index,
    schema: &Schema,
    bert: &BertModel,
    cache: &EmbeddingCache,
    registry: &FileRegistry,
    file_count: &mut usize,
) -> Result<()> {
    let walker_config = &CONFIG.walker;
    
    // 构建 WalkBuilder
    let mut builder = WalkBuilder::new(watch_path);
    
    // 配置遍历选项
    builder
        .hidden(!walker_config.skip_hidden)  // 注意: ignore crate 中 hidden(false) 表示不跳过隐藏文件
        .git_ignore(walker_config.respect_gitignore)
        .git_global(walker_config.respect_gitignore)
        .git_exclude(walker_config.respect_gitignore)
        .ignore(walker_config.respect_ignore)
        .follow_links(walker_config.follow_symlinks);
    
    // 设置最大深度
    if walker_config.max_depth > 0 {
        builder.max_depth(Some(walker_config.max_depth));
    }
    
    // 添加自定义忽略模式
    for pattern in &walker_config.custom_ignore_patterns {
        // 使用 overrides 添加忽略模式
        let mut overrides = ignore::overrides::OverrideBuilder::new(watch_path);
        overrides.add(&format!("!{}", pattern))?;
        if let Ok(ovr) = overrides.build() {
            builder.overrides(ovr);
        }
    }
    
    // 遍历文件
    for result in builder.build() {
        match result {
            Ok(entry) => {
                let path = entry.path();
                
                // 跳过目录
                if path.is_dir() {
                    continue;
                }
                
                // 检查文件扩展名
                if !is_supported_file(path) {
                    continue;
                }
                
                // 处理文件
                process_file_entry(path, index, schema, bert, cache, registry, file_count);
            }
            Err(e) => {
                eprintln!(" [Walker] 遍历错误: {}", e);
            }
        }
    }
    
    Ok(())
}

/// 使用标准递归遍历
fn scan_with_std_walker(
    watch_path: &Path,
    index: &Index,
    schema: &Schema,
    bert: &BertModel,
    cache: &EmbeddingCache,
    registry: &FileRegistry,
    file_count: &mut usize,
) -> Result<()> {
    fn visit_dirs(
        dir: &Path, 
        index: &Index, 
        schema: &Schema, 
        file_count: &mut usize, 
        bert: &BertModel, 
        cache: &EmbeddingCache,
        registry: &FileRegistry,
    ) -> Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, index, schema, file_count, bert, cache, registry)?;
                } else if path.is_file() && is_supported_file(&path) {
                    process_file_entry(&path, index, schema, bert, cache, registry, file_count);
                }
            }
        }
        Ok(())
    }

    visit_dirs(watch_path, index, schema, file_count, bert, cache, registry)
}

/// 处理单个文件入口 (共用逻辑)
fn process_file_entry(
    path: &Path,
    index: &Index,
    schema: &Schema,
    bert: &BertModel,
    cache: &EmbeddingCache,
    registry: &FileRegistry,
    file_count: &mut usize,
) {
    let path_buf = path.to_path_buf();
    // 获取文件修改时间
    if let Some(modified_time) = get_modified_time(path) {
        // 使用 registry 判断是否需要处理（原子操作）
        if registry.try_start_processing(&path_buf, modified_time) {
            // 额外检查索引中是否已有最新版本
            if should_index_file(path, index, schema) {
                match process_and_index(path, index, schema, bert, cache) {
                    Ok(_) => *file_count += 1,
                    Err(e) => eprintln!("处理文件失败 {:?}: {}", path, e),
                }
            }
            registry.finish_processing(&path_buf);
        }
    }
}

/// 辅助函数：检查文件扩展名是否支持
fn is_supported_file(path: &Path) -> bool {
    // 过滤 .DS_Store 等系统文件
    if path.to_string_lossy().contains(".DS_Store") {
        return false;
    }
    
    if let Some(extension) = path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        // 使用配置中的支持扩展名列表
        CONFIG.walker.supported_extensions
            .iter()
            .any(|supported| supported.eq_ignore_ascii_case(&ext))
    } else {
        false
    }
}

/// 辅助函数：检查文件扩展名是否支持（公开版本，供 watcher 使用）
pub fn is_file_supported(path: &Path) -> bool {
    is_supported_file(path)
}

/// 辅助函数：获取文件修改时间
fn get_modified_time(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

/// 处理单个文件事件（统一入口）
fn handle_file_event(
    path: &Path,
    index: &Index,
    schema: &Schema,
    bert: &BertModel,
    cache: &EmbeddingCache,
    registry: &FileRegistry,
) -> Result<bool> {
    // 检查文件是否存在
    if !path.exists() {
        // 文件已删除
        delete_from_index(path, index, schema, Some(cache))?;
        registry.mark_deleted(&path.to_path_buf());
        println!("\n[删除] 已从索引移除: {:?}", path.file_name().unwrap_or_default());
        print!("> ");
        let _ = io::stdout().flush();
        return Ok(true);
    }

    // 获取文件修改时间
    let modified_time = match get_modified_time(path) {
        Some(t) => t,
        None => return Ok(false),
    };

    // 尝试获取处理权（原子操作）
    if !registry.try_start_processing(&path.to_path_buf(), modified_time) {
        // 已被处理或正在处理中
        return Ok(false);
    }

    // 等待文件写入完成
    thread::sleep(Duration::from_millis(200));

    // 执行索引
    let result = process_and_index(path, index, schema, bert, cache);

    // 完成处理
    registry.finish_processing(&path.to_path_buf());

    result.map(|_| true)
}

/// 启动监控线程（先于扫描启动）
/// 返回一个信号发送器，用于通知扫描完成
pub fn start_watcher_thread(
    watch_path: PathBuf, 
    index: Index, 
    schema: Schema, 
    bert: Arc<BertModel>, 
    cache: Arc<EmbeddingCache>,
    registry: FileRegistry,
) -> Sender<()> {
    let (scan_complete_tx, scan_complete_rx): (Sender<()>, Receiver<()>) = channel();
    
    thread::spawn(move || {
        let (tx, rx) = channel();
        let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("监控启动失败: {:?}", e);
                return;
            }
        };

        if let Err(e) = watcher.watch(&watch_path, RecursiveMode::Recursive) {
            eprintln!("监控启动失败: {:?}", e);
            return;
        }

        println!(" [监控] 文件监控已启动");

        // 等待扫描完成信号
        let _ = scan_complete_rx.recv();
        println!(" [监控] 扫描完成，开始处理实时事件");

        // 处理扫描期间收集的待处理事件
        let pending_events = registry.complete_scan();
        for event in pending_events {
            if is_supported_file(&event.path) {
                match event.event_type {
                    EventType::Create | EventType::Modify => {
                        let _ = handle_file_event(&event.path, &index, &schema, &bert, &cache, &registry);
                    }
                    EventType::Delete => {
                        let _ = delete_from_index(&event.path, &index, &schema, Some(&cache));
                        registry.mark_deleted(&event.path);
                    }
                }
            }
        }

        // 处理后续实时事件
        for res in rx {
            match res {
                Ok(event) => {
                    // 只处理有意义的事件
                    let event_type = match event.kind {
                        EventKind::Create(_) => Some(EventType::Create),
                        EventKind::Modify(notify::event::ModifyKind::Data(_)) => Some(EventType::Modify),
                        EventKind::Remove(_) => Some(EventType::Delete),
                        _ => None,
                    };

                    let event_type = match event_type {
                        Some(t) => t,
                        None => continue,
                    };

                    for path in event.paths {
                        if !is_supported_file(&path) {
                            continue;
                        }

                        match event_type {
                            EventType::Create | EventType::Modify => {
                                // 检查文件是否存在（macOS 可能把删除误报为 Modify）
                                if !path.exists() {
                                    let _ = delete_from_index(&path, &index, &schema, Some(&cache));
                                    registry.mark_deleted(&path);
                                    println!("\n[删除] 已从索引移除: {:?}", path.file_name().unwrap_or_default());
                                    print!("> ");
                                    let _ = io::stdout().flush();
                                } else {
                                    let _ = handle_file_event(&path, &index, &schema, &bert, &cache, &registry);
                                }
                            }
                            EventType::Delete => {
                                let _ = delete_from_index(&path, &index, &schema, Some(&cache));
                                registry.mark_deleted(&path);
                                println!("\n[删除] 已从索引移除: {:?}", path.file_name().unwrap_or_default());
                                print!("> ");
                                let _ = io::stdout().flush();
                            }
                        }
                    }
                }
                Err(e) => eprintln!("Watch error: {:?}", e),
            }
        }
    });

    scan_complete_tx
}