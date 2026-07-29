//! 内存分项报告：把「进程常驻内存（RSS）」按加载阶段逐段量出来。
//!
//! 目的是别再凭感觉猜内存去哪了。用法：
//! ```bash
//! cargo run --release --example mem_report
//! cargo run --release --example mem_report -- sample/tea-app pages/index/index
//! ```
//! 输出每一步之后的 RSS 以及相对上一步的增量，最后给一份汇总。

use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::renderer::WxmlRenderer;
use mini_render::{Canvas, Color};

/// 当前进程的常驻内存（MB）。macOS/Linux 都能用 `ps`。
fn rss_mb() -> f32 {
    let pid = std::process::id().to_string();
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid])
        .output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .trim()
            .parse::<f32>()
            .map(|kb| kb / 1024.0)
            .unwrap_or(0.0),
        Err(_) => 0.0,
    }
}

struct Stage {
    last: f32,
    rows: Vec<(String, f32, f32)>,
}

impl Stage {
    fn new() -> Self {
        let last = rss_mb();
        println!("{:<44} {:>10} {:>10}", "阶段", "RSS(MB)", "增量(MB)");
        println!("{}", "-".repeat(66));
        println!("{:<44} {:>10.1} {:>10}", "进程启动", last, "-");
        Stage { last, rows: Vec::new() }
    }

    fn mark(&mut self, name: &str) {
        let now = rss_mb();
        let delta = now - self.last;
        println!("{:<44} {:>10.1} {:>+10.1}", name, now, delta);
        self.rows.push((name.to_string(), now, delta));
        self.last = now;
    }

    fn summary(&self) {
        println!("{}", "-".repeat(66));
        let mut rows = self.rows.clone();
        rows.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        println!("增量排行：");
        for (name, _, delta) in rows.iter().take(6) {
            println!("  {:<42} {:>+8.1} MB", name, delta);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let app = args.first().cloned();
    let route = args.get(1).cloned();

    let mut s = Stage::new();

    // 1) 系统主字体（进程级单例，引擎里所有渲染器共用这一份）
    let sys = mini_render::text::shared_fonts().expect("系统字体");
    s.mark("系统主字体 shared_fonts()（常规字面）");
    std::hint::black_box(&sys);

    // 2) 粗体字面（懒加载：第一次真的要画粗体时才解析）
    let has_bold = sys.has_bold_face();
    s.mark(&format!("首次用到粗体字面（存在={has_bold}）"));

    // 3) 一个大字体集合（宋体 63.8MB，tea-app 的字体栈会落到它）
    let songti = "/System/Library/Fonts/Supplemental/Songti.ttc";
    if std::path::Path::new(songti).exists() {
        let r = mini_render::text_family::renderer_for_family(Some("\"Songti SC\", serif"));
        s.mark("加载 Songti.ttc（63.8MB 字体集合）");
        std::hint::black_box(&r);
    }

    // 4) 画一屏纯文字（走字形位图缓存）
    {
        let wxml = r#"<view class="p"><text class="t">{{s}}</text></view>"#;
        let css = ".p{ padding:20px; } .t{ font-size:16px; line-height:24px; }";
        let long: String = (0..1200)
            .map(|i| char::from_u32(0x4E00 + (i % 900)).unwrap_or('字'))
            .collect();
        let sheet = WxssParser::new(css).parse().unwrap_or_default();
        let nodes = WxmlParser::new(wxml).parse().unwrap();
        let mut r = WxmlRenderer::new_with_scale(sheet, 375.0, 667.0, 2.0);
        let mut canvas = Canvas::new(750, 2400);
        canvas.clear(Color::WHITE);
        r.render(&mut canvas, &nodes, &serde_json::json!({ "s": long }));
        s.mark("渲染 1200 个不同汉字（字形位图缓存）");
    }

    // 5) 一块页面画布（750 × 4000 × RGBA）
    {
        let c = Canvas::new(750, 4000);
        s.mark("一块 750x4000 的页面画布");
        std::hint::black_box(&c);
    }

    // 6) 指定的小程序页面（可选）
    if let (Some(app), Some(route)) = (app, route) {
        println!("\n（下面这段需要窗体环境，这里只做资源加载：{app} / {route}）");
        let root = mini_render::app_dir::resolve(&app);
        let wxml_path = root.join(&route).with_extension("wxml");
        let wxss_path = root.join(&route).with_extension("wxss");
        let wxml = std::fs::read_to_string(&wxml_path).unwrap_or_default();
        let wxss = std::fs::read_to_string(&wxss_path).unwrap_or_default();
        if !wxml.is_empty() {
            let sheet = WxssParser::new(&wxss).parse().unwrap_or_default();
            let nodes = WxmlParser::new(&wxml).parse().unwrap_or_default();
            let mut r = WxmlRenderer::new_with_scale(sheet, 375.0, 667.0, 2.0);
            let mut canvas = Canvas::new(750, 1334);
            canvas.clear(Color::WHITE);
            r.render(&mut canvas, &nodes, &serde_json::json!({}));
            s.mark(&format!("渲染 {route}"));
        }
    }

    s.summary();
    println!("\n图片缓存：{}", mini_render::renderer::components::image_cache_report());
}
