//! 性能基准：渲染器创建成本与整页渲染成本。
use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::renderer::WxmlRenderer;
use mini_render::{Canvas, Color};
use serde_json::json;
use std::time::Instant;

fn main() {
    // 1) 渲染器创建（此前每次都会重新加载系统字体）
    for i in 0..5 {
        let ss = WxssParser::new("").parse().unwrap();
        let t = Instant::now();
        let _r = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 2.0);
        eprintln!("renderer_new #{i}: {:?}", t.elapsed());
    }

    // 2) 整页渲染（真实页面：分类页）
    let root = "sample-app";
    let route = "pages/category/category";
    let wxml_src = std::fs::read_to_string(format!("{root}/{route}.wxml")).unwrap()
        .replace("/assets/", "sample-app/assets/");
    let wxss_src = std::fs::read_to_string(format!("{root}/{route}.wxss")).unwrap();
    let nodes = WxmlParser::new(&wxml_src).parse().unwrap();
    let data = json!({
        "categories": (0..16).map(|i| json!({"id": i, "name": format!("分类{i}"), "count": i})).collect::<Vec<_>>(),
        "currentCategory": 0, "currentCategoryName": "热销",
        "products": (0..8).map(|i| json!({"id": i, "name": format!("商品{i}"), "desc": "描述文本", "price": 18 + i, "icon": "success", "color": "#FF6B35", "quantity": 0})).collect::<Vec<_>>(),
        "totalCount": 0, "totalPrice": "0.00", "cartItems": [], "showCartPopup": false
    });
    let ss = WxssParser::new(&wxss_src).parse().unwrap();
    let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 2.0);
    let mut canvas = Canvas::new(750, 1334);
    for i in 0..5 {
        canvas.clear(Color::WHITE);
        let t = Instant::now();
        r.render(&mut canvas, &nodes, &data);
        eprintln!("render_page #{i}: {:?}", t.elapsed());
    }
    bench_js();
}

// 附加：JS 运行时初始化与页面脚本加载成本（切页路径的另一半）
#[allow(dead_code)]
fn bench_js() {
    use mini_render::runtime::MiniApp;
    let app_js = std::fs::read_to_string("sample-app/app.js").unwrap_or_default();
    let page_js = std::fs::read_to_string("sample-app/pages/category/category.js").unwrap_or_default();
    for i in 0..3 {
        let t = Instant::now();
        let mut app = MiniApp::new(375, 667).unwrap();
        app.init().unwrap();
        eprintln!("js_new_init #{i}: {:?}", t.elapsed());
        let t = Instant::now();
        app.load_script(&app_js).ok();
        app.load_script(&page_js).ok();
        app.eval("if (__currentPage && __currentPage.onLoad) __currentPage.onLoad({})").ok();
        let _ = app.eval("__getPageData()");
        eprintln!("js_load_page #{i}: {:?}", t.elapsed());
    }
}
