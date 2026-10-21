//! `<video>`：`wx.createVideoContext` 的命令、组件事件派发、点击播放（SDK 与桌面共用）。
//!
//! 播放器本身在 `renderer::components::video`（按墙上时钟走）；这里每帧：
//! 1. 取走逻辑层发来的 VideoContext 命令（`__native_host_request('video', …)`），按 `id` 找到
//!    `<video>` 节点、用它的 `src` 找播放器执行；
//! 2. 问每个 `<video>` 的播放器「该发什么事件」，派发到它自己身上的绑定（组件事件，不冒泡）。
//!
//! 只有「有命令」或「有播放器还有事件没报」时才遍历布局树，静止页面零开销。
//! 宿主接管 `<video>`（`nativeComponents` 含 video）时引擎不解码，这里也无事可做 ——
//! 那时事件由宿主的播放器回传（见 doc/原生App集成指南.md）。

use crate::renderer::components::{self as c, VideoCommand};
use crate::renderer::{QueryHit, WxmlRenderer};
use crate::runtime::MiniApp;
use crate::ui::interaction::InteractionManager;
use serde_json::Value;

/// 控制条高度与左侧播放键宽度（逻辑像素，与 `VideoComponent::draw_controls` 一致）
const BAR_H: f32 = 36.0;
const PLAY_KEY_W: f32 = 48.0;

/// 每帧一次。返回是否要重画（画面 / 控制条变了）
pub fn tick(app: &mut MiniApp, renderer: Option<&WxmlRenderer>, interaction: &InteractionManager, t_ms: u64) -> bool {
    let reqs = app.take_host_requests("video");
    let Some(r) = renderer else { return false };
    if reqs.is_empty() && !c::video_events_pending() {
        return false;
    }
    let videos = r.query_selector("video", true, interaction);
    let mut changed = false;
    for req in reqs {
        let id = req.get("id").and_then(Value::as_str).unwrap_or("");
        let Some(src) = videos.iter().find(|v| v.id == id).and_then(src_of) else {
            eprintln!("⚠️ VideoContext：页面上没有 id 为 {id:?} 的 <video>");
            continue;
        };
        if let Some(cmd) = parse_command(&req) {
            changed |= c::video_command(src, &cmd);
        }
    }
    for v in &videos {
        let Some(src) = src_of(v) else { continue };
        for (name, detail) in c::video_poll_events(src) {
            if std::env::var("MINI_VIDEO_LOG").is_ok() {
                eprintln!("🎬 {} {name} {detail}", v.component_id);
            }
            super::own_events::dispatch_own(app, r, &v.component_id, name, detail, t_ms);
            changed = true;
        }
    }
    changed
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

/// 点在一个 `<video>` 上（`fixed` 与坐标系一致）：暂停着就播放；播放中点控制条左侧的
/// 播放键就暂停。微信里点画面是显隐控制条、不切播放；这里控制条常显，点画面只在暂停时起播
/// （等同于点中间的播放按钮）。`controls` 与 `show-center-play-btn` 都关掉时不响应。
/// 返回是否切换了播放状态（要重画）
pub fn on_tap(renderer: &WxmlRenderer, interaction: &InteractionManager, x: f32, y: f32, fixed: bool) -> bool {
    if renderer.is_host_native("video") {
        return false;
    }
    let hits = renderer.query_selector("video", true, interaction);
    let Some(v) = hits.iter().rev().find(|v| v.is_fixed == fixed && v.rect.contains(&crate::Point::new(x, y))) else {
        return false;
    };
    let Some(src) = src_of(v) else { return false };
    let flag = |k: &str, default: bool| {
        v.attrs.get(k).map(|s| !matches!(s.trim(), "false" | "{{false}}" | "0")).unwrap_or(default)
    };
    let controls = flag("controls", true);
    let center = flag("show-center-play-btn", true);
    // 点在控制条的进度轨道上：跳到那个位置（`show-progress` 关掉时没有这条轨道）
    if controls && flag("show-progress", true) && y > v.rect.y + v.rect.height - BAR_H {
        let (t0, tw) = c::progress_track(v.rect.width);
        let rel = x - v.rect.x - t0;
        if tw > 0.0 && (0.0..=tw).contains(&rel) {
            if let Some((_, dur)) = c::video_position(src).filter(|(_, d)| *d > 0.0) {
                return c::video_command(src, &VideoCommand::Seek(dur * (rel / tw) as f64));
            }
        }
    }
    if c::is_video_playing(src) {
        let on_key = x < v.rect.x + PLAY_KEY_W && y > v.rect.y + v.rect.height - BAR_H;
        if controls && on_key {
            return c::video_command(src, &VideoCommand::Pause);
        }
        false
    } else if controls || center {
        c::video_command(src, &VideoCommand::Play)
    } else {
        false
    }
}
