//! 同层渲染的 `<live-player>`：黑底 + 宿主播放器交来的最新一帧（`host_feed`，与同层视频同一个帧表）。
//! 没有控制条、封面、播放键 —— 微信的 live-player 本来就只有画面。
//! `object-fit`：`contain`（默认，留黑边）/ `fillCrop`（铺满裁边）。约定见 `host/engine_live_player.rs`。
use super::base::RenderNode;
use super::host_feed;
use crate::{Canvas, Color};

/// 框的背景画好之后调（`draw_component`）
pub fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32) {
    let radius = node.style.border_radius;
    canvas.fill_round_rect(x, y, w, h, radius, Color::BLACK);
    let key = host_feed::video_key(&node.attrs);
    host_feed::note_box(&key, w, h);
    let Some(f) = host_feed::frame(&key) else { return };
    let mode = match node.attrs.get("object-fit").map(|s| s.trim()) {
        Some("fillCrop") => "aspectFill",
        _ => "aspectFit",
    };
    // 宿主按 `videos` 事件里的 `box` 读回、已经缩好：整行拷贝；尺寸对不上（框刚变）再逐像素缩放
    if !canvas.blit_image_exact(&f.rgba, f.width, f.height, x, y, w, h, mode, radius) {
        canvas.draw_image(&f.rgba, f.width, f.height, x, y, w, h, mode, radius);
    }
}
