//! 动图（GIF）能力测试
//!
//! 覆盖：多帧 GIF 会被识别为动图、随时间推进换帧、渲染结果确实发生变化；
//! 静态图（JPG/PNG）不会被误判为动图。

use crate::parser::{WxmlParser, WxssParser};
use crate::renderer::components::{animation_total_ms, is_animated};
use crate::renderer::WxmlRenderer;
use crate::{Canvas, Color};
use serde_json::json;

const GIF: &str = "sample-app/assets/loading.gif";

fn render_gif(src: &str) -> Vec<u8> {
    let css = ".g{ width:60px; height:60px; }";
    let wxml = format!(
        r#"<view><image class="g" src="{}" mode="scaleToFill"></image></view>"#,
        src
    );
    let ss = WxssParser::new(css).parse().unwrap_or_default();
    let nodes = WxmlParser::new(&wxml).parse().unwrap_or_default();
    let mut renderer = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 2.0);
    let mut canvas = Canvas::new(200, 200);
    canvas.clear(Color::WHITE);
    renderer.render(&mut canvas, &nodes, &json!({}));
    canvas
        .pixels()
        .iter()
        .flat_map(|c| [c.r, c.g, c.b, c.a])
        .collect()
}

#[test]
fn test_multi_frame_gif_is_detected_as_animated() {
    if !std::path::Path::new(GIF).exists() {
        return; // 资源缺失时跳过（不影响其它环境）
    }
    let _ = render_gif(GIF);
    assert!(is_animated(GIF), "多帧 GIF 应被识别为动图");
    let total = animation_total_ms(GIF).unwrap_or(0);
    assert!(total >= 100, "动图一轮时长应为各帧延时之和，实际 {}ms", total);
}

#[test]
fn test_gif_frame_advances_over_time() {
    if !std::path::Path::new(GIF).exists() {
        return;
    }
    let first = render_gif(GIF);
    assert!(is_animated(GIF));
    let total = animation_total_ms(GIF).unwrap_or(0);
    // 等待超过一帧的时长后再渲染，画面应发生变化
    let wait = (total / 4).clamp(120, 600) as u64;
    std::thread::sleep(std::time::Duration::from_millis(wait));
    let later = render_gif(GIF);
    assert_eq!(first.len(), later.len());
    let changed = first
        .iter()
        .zip(later.iter())
        .filter(|(a, b)| a != b)
        .count();
    assert!(changed > 0, "等待 {}ms 后 GIF 帧应推进，但画面完全相同", wait);
}

#[test]
fn test_static_image_is_not_animated() {
    let jpg = "sample-app/assets/p_phone.jpg";
    if !std::path::Path::new(jpg).exists() {
        return;
    }
    let _ = render_gif(jpg);
    assert!(!is_animated(jpg), "静态 JPG 不应被判定为动图");
    assert!(animation_total_ms(jpg).is_none());
}
