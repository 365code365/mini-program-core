//! `<video>`：`wx.createVideoContext` 的命令、组件事件派发、点控制条（SDK 与桌面共用）。
//!
//! 播放器本身在 `renderer::components::video`（内置的按墙上时钟走；同层渲染时在宿主那边）；这里每帧：
//! 1. 取走逻辑层发来的 VideoContext 命令（`__native_host_request('video', …)`），按 `id` 找到
//!    `<video>` 节点交给它的播放器（同层渲染时攒成 `videoCommand` 给宿主，见 engine_video_feed.rs）。
//!    全屏（`requestFullScreen` / `exitFullScreen`）两种播放器都画在引擎里（`video/fullscreen.rs`）；
//! 2. 内置播放器：问每个 `<video>`「该发什么事件」，派发到它自己身上的绑定（组件事件，不冒泡）；
//!    同层渲染时这些事件由宿主回传（`video_feed_event`）；
//! 3. 引擎自己产生的事件（`controlstoggle`、`fullscreenchange`，`video/ui.rs`）照样派发到节点上。
//!
//! 只有「有命令」「有事件没报」「播放中等着自动收起控制条」时才遍历布局树，静止页面零开销。
//! 宿主接管 `<video>`（`nativeComponents` 含 video、不走同层）时引擎不解码，这里无事可做 ——
//! 那时事件由宿主的播放器回传（见 doc/原生App集成指南.md）。
//!
//! 点击（[`on_tap`]）与拖进度（[`seek_target_at`]，手势仲裁横向锁定后交给它）用控制条画的那一套几何
//! （`video/chrome.rs`），行为照微信：点画面切换控制条显隐、点按钮做按钮的事、点进度条跳过去、
//! 暂停时点中间的播放键起播。

use crate::renderer::components::video_chrome::{self as chrome, Flags};
use crate::renderer::components::video_fullscreen as fullscreen;
use crate::renderer::components::video_ui::{self as ui, VideoRef};
use crate::renderer::components::{self as c, VideoCommand};
use crate::renderer::{QueryHit, WxmlRenderer};
use crate::runtime::MiniApp;
use crate::ui::interaction::InteractionManager;
use serde_json::Value;

/// 每帧一次。返回是否要重画：内置播放器的画面 / 控制条变了（同层渲染那边只重画视频框，
/// 由 `ui::mark_seq` 驱动，不在这里要整帧）
pub fn tick(app: &mut MiniApp, renderer: Option<&WxmlRenderer>, interaction: &InteractionManager, t_ms: u64) -> bool {
    let reqs = app.take_host_requests("video");
    let host = c::host_feed::host_feeds_video();
    let ui_changed = ui::tick(t_ms) && !host;
    let Some(r) = renderer else { return false };
    if reqs.is_empty() && !c::video_events_pending() && !ui::pending() {
        return ui_changed;
    }
    let videos = r.query_selector("video", true, interaction);
    let mut changed = ui_changed;
    for req in reqs {
        let id = req.get("id").and_then(Value::as_str).unwrap_or("");
        let Some(v) = videos.iter().find(|v| v.id == id) else {
            eprintln!("⚠️ VideoContext：页面上没有 id 为 {id:?} 的 <video>");
            continue;
        };
        let video = VideoRef::from_attrs(&v.attrs);
        match parse_command(&req) {
            Some(VideoCommand::RequestFullScreen(dir)) => fullscreen::enter(video, dir),
            Some(VideoCommand::ExitFullScreen) => {
                if c::fullscreen_video().map(|(k, _)| k == video.key).unwrap_or(false) {
                    fullscreen::exit();
                }
            }
            Some(VideoCommand::SendDanmu(..)) if host => eprintln!("⚠️ 同层渲染的 <video> 还不支持 VideoContext.sendDanmu"),
            Some(cmd) => {
                if !host && video.src.is_empty() {
                    continue;
                }
                video.command(&cmd);
                changed |= !host;
            }
            None => {}
        }
    }
    if !host {
        for v in &videos {
            let Some(src) = src_of(v) else { continue };
            for (name, detail) in c::video_poll_events(src) {
                dispatch(app, r, v, name, detail, t_ms);
                changed = true;
            }
        }
    }
    for (key, name, detail) in ui::take_events() {
        if let Some(v) = videos.iter().find(|v| c::host_feed::video_key(&v.attrs) == key) {
            dispatch(app, r, v, name, detail, t_ms);
        }
    }
    changed
}

fn dispatch(app: &mut MiniApp, r: &WxmlRenderer, v: &QueryHit, name: &str, detail: Value, t_ms: u64) {
    if std::env::var("MINI_VIDEO_LOG").is_ok() {
        eprintln!("🎬 {} {name} {detail}", v.component_id);
    }
    super::own_events::dispatch_own(app, r, &v.component_id, name, detail, t_ms);
}

fn src_of(v: &QueryHit) -> Option<&str> {
    v.attrs.get("src").map(|s| s.as_str()).filter(|s| !s.trim().is_empty())
}

/// `{id, type, position?, rate?, direction?}` → 命令
fn parse_command(req: &Value) -> Option<VideoCommand> {
    let num = |k: &str| req.get(k).and_then(Value::as_f64);
    Some(match req.get("type").and_then(Value::as_str)? {
        "play" => VideoCommand::Play,
        "pause" => VideoCommand::Pause,
        "stop" => VideoCommand::Stop,
        "seek" => VideoCommand::Seek(num("position").unwrap_or(0.0)),
        "playbackRate" => VideoCommand::PlaybackRate(num("rate").unwrap_or(1.0)),
        "requestFullScreen" => VideoCommand::RequestFullScreen(num("direction").map(|d| d as i32)),
        "exitFullScreen" => VideoCommand::ExitFullScreen,
        "sendDanmu" => VideoCommand::SendDanmu(
            req.get("text").and_then(Value::as_str).unwrap_or("").to_string(),
            req.get("color").and_then(Value::as_str).unwrap_or("#ffffff").to_string(),
        ),
        _ => return None,
    })
}

/// 坐标下最上层的那个 `<video>`（`fixed` 与坐标系一致）
fn video_at<'a>(hits: &'a [QueryHit], x: f32, y: f32, fixed: bool) -> Option<&'a QueryHit> {
    hits.iter().rev().find(|v| v.is_fixed == fixed && v.rect.contains(&crate::Point::new(x, y)))
}

/// 点在一个 `<video>` 上（与微信一致）：
/// - 控制条显示着：点播放键切换播放、点静音键切换静音、点全屏键进全屏、点进度条跳过去；
/// - 暂停 / 没起播时点中间的播放键起播（`show-center-play-btn`）；
/// - 别的地方：控制条显隐切换（`controlstoggle`；`controls` 关掉时什么也不做）。
///
/// 返回是否吃掉了这一下（点视频不冒泡到下面的元素）
pub fn on_tap(renderer: &WxmlRenderer, interaction: &InteractionManager, x: f32, y: f32, fixed: bool) -> bool {
    if renderer.is_host_native("video") {
        return false;
    }
    let hits = renderer.query_selector("video", true, interaction);
    let Some(v) = video_at(&hits, x, y, fixed) else { return false };
    let video = VideoRef::from_attrs(&v.attrs);
    if video.src.is_empty() && !c::host_feed::host_feeds_video() {
        return false;
    }
    ui::note(&video);
    let r = v.rect;
    let flags = Flags::of(&v.attrs, r.width);
    let st = video.status();
    let key = &video.key;
    if flags.controls && ui::controls_shown(key) {
        let bar = chrome::bar_layout(r.x, r.y, r.width, r.height, &flags, 1.0, false, st.duration);
        if bar.contains(x, y) {
            if bar.on_play(x, y) {
                video.command(if st.playing { &VideoCommand::Pause } else { &VideoCommand::Play });
            } else if bar.on_mute(x, y) {
                video.set_muted(!st.muted);
            } else if bar.on_fullscreen(x, y) {
                fullscreen::enter(video.clone(), None);
            } else if let (Some(ratio), true) = (bar.track_ratio(x, y, 1.0), st.duration > 0.0) {
                video.command(&VideoCommand::Seek(st.duration * ratio));
            }
            ui::poke(key);
            return true;
        }
    }
    if !st.playing && flags.center_btn && chrome::center_hit(r.x, r.y, r.width, r.height, 1.0, x, y) {
        video.command(&VideoCommand::Play);
        ui::poke(key);
        return true;
    }
    if flags.controls {
        ui::toggle(key);
    }
    true
}

/// 按下点落在某个 `<video>` 显示着的进度条上：记下它（`ui::arm_seek`），返回它的键。
/// 手势仲裁横向锁定后由它跟手（竖着划照样滚页面，与 slider 同一个道理）
pub fn seek_target_at(renderer: Option<&WxmlRenderer>, interaction: &InteractionManager, x: f32, y: f32, fixed: bool) -> Option<String> {
    ui::disarm_seek();
    let r = renderer?;
    if r.is_host_native("video") {
        return None;
    }
    let hits = r.query_selector("video", true, interaction);
    let v = video_at(&hits, x, y, fixed)?;
    let video = VideoRef::from_attrs(&v.attrs);
    let flags = Flags::of(&v.attrs, v.rect.width);
    if !flags.controls || !ui::controls_shown(&video.key) {
        return None;
    }
    let st = video.status();
    let bar = chrome::bar_layout(v.rect.x, v.rect.y, v.rect.width, v.rect.height, &flags, 1.0, false, st.duration);
    bar.track_ratio(x, y, 1.0)?;
    let (x0, w) = bar.track?;
    let key = video.key.clone();
    ui::arm_seek(video, x0, w);
    Some(key)
}
