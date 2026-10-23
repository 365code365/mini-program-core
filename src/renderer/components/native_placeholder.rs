//! 没被宿主接管的原生组件（`map` / `camera` / `live-player` / `live-pusher` / `voip-room` / `web-view` /
//! `channel-live` / `channel-video`）画一块**占位**：浅灰底 + 居中的组件名，像开发者工具那样 —— 而不是一块
//! 看不出是什么的空白。宿主在创建选项 `nativeComponents` 里点名了的照旧只报位置（宿主的控件盖在上面），不画占位。
//!
//! 也管这批标签与几个新组件的**默认样式**（`build_base_style` 在应用页面 CSS 之前调 [`ua_declarations`]，
//! 页面 CSS 照样能覆盖）：微信里 map 默认 300×150、live-player / live-pusher 默认 300×225、web-view 铺满页面、
//! `span` 是行内混排（这里按「横排、换行、基线对齐」近似）、`keyboard-accessory` 不在页面里占位。
use super::base::*;
use crate::{Canvas, Color, Paint, PaintStyle};

/// 要占位的原生组件
pub const NATIVE_MEDIA: &[(&str, &str)] = &[
    ("map", "地图"),
    ("camera", "相机"),
    ("live-player", "直播播放"),
    ("live-pusher", "直播推流"),
    ("voip-room", "音视频通话"),
    ("web-view", "网页"),
    ("channel-live", "视频号直播"),
    ("channel-video", "视频号视频"),
];

pub fn label_of(tag: &str) -> Option<&'static str> {
    NATIVE_MEDIA.iter().find(|(t, _)| *t == tag).map(|(_, l)| *l)
}

/// 标签的默认样式声明（CSS 写法，按页面 CSS 之前的顺序落地）
pub fn ua_declarations(tag: &str) -> &'static [(&'static str, &'static str)] {
    match tag {
        "map" => &[("width", "300px"), ("height", "150px")],
        "live-player" | "live-pusher" => &[("width", "300px"), ("height", "225px")],
        "camera" | "voip-room" | "channel-video" => &[("width", "100%"), ("height", "300px")],
        "channel-live" => &[("width", "100%"), ("height", "200px")],
        "web-view" => &[("position", "fixed"), ("left", "0"), ("top", "0"), ("width", "100%"), ("height", "100%")],
        "span" => &[("flex-direction", "row"), ("flex-wrap", "wrap"), ("align-items", "baseline")],
        // 官方：两者不写宽高默认 10px；movable-view 默认绝对定位在 movable-area 左上角
        "movable-area" => &[("position", "relative"), ("width", "10px"), ("height", "10px")],
        "movable-view" => &[("position", "absolute"), ("left", "0"), ("top", "0"), ("width", "10px"), ("height", "10px")],
        "keyboard-accessory" => &[("display", "none")],
        _ => &[],
    }
}

/// 占位：没写背景就铺浅灰，中间写组件名
pub fn draw(node: &RenderNode, canvas: &mut Canvas, text: Option<&crate::text::TextRenderer>, x: f32, y: f32, w: f32, h: f32, sf: f32) {
    let Some(label) = label_of(&node.tag) else { return };
    if w < 2.0 || h < 2.0 {
        return;
    }
    if node.style.background_color.is_none() && node.style.background_gradient.is_none() {
        let paint = Paint::new().with_color(Color::from_hex(0xf0f0f0)).with_style(PaintStyle::Fill);
        canvas.draw_rect(&crate::Rect::new(x, y, w, h), &paint);
    }
    let Some(tr) = text else { return };
    let size = 13.0 * sf;
    let s = format!("{label}（宿主未接入 <{}>）", node.tag);
    let tw = tr.measure_text(&s, size);
    if tw > w - 8.0 * sf {
        let short = label.to_string();
        let sw = tr.measure_text(&short, size);
        let paint = Paint::new().with_color(Color::from_hex(0x999999)).with_style(PaintStyle::Fill);
        tr.draw_text(canvas, &short, x + (w - sw) / 2.0, y + (h + size) / 2.0 - 2.0 * sf, size, &paint);
        return;
    }
    let paint = Paint::new().with_color(Color::from_hex(0x999999)).with_style(PaintStyle::Fill);
    tr.draw_text(canvas, &s, x + (w - tw) / 2.0, y + (h + size) / 2.0 - 2.0 * sf, size, &paint);
}
