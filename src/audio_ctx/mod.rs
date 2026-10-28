//! `wx.createInnerAudioContext()` / `getBackgroundAudioManager()` 的原生侧：解码、播放时钟、事件。
//!
//! - 解码在后台线程（远程地址先下载），完成后本帧发 `canplay`，`autoplay` 的随即开播；
//! - **播放位置按时钟算**（开播时刻 + 已过时间 × 倍速），不从声卡回读：没有声卡的机器（CI、
//!   无头快照）与关掉 `audio` 特性的移动端构建，`timeUpdate` / `ended` 照样按时发生；
//! - 事件排进队列，`MiniApp::update` 每帧取走回给 JS（`__resolveAudio(id, type, json)`）。
//!
//! 与 JS 运行时同线程（thread_local）：rodio 的设备句柄不是 Send，而且所有调用都来自逻辑层所在线程。

pub mod decode;
mod mp4_edit;
mod output;

use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver};
use std::time::Instant;

/// `timeUpdate` 的间隔（微信约 250ms 一次）
const TIME_UPDATE_SEC: f64 = 0.25;

#[derive(Default)]
struct Ctx {
    src: String,
    pcm: Option<decode::Pcm>,
    loading: Option<Receiver<Result<decode::Pcm, (i32, String)>>>,
    autoplay: bool,
    looping: bool,
    volume: f32,
    rate: f32,
    start_time: f64,
    /// 暂停时的位置；播放中时是「开播那一刻」的位置
    offset: f64,
    started: Option<Instant>,
    /// 页面在加载完之前就调了 play()
    play_when_ready: bool,
    last_update: f64,
}

impl Ctx {
    fn position(&self) -> f64 {
        match self.started {
            Some(t) => self.offset + t.elapsed().as_secs_f64() * self.rate as f64,
            None => self.offset,
        }
    }
    fn duration(&self) -> f64 {
        self.pcm.as_ref().map(|p| p.duration()).unwrap_or(0.0)
    }
}

thread_local! {
    static CTXS: RefCell<HashMap<u32, Ctx>> = RefCell::new(HashMap::new());
    static EVENTS: RefCell<Vec<(u32, &'static str, String)>> = const { RefCell::new(Vec::new()) };
}

fn emit(id: u32, ty: &'static str, detail: Value) {
    EVENTS.with(|e| e.borrow_mut().push((id, ty, detail.to_string())));
}

/// 取走待回给 JS 的事件：(上下文 id, 事件名, 参数 JSON)
pub fn take_events() -> Vec<(u32, &'static str, String)> {
    EVENTS.with(|e| std::mem::take(&mut *e.borrow_mut()))
}

/// 有在播 / 在加载的、或有事件没送回：宿主据此续帧
pub fn has_pending() -> bool {
    EVENTS.with(|e| !e.borrow().is_empty())
        || CTXS.with(|c| c.borrow().values().any(|x| x.started.is_some() || x.loading.is_some()))
}

fn start_output(id: u32, c: &Ctx) {
    if let Some(p) = &c.pcm {
        output::start(id, p.samples.clone(), p.rate, p.channels, c.offset, c.volume, c.rate);
    }
}

fn play(id: u32, c: &mut Ctx) {
    if c.pcm.is_none() {
        c.play_when_ready = true;
        if c.loading.is_some() {
            emit(id, "waiting", json!({}));
        }
        return;
    }
    if c.started.is_some() {
        return;
    }
    if c.offset >= c.duration() {
        c.offset = 0.0;
    }
    c.started = Some(Instant::now());
    c.last_update = c.offset;
    start_output(id, c);
    emit(id, "play", json!({}));
}

fn pause(id: u32, c: &mut Ctx) {
    c.play_when_ready = false;
    if c.started.is_none() {
        return;
    }
    c.offset = c.position().min(c.duration());
    c.started = None;
    output::stop(id);
    emit(id, "pause", json!({}));
}

fn load(id: u32, c: &mut Ctx) {
    output::stop(id);
    c.pcm = None;
    c.started = None;
    c.offset = c.start_time;
    if c.src.is_empty() {
        c.loading = None;
        return;
    }
    let (tx, rx) = channel();
    let src = c.src.clone();
    std::thread::spawn(move || {
        tx.send(decode::load(&src)).ok();
    });
    c.loading = Some(rx);
}

/// `__native_audio(op, id, json)`：返回 JSON（`state` 给当前状态，其余给 `{}`）
pub fn call(op: &str, id: u32, args_json: &str) -> String {
    let a: Value = serde_json::from_str(args_json).unwrap_or(Value::Null);
    CTXS.with(|ctxs| {
        let mut ctxs = ctxs.borrow_mut();
        if op == "create" {
            ctxs.insert(id, Ctx { volume: 1.0, rate: 1.0, ..Default::default() });
            return "{}".to_string();
        }
        let Some(c) = ctxs.get_mut(&id) else { return "{}".to_string() };
        match op {
            "set" => {
                let f = |k: &str| a.get(k).and_then(Value::as_f64);
                if let Some(v) = a.get("autoplay").and_then(Value::as_bool) {
                    c.autoplay = v;
                }
                if let Some(v) = a.get("loop").and_then(Value::as_bool) {
                    c.looping = v;
                }
                if let Some(v) = f("volume") {
                    c.volume = v.clamp(0.0, 1.0) as f32;
                    output::set_volume(id, c.volume);
                }
                if let Some(v) = f("playbackRate") {
                    // 微信只认 0.5 ~ 2.0
                    let pos = c.position();
                    c.rate = v.clamp(0.5, 2.0) as f32;
                    if c.started.is_some() {
                        c.offset = pos;
                        c.started = Some(Instant::now());
                        start_output(id, c);
                    }
                }
                if let Some(v) = f("startTime") {
                    c.start_time = v.max(0.0);
                }
                if let Some(v) = a.get("src").and_then(Value::as_str) {
                    if v != c.src {
                        c.src = v.to_string();
                        load(id, c);
                    }
                }
                "{}".to_string()
            }
            "play" => {
                play(id, c);
                "{}".to_string()
            }
            "pause" => {
                pause(id, c);
                "{}".to_string()
            }
            "stop" => {
                let was = c.started.is_some() || c.offset > 0.0;
                output::stop(id);
                c.started = None;
                c.offset = 0.0;
                c.play_when_ready = false;
                if was || c.pcm.is_some() {
                    emit(id, "stop", json!({}));
                }
                "{}".to_string()
            }
            "seek" => {
                let to = a.get("position").and_then(Value::as_f64).unwrap_or(0.0).max(0.0);
                emit(id, "seeking", json!({}));
                let playing = c.started.is_some();
                c.offset = if c.pcm.is_some() { to.min(c.duration()) } else { to };
                if playing {
                    c.started = Some(Instant::now());
                    start_output(id, c);
                }
                c.last_update = c.offset;
                emit(id, "seeked", json!({}));
                "{}".to_string()
            }
            "destroy" => {
                output::stop(id);
                ctxs.remove(&id);
                "{}".to_string()
            }
            "state" => {
                let dur = c.duration();
                json!({
                    "currentTime": c.position().min(if dur > 0.0 { dur } else { f64::MAX }),
                    "duration": dur,
                    "paused": c.started.is_none(),
                    "buffered": if c.pcm.is_some() { dur } else { 0.0 },
                })
                .to_string()
            }
            _ => "{}".to_string(),
        }
    })
}

/// 每帧推进：加载完成 → canplay（+ 自动播放）；播放中 → timeUpdate / ended / loop
pub fn tick() {
    CTXS.with(|ctxs| {
        let mut ctxs = ctxs.borrow_mut();
        for (&id, c) in ctxs.iter_mut() {
            if let Some(rx) = &c.loading {
                if let Ok(r) = rx.try_recv() {
                    c.loading = None;
                    match r {
                        Ok(pcm) => {
                            c.pcm = Some(pcm);
                            c.offset = c.start_time.min(c.duration());
                            emit(id, "canplay", json!({}));
                            if c.autoplay || c.play_when_ready {
                                c.play_when_ready = false;
                                play(id, c);
                            }
                        }
                        Err((code, msg)) => emit(id, "error", json!({ "errCode": code, "errMsg": msg })),
                    }
                }
            }
            if c.started.is_none() {
                continue;
            }
            let pos = c.position();
            let dur = c.duration();
            if pos >= dur {
                if c.looping {
                    c.offset = 0.0;
                    c.started = Some(Instant::now());
                    c.last_update = 0.0;
                    start_output(id, c);
                    emit(id, "timeUpdate", json!({}));
                } else {
                    c.started = None;
                    c.offset = dur;
                    output::stop(id);
                    emit(id, "timeUpdate", json!({}));
                    emit(id, "ended", json!({}));
                }
                continue;
            }
            if pos - c.last_update >= TIME_UPDATE_SEC {
                c.last_update = pos;
                emit(id, "timeUpdate", json!({}));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plays_a_wav_to_the_end_by_clock() {
        let dir = std::env::temp_dir().join(format!("mini-audio-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("beep.wav");
        std::fs::write(&f, decode::make_wav(8000, 1, 0.3)).unwrap();
        call("create", 1, "{}");
        call("set", 1, &json!({ "src": f.to_string_lossy(), "autoplay": true }).to_string());
        let t0 = Instant::now();
        let mut seen = Vec::new();
        while t0.elapsed().as_secs_f64() < 3.0 && !seen.contains(&"ended") {
            tick();
            seen.extend(take_events().into_iter().map(|(_, t, _)| t));
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(seen.first(), Some(&"canplay"), "{seen:?}");
        assert!(seen.contains(&"play") && seen.contains(&"timeUpdate") && seen.contains(&"ended"), "{seen:?}");
        let st: Value = serde_json::from_str(&call("state", 1, "{}")).unwrap();
        assert_eq!(st["paused"], true);
        assert!((st["duration"].as_f64().unwrap() - 0.3).abs() < 0.01);
        call("destroy", 1, "{}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
