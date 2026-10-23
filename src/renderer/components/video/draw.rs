//! `<video>` 在页面里的样子：黑底、画面、封面、弹幕、控制条、中间播放键。
//! 画面有三个来源：内置播放器解出的帧、同层渲染时宿主交来的帧（`host_feed`），
//! 宿主用原生控件盖在上面时只画黑底。控制条两种播放器同一套（`chrome.rs` 画、`ui.rs` 管显隐与拖动）。
use super::chrome::{self, Flags};
use super::ui::{self, Status, VideoRef};
use super::*;

impl VideoComponent {
    pub fn draw(
        node: &RenderNode,
        canvas: &mut Canvas,
        text_renderer: Option<&TextRenderer>,
        x: f32, y: f32, w: f32, h: f32, sf: f32
    ) {
        let radius = node.style.border_radius;
        // 黑底走圆角快路径（内部整行填充、只有四角逐像素）。通用的 `draw_path` 是 4× 超采样扫描线，
        // 遍历整块面积：同层渲染时这一块每来一帧就重画一次，952×518 的框在桌面上要 7ms 多，
        // 占这一块重画的近九成，真机上一帧 20~30ms
        canvas.fill_round_rect(x, y, w, h, radius, Color::BLACK);
        if host_owns_video() {
            return; // 宿主的播放器盖在这块黑底上
        }
        // `object-fit`：contain（默认）/ fill / cover，对应图片的三种缩放模式
        let mode = match node.attrs.get("object-fit").map(|s| s.trim()) {
            Some("fill") => "scaleToFill",
            Some("cover") => "aspectFill",
            _ => "aspectFit",
        };
        let video = VideoRef::from_attrs(&node.attrs);
        if video.src.is_empty() && !host_feed::host_feeds_video() {
            return;
        }
        ui::note(&video);
        let host = host_feed::host_feeds_video();
        if host {
            host_feed::note_box(&video.key, w, h);
        }
        let st = video.status();
        // 还没开始播：`poster` 盖在画面上
        if !Self::draw_poster(node, canvas, x, y, w, h, st.started) {
            if let Some(f) = video.frame() {
                let (rgba, fw, fh) = f.rgba();
                // 宿主按 `videos` 事件里的 `box` 读回、已经缩好：整行拷贝；尺寸对不上（框刚变）再逐像素缩放
                if !canvas.blit_image_exact(rgba, fw, fh, x, y, w, h, mode, radius) {
                    canvas.draw_image(rgba, fw, fh, x, y, w, h, mode, radius);
                }
            }
        }
        if !host {
            Self::draw_danmu(node, canvas, text_renderer, x, y, w, sf);
        }
        Self::draw_chrome(node, canvas, text_renderer, &video, &st, (x, y, w, h), sf);
    }

    /// 控制条（显示着时）/ 收起后的细进度条，与暂停时中间的播放键
    fn draw_chrome(
        node: &RenderNode,
        canvas: &mut Canvas,
        tr: Option<&TextRenderer>,
        video: &VideoRef,
        st: &Status,
        (x, y, w, h): (f32, f32, f32, f32),
        sf: f32,
    ) {
        let flags = Flags::of(&node.attrs, w / sf.max(0.1));
        if flags.controls {
            if ui::controls_shown(&video.key) {
                let bar = chrome::bar_layout(x, y, w, h, &flags, sf, false, st.duration);
                chrome::draw_bar(canvas, tr, &bar, st, ui::drag_ratio(&video.key), sf, false);
            } else if st.playing && flags.bottom_progress {
                chrome::draw_bottom_progress(canvas, x, y, w, h, sf, st);
            }
        }
        // 暂停 / 没起播：画面正中一个播放键（微信 `show-center-play-btn` 默认开）
        if !st.playing && flags.center_btn {
            chrome::draw_center(canvas, x, y, w, h, sf);
        }
    }
}
