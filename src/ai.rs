use anyhow::Result;
use candle_core::{Device, Tensor, IndexOp};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel as CandleBert, Config};
use hf_hub::api::sync::Api;
use jieba_rs::Jieba;
use std::collections::HashSet;
use std::path::Path;
use tokenizers::Tokenizer;

pub struct BertModel {
    model: CandleBert,
    tokenizer: Tokenizer,
    jieba: Jieba,
    device: Device,
}

impl BertModel {
    pub fn new() -> Result<Self> {
        // 1. 定义模型路径查找逻辑
        // 优先查找本地 ./model 目录，如果不存在，再尝试联网下载
        let local_model_dir = Path::new("model");
        
        let (model_path, tokenizer_path, config_path) = if local_model_dir.exists() {
            println!(" [AI] 发现本地模型目录 ./model，进入离线模式...");
            (
                local_model_dir.join("model.safetensors"),
                local_model_dir.join("tokenizer.json"),
                local_model_dir.join("config.json"),
            )
        } else {
            println!(" [AI] 本地未找到模型，正在尝试从 HuggingFace 下载 BAAI/bge-small-zh-v1.5...");
            let api = Api::new()?;
            let repo = api.model("BAAI/bge-small-zh-v1.5".to_string());
            (
                repo.get("model.safetensors")?,
                repo.get("tokenizer.json")?,
                repo.get("config.json")?,
            )
        };

        let device = Device::Cpu;

        // 2. 加载配置和分词器
        let config_content = std::fs::read_to_string(config_path)?;
        let config: Config = serde_json::from_str(&config_content)?;
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)?;

        // 3. 加载模型权重
        let vb = unsafe { 
            VarBuilder::from_mmaped_safetensors(&[model_path], candle_core::DType::F32, &device)? 
        };
        
        // 4. 初始化模型
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

        let output = self.model.forward(&token_ids, &token_type_ids, None)?;
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

/// 关键词提取器（封装 BertModel）
pub struct KeywordExtractor {
    model: BertModel,
}

impl KeywordExtractor {
    pub fn new(model_path: &Path) -> Result<Self> {
        // TODO: 使用 model_path 参数
        let model = BertModel::new()?;
        Ok(Self { model })
    }
    
    /// 提取关键词
    pub fn extract(&self, text: &str) -> Result<Vec<String>> {
        self.model.extract_keywords(text, 3)
    }
    
    /// 优化查询
    pub fn refine(&self, query: &str) -> String {
        self.model.refine_query(query)
    }
}