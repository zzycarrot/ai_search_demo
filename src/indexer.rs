// indexer.rs
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;
use anyhow::Result;
use std::sync::Arc;

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher, EventKind};
use tantivy::schema::*;
use tantivy::{Index, doc, IndexWriter, Term};
use tantivy_jieba::JiebaTokenizer;

use crate::ai::BertModel;
use crate::extract::extract_text; // 使用 crate 内部引用

// 初始化持久化索引
pub fn init_persistent_index(index_path: &Path) -> Result<(Index, Schema)> {
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

    let schema = schema_builder.build();

    if !index_path.exists() {
        fs::create_dir_all(index_path)?;
    }

    let index = Index::open_or_create(tantivy::directory::MmapDirectory::open(index_path)?, schema.clone())?;

    let tokenizer = JiebaTokenizer {};
    index.tokenizers().register("jieba", tokenizer);

    Ok((index, schema))
}

// 处理单个文件 (改为 pub 供 watcher 使用)
pub fn process_and_index(file_path: &Path, index: &Index, schema: &Schema, bert: &BertModel) -> Result<()> {
    // 调用 extract 模块的功能
    let doc_data = extract_text(file_path)?;

    // --- AI 核心步骤：生成关键词 ---
    println!("   [AI] 正在分析文档语义...");
    let keywords = bert.extract_keywords(&doc_data.content, 3)?; // 提取 3 个关键词
    let tags_str = keywords.join(" "); // 变成 "Rust 编程 教程" 这样的字符串存入
    println!("   [AI] 生成标签: {:?}", keywords);
    // ---------------------------

    let title_field = schema.get_field("title").unwrap();
    let body_field = schema.get_field("body").unwrap();
    let path_field = schema.get_field("path").unwrap();
    let tags_field = schema.get_field("tags").unwrap();
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
        tags_field => tags_str // <--- 存入 AI 生成的标签
    ))?;

    index_writer.commit()?;

    println!("\n[Done] [后台] 新文件已索引: {} (输入搜索词继续)", doc_data.title);
    print!("> ");
    io::stdout().flush()?;

    Ok(())
}

// 扫描现有文件
pub fn scan_existing_files(watch_path: &Path, index: &Index, schema: &Schema, bert: &BertModel) -> Result<()> {
    println!(" [后台] 正在扫描现有文件...");
    let mut file_count = 0;

    fn visit_dirs(dir: &Path, index: &Index, schema: &Schema, file_count: &mut usize, bert: &BertModel) -> Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, index, schema, file_count, bert)?;
                } else if path.is_file() {
                    if let Some(extension) = path.extension() {
                        let ext = extension.to_string_lossy().to_lowercase();
                        if matches!(ext.as_str(), "txt" | "md" | "pdf") {
                             if !path.to_string_lossy().contains(".DS_Store") {
                                match process_and_index(&path, index, schema, bert) {
                                    Ok(_) => *file_count += 1,
                                    Err(e) => eprintln!("处理文件失败 {:?}: {}", path, e),
                                }
                             }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    visit_dirs(watch_path, index, schema, &mut file_count, bert)?;
    println!(" [后台] 初始索引完成，共处理 {} 个文件", file_count);
    Ok(())
}

// 启动监控线程
pub fn start_watcher_thread(watch_path: PathBuf, index: Index, schema: Schema, bert: Arc<BertModel>) {
    thread::spawn(move || {
        let (tx, rx) = channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default()).unwrap();
        // 使用文件修改时间而不是处理时间戳来判断文件是否真的变化了
        let mut file_mod_times: HashMap<PathBuf, std::time::SystemTime> = HashMap::new();

        if let Err(e) = watcher.watch(&watch_path, RecursiveMode::Recursive) {
            eprintln!("监控启动失败: {:?}", e);
            return;
        }

        for res in rx {
            match res {
                Ok(event) => {
                    match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) => {
                            for path in event.paths {
                                if path.is_file() && !path.to_string_lossy().contains(".DS_Store") {
                                    // 检查文件扩展名
                                    if let Some(extension) = path.extension() {
                                        let ext = extension.to_string_lossy().to_lowercase();
                                        if matches!(ext.as_str(), "txt" | "md" | "pdf") {
                                            // 检查文件修改时间是否真的发生了变化
                                            if let Ok(metadata) = fs::metadata(&path) {
                                                if let Ok(modified) = metadata.modified() {
                                                    let should_process = match file_mod_times.get(&path) {
                                                        Some(&last_mod) => modified != last_mod,
                                                        None => true, // 新文件
                                                    };

                                                    if should_process {
                                                        file_mod_times.insert(path.clone(), modified);
                                                        // 等待文件写入完成
                                                        thread::sleep(Duration::from_millis(500));
                                                        let _ = process_and_index(&path, &index, &schema, &bert);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        _ => {},
                    }
                },
                Err(e) => eprintln!("Watch error: {:?}", e),
            }
        }
    });
}