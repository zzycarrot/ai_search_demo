// 测试查询解析功能
use ai_search_demo::query::QueryParser;

fn main() {
    let parser = QueryParser::new();
    
    println!("=== 查询解析器功能测试 ===\n");
    
    // 测试1: 简单查询
    let q1 = parser.parse("操作系统内核");
    println!("1. 简单查询: \"操作系统内核\"");
    println!("   解析文本: {:?}", q1.text);
    println!("   过滤条件: {:?}\n", q1.filters);
    
    // 测试2: 带路径过滤
    let q2 = parser.parse("内核代码 --path=/usr/src/*");
    println!("2. 路径过滤: \"内核代码 --path=/usr/src/*\"");
    println!("   解析文本: {:?}", q2.text);
    println!("   路径过滤: {:?}\n", q2.filters.paths);
    
    // 测试3: 带时间过滤
    let q3 = parser.parse("文档 --time=7d");
    println!("3. 时间过滤: \"文档 --time=7d\"");
    println!("   解析文本: {:?}", q3.text);
    println!("   时间过滤: {:?}\n", q3.filters.time);
    
    // 测试4: 带类型过滤
    let q4 = parser.parse("报告 --type=pdf,txt");
    println!("4. 类型过滤: \"报告 --type=pdf,txt\"");
    println!("   解析文本: {:?}", q4.text);
    println!("   类型过滤: {:?}\n", q4.filters.file_types);
    
    // 测试5: 带大小过滤
    let q5 = parser.parse("大文件 --size=>1mb");
    println!("5. 大小过滤: \"大文件 --size=>1mb\"");
    println!("   解析文本: {:?}", q5.text);
    println!("   大小过滤: {:?}\n", q5.filters.size);
    
    // 测试6: 复合过滤
    let q6 = parser.parse("操作系统 --path=/docs/* --type=pdf --time=30d --sort=modified");
    println!("6. 复合过滤:");
    println!("   输入: \"操作系统 --path=/docs/* --type=pdf --time=30d --sort=modified\"");
    println!("   解析文本: {:?}", q6.text);
    println!("   路径: {:?}", q6.filters.paths);
    println!("   类型: {:?}", q6.filters.file_types);
    println!("   时间: {:?}\n", q6.filters.time);
    
    // 测试7: 排除路径
    let q7 = parser.parse("代码 --exclude-path=*/test/*");
    println!("7. 排除路径: \"代码 --exclude-path=*/test/*\"");
    println!("   解析文本: {:?}", q7.text);
    println!("   路径过滤: {:?}\n", q7.filters.paths);
    
    println!("=== 测试完成 ===");
}
