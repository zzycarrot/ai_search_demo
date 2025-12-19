use std::path::Path;
use std::env;
use anyhow::Result;

use ai_search_demo::{extract_text, format_content_preview, run_search_demo};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("用法: {} <文件路径> <搜索关键词>", args[0]);
        return Ok(());
    }
    let path = Path::new(&args[1]);
    let search_key = &args[2];

    if path.exists() {
        let doc = extract_text(path)?;
        println!("标题: {}", doc.title);

        let preview = format_content_preview(&doc.content);
        println!("内容摘要: {}", preview);

        run_search_demo(doc, search_key)?;
    } else {
        println!("文件不存在: {:?}", path);
    }

    Ok(())
}