//! `<live-player>`（官方 component/live-player.html）：宿主的系统播放器拉流解码，**帧交给引擎画进页面**。
//!
//! 与同层渲染的 `<video>` 同一套约定（创建选项 `sameLayer` 同时有 `video` 与 `live-player`）：
//! - 页面上的直播也进 `videos` 事件，多一个 `"tag": "live-player"`；属性按播放器认得的名字给
//!   （`object-fit` 的 `fillCrop` → `cover`，`loop` 恒为 false，`muted` 以 `LivePlayerContext.mute` 为准）。
//!   宿主照样建播放器、按 `box` 交帧（`mr_video_frame_*`）—— 已经接了同层视频的宿主不用改一行就能放 HLS / mp4 的流。
//! - 播放命令照样是 `videoCommand`（`play` / `pause` / `stop`；`resume` 就是 `play`）。
//! - 宿主回传的还是播放器事件（`play` / `waiting` / `loadedmetadata` / `ended` / `error` / `timeupdate`），
//!   这里翻成直播的状态码派发给 `bindstatechange`（[`state_codes`]），`timeupdate` 每秒折成一次 `bindnetstatus`。
//!
//! 画：黑底 + 最新一帧（`components::live_player`），没有控制条（微信的 live-player 本来就没有）。
//! 全屏（`requestFullScreen`）借视频全屏那一层画，控制条关掉。截图（`snapshot`）取引擎手里的最新一帧存 PNG。
use super::engine::MiniEngine;
use crate::renderer::components::{host_feed, video_fullscreen, video_ui};
use crate::renderer::QueryHit;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Default, Clone, Copy)]
struct LiveState {
    /// 这一路已经报过「开始播放」（2003 / 2004 只在第一次起播时成对报）
    started: bool,
    /// 上一次报 netstatus 的时刻
    net_ms: u64,
}

thread_local! {
    static STATE: RefCell<HashMap<String, LiveState>> = RefCell::new(HashMap::new());
}

/// 交给宿主播放器的属性（`videos` 事件里那一份）
pub(super) fn feed_attrs(attrs: &HashMap<String, String>, key: &str) -> HashMap<String, String> {
    let mut a = attrs.clone();
    let fit = match attrs.get("object-fit").map(|s| s.trim()) {
        Some("fillCrop") => "cover",
        _ => "contain",
    };
    a.insert("object-fit".into(), fit.into());
    a.insert("loop".into(), "false".into());
    let muted = video_ui::effective_muted(key, video_ui::attr_flag(attrs, "muted", false));
    a.insert("muted".into(), muted.to_string());
    a
}

/// 播放器事件 → 直播状态码（官方 live-player 的 statechange 码表里对应的那几个）
fn state_codes(name: &str, first: bool) -> &'static [(i32, &'static str)] {
    match name {
        "loadedmetadata" => &[(2001, "已经连接服务器"), (2002, "已经连接服务器，开始拉流"), (2008, "解码器启动"), (2009, "视频分辨率改变")],
        "play" if first => &[(2003, "网络接收到首个视频数据包（IDR）"), (2004, "视频播放开始")],
        "play" => &[(2004, "视频播放开始")],
        "waiting" => &[(2007, "视频播放 loading")],
        "ended" => &[(2006, "视频播放结束")],
        "error" => &[(-2301, "网络断连，且经多次重连抢救无效，更多重试请自行重启播放")],
        _ => &[],
    }
}

/// 全屏借视频那一层：控制条、中间的播放键都关掉（直播没有进度）
fn fullscreen_ref(v: &QueryHit) -> video_ui::VideoRef {
    let mut r = video_ui::VideoRef::from_attrs(&v.attrs);
    for k in ["controls", "show-center-play-btn", "show-progress", "show-bottom-progress"] {
        r.attrs.insert(k.into(), "false".into());
    }
    r
}

impl MiniEngine {
    /// 宿主回传的播放器事件落在一个 `<live-player>` 上：翻成 statechange / netstatus 派发。返回是否派发了
    pub(super) fn live_feed_event(&mut self, r: &crate::renderer::WxmlRenderer, v: &QueryHit, key: &str, name: &str, detail: &Value) -> bool {
        host_feed::note_event(key, name, detail);
        let now = self.mono_ms();
        let mut st = STATE.with(|s| s.borrow().get(key).copied().unwrap_or_default());
        if name == "loadedmetadata" {
            st = LiveState::default();
        }
        let mut out: Vec<(&str, Value)> = state_codes(name, !st.started)
            .iter()
            .map(|(code, msg)| ("statechange", json!({ "code": code, "message": msg })))
            .collect();
        if name == "play" {
            st.started = true;
        }
        if name == "timeupdate" && now.saturating_sub(st.net_ms) >= 1000 {
            st.net_ms = now;
            let s = host_feed::state(key);
            // 系统播放器不报码率 / 帧率：只给得出来的画面尺寸，其余按 0（字段齐全，页面按微信的结构读不会出错）
            out.push(("netstatus", json!({ "info": {
                "videoBitrate": 0, "audioBitrate": 0, "videoFPS": 0, "videoGOP": 0, "netSpeed": 0, "netJitter": 0,
                "videoWidth": s.width, "videoHeight": s.height,
            }})));
        }
        STATE.with(|s| s.borrow_mut().insert(key.to_string(), st));
        let mut found = false;
        for (ev, d) in out {
            if std::env::var("MINI_VIDEO_LOG").is_ok() {
                eprintln!("📡 live-player {key} {ev} {d}");
            }
            found |= super::own_events::dispatch_own(&mut self.app, r, &v.component_id, ev, d, now);
        }
        found
    }

    /// 每帧一次（`tick_scroll_views`）：`LivePlayerContext` 的命令（`__native_host_request('livePlayer', …)`）
    pub(super) fn tick_live_players(&mut self) {
        let reqs = self.app.take_host_requests("livePlayer");
        if reqs.is_empty() {
            return;
        }
        let players = self.renderer.as_ref().map(|r| r.query_selector("live-player", true, &self.interaction)).unwrap_or_default();
        for req in reqs {
            let id = req.get("id").and_then(Value::as_str).unwrap_or("");
            let ty = req.get("type").and_then(Value::as_str).unwrap_or("");
            let seq = req.get("seq").and_then(Value::as_u64).unwrap_or(0);
            let res = match players.iter().find(|v| v.id == id) {
                Some(v) => self.live_command(v, ty, &req),
                None => Err(format!("页面上没有 id 为 {id:?} 的 <live-player>")),
            };
            let (ok, res) = match res {
                Ok(mut r) => {
                    r["errMsg"] = json!(format!("{ty}:ok"));
                    (true, r)
                }
                Err(e) => (false, json!({ "errMsg": format!("{ty}:fail {e}") })),
            };
            self.app.eval(&format!("__liveReply({seq}, {ok}, {res})")).ok();
        }
        self.app.pump_jobs();
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
        }
    }

    fn live_command(&mut self, v: &QueryHit, ty: &str, req: &Value) -> Result<Value, String> {
        let key = host_feed::video_key(&v.attrs);
        let cmd = |c: &str| host_feed::queue_command(json!({ "key": key, "command": c }));
        match ty {
            "play" | "resume" => cmd("play"),
            "pause" => cmd("pause"),
            "stop" => {
                cmd("stop");
                STATE.with(|s| s.borrow_mut().remove(&key));
            }
            "mute" => {
                // 官方：mute 是「切换」静音。经 `videos` 里的 muted 交给宿主（feed_generation 变了，下一帧重发列表）
                let r = video_ui::VideoRef::from_attrs(&v.attrs);
                r.set_muted(!video_ui::effective_muted(&key, video_ui::attr_flag(&v.attrs, "muted", false)));
            }
            "requestFullScreen" => {
                let d = req.get("direction").and_then(Value::as_i64).map(|d| d as i32);
                video_fullscreen::enter(fullscreen_ref(v), d);
                self.dmg.compose_dirty = true;
            }
            "exitFullScreen" => {
                if video_fullscreen::fullscreen_video().map(|(k, _)| k == key).unwrap_or(false) {
                    video_fullscreen::exit();
                    self.dmg.compose_dirty = true;
                }
            }
            "snapshot" => return snapshot(&key, req),
            // 小窗 / 后台播放 / 投屏：引擎没有这些系统能力，照官方的签名回 fail
            _ => return Err("当前宿主不支持".into()),
        }
        Ok(json!({}))
    }
}

/// 最新一帧存成 PNG（`quality: compressed` 时缩到一半）
fn snapshot(key: &str, req: &Value) -> Result<Value, String> {
    let f = host_feed::frame(key).ok_or("还没有画面")?;
    let (mut w, mut h, mut rgba) = (f.width, f.height, f.rgba.clone());
    if req.get("quality").and_then(Value::as_str) == Some("compressed") && w >= 2 && h >= 2 {
        let img = image::RgbaImage::from_raw(w, h, rgba).ok_or("画面尺寸不对")?;
        let small = image::imageops::resize(&img, w / 2, h / 2, image::imageops::FilterType::Triangle);
        (w, h, rgba) = (small.width(), small.height(), small.into_raw());
    }
    let (uri, path) = crate::wxfile::new_temp("png");
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    image::save_buffer(&path, &rgba, w, h, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())?;
    Ok(json!({ "tempImagePath": uri, "width": w, "height": h }))
}
