use anyhow::Result;
// 【修改 1】引入 IndexOp trait，解决 output.i() 报错
use candle_core::{Device, Tensor, IndexOp};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel as CandleBert, Config};
use hf_hub::api::sync::Api;
use jieba_rs::Jieba;
use std::collections::HashSet;
use tokenizers::Tokenizer;

pub struct BertModel {
    model: CandleBert,
    tokenizer: Tokenizer,
    jieba: Jieba,
    device: Device,
}

impl BertModel {
    pub fn new() -> Result<Self> {
        println!(" [AI] 正在加载模型 BAAI/bge-small-zh-v1.5 (RISC-V 兼容模式)...");

        let api = Api::new()?;
        let repo = api.model("BAAI/bge-small-zh-v1.5".to_string());

        let model_path = repo.get("model.safetensors")?;
        let tokenizer_path = repo.get("tokenizer.json")?;
        let config_path = repo.get("config.json")?;

        let device = Device::Cpu;

        let config_content = std::fs::read_to_string(config_path)?;
        let config: Config = serde_json::from_str(&config_content)?;
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)?;

        let vb = unsafe { 
            VarBuilder::from_mmaped_safetensors(&[model_path], candle_core::DType::F32, &device)? 
        };
        
        // 【修改 2】使用 load() 替代 new()
        let model = CandleBert::load(vb, &config)?;

        println!(" [AI] 模型加载完成！");

        Ok(Self {
            model,
            tokenizer,
            jieba: Jieba::new(),
            device,
        })
    }

    fn get_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let tokens = self.tokenizer.encode(text, true).map_err(anyhow::Error::msg)?;
        
        let token_ids = Tensor::new(tokens.get_ids(), &self.device)?.unsqueeze(0)?;
        let token_type_ids = Tensor::new(tokens.get_type_ids(), &self.device)?.unsqueeze(0)?;

        // 【修改 3】forward 增加第三个参数 None (代表没有 Attention Mask)
        let output = self.model.forward(&token_ids, &token_type_ids, None)?;

        // 现在 .i() 可以用了，因为引入了 IndexOp
        let cls_embedding = output.i((0, 0))?; 

        let vec = cls_embedding.flatten_all()?.to_vec1()?;
        Ok(vec)
    }

    pub fn refine_query(&self, origin_query: &str) -> String {
        if origin_query.chars().count() < 4 {
            return origin_query.to_string();
        }

        match self.extract_keywords(origin_query, 2) {
            Ok(keywords) => {
                if keywords.is_empty() {
                    origin_query.to_string()
                } else {
                    let refined = keywords.join(" ");
                    if refined != origin_query {
                        println!("   [AI] 意图识别: '{}' -> '{}'", origin_query, refined);
                        return refined;
                    }
                    origin_query.to_string()
                }
            },
            Err(e) => {
                eprintln!(" [AI] 意图分析失败: {}", e);
                origin_query.to_string()
            }
        }
    }

    pub fn extract_keywords(&self, text: &str, top_k: usize) -> Result<Vec<String>> {
        let truncated_text = if text.chars().count() > 512 {
            text.chars().take(512).collect::<String>()
        } else {
            text.to_string()
        };

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

        let doc_vec = self.get_embedding(&truncated_text)?;

        let mut scored_candidates: Vec<(f32, String)> = Vec::new();

        for candidate in &candidates {
            if let Ok(cand_vec) = self.get_embedding(candidate) {
                let score = cosine_similarity(&doc_vec, &cand_vec);
                scored_candidates.push((score, candidate.clone()));
            }
        }

        scored_candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let keywords = scored_candidates.into_iter()
            .take(top_k)
            .map(|(_, word)| word)
            .collect();

        Ok(keywords)
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot_product / (norm_a * norm_b) }
}