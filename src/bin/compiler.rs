//! 小程序 → HTML 静态工程编译器
//!
//! 把整个小程序编译成一套可直接在浏览器打开的静态 HTML 工程：
//! - 每个页面 → 一个自包含 HTML（内联 base.css + app.wxss + page.wxss，body 为编译后的 HTML）
//! - 生成 index.html 汇总所有页面入口
//! 页面初始数据由内置 JS 运行时执行 `Page().data` + `onLoad` 得到（静态快照）。
//!
//! 这是「小程序源码 → HTML 工程源码」的编译能力，也是后续「编译到 Android/iOS
//! 原生源码」的中间层基础。
//!
//! 运行：cargo run --bin mini-compiler [小程序根目录] [输出目录]
//!   默认：sample-app -> dist-html

use mini_render::runtime::MiniApp;
use mini_render::parser::WxmlParser;
use mini_render::transpile::{wxml_to_html, wxss_to_css, make_html_doc, base_css};
use std::collections::HashMap;

const WIDTH: u32 = 375;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let app_root = args.get(1).cloned().unwrap_or_else(|| "sample-app".to_string());
    let out_dir = args.get(2).cloned().unwrap_or_else(|| "dist-html".to_string());

    println!("📦 编译小程序 -> HTML 工程");
    println!("   源: {}\n   目标: {}", app_root, out_dir);

    // 读取 app.json 页面列表
    let app_json = std::fs::read_to_string(format!("{}/app.json", app_root))
        .map_err(|e| format!("读取 app.json 失败: {}", e))?;
    let cfg: serde_json::Value = serde_json::from_str(&app_json).map_err(|e| format!("app.json: {}", e))?;
    let pages: Vec<String> = cfg.get("pages")
        .and_then(|p| p.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    if pages.is_empty() {
        return Err("app.json 未配置 pages".into());
    }

    let app_css = std::fs::read_to_string(format!("{}/app.wxss", app_root))
        .map(|s| wxss_to_css(&s)).unwrap_or_default();
    let app_js = std::fs::read_to_string(format!("{}/app.js", app_root)).unwrap_or_default();

    std::fs::create_dir_all(&out_dir).map_err(|e| format!("创建输出目录失败: {}", e))?;

    let mut index_links = String::new();
    let mut ok_count = 0;

    for page in &pages {
        // 每个页面用独立运行时取初始数据，避免相互污染
        let mut app = MiniApp::new(WIDTH, 667)?;
        app.init()?;
        if !app_js.is_empty() {
            app.load_script(&app_js).ok();
            app.eval("if (typeof __app!=='undefined' && __app && __app.onLaunch) __app.onLaunch({})").ok();
        }
        let prefix = format!("{}/{}", app_root, page);
        let js = std::fs::read_to_string(format!("{}.js", prefix)).unwrap_or_default();
        let wxml_src = match std::fs::read_to_string(format!("{}.wxml", prefix)) {
            Ok(s) => s,
            Err(_) => { eprintln!("  跳过 {}（无 wxml）", page); continue; }
        };
        let wxss_src = std::fs::read_to_string(format!("{}.wxss", prefix)).unwrap_or_default();

        if !js.is_empty() {
            app.load_script(&js).ok();
            app.eval("if (__currentPage && __currentPage.onLoad) __currentPage.onLoad({})").ok();
            app.eval("if (__currentPage && __currentPage.onShow) __currentPage.onShow()").ok();
            app.eval("if (__currentPage && __currentPage.onReady) __currentPage.onReady()").ok();
        }
        let data = app.eval("__getPageData()").ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        let nodes = WxmlParser::new(&wxml_src).parse().map_err(|e| format!("{} WXML: {}", page, e))?;
        let body = wxml_to_html(&nodes, &data);
        let css = format!("{}\n{}", app_css, wxss_to_css(&wxss_src));
        let doc = make_html_doc(page, &css, &body, WIDTH);

        // 输出文件名：pages_index_index.html
        let file = format!("{}.html", page.replace('/', "_"));
        std::fs::write(format!("{}/{}", out_dir, file), doc)
            .map_err(|e| format!("写入 {} 失败: {}", file, e))?;
        index_links.push_str(&format!(
            "<li><a href=\"{}\">{}</a></li>",
            file, page
        ));
        ok_count += 1;
        println!("  ✓ {} -> {}", page, file);
    }

    // 生成 index 汇总页
    let index = format!(
        "<!doctype html><html lang=\"zh\"><head><meta charset=\"utf-8\"><title>Mini App · HTML 工程</title>\
<style>{base} body{{background:#20232a;color:#eee;font-family:-apple-system,system-ui,sans-serif;padding:32px}} \
h1{{color:#9ad;font-size:20px}} ul{{line-height:2}} a{{color:#07c160;font-size:16px}}</style></head>\
<body><h1>Mini App · 编译为 HTML 工程</h1><p>共 {n} 个页面，点击查看：</p><ul>{links}</ul></body></html>",
        base = base_css(), n = ok_count, links = index_links,
    );
    std::fs::write(format!("{}/index.html", out_dir), index).map_err(|e| e.to_string())?;

    println!("\n✅ 完成：{} 个页面 -> {}/  （打开 {}/index.html 查看）", ok_count, out_dir, out_dir);
    Ok(())
}
