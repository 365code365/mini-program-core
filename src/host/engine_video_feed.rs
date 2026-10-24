//! [`MiniEngine`] 与宿主播放器之间的同层渲染视频（创建选项 `sameLayer` 含 `video`）。
//!
//! 画面怎么进来、怎么画见 `components::video::host_feed`。这里只管两件与宿主的约定：
//! - **`videos` 事件**：页面上有哪些视频（`key` / `url` / 属性）。只在布局树整棵重建过、
//!   或换了渲染器（换页）之后重新数一遍，变了才发 —— 宿主据此建 / 换片源 / 释放播放器。
//!   滚动、视频帧那种只重画一块的帧不遍历树。
//! - **`videoCommand` 事件**：点了画面 / 控制条、`wx.createVideoContext` 的命令，宿主照着执行。
//!
//! 宿主回传的播放器事件（`play` / `pause` / `timeupdate` / `ended` …）走 `mr_app_native_view_event`：
//! 先更新引擎画控制条用的状态（变了就只重画那个框），再派发给节点上的 `bind<name>`。
//!
//! 全屏（两种播放器都有，`video/fullscreen.rs`）在这里接上宿主：
//! - **`videoFullscreen` 事件**：进 / 出全屏，宿主据此收起 / 放出状态栏与导航条；
//! - 同层渲染时全屏那一个的 `box` 换成全屏画面的尺寸（横屏时宽高对调）、`object-fit` 换成 `contain`，
//!   宿主按它读回，引擎整行拷贝；
//! - 全屏那一层的版本号变了（新帧、控制条）只重新合成，页面一个像素不画；
//! - 全屏时宿主收起状态栏会报一次新的安全区：先按住不用（用了要整页重排样式，退出又排回来），
//!   退出后宿主多半会把原来的值报回来；[`SAFE_AREA_HOLD_MS`] 内没报回来再用按住的那个。
use super::engine::MiniEngine;
use super::engine_options::SafeAreaInsets;
use crate::host::ui_overlay::image_preview::video as preview;
use crate::renderer::components::{host_feed, video_fullscreen, video_ui};
use serde_json::{json, Value};

/// 退出全屏后，全屏期间按住的安全区等宿主这么久（毫秒）
const SAFE_AREA_HOLD_MS: u64 = 800;

#[derive(Default)]
pub struct VideoFeedSync {
    /// 换了渲染器（换页、重建）：下一帧一定重新数一遍
    pub(super) stale: bool,
    /// 上次数的时候布局树的代数、静音 / 全屏的代数（`video_ui::feed_generation`）
    generation: Option<(u64, u64)>,
    /// 上次报的列表（变了才发）
    last: Option<String>,
    /// 全屏那一层上次合成时的版本号
    fs_compose: Option<u64>,
    /// 全屏期间宿主报来的安全区（还没用），与退出全屏的时刻
    pub(super) held_safe_area: Option<SafeAreaInsets>,
    fs_exit_ms: Option<u64>,
}

impl MiniEngine {
    /// 每帧 tick 之后（`tick_scroll_views`）：全屏那一层变了就重新合成；退出全屏后补上按住的安全区
    pub(super) fn sync_fullscreen(&mut self, now_ms: u64) {
        let seq = video_fullscreen::compose_seq();
        if seq != self.video_feed.fs_compose {
            if seq.is_none() {
                self.video_feed.fs_exit_ms = Some(now_ms);
            }
            self.video_feed.fs_compose = seq;
            self.dmg.compose_dirty = true;
        }
        if seq.is_none() {
            if let Some(t) = self.video_feed.fs_exit_ms {
                if now_ms.saturating_sub(t) >= SAFE_AREA_HOLD_MS {
                    self.video_feed.fs_exit_ms = None;
                    if let Some(s) = self.video_feed.held_safe_area.take() {
                        self.set_safe_area(s);
                    }
                }
            }
        }
    }

    /// 视频全屏时的一帧（`redraw_as_needed` 开头调；没有全屏返回 None，照常走局部重画）。
    ///
    /// 全屏那一层盖住整个画面，页面 / 覆盖层 / tabBar 一个像素也看不见：该重画的（setData、CSS 动画、
    /// 动图换帧……）标记都留着，退出全屏的第一帧照常画。这里只在全屏那一层变了时重新合成 ——
    /// 不然综合场景页上那个一直在转的方块会让每个 vsync 都整屏合成一次全屏画面（一加上一次十几毫秒）
    pub(super) fn fullscreen_frame(&mut self) -> Option<bool> {
        video_fullscreen::fullscreen_video()?;
        // 内置播放器的画面没有版本号：在播就每帧合成（桌面 / iOS 原生示例才走它）
        let builtin_playing = !host_feed::host_feeds_video() && crate::renderer::components::has_playing_video();
        let compose = std::mem::take(&mut self.dmg.compose_dirty) || self.present_pending || builtin_playing;
        if !compose {
            self.dmg.last_kind = super::engine_damage::FrameKind::Idle;
            return Some(false);
        }
        let t0 = self.dmg.log.then(std::time::Instant::now);
        self.dmg.last_kind = super::engine_damage::FrameKind::Compose;
        self.compose_current();
        if let Some(t0) = t0 {
            let gpu = self.dmg.gpu_note.take().unwrap_or_default();
            let (paint, rotate) = video_fullscreen::last_cost();
            crate::compositor::diag::emit(&format!(
                "🩹 [SDK] 合成·全屏视频 〔 画 {paint:.1} 转 {rotate:.1} 〕{gpu}  {:.1}ms",
                t0.elapsed().as_secs_f32() * 1000.0
            ));
        }
        Some(true)
    }

    /// 每次 pump 末尾（`sync_host_events`）
    pub(super) fn sync_video_feed(&mut self) {
        for ev in video_fullscreen::take_host_events() {
            self.events.push(ev);
        }
        if !self.opts.same_layer_video() {
            return;
        }
        for cmd in host_feed::take_commands() {
            let mut ev = json!({ "type": "videoCommand" });
            if let (Some(o), Some(c)) = (ev.as_object_mut(), cmd.as_object()) {
                o.extend(c.clone());
            }
            self.events.push(ev);
        }
        let Some(r) = self.renderer.as_ref() else { return };
        // 预览层（wx.previewMedia）里在放的视频也算一路：换了 / 关了要重新报
        let generation = (r.layout_generation(), video_ui::feed_generation() ^ (preview::generation() << 32));
        if !self.video_feed.stale && self.video_feed.generation == Some(generation) {
            return;
        }
        self.video_feed.stale = false;
        self.video_feed.generation = Some(generation);
        let page_dir = self.current_route().rsplit_once('/').map(|(d, _)| d.to_string()).unwrap_or_default();
        let dpr = self.dpr;
        let fs = video_fullscreen::fullscreen_ref();
        let (pw, ph) = self.pixel_size();
        let mut keys = Vec::new();
        let views: Vec<Value> = r
            .query_selector("video", true, &self.interaction)
            .into_iter()
            .filter_map(|v| {
                let key = host_feed::video_key(&v.attrs);
                if key.is_empty() || keys.contains(&key) {
                    return None;
                }
                keys.push(key.clone());
                let src = v.attrs.get("src").map(|s| s.trim()).unwrap_or("");
                let mut attrs = v.attrs.clone();
                // 用户按过控制条的静音键：以它为准（宿主按 attrs 里的 muted 设音量）
                let muted = video_ui::effective_muted(&key, video_ui::attr_flag(&v.attrs, "muted", false));
                attrs.insert("muted".into(), muted.to_string());
                // 框的设备像素尺寸：宿主读回画面时直接按它（与 object-fit）缩好，引擎只剩整行拷贝。
                // 全屏的那一个按全屏画面的尺寸（横屏宽高对调）、contain
                let bx = match fs.as_ref().filter(|(f, _)| f.key == key) {
                    Some((_, d)) => {
                        attrs.insert("object-fit".into(), "contain".into());
                        if *d == 0 { [pw as f32, ph as f32] } else { [ph as f32, pw as f32] }
                    }
                    None => [(v.rect.width * dpr).round(), (v.rect.height * dpr).round()],
                };
                Some(json!({ "key": key, "url": resolve_url(src, &page_dir), "attrs": attrs, "box": bx }))
            })
            .collect();
        let mut views = views;
        if let Some(url) = preview::active() {
            // 不对应任何节点：宿主照常建播放器、按这个键交帧，画在预览层里（image_preview/video.rs）
            keys.push(preview::VIDEO_KEY.to_string());
            let attrs = json!({ "autoplay": "true", "object-fit": "contain", "muted": "false", "loop": "false" });
            views.push(json!({ "key": preview::VIDEO_KEY, "url": resolve_url(&url, &page_dir), "attrs": attrs, "box": [pw, ph] }));
        }
        let sig = serde_json::to_string(&views).unwrap_or_default();
        if self.video_feed.last.as_deref() == Some(sig.as_str()) {
            return;
        }
        self.video_feed.last = Some(sig);
        // 页面上已经没有的视频：帧（几 MB 一张）与状态一起丢掉
        host_feed::retain(&keys);
        self.events.push(json!({ "type": "videos", "views": views }));
    }

    /// 宿主回传的播放器事件（`dispatch_native_view_event` 在盖上去的控件里没找到这个键时转到这里）。
    /// 返回是否派发给了节点上的绑定
    pub(super) fn video_feed_event(&mut self, key: &str, name: &str, detail: Value) -> bool {
        if !self.opts.same_layer_video() {
            return false;
        }
        if key == preview::VIDEO_KEY {
            // 预览层里那一路：没有节点可派发，只更新播放状态（播放键跟着变），重新合成
            host_feed::note_event(key, name, &detail);
            self.dmg.compose_dirty = true;
            return true;
        }
        let Some(r) = self.renderer.take() else { return false };
        let hit = r.query_selector("video", true, &self.interaction).into_iter().find(|v| host_feed::video_key(&v.attrs) == key);
        let mut found = false;
        if let Some(v) = hit {
            // 控制条的播放键 / 进度变了：版本号加一，下一帧只重画这个框（wxml_renderer/animated_image.rs）
            host_feed::note_event(key, name, &detail);
            let t = self.mono_ms();
            found = super::own_events::dispatch_own(&mut self.app, &r, &v.component_id, name, detail, t);
            self.app.pump_jobs();
        }
        self.renderer = Some(r);
        // 只标数据脏：重画多大由失效分析定（timeupdate 每秒四次，不该每次整帧）
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
        }
        found
    }
}

/// 网络地址原样；包内路径解析成本地文件的绝对路径（`/` 开头按小程序根，否则先当前页目录再根）。
/// 找不到就原样给宿主（它会回 `error`）
fn resolve_url(src: &str, page_dir: &str) -> String {
    // `wxfile://`（下载 / 选的文件）也要解析成本地路径，只有网络地址原样
    if src.is_empty() || src.starts_with("http://") || src.starts_with("https://") {
        return src.to_string();
    }
    let rel = (!src.starts_with('/') && !page_dir.is_empty()).then(|| format!("{page_dir}/{src}"));
    rel.as_deref()
        .and_then(crate::assets::resolve)
        .or_else(|| crate::assets::resolve(src))
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| src.to_string())
}
