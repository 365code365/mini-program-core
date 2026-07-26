//! Mini App 运行时主程序

use mini_render::runtime::MiniApp;
use mini_render::parser::{WxmlParser, WxssParser, TemplateEngine};
use mini_render::renderer::WxmlRenderer;
use mini_render::{Canvas, Color};
use serde_json::json;
use std::time::{Duration, Instant};

fn main() -> Result<(), String> {
    println!("🚀 Mini App Engine Starting...");
    
    // 创建应用
    let mut app = MiniApp::new(375, 667)?;
    app.init()?;
    
    // 加载页面 JS
    let page_js = include_str!("../../sample/sample-app/pages/index/index.js");
    app.load_script(page_js)?;
    // 触发 onLoad 生命周期
    app.eval("if (__currentPage && __currentPage.onLoad) __currentPage.onLoad({})").ok();
    
    // 解析 WXML
    let wxml_content = include_str!("../../sample/sample-app/pages/index/index.wxml");
    let mut wxml_parser = WxmlParser::new(wxml_content);
    let wxml_nodes = wxml_parser.parse().map_err(|e| format!("WXML parse error: {}", e))?;
    println!("✅ WXML parsed: {} root nodes", wxml_nodes.len());
    
    // 解析 WXSS
    let wxss_content = include_str!("../../sample/sample-app/pages/index/index.wxss");
    let mut wxss_parser = WxssParser::new(wxss_content);
    let stylesheet = wxss_parser.parse().map_err(|e| format!("WXSS parse error: {}", e))?;
    println!("✅ WXSS parsed: {} rules", stylesheet.rules.len());
    
    // 创建渲染器
    let mut renderer = WxmlRenderer::new(stylesheet, 375.0, 667.0);
    
    // 页面数据 —— 使用页面 JS 中的真实 data（通过 __getPageData 读取）
    let page_data = app.eval("__getPageData()")
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .unwrap_or_else(|| json!({}));
    println!("✅ Page data loaded: hotProducts={}, newProducts={}",
        page_data.get("hotProducts").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
        page_data.get("newProducts").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0));
    
    // 创建画布
    let mut canvas = Canvas::new(375, 667);
    canvas.clear(Color::from_hex(0xF5F5F5));
    
    // 渲染 WXML
    renderer.render(&mut canvas, &wxml_nodes, &page_data);
    
    // 获取事件绑定
    let bindings = renderer.get_event_bindings();
    println!("✅ Event bindings: {}", bindings.len());
    for binding in bindings {
        println!("   - {} -> {} at ({}, {}, {}, {})", 
            binding.event_type, 
            binding.handler,
            binding.bounds.x,
            binding.bounds.y,
            binding.bounds.width,
            binding.bounds.height
        );
    }
    
    // 保存渲染结果
    canvas.save_png("mini_app_ui.png")?;
    println!("\n✅ UI rendered to mini_app_ui.png");
    
    // 模拟点击交互
    println!("\n--- Simulating interactions ---");
    
    // 模拟点击 +1 按钮 3 次
    for i in 0..3 {
        if let Some(binding) = renderer.hit_test(75.0, 390.0) {
            println!("\n[Click {}] {} -> {}", i + 1, binding.event_type, binding.handler);
            
            // 调用 JS 事件处理函数
            let _ = app.eval(&format!(
                "__callPageMethod('{}', {{}})",
                binding.handler
            ));
        }
    }
    
    // 获取更新后的页面数据
    if let Ok(data) = app.eval("__getPageData()") {
        println!("\n📊 Final page data:");
        // 解析并美化输出
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(count) = json.get("count") {
                println!("   count = {}", count);
            }
        }
    }
    
    // 打印 JS 输出
    if let Ok(output) = app.eval("__print_buffer.join('\\n')") {
        if !output.is_empty() && output != "undefined" && output != "" {
            println!("\n--- JS Console Output ---");
            println!("{}", output);
        }
    }
    
    // 使用更新后的数据重新渲染
    println!("\n🔄 Re-rendering with updated data...");
    if let Ok(data_str) = app.eval("__getPageData()") {
        if let Ok(updated_data) = serde_json::from_str::<serde_json::Value>(&data_str) {
            canvas.clear(Color::from_hex(0xF5F5F5));
            renderer.render(&mut canvas, &wxml_nodes, &updated_data);
            canvas.save_png("mini_app_ui_updated.png")?;
            println!("✅ Updated UI rendered to mini_app_ui_updated.png");
        }
    }
    
    // 启动应用
    app.start()?;
    
    // 简单的主循环
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        app.update()?;
        std::thread::sleep(Duration::from_millis(16));
    }
    
    app.stop();
    println!("\n👋 Mini App Engine Stopped");
    
    Ok(())
}
