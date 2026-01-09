// 配置模块 - 支持外部配置文件
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// 配置文件路径
const CONFIG_FILE: &str = "./config.toml";

// ============== 配置结构体 ==============

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub paths: PathsConfig,
    pub display: DisplayConfig,
    pub walker: WalkerConfig,
    pub ai: AiConfig,
    pub performance: PerformanceConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PathsConfig {
    pub watch_path: String,
    pub storage_path: String,
    pub cache_path: String,
    pub model_path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DisplayConfig {
    pub preview_max_length: usize,
    pub sentence_search_start: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WalkerConfig {
    /// 是否启用 ripgrep 风格遍历
    pub use_ripgrep_walker: bool,
    /// 是否遵循 .gitignore 规则
    pub respect_gitignore: bool,
    /// 是否遵循 .ignore 文件规则
    pub respect_ignore: bool,
    /// 是否跳过隐藏文件
    pub skip_hidden: bool,
    /// 是否跟随符号链接
    pub follow_symlinks: bool,
    /// 最大遍历深度 (0 表示无限制)
    pub max_depth: usize,
    /// 自定义忽略模式
    pub custom_ignore_patterns: Vec<String>,
    /// 支持的文件扩展名
    pub supported_extensions: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AiConfig {
    pub keyword_count: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PerformanceConfig {
    pub index_writer_memory: usize,
}

// ============== 默认配置 ==============

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            paths: PathsConfig {
                watch_path: "./docs".to_string(),
                storage_path: "./storage".to_string(),
                cache_path: "./cache".to_string(),
                model_path: "./model".to_string(),
            },
            display: DisplayConfig {
                preview_max_length: 200,
                sentence_search_start: 50,
            },
            walker: WalkerConfig {
                use_ripgrep_walker: true,
                respect_gitignore: true,
                respect_ignore: true,
                skip_hidden: true,
                follow_symlinks: false,
                max_depth: 0,
                custom_ignore_patterns: vec![
                    "*.log".to_string(),
                    "*.tmp".to_string(),
                    "node_modules".to_string(),
                    "target".to_string(),
                    ".git".to_string(),
                ],
                supported_extensions: vec![
                    "txt".to_string(),
                    "md".to_string(),
                    "pdf".to_string(),
                ],
            },
            ai: AiConfig {
                keyword_count: 3,
            },
            performance: PerformanceConfig {
                index_writer_memory: 50_000_000,
            },
        }
    }
}

// ============== 配置加载 ==============

impl AppConfig {
    /// 从配置文件加载，失败则使用默认配置
    pub fn load() -> Self {
        Self::load_from_file(CONFIG_FILE).unwrap_or_else(|e| {
            eprintln!(" [Config] 无法加载配置文件 '{}': {}", CONFIG_FILE, e);
            eprintln!(" [Config] 使用默认配置");
            Self::default()
        })
    }

    /// 从指定文件加载配置
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path.as_ref())?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// 生成默认配置文件
    pub fn generate_default_config<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
        let default_content = include_str!("../config.toml");
        fs::write(path, default_content)?;
        Ok(())
    }
}

// ============== 全局配置实例 ==============

/// 全局配置实例 (懒加载)
pub static CONFIG: Lazy<AppConfig> = Lazy::new(|| {
    let config = AppConfig::load();
    println!(" [Config] 配置已加载");
    if config.walker.use_ripgrep_walker {
        println!(" [Config] 使用 ripgrep 遍历模式 (respect .gitignore)");
    } else {
        println!(" [Config] 使用标准遍历模式");
    }
    config
});

// ============== 兼容旧代码的常量访问 ==============

/// 获取预览最大长度
pub fn preview_max_length() -> usize {
    CONFIG.display.preview_max_length
}

/// 获取句子搜索起始位置
pub fn sentence_search_start() -> usize {
    CONFIG.display.sentence_search_start
}

/// 获取监控目录路径
pub fn watch_path() -> &'static str {
    &CONFIG.paths.watch_path
}

/// 获取存储路径
pub fn storage_path() -> &'static str {
    &CONFIG.paths.storage_path
}

/// 获取缓存路径
pub fn cache_path() -> &'static str {
    &CONFIG.paths.cache_path
}

/// 获取模型路径
pub fn model_path() -> &'static str {
    &CONFIG.paths.model_path
}

// 保留旧常量以兼容（标记为废弃）
#[deprecated(note = "请使用 CONFIG.display.preview_max_length 或 preview_max_length()")]
pub const PREVIEW_MAX_LENGTH: usize = 200;

#[deprecated(note = "请使用 CONFIG.display.sentence_search_start 或 sentence_search_start()")]
pub const SENTENCE_SEARCH_START: usize = 50;

#[deprecated(note = "请使用 CONFIG.paths.watch_path 或 watch_path()")]
pub const WATCH_PATH: &str = "./docs";

#[deprecated(note = "请使用 CONFIG.paths.storage_path 或 storage_path()")]
pub const STORAGE_PATH: &str = "./storage";

#[deprecated(note = "请使用 CONFIG.paths.cache_path 或 cache_path()")]
pub const CACHE_PATH: &str = "./cache";

// ============== 新架构配置封装 ==============

/// 索引配置（用于 SearchEngine）
#[derive(Debug, Clone)]
pub struct IndexConfig {
    pub storage_path: Option<String>,
    pub writer_memory: usize,
}

/// AI 配置（用于 SearchEngine）
#[derive(Debug, Clone)]
pub struct AiEngineConfig {
    pub model_path: Option<String>,
    pub keyword_count: usize,
}

/// 统一配置封装（新架构）
#[derive(Debug, Clone)]
pub struct Config {
    pub index_config: IndexConfig,
    pub ai_config: AiEngineConfig,
}

impl Config {
    /// 从全局配置创建
    pub fn global() -> &'static Self {
        static INSTANCE: once_cell::sync::OnceCell<Config> = once_cell::sync::OnceCell::new();
        INSTANCE.get_or_init(|| {
            Config {
                index_config: IndexConfig {
                    storage_path: Some(CONFIG.paths.storage_path.clone()),
                    writer_memory: CONFIG.performance.index_writer_memory,
                },
                ai_config: AiEngineConfig {
                    model_path: Some(CONFIG.paths.model_path.clone()),
                    keyword_count: CONFIG.ai.keyword_count,
                },
            }
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Config::global().clone()
    }
}