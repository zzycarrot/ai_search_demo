# 智能文档搜索引擎 (AI-Enhanced Document Search)

基于 Rust 开发的高性能本地文档搜索引擎。集成了 **Tantivy** 倒排索引与 **BERT** 深度学习模型，实现了从“传统关键词搜索”到“AI 语义理解”的跨越。支持实时文件监控、增量索引、自动语义打标签及自然语言查询。

## ✨ 主要特性

* **🧠 AI 语义驱动**:
* **自动打标签 (Auto-Tagging)**: 使用 KeyBERT 算法分析文档语义，自动生成核心关键词标签。
* **意图识别 (Query Refinement)**: 理解用户自然语言查询（如“找一下关于...的文档”），自动提取核心检索词。


* **🚀 高性能架构**:
* **多线程模型**: 后台监听线程 + 前台交互线程分离。
* **内存映射 (Mmap)**: 基于 Tantivy 的 MmapDirectory，实现极速读写。


* **⚡️ 增量索引**:
* **智能检测**: 基于文件时间戳 (Timestamp) 记录。
* **极速启动**: 重启时只处理新增或修改过的文件，跳过未变动文件。


* **📂 全面支持**:
* **多格式**: `.txt`, `.md`, `.pdf` (自动提取文本)。
* **中文优化**: 集成 Jieba 分词，精准切分中文词汇。


* **🔄 自动化**:
* **实时监听**: 文件拖入即刻索引（支持防抖去重）。
* **持久化**: 数据永久保存至磁盘，ACID 事务保证。



## 系统要求

* Linux / macOS
* Rust 工具链
* 网络连接（首次运行需下载约 200MB 的量化 BERT 模型）

## 安装和使用

### 1. 编译项目

```bash
cargo build --release

```

### 2. 准备目录

程序启动时会自动创建，手动创建也可以：

```bash
mkdir -p docs storage

```

### 3. 启动全栈服务

```bash
cargo run

```

服务启动后将显示 AI 模型加载及增量扫描状态：

```text
 [AI] 正在加载 BERT 模型 (首次运行需下载)...
 [AI] 模型加载完毕！
--- 文件搜索系统 ---
 [后台] 正在监控: "./docs"
 [前台] 输入关键词进行搜索 (输入 'quit' 退出)
 [后台] 正在扫描现有文件...
 正在解析: "./docs/paper.pdf"
 [AI] 生成标签: ["神经网络", "深度学习", "算法"]
 [后台] 新文件已索引: paper (输入搜索词继续)
 [后台] 初始索引完成，共处理 1 个文件
>

```

### 4. 实时智能搜索

支持直接输入自然语言，AI 会自动优化查询：

```bash
> 帮我找一下关于机器学习的资料
 🧠 [AI] 意图识别: '帮我找一下关于机器学习的资料' -> '机器学习'
 [文档标题] 机器学习导论.pdf
    路径: docs/机器学习导论.pdf
    标签: 监督学习 无监督学习 算法

> quit

```

### 5. 后台自动索引

保持程序运行，在另一个终端添加文档：

```bash
cp new_research.pdf docs/

```

主程序会自动响应：

```text
 正在解析: "./docs/new_research.pdf"
 [AI] 生成标签: ["Transformer", "NLP", "Attention"]
 [后台] 新文件已索引: new_research (输入搜索词继续)

```

## 🔍 核心架构

### 多线程与 AI 协作

```mermaid
graph TD
    User[用户输入] --> MainThread[主线程]
    FileSys[文件系统] --> WatcherThread[后台线程]
    
    subgraph MainThread
        Input[接收查询] --> Refine[AI 意图识别]
        Refine --> Search[Tantivy 搜索]
        Search --> Result[显示结果]
    end
    
    subgraph WatcherThread
        Event[监听变化] --> Extract[文本提取]
        Extract --> Embed[BERT 语义分析]
        Embed --> Index[写入索引]
        Index --> Commit[持久化存储]
    end
    
    MainThread <-->|共享 Index (MVCC)| WatcherThread

```

## 📁 目录结构

```text
.
├── docs/          # [监控] 放入文档即可自动索引
├── storage/       # [数据] 索引数据库 (可随时删除重建)
├── src/
│   ├── main.rs    # 主程序 (线程调度与交互)
│   ├── ai.rs      # AI 模块 (BERT 模型封装)
│   ├── indexer.rs # 索引逻辑 (增量扫描、写入)
│   ├── search.rs  # 搜索逻辑 (查询解析)
│   ├── extract.rs # 提取逻辑 (PDF/文本解析)
│   ├── models.rs  # 数据结构
│   └── config.rs  # 配置常量
└── Cargo.toml     # 依赖配置

```

## 🏗️ 技术栈

1. **NLP / AI**: `fastembed` (BERT 模型), `jieba-rs` (关键词提取)
2. **搜索引擎**: `tantivy` (倒排索引), `tantivy-jieba` (中文分词)
3. **文件监控**: `notify` (Inotify/FSEvents)
4. **文档解析**: `pdf-extract`
5. **并发模型**: Rust `std::thread`, `crossbeam-channel`

## ⚙️ 工作流程详情

1. **启动阶段**
* 加载 BERT 模型。
* 初始化 Tantivy 索引（Schema 包含 `title`, `body`, `path`, `tags`, `timestamp`）。
* **增量扫描**: 遍历 `docs/`，对比文件修改时间与索引记录，仅处理变更文件。


2. **处理阶段 (后台)**
* **提取**: 解析 PDF/文本内容。
* **分析**: BERT 计算文档向量，提取 Top-3 语义关键词 (Tags)。
* **索引**: 存入文档内容、路径、标签及时间戳。


3. **查询阶段 (前台)**
* **优化**: AI 分析查询语句，去除停用词，提取核心意图。
* **检索**: 在 `title`, `body`, `tags` 字段中进行联合搜索。



## 故障排除

* **服务启动慢**: 首次运行需下载模型，请检查网络。后续启动为秒级。
* **文件未索引**: 检查文件是否在子文件夹中（支持递归），或检查是否为支持的格式。
* **Schema 错误**: 若修改了代码中的索引结构，请删除 `storage/` 目录并重启，让程序重新构建索引。

## 🤝 贡献

欢迎提交 Issue 和 Pull Request 改进 AI 模型效果或支持更多格式！
