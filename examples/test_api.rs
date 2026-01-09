// 测试 API 响应类型
use ai_search_demo::api::{SearchRequest, SearchResponse, SearchResult, QueryInfo, FileMetadataResponse};
use ai_search_demo::query::filter::PathMatcher;
use ai_search_demo::query::PathFilter;

fn main() {
    println!("=== API 类型功能测试 ===\n");
    
    // 测试1: SearchRequest 创建
    let request = SearchRequest::new("操作系统 --type=pdf")
        .with_limit(10)
        .with_offset(0);
    println!("1. SearchRequest:");
    println!("   query: {:?}", request.query);
    println!("   limit: {}, offset: {}", request.limit, request.offset);
    println!("   use_ai: {}\n", request.use_ai);
    
    // 测试2: SearchResult 创建
    let result = SearchResult::new("/home/user/docs/report.pdf", 0.95)
        .with_title("年度报告 2024")
        .with_metadata(FileMetadataResponse::new("pdf", 1024 * 1024)
            .with_times(Some(1704067200), Some(1704153600), Some(1704240000)));
    println!("2. SearchResult:");
    println!("   path: {}", result.path);
    println!("   filename: {}", result.filename);
    println!("   parent: {}", result.parent_path);
    println!("   score: {}", result.score);
    println!("   title: {:?}", result.title);
    println!("   metadata.file_size_display: {}\n", result.metadata.file_size_display);
    
    // 测试3: SearchResponse 构建
    let query_info = QueryInfo::new("操作系统 --type=pdf", "操作系统")
        .with_keywords(vec!["操作系统".to_string(), "内核".to_string()])
        .add_filter("类型: pdf");
    
    let response = SearchResponse::new(query_info, vec![result], 1)
        .with_pagination(0, 10)
        .with_took(15);
    
    println!("3. SearchResponse:");
    println!("   total: {}", response.total);
    println!("   took_ms: {}ms", response.took_ms);
    println!("   query.keywords: {:?}", response.query.keywords);
    println!("   query.applied_filters: {:?}", response.query.applied_filters);
    println!("   pagination: offset={}, limit={}, has_more={}\n", 
        response.pagination.offset, response.pagination.limit, response.pagination.has_more);
    
    // 测试4: JSON 序列化
    let json = serde_json::to_string_pretty(&response).unwrap();
    println!("4. JSON 序列化:");
    println!("{}\n", json);
    
    // 测试5: PathMatcher
    println!("5. PathMatcher 测试:");
    let filters = vec![
        PathFilter::include("/home/user/docs/*"),
        PathFilter::exclude("/home/user/docs/temp/*"),
    ];
    let matcher = PathMatcher::new(&filters);
    
    let test_paths = [
        "/home/user/docs/report.pdf",
        "/home/user/docs/temp/draft.txt",
        "/home/user/other/file.txt",
    ];
    
    for path in &test_paths {
        let matched = matcher.matches(path);
        println!("   {} -> {}", path, if matched { "✓ 包含" } else { "✗ 排除" });
    }
    
    println!("\n=== 测试完成 ===");
}
