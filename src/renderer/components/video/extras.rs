//! `<video>` 的封面、弹幕、点进度条跳转，以及 `bindprogress` / `bindwaiting`（G-6）。
//!
//! 依据官方 component/video.html：
//! - `poster`：视频封面（还没开始播时盖在画面上；`object-fit` 同视频）；
//! - `danmu-list`：`[{text, color, time}]`（`time` 是秒），`enable-danmu` 开着才显示；
//!   `VideoContext.sendDanmu({text, color})` 在当前播放位置加一条；
//! - `bindprogress`：加载进度，`detail = {buffered}`（百分比）—— 内置解码是整段预解码，加载完就是 100；
//! - `bindwaiting`：没加载完就要播（缓冲中）。
//!
//! 弹幕的画法：每条出现后 5 秒内从右边缘匀速走到左边缘外，按出现顺序分 3 行；15px 字、带 1px 深色描边感的阴影。

use super::*;
use std::collections::HashMap;

/// 一条弹幕从右走到左的时长（秒）
const DANMU_SECS: f64 = 5.0;
const DANMU_ROWS: usize = 3;

#[derive(Clone, Debug)]
struct Danmu {
    text: String,
    color: Color,
    time: f64,
}

thread_local! {
    /// src → sendDanmu 发出来的弹幕（属性里的 danmu-list 每帧从节点读，不存这里）
    static SENT: std::cell::RefCell<HashMap<String, Vec<Danmu>>> = std::cell::RefCell::new(HashMap::new());
}

/// `VideoContext.sendDanmu`：在当前播放位置加一条
pub fn send_danmu(src: &str, text: &str, color: &str) {
    let time = video_position(src).map(|(t, _)| t).unwrap_or(0.0);
    send_danmu_at(src, text, color, time);
}

/// 同上，位置由调用方给（命令执行时播放器已经锁着，不能再去问它）
pub(super) fn send_danmu_at(src: &str, text: &str, color: &str, time: f64) {
    let color = crate::renderer::components::color_parse::parse_color_str(color).unwrap_or(Color::WHITE);
    SENT.with(|s| s.borrow_mut().entry(src.to_string()).or_default().push(Danmu { text: text.to_string(), color, time }));
}

/// 换页：发出去的弹幕跟着页面走
pub fn clear_danmu() {
    SENT.with(|s| s.borrow_mut().clear());
}

fn attr_danmu(node: &RenderNode) -> Vec<Danmu> {
    let v = crate::renderer::components::parse_attr_json(node.attrs.get("danmu-list").map(|s| s.as_str()).unwrap_or(""));
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|d| {
                    Some(Danmu {
                        text: d.get("text")?.as_str()?.to_string(),
                        color: d.get("color").and_then(|c| c.as_str()).and_then(crate::renderer::components::color_parse::parse_color_str).unwrap_or(Color::WHITE),
                        time: d.get("time").and_then(|t| t.as_f64()).unwrap_or(0.0),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 这个视频开始播过没有（没播过时显示封面）
pub fn video_started(src: &str) -> bool {
    let Ok(players) = get_video_players().lock() else { return false };
    players.get(src).map(|p| p.is_playing || p.ended || p.current_time() > 0.0).unwrap_or(false)
}

impl VideoComponent {
    /// 封面：`poster` 写了且还没开始播
    pub(super) fn draw_poster(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32) -> bool {
        let Some(poster) = node.attrs.get("poster").map(|s| s.trim()).filter(|s| !s.is_empty()) else { return false };
        if video_started(&node.text) {
            return false;
        }
        let Some((rgba, iw, ih)) = crate::renderer::components::image_rgba(poster) else { return false };
        let mode = match node.attrs.get("object-fit").map(|s| s.trim()) {
            Some("fill") => "scaleToFill",
            Some("cover") => "aspectFill",
            _ => "aspectFit",
        };
        canvas.draw_image(&rgba, iw, ih, x, y, w, h, mode, node.style.border_radius);
        true
    }

    /// 弹幕：`enable-danmu` 开着时，画当前时刻在屏上的那几条
    pub(super) fn draw_danmu(node: &RenderNode, canvas: &mut Canvas, tr: Option<&TextRenderer>, x: f32, y: f32, w: f32, sf: f32) {
        let on = node.attrs.get("enable-danmu").map(|s| !matches!(s.trim(), "false" | "0" | "{{false}}")).unwrap_or(false);
        let (Some(tr), true) = (tr, on) else { return };
        let src = &node.text;
        let Some((now, _)) = video_position(src) else { return };
        let mut list = attr_danmu(node);
        SENT.with(|s| list.extend(s.borrow().get(src).cloned().unwrap_or_default()));
        let size = 15.0 * sf;
        canvas.save();
        canvas.clip_rect(GeoRect::new(x, y, w, (DANMU_ROWS as f32 * 24.0 + 8.0) * sf));
        for (i, d) in list.iter().enumerate() {
            let t = now - d.time;
            if !(0.0..DANMU_SECS).contains(&t) {
                continue;
            }
            let tw = tr.measure_text(&d.text, size);
            let dx = x + w - (t / DANMU_SECS) as f32 * (w + tw);
            let dy = y + (8.0 + (i % DANMU_ROWS) as f32 * 24.0) * sf;
            let shadow = Paint::new().with_color(Color::new(0, 0, 0, 140));
            tr.draw_text(canvas, &d.text, dx + sf, dy + sf, size, &shadow);
            tr.draw_text(canvas, &d.text, dx, dy, size, &Paint::new().with_color(d.color));
        }
        canvas.restore();
    }
}

/// 控制条进度轨道的横向范围（逻辑 px，相对视频左边）：与 `draw_controls` 同一套算式
pub fn progress_track(w: f32) -> (f32, f32) {
    let x0 = 12.0 + 24.0 + 12.0;
    (x0, (w - x0 - 80.0).max(0.0))
}
