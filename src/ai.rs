// src/ai.rs
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use anyhow::Result;
use jieba_rs::Jieba;
use std::collections::HashSet;

pub struct BertModel {
    model: TextEmbedding,
    jieba: Jieba,
}

impl BertModel {
    pub fn new() -> Result<Self> {
        // 修复 1 & 2: 使用 new() 方法初始化，并修正模型名称
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGESmallZHV15)
                .with_show_download_progress(true)
        )?;

        Ok(Self {
            model,
            jieba: Jieba::new(),
        })
    }

    pub fn extract_keywords(&self, text: &str, top_k: usize) -> Result<Vec<String>> {
        let truncated_text = if text.chars().count() > 512 {
            text.chars().take(512).collect::<String>()
        } else {
            text.to_string()
        };

        // 修复 3: 显式标注闭包参数类型 |w: &str|
        let words = self.jieba.cut(&truncated_text, false);
        let candidates: Vec<String> = words.into_iter()
            .map(|w: &str| w.to_string())
            .filter(|w: &String| w.chars().count() > 1) 
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        if candidates.is_empty() {
            return Ok(vec![]);
        }

        let doc_embeddings = self.model.embed(vec![truncated_text], None)?;
        let doc_vec = &doc_embeddings[0];

        let candidate_embeddings = self.model.embed(candidates.clone(), None)?;

        // 修复 4: 显式标注 map 参数类型
        let mut scored_candidates: Vec<(f32, String)> = candidates.iter()
            .zip(candidate_embeddings.iter())
            .map(|(word, vec): (&String, &Vec<f32>)| {
                // 调用下方的辅助函数
                let score = cosine_similarity(doc_vec, vec);
                (score, word.clone())
            })
            .collect();

        scored_candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        let keywords = scored_candidates.into_iter()
            .take(top_k)
            .map(|(_, word)| word)
            .collect();

        Ok(keywords)
    }
}

// 辅助函数放在 impl 块外面
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot_product / (norm_a * norm_b) }
}