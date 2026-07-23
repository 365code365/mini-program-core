use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::renderer::WxmlRenderer;
use mini_render::renderer::components::{ensure_canvas_context, execute_canvas_draw};
use mini_render::{Canvas, Color};
use serde_json::{json, Value};

const W: u32 = 375;
const H: u32 = 667;
const SCALE: f32 = 2.0;

fn render(name: &str, bg: u32, wxml: &str, wxss: &str, data: Value) {
    let nodes = WxmlParser::new(wxml).parse().expect("wxml");
    let ss = WxssParser::new(wxss).parse().expect("wxss");
    let r = WxmlRenderer::new_with_scale(ss, W as f32, H as f32, SCALE);
    // 按内容高度自适应画布：保证截图完整、无裁切、无大片空白
    let content_h = r.measure_content_height(&nodes, &data).max(240.0);
    let mut r = r;
    let cw = (W as f32 * SCALE) as u32;
    let ch = (content_h * SCALE).ceil() as u32;
    let mut canvas = Canvas::new(cw, ch);
    canvas.clear(Color::from_hex(bg));
    r.render(&mut canvas, &nodes, &data);
    let path = format!("doc/gallery/{}.png", name);
    canvas.save_png(&path).expect("save");
    println!("  ✓ {} ({}x{})", path, cw, ch);
}


fn render_screen(name: &str, bg: u32, wxml: &str, wxss: &str, data: Value) {
    let nodes = WxmlParser::new(wxml).parse().expect("wxml");
    let ss = WxssParser::new(wxss).parse().expect("wxss");
    let mut r = WxmlRenderer::new_with_scale(ss, W as f32, H as f32, SCALE);
    let cw = (W as f32 * SCALE) as u32;
    let ch = (H as f32 * SCALE) as u32;
    let mut canvas = Canvas::new(cw, ch);
    canvas.clear(Color::from_hex(bg));
    r.render(&mut canvas, &nodes, &data);
    let path = format!("doc/gallery/{}.png", name);
    canvas.save_png(&path).expect("save");
    println!("  ✓ {} ({}x{})", path, cw, ch);
}

// 通用样式片段
fn common() -> &'static str {
    r#"
    .page{ padding:24rpx; }
    .card{ background-color:#ffffff; border-radius:20rpx; padding:28rpx; margin-bottom:20rpx; box-shadow:0 4rpx 16rpx rgba(0,0,0,0.06); }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .between{ justify-content:space-between; }
    .col{ display:flex; flex-direction:column; }
    .grow{ flex:1; }
    .title{ font-size:36rpx; color:#1a1a1a; font-weight:bold; }
    .sub{ font-size:26rpx; color:#999999; }
    .price{ font-size:34rpx; color:#ff5000; font-weight:bold; }
    .primary{ background-color:#07c160; color:#ffffff; border-radius:44rpx; padding:20rpx; text-align:center; font-size:30rpx; }
    .avatar{ width:88rpx; height:88rpx; border-radius:44rpx; background-color:#c7e0ff; }
    .thumb{ width:140rpx; height:140rpx; border-radius:16rpx; background-color:#e8eaf0; }
    .tag{ background-color:#fff0e8; color:#ff5000; font-size:22rpx; padding:6rpx; border-radius:8rpx; margin-right:12rpx; }
    "#
}

