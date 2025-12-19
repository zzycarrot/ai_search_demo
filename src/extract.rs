use std::fs;
use std::path::Path;
use std::time::Duration;
use anyhow::{Result, Context};
use pdf_extract;

use crate::models::FileDoc;
use crate::config::{PREVIEW_MAX_LENGTH, SENTENCE_SEARCH_START};

pub fn extract_text(path: &Path) -> Result<FileDoc> {
    // 简单的防抖动：如果是刚创建的文件，可能还在写入中，稍微等一下
    // 实际生产中通常用 Debouncer，这里简化处理
    std::thread::sleep(Duration::from_millis(100));

    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    println!("📄 正在解析: {:?}", path);

    let content = match extension {
        "txt" | "md" | "rs" => fs::read_to_string(path)?,
        "pdf" => pdf_extract::extract_text(path).with_context(|| "无法解析 PDF")?,
        _ => return Err(anyhow::anyhow!("跳过不支持的文件格式")),
    };

    Ok(FileDoc {
        title: path.file_stem().unwrap().to_string_lossy().to_string(),
        content,
        path: path.to_string_lossy().to_string(),
    })
}

pub fn format_content_preview(content: &str) -> String {
    // 清理内容：移除多余的空白字符
    let cleaned_content = content.trim();
    if cleaned_content.is_empty() {
        return "[无文本内容]".to_string();
    }

    // 显示前PREVIEW_MAX_LENGTH个字符，但保留完整的句子
    if cleaned_content.len() > PREVIEW_MAX_LENGTH {
        // 查找句子结束符的位置
        let sentence_endings = ['。', '！', '？', '.', '!', '?', '\n', '；', ';'];
        let mut end_pos = PREVIEW_MAX_LENGTH;
        let mut found_sentence_end = false;

        // 从第PREVIEW_MAX_LENGTH个字符开始向前查找最近的句子结束符
        for i in (SENTENCE_SEARCH_START..=PREVIEW_MAX_LENGTH).rev() {  // 从PREVIEW_MAX_LENGTH向前到SENTENCE_SEARCH_START查找，给出更大的搜索范围
            if i < cleaned_content.len() {
                if let Some(ch) = cleaned_content.chars().nth(i) {
                    if sentence_endings.contains(&ch) {
                        end_pos = i + 1;  // 包含句子结束符
                        found_sentence_end = true;
                        break;
                    }
                }
            }
        }

        // 如果没找到句子结束符，则在单词边界处截断
        if !found_sentence_end {
            end_pos = PREVIEW_MAX_LENGTH;
            // 尝试在单词边界处截断（查找空格或标点）
            for i in ((PREVIEW_MAX_LENGTH - SENTENCE_SEARCH_START)..=PREVIEW_MAX_LENGTH).rev() {
                if i < cleaned_content.len() {
                    if let Some(ch) = cleaned_content.chars().nth(i) {
                        if ch.is_whitespace() || ch == '，' || ch == '。' || ch == '；' {
                            end_pos = i;
                            break;
                        }
                    }
                }
            }
        }

        // 确保在UTF-8字符边界处截断
        while end_pos > 0 && !cleaned_content.is_char_boundary(end_pos) {
            end_pos -= 1;
        }

        if end_pos == 0 { end_pos = PREVIEW_MAX_LENGTH; }
        format!("{}...", &cleaned_content[..end_pos])
    } else {
        cleaned_content.to_string()
    }
}