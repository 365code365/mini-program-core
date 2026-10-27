//! 小游戏的音频（game-doc/02 §8、04 G5）：InnerAudioContext 接真混音器（mixer.rs）与真设备（output.rs，cpal）。
//! 小程序的 `audio_ctx` 不动（它按墙上时钟算位置、移动端不出声）；小游戏实例的 `__native_audio` 换成这里的 `__game_audio`，
//! JS 外壳（audio.js 的 `__makeAudio`）与事件派发（`__resolveAudio`）两边共用，事件名与顺序也与 audio_ctx 一致：
//! 设 src → 后台解码 → `canplay`（autoplay / 之前调过 play 的随即开播）→ `play` → `timeUpdate`（每 250ms 源时间）… → `ended`。
//!
//! - **位置取混音器回报的已播放源位置**（不是墙上时钟）：倍速、欠载、设备延迟都反映在里面；离线模式按帧时钟逐样本推进；
//! - 槽位 32 个（mixer::MAX_VOICES）。满了新的 play 抢占**最早开始的非循环**那个，被抢的发 `stop`、日志一行（微信的上限与行为待取证）；
//! - 同一个 src 的解码结果共用（`Weak` 表）：32 个音效对象用同一个 click.wav 只解码一次；
//! - 进后台：混音器整体挂起，在播的全部 `pause`（回前台不自动恢复，与 02 §8.1 一致，待取证）；
//! - 长音乐目前也整段解码（流式解码没做，3 分钟立体声 44.1k 约 60MB，记在验收文档）。

#[cfg(all(feature = "game-audio", target_os = "android"))]
mod aaudio;
mod js;
mod mixer;
mod output;
mod ring;

pub use js::register;
use js::load_src;

use crate::audio_ctx::decode::Pcm;
use mixer::{Cmd, Mixer, Report, MAX_VOICES};
use ring::{ring, Consumer, Producer};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Weak};

const TIME_UPDATE_SEC: f64 = 0.25;

#[derive(Default)]
struct Ctx {
    src: String,
    pcm: Option<Arc<Pcm>>,
    loading: bool,
    autoplay: bool,
    looping: bool,
    volume: f32,
    rate: f32,
    start_time: f64,
    /// 不在播时的位置；在播时由混音器回报更新
    pos: f64,
    playing: bool,
    slot: Option<usize>,
    gen: u32,
    play_when_ready: bool,
    last_update: f64,
}

impl Ctx {
    fn duration(&self) -> f64 {
        self.pcm.as_ref().map(|p| p.duration()).unwrap_or(0.0)
    }
}

struct Ends {
    cmds: Producer<Cmd>,
    reports: Consumer<Report>,
    retire: Consumer<Arc<Pcm>>,
}

pub struct GameAudio {
    root: PathBuf,
    out: output::Output,
    rate: u32,
    ends: Ends,
    ctxs: HashMap<u32, Ctx>,
    /// 槽位 → (上下文 id, 代数, 开始顺序)
    slots: [Option<(u32, u32, u64)>; MAX_VOICES],
    seq: u64,
    next_gen: u32,
    cache: HashMap<String, Weak<Pcm>>,
    /// 解码中：src → 等它的上下文
    pending: HashMap<String, Vec<u32>>,
    done_tx: Sender<(String, Result<Pcm, (i32, String)>)>,
    done_rx: Receiver<(String, Result<Pcm, (i32, String)>)>,
    events: Vec<(u32, &'static str, Value)>,
    logs: Vec<String>,
    suspended: bool,
    /// 固定时钟（无头）：帧与帧之间不真等，解码没做完就等它做完（与图片同一个做法），结果才逐次可复现
    wait_decodes: bool,
}

impl GameAudio {
    /// `offline`：无头 / 测试（不开设备，按帧时钟渲染）
    pub fn new(root: PathBuf, offline: bool) -> Self {
        let offline = offline || std::env::var("MINI_AUDIO_OFFLINE").is_ok();
        let (opened, ends) = output::open(offline, |rate, channels| {
            let (cp, cc) = ring(256);
            let (rp, rc) = ring(4096);
            let (tp, tc) = ring(256);
            (Mixer::new(rate, channels, cc, rp, tp), Ends { cmds: cp, reports: rc, retire: tc })
        });
        let (done_tx, done_rx) = channel();
        let mut a = Self {
            root,
            out: opened.out,
            rate: opened.rate,
            ends,
            ctxs: HashMap::new(),
            slots: Default::default(),
            seq: 0,
            next_gen: 1,
            cache: HashMap::new(),
            pending: HashMap::new(),
            done_tx,
            done_rx,
            events: Vec::new(),
            logs: Vec::new(),
            suspended: false,
            wait_decodes: offline,
        };
        if std::env::var("MINI_AUDIO_LOG").is_ok() || !a.out.is_device() && !offline {
            a.logs.push(format!("🔊 音频：{}", opened.note));
        }
        a
    }

    fn send(&mut self, c: Cmd) {
        if self.ends.cmds.push(c).is_err() {
            self.logs.push("⚠️ 音频命令队列满了，丢掉一条".into());
        }
    }

    fn emit(&mut self, id: u32, ty: &'static str) {
        self.events.push((id, ty, json!({})));
    }

    fn load(&mut self, id: u32) {
        if !self.ctxs.contains_key(&id) {
            return;
        }
        self.stop_voice(id);
        let c = self.ctxs.get_mut(&id).unwrap();
        (c.pcm, c.playing, c.pos, c.loading) = (None, false, c.start_time, false);
        let src = c.src.clone();
        if src.is_empty() {
            return;
        }
        c.loading = true;
        let waiters = self.pending.entry(src.clone()).or_default();
        waiters.push(id);
        if waiters.len() > 1 {
            return; // 同一个 src 已经在解码
        }
        if let Some(p) = self.cache.get(&src).and_then(Weak::upgrade) {
            // 解过且还有人在用：仍走「下一帧 canplay」（与解码完成同一条路：游戏常在设完 src 才挂 onCanplay）
            let _ = self.done_tx.send((src, Ok((*p).clone())));
            return;
        }
        let (tx, root) = (self.done_tx.clone(), self.root.clone());
        std::thread::Builder::new()
            .name("mini-game-audio-dec".into())
            .spawn(move || {
                let r = std::panic::catch_unwind(|| load_src(&src, &root)).unwrap_or_else(|_| Err((10004, "解码器内部错误".into())));
                let _ = tx.send((src, r));
            })
            .ok();
    }

    fn free_slot(&mut self) -> usize {
        if let Some(i) = self.slots.iter().position(Option::is_none) {
            return i;
        }
        // 抢占最早开始的非循环那个（都在循环就抢最早的）
        let pick = |want_nonloop: bool, s: &Self| {
            s.slots
                .iter()
                .enumerate()
                .filter(|(_, x)| !want_nonloop || x.map(|(id, _, _)| !s.ctxs.get(&id).is_some_and(|c| c.looping)).unwrap_or(false))
                .min_by_key(|(_, x)| x.map(|v| v.2).unwrap_or(u64::MAX))
                .map(|(i, _)| i)
        };
        let i = pick(true, self).or_else(|| pick(false, self)).unwrap_or(0);
        if let Some((id, _, _)) = self.slots[i] {
            self.logs.push(format!("⚠️ 同时发声超过 {MAX_VOICES} 路：音频 {id} 被抢占（发 stop）"));
            self.stop_voice(id);
            if let Some(c) = self.ctxs.get_mut(&id) {
                c.playing = false;
                c.pos = 0.0;
            }
            self.emit(id, "stop");
        }
        i
    }

    fn stop_voice(&mut self, id: u32) {
        let Some(c) = self.ctxs.get_mut(&id) else { return };
        if let Some(slot) = c.slot.take() {
            let gen = c.gen;
            self.slots[slot] = None;
            self.send(Cmd::Stop { slot, gen });
        }
    }

    fn play(&mut self, id: u32) {
        let Some(c) = self.ctxs.get_mut(&id) else { return };
        if c.pcm.is_none() {
            c.play_when_ready = true;
            if c.loading {
                self.emit(id, "waiting");
            }
            return;
        }
        if c.playing {
            return;
        }
        if c.pos >= c.duration() {
            c.pos = 0.0;
        }
        let (gen, slot) = match c.slot {
            // 暂停过：原地接着放
            Some(slot) => (c.gen, slot),
            None => {
                let slot = self.free_slot();
                let gen = self.next_gen;
                self.next_gen += 1;
                self.seq += 1;
                self.slots[slot] = Some((id, gen, self.seq));
                let c = self.ctxs.get_mut(&id).unwrap();
                c.slot = Some(slot);
                c.gen = gen;
                let cmd = Cmd::Start { slot, gen, pcm: c.pcm.clone().unwrap(), at: c.pos, gain: c.volume, rate: c.rate, looping: c.looping };
                c.playing = true;
                c.last_update = c.pos;
                self.send(cmd);
                self.emit(id, "play");
                return;
            }
        };
        let c = self.ctxs.get_mut(&id).unwrap();
        c.playing = true;
        c.last_update = c.pos;
        self.send(Cmd::Resume { slot, gen });
        self.emit(id, "play");
    }

    fn pause(&mut self, id: u32) {
        let Some(c) = self.ctxs.get_mut(&id) else { return };
        c.play_when_ready = false;
        if !c.playing {
            return;
        }
        c.playing = false;
        if let Some(slot) = c.slot {
            let gen = c.gen;
            self.send(Cmd::Pause { slot, gen });
        }
        self.emit(id, "pause");
    }

    /// `__game_audio(op, id, json)`：与 `audio_ctx::call` 同一个约定（`state` 返回当前状态，其余 `{}`）
    pub fn call(&mut self, op: &str, id: u32, args: &str) -> String {
        let a: Value = serde_json::from_str(args).unwrap_or(Value::Null);
        if op == "create" {
            self.ctxs.insert(id, Ctx { volume: 1.0, rate: 1.0, ..Default::default() });
            return "{}".into();
        }
        if !self.ctxs.contains_key(&id) {
            return "{}".into();
        }
        match op {
            "set" => self.set(id, &a),
            "play" => self.play(id),
            "pause" => self.pause(id),
            "stop" => {
                self.stop_voice(id);
                let c = self.ctxs.get_mut(&id).unwrap();
                let was = c.playing || c.pos > 0.0 || c.pcm.is_some();
                (c.playing, c.pos, c.play_when_ready) = (false, 0.0, false);
                if was {
                    self.emit(id, "stop");
                }
            }
            "seek" => {
                let to = a.get("position").and_then(Value::as_f64).unwrap_or(0.0).max(0.0);
                self.emit(id, "seeking");
                let c = self.ctxs.get_mut(&id).unwrap();
                c.pos = if c.pcm.is_some() { to.min(c.duration()) } else { to };
                c.last_update = c.pos;
                if let Some(slot) = c.slot {
                    let (gen, at) = (c.gen, c.pos);
                    self.send(Cmd::Seek { slot, gen, at });
                }
                self.emit(id, "seeked");
            }
            "destroy" => {
                self.stop_voice(id);
                self.ctxs.remove(&id);
            }
            "state" => {
                let c = &self.ctxs[&id];
                let dur = c.duration();
                return json!({ "currentTime": c.pos.min(if dur > 0.0 { dur } else { f64::MAX }), "duration": dur,
                    "paused": !c.playing, "buffered": if c.pcm.is_some() { dur } else { 0.0 } })
                .to_string();
            }
            _ => {}
        }
        "{}".into()
    }

    fn set(&mut self, id: u32, a: &Value) {
        let f = |k: &str| a.get(k).and_then(Value::as_f64);
        let c = self.ctxs.get_mut(&id).unwrap();
        let mut cmds = Vec::new();
        if let Some(v) = a.get("autoplay").and_then(Value::as_bool) {
            c.autoplay = v;
        }
        if let Some(v) = a.get("loop").and_then(Value::as_bool) {
            c.looping = v;
            cmds.push(Cmd::Loop { slot: 0, gen: 0, looping: v });
        }
        if let Some(v) = f("volume") {
            c.volume = v.clamp(0.0, 1.0) as f32;
            cmds.push(Cmd::Gain { slot: 0, gen: 0, gain: c.volume });
        }
        if let Some(v) = f("playbackRate") {
            // 微信只认 0.5 ~ 2.0
            c.rate = v.clamp(0.5, 2.0) as f32;
            cmds.push(Cmd::Rate { slot: 0, gen: 0, rate: c.rate });
        }
        if let Some(v) = f("startTime") {
            c.start_time = v.max(0.0);
        }
        if let Some(slot) = c.slot {
            let gen = c.gen;
            for cmd in cmds {
                self.send(match cmd {
                    Cmd::Loop { looping, .. } => Cmd::Loop { slot, gen, looping },
                    Cmd::Gain { gain, .. } => Cmd::Gain { slot, gen, gain },
                    Cmd::Rate { rate, .. } => Cmd::Rate { slot, gen, rate },
                    other => other,
                });
            }
        }
        if let Some(v) = a.get("src").and_then(Value::as_str) {
            let c = self.ctxs.get_mut(&id).unwrap();
            if v != c.src {
                c.src = v.to_string();
                self.load(id);
            }
        }
    }

    /// pump 里调：离线渲染补到此刻、收解码结果与混音器回报、发 timeUpdate / ended
    pub fn tick(&mut self, t_us: u64) {
        self.out.advance(t_us, self.rate);
        if let Some(l) = self.out.poll() {
            self.logs.push(l);
        }
        while self.ends.retire.pop().is_some() {}
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let got = if self.wait_decodes && !self.pending.is_empty() {
                self.done_rx.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now())).ok()
            } else {
                self.done_rx.try_recv().ok()
            };
            let Some((src, r)) = got else { break };
            let waiters = self.pending.remove(&src).unwrap_or_default();
            let r = r.map(Arc::new);
            if let Ok(p) = &r {
                self.cache.insert(src.clone(), Arc::downgrade(p));
            }
            for id in waiters {
                let Some(c) = self.ctxs.get_mut(&id).filter(|c| c.src == src && c.loading) else { continue };
                c.loading = false;
                match &r {
                    Ok(p) => {
                        c.pcm = Some(p.clone());
                        c.pos = c.start_time.min(c.duration());
                        let go = c.autoplay || c.play_when_ready;
                        c.play_when_ready = false;
                        self.emit(id, "canplay");
                        if go {
                            self.play(id);
                        }
                    }
                    Err((code, msg)) => self.events.push((id, "error", json!({ "errCode": code, "errMsg": msg }))),
                }
            }
        }
        while let Some(r) = self.ends.reports.pop() {
            let Some((id, gen, _)) = self.slots[r.slot] else { continue };
            if gen != r.gen {
                continue;
            }
            let Some(c) = self.ctxs.get_mut(&id) else { continue };
            c.pos = r.pos;
            if r.ended {
                c.playing = false;
                c.slot = None;
                c.pos = c.duration();
                self.slots[r.slot] = None;
                self.emit(id, "timeUpdate");
                self.emit(id, "ended");
                continue;
            }
            // 循环时位置回绕：重新起算 timeUpdate
            if c.pos < c.last_update {
                c.last_update = c.pos;
            }
            if c.playing && c.pos - c.last_update >= TIME_UPDATE_SEC {
                c.last_update = c.pos;
                self.emit(id, "timeUpdate");
            }
        }
    }

    /// 前后台：后台整体挂起，在播的发 pause（回前台不自动恢复）
    pub fn set_background(&mut self, bg: bool) {
        if bg == self.suspended {
            return;
        }
        self.suspended = bg;
        if bg {
            let playing: Vec<u32> = self.ctxs.iter().filter(|(_, c)| c.playing).map(|(&id, _)| id).collect();
            for id in playing {
                self.pause(id);
            }
        }
        self.send(Cmd::Suspend(bg));
        self.out.set_paused(bg);
    }

    pub fn take_events(&mut self) -> Vec<(u32, &'static str, Value)> {
        std::mem::take(&mut self.events)
    }

    pub fn take_logs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.logs)
    }

    /// 有在解码 / 在播的：宿主要继续出帧（位置回报、ended 要有帧去取）
    pub fn busy(&self) -> bool {
        !self.pending.is_empty() || !self.events.is_empty() || self.ctxs.values().any(|c| c.playing)
    }

    /// 诊断 JSON：输出帧数、延迟、回调块、回调次数、欠载、采样率、是否真设备、在播几路
    pub fn stats_json(&self) -> Value {
        let (l, b, n) = self.out.stats();
        json!({ "frames": self.out.frames(), "latencyMs": l as f64 / 1000.0, "block": b, "callbacks": n, "xruns": self.out.xruns(),
            "rate": self.rate, "device": self.out.is_device(), "voices": self.slots.iter().filter(|s| s.is_some()).count() })
    }

    pub fn is_device(&self) -> bool {
        self.out.is_device()
    }
}
