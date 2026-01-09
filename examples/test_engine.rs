// 测试 SearchEngine
use ai_search_demo::engine::SearchEngineBuilder;
use ai_search_demo::api::SearchRequest;
use std::path::Path;
use std::fs;

fn main() {
    println!("=== SearchEngine 功能测试 ===\n");
    
    // 使用临时目录
    let temp_dir = std::env::temp_dir().join("ai_search_test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();
    
    println!("1. 创建 SearchEngine (无 AI)...");
    let engine = SearchEngineBuilder::new()
        .with_storage_path(&temp_dir)
        .with_ai(false)  // 禁用 AI 加快测试
        .build();
    
    match engine {
        Ok(engine) => {
            println!("   ✓ 引擎创建成功\n");
            
            // 测试索引统计
            println!("2. 获取索引统计...");
            match engine.stats() {
                Ok(stats) => {
                    println!("   ✓ 文档数: {}", stats.total_documents);
                    println!("   ✓ 索引大小: {} 字节\n", stats.index_size_bytes);
                }
                Err(e) => println!("   ✗ 统计失败: {}\n", e),
            }
            
            // 测试搜索（空索引）
            println!("3. 测试空索引搜索...");
            let request = SearchRequest::new("test query");
            match engine.search(&request) {
                Ok(response) => {
                    println!("   ✓ 搜索成功");
                    println!("   ✓ 结果数: {}", response.total);
                    println!("   ✓ 耗时: {}ms\n", response.took_ms);
                }
                Err(e) => println!("   ✗ 搜索失败: {}\n", e),
            }
            
            // 创建测试文件并索引
            println!("4. 创建并索引测试文件...");
            let test_file = temp_dir.join("test.txt");
            fs::write(&test_file, "这是一个关于操作系统内核的测试文档。包含 Linux 内核编程相关内容。").unwrap();
            
            match engine.index_file(&test_file) {
                Ok(resp) => {
                    println!("   ✓ 索引成功: {}", resp.path);
                    println!("   ✓ 耗时: {}ms\n", resp.took_ms);
                }
                Err(e) => println!("   ✗ 索引失败: {}\n", e),
            }
            
            // 再次搜索
            println!("5. 搜索已索引内容...");
            let request = SearchRequest::new("操作系统");
            match engine.search(&request) {
                Ok(response) => {
                    println!("   ✓ 搜索成功");
                    println!("   ✓ 结果数: {}", response.total);
                    for result in &response.results {
                        println!("   ✓ 找到: {} (score: {:.2})", result.filename, result.score);
                    }
                    println!();
                }
                Err(e) => println!("   ✗ 搜索失败: {}\n", e),
            }
            
            // 测试带过滤的搜索
            println!("6. 带过滤条件搜索...");
            let request = SearchRequest::new("内核 --type=txt");
            match engine.search(&request) {
                Ok(response) => {
                    println!("   ✓ 搜索成功");
                    println!("   ✓ 查询文本: {:?}", response.query.search_text);
                    println!("   ✓ 应用过滤: {:?}", response.query.applied_filters);
                    println!("   ✓ 结果数: {}\n", response.total);
                }
                Err(e) => println!("   ✗ 搜索失败: {}\n", e),
            }
            
            // 测试删除
            println!("7. 删除文件索引...");
            match engine.delete_file(&test_file) {
                Ok(_) => println!("   ✓ 删除成功\n"),
                Err(e) => println!("   ✗ 删除失败: {}\n", e),
            }
            
            // 验证删除
            println!("8. 验证删除...");
            match engine.is_indexed(&test_file) {
                Ok(indexed) => {
                    if indexed {
                        println!("   ✗ 文件仍在索引中\n");
                    } else {
                        println!("   ✓ 文件已从索引中移除\n");
                    }
                }
                Err(e) => println!("   ✗ 检查失败: {}\n", e),
            }
        }
        Err(e) => {
            println!("   ✗ 引擎创建失败: {}\n", e);
        }
    }
    
    // 清理
    let _ = fs::remove_dir_all(&temp_dir);
    
    println!("=== 测试完成 ===");
}
