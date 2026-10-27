//! WebAudio 的引擎线程一侧（game-doc/04 G5.3）：JS 的 `__game_webaudio(op, json, bytes?)` 落到这里。
//!
//! 这里留一份图的镜像（节点、连线、每个参数的时间线），改了什么就把对应的命令发给音频线程（graph/mod.rs）：
//! 建节点（缓冲在这边分配好）、换处理计划、换时间线、发节点消息。音频线程那边只按命令改表。
//!
//! 节点的寿命（规范 §1.5.2 的意思，按可观察的效果做）：
//! - JS 对象被回收（FinalizationRegistry → `free`）**且**没有活着的上游，才真正从图里拿掉；
//!   `src.connect(ctx.createGain()).connect(ctx.destination)` 里那个没人引用的 gain 照样在响；
//! - 源节点播完：它的出边全部断开、槽位放掉（播完的源只会输出静音，断开与不断开听不出差别），下游跟着检查一遍；
//!   JS 那边的对象还在，之后对它的调用都是无害的空操作；
//! - 有「尾巴」的节点（Delay、BiquadFilter、IIRFilter）上游没了之后还要把尾巴放完（规范的 tail-time 引用）：
//!   先记下尾巴放完的时刻，到点了（web_tick 里查）再拿掉。Delay 的尾巴 = maxDelayTime；Biquad 按极点半径算衰减到
//!   -80dB 的时长（上限 10s）；IIR 固定 1s；
//! - close()：整个上下文的节点一起拿掉。

mod kinds;
mod ops;
mod plan;
mod query;

use super::graph::analyser::{Shared, Spectrum};
use super::graph::bus::{Interp, Mode};
use super::graph::param::Timeline;
use super::graph::{osc, AudioBuf, Clocks, GCmd, InSpec, MAX_CTX, MAX_NODES};
use super::mixer::Cmd;
use super::GameAudio;
use kinds::{Kind, PSpec};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Weak};

pub use query::register;

/// 一个参数的镜像
pub struct PMirror {
    pub spec: PSpec,
    pub tl: Timeline,
    pub base: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Conn {
    pub to: u32,
    pub out: u16,
    pub port: u16,
    pub param: bool,
}

pub struct WNode {
    pub ctx: usize,
    pub kind: Kind,
    /// 音频线程上的槽位；播完 / 被回收之后是 None
    pub slot: Option<u32>,
    pub params: Vec<PMirror>,
    pub outs: Vec<Conn>,
    pub n_in: usize,
    pub n_out: usize,
    pub spec: InSpec,
    pub started: bool,
    pub stopped: bool,
    pub js_alive: bool,
    pub has_buffer: bool,
    pub analyser: Option<(Arc<Shared>, Spectrum)>,
    /// 节点自己的属性（JS 读回用：loop、type、fftSize…），原样存 JSON
    pub props: serde_json::Map<String, Value>,
    pub iir: Option<(Vec<f64>, Vec<f64>)>,
    /// 上游都没了、正在放尾巴：放完的时刻
    pub tail_until: Option<f64>,
    /// DelayNode 的 maxDelayTime（尾巴长度）
    pub max_delay: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Running,
    Suspended,
    Closed,
}

pub struct WCtx {
    pub state: State,
    pub dest: u32,
    pub nodes: Vec<u32>,
    pub listener: Vec<PMirror>,
}

type Decoded = (u32, usize, Result<crate::audio_ctx::decode::Pcm, (i32, String)>);

pub struct WebAudio {
    pub rate: f64,
    pub clocks: Arc<Clocks>,
    pub ctxs: HashMap<usize, WCtx>,
    pub nodes: HashMap<u32, WNode>,
    /// 槽位 → 节点 id（播完的回报只带槽位）
    pub slot_owner: HashMap<u32, u32>,
    free_slots: Vec<u32>,
    next_slot: u32,
    next_node: u32,
    pub bufs: HashMap<u32, Arc<AudioBuf>>,
    pub next_buf: u32,
    pub waves: HashMap<u32, Arc<osc::WaveTables>>,
    builtin: HashMap<osc::Builtin, Weak<osc::WaveTables>>,
    pub decode_tx: Sender<Decoded>,
    decode_rx: Receiver<Decoded>,
    pub decoding: usize,
    pub next_job: u32,
    pub events: Vec<Value>,
}

impl WebAudio {
    pub fn new(rate: u32, clocks: Arc<Clocks>) -> Self {
        let (decode_tx, decode_rx) = channel();
        Self {
            rate: rate as f64,
            clocks,
            ctxs: HashMap::new(),
            nodes: HashMap::new(),
            slot_owner: HashMap::new(),
            free_slots: Vec::new(),
            next_slot: 0,
            next_node: 1,
            bufs: HashMap::new(),
            next_buf: 1,
            waves: HashMap::new(),
            builtin: HashMap::new(),
            decode_tx,
            decode_rx,
            decoding: 0,
            next_job: 1,
            events: Vec::new(),
        }
    }

    pub fn now(&self, ctx: usize) -> f64 {
        self.clocks.get(ctx).map(|c| c.load(Ordering::Relaxed) as f64 / self.rate).unwrap_or(0.0)
    }

    pub fn wave(&mut self, b: osc::Builtin) -> Arc<osc::WaveTables> {
        if let Some(w) = self.builtin.get(&b).and_then(Weak::upgrade) {
            return w;
        }
        let w = Arc::new(osc::WaveTables::builtin(b, self.rate));
        self.builtin.insert(b, Arc::downgrade(&w));
        w
    }

    fn alloc_slot(&mut self) -> Option<u32> {
        if let Some(s) = self.free_slots.pop() {
            return Some(s);
        }
        (self.next_slot < MAX_NODES as u32).then(|| {
            self.next_slot += 1;
            self.next_slot - 1
        })
    }

    pub fn busy(&self) -> bool {
        self.decoding > 0 || !self.events.is_empty() || self.ctxs.values().any(|c| c.state == State::Running && c.nodes.len() > 1)
    }
}

/// 结果：JSON 文本 + 可选的二进制返回值。错误统一 `{"error": "<DOMException 名>: 说明"}`，JS 外壳照名字抛
pub type Out = (String, Option<Vec<u8>>);

pub fn err(name: &str, msg: &str) -> Out {
    (json!({ "error": format!("{name}: {msg}") }).to_string(), None)
}

pub fn ok(v: Value) -> Out {
    (v.to_string(), None)
}

impl GameAudio {
    pub(super) fn web_send(&mut self, g: GCmd) {
        self.send(Cmd::Graph(g));
    }

    fn ctx_new(&mut self) -> Out {
        let Some(idx) = (0..MAX_CTX).find(|i| !self.web.ctxs.contains_key(i)) else {
            return err("NotSupportedError", &format!("同时存在的 WebAudioContext 不能超过 {MAX_CTX} 个"));
        };
        // 新上下文从 0 时刻起：时钟清零之后才让音频线程开始渲染它
        self.web.clocks[idx].store(0, Ordering::Relaxed);
        let listener = kinds::LISTENER.iter().map(|s| PMirror { spec: *s, tl: Timeline::default(), base: s.base }).collect();
        self.web.ctxs.insert(idx, WCtx { state: State::Running, dest: 0, nodes: Vec::new(), listener });
        let dest = match self.node_new(idx, Kind::Destination, &kinds::Opts::default()) {
            Ok(id) => id,
            Err(e) => {
                self.web.ctxs.remove(&idx);
                return e;
            }
        };
        self.web.ctxs.get_mut(&idx).unwrap().dest = dest;
        self.flush_plan(idx);
        self.web_send(GCmd::Running { ctx: idx, on: true });
        ok(json!({ "ctx": idx, "sampleRate": self.web.rate, "dest": dest }))
    }

    pub(super) fn node_new(&mut self, ctx: usize, kind: Kind, o: &kinds::Opts) -> Result<u32, Out> {
        let Some(slot) = self.web.alloc_slot() else {
            return Err(err("NotSupportedError", &format!("音频节点总数不能超过 {MAX_NODES}")));
        };
        let rate = self.web.rate;
        let mut sine = None;
        if kind == Kind::Oscillator {
            sine = Some(self.web.wave(osc::Builtin::Sine));
        }
        let b = kinds::build(kind, rate, o, || sine.take().unwrap());
        let id = self.web.next_node;
        self.web.next_node += 1;
        let mut params: Vec<PMirror> = b.params.iter().map(|s| PMirror { spec: *s, tl: Timeline::default(), base: s.base }).collect();
        // Panner 的 listener 参数跟着上下文当前的 listener 走
        if kind == Kind::Panner {
            let l = &self.web.ctxs[&ctx].listener;
            for (i, m) in l.iter().enumerate() {
                params[6 + i].tl = m.tl.clone();
                params[6 + i].base = m.base;
            }
        }
        let n = WNode {
            ctx,
            kind,
            slot: Some(slot),
            params,
            outs: Vec::new(),
            n_in: b.n_in,
            n_out: b.n_out,
            spec: b.spec,
            started: false,
            stopped: false,
            js_alive: true,
            has_buffer: false,
            analyser: b.analyser.map(|s| (s, Spectrum::default())),
            props: Default::default(),
            iir: (kind == Kind::Iir).then(|| (o.ff.clone(), o.fb.clone())),
            tail_until: None,
            max_delay: o.max_delay,
        };
        self.web.nodes.insert(id, n);
        self.web.slot_owner.insert(slot, id);
        self.web.ctxs.get_mut(&ctx).unwrap().nodes.push(id);
        self.web_send(GCmd::Add { slot, s: b.slot });
        if kind == Kind::Panner {
            for i in 6..15 {
                self.send_timeline(id, i);
            }
        }
        Ok(id)
    }

    pub(super) fn send_timeline(&mut self, id: u32, param: usize) {
        let Some(n) = self.web.nodes.get(&id) else { return };
        let (Some(slot), Some(p)) = (n.slot, n.params.get(param)) else { return };
        let (tl, base) = (Box::new(p.tl.clone()), p.base);
        self.web_send(GCmd::Timeline { slot, param: param as u16, tl, base });
    }

    /// 重新算一个上下文的处理计划发过去
    pub(super) fn flush_plan(&mut self, ctx: usize) {
        if let Some(p) = plan::build(&self.web, ctx) {
            self.web_send(GCmd::Plan { ctx, plan: Box::new(p) });
        }
    }

    /// 节点有没有活着的上游（还有槽位的节点连过来）
    fn has_live_input(&self, id: u32) -> bool {
        self.web.nodes.values().any(|n| n.slot.is_some() && n.outs.iter().any(|c| c.to == id))
    }

    /// 从图里拿掉一个节点（槽位放掉；镜像在 JS 也放掉之前保留）。下游没人要了的跟着拿掉
    pub(super) fn retire_node(&mut self, id: u32) {
        let Some(n) = self.web.nodes.get_mut(&id) else { return };
        let Some(slot) = n.slot.take() else { return };
        let ctx = n.ctx;
        let downstream: Vec<u32> = std::mem::take(&mut n.outs).iter().map(|c| c.to).collect();
        // 先发不含它的计划，再拿掉槽位（同一条队列，顺序保证音频线程不会再处理它）
        self.flush_plan(ctx);
        self.web_send(GCmd::Remove { slot });
        self.web.slot_owner.remove(&slot);
        self.web.free_slots.push(slot);
        if !self.web.nodes[&id].js_alive {
            self.forget(id);
        }
        for d in downstream {
            self.maybe_collect(d);
        }
    }

    fn forget(&mut self, id: u32) {
        if let Some(n) = self.web.nodes.remove(&id) {
            if let Some(c) = self.web.ctxs.get_mut(&n.ctx) {
                c.nodes.retain(|&x| x != id);
            }
        }
        for n in self.web.nodes.values_mut() {
            n.outs.retain(|c| c.to != id);
        }
    }

    /// JS 不再引用、也没有活着的上游 → 拿掉。没开始的源节点 JS 一放就拿掉；开始了的等它播完
    pub(super) fn maybe_collect(&mut self, id: u32) {
        let Some(n) = self.web.nodes.get(&id) else { return };
        if n.js_alive || n.kind == Kind::Destination {
            return;
        }
        if n.slot.is_none() {
            self.forget(id);
            return;
        }
        if n.kind.is_source() && n.started {
            return;
        }
        if !n.kind.is_source() && self.has_live_input(id) {
            self.web.nodes.get_mut(&id).unwrap().tail_until = None;
            return;
        }
        let now = self.web.now(n.ctx);
        let tail = query::tail_time(&self.web, id);
        let n = self.web.nodes.get_mut(&id).unwrap();
        if tail > 0.0 {
            let until = *n.tail_until.get_or_insert(now + tail);
            if now < until {
                return;
            }
        }
        self.retire_node(id);
    }

    fn set_state(&mut self, ctx: usize, s: State) -> Out {
        let Some(c) = self.web.ctxs.get_mut(&ctx) else { return err("InvalidStateError", "上下文不存在") };
        if c.state == State::Closed {
            return err("InvalidStateError", "上下文已经关闭");
        }
        c.state = s;
        self.web_send(GCmd::Running { ctx, on: s == State::Running });
        if s == State::Closed {
            let ids = self.web.ctxs[&ctx].nodes.clone();
            for id in ids {
                if let Some(n) = self.web.nodes.get_mut(&id) {
                    n.js_alive = false;
                }
                self.retire_node(id);
                self.forget(id);
            }
            self.web.ctxs.remove(&ctx);
            self.web_send(GCmd::Plan { ctx, plan: Box::default() });
        }
        ok(json!({}))
    }

    /// pump 里调：源节点播完的回报、解码结果
    pub(super) fn web_tick(&mut self) {
        let tails: Vec<u32> = self.web.nodes.iter().filter(|(_, n)| n.tail_until.is_some() && n.slot.is_some()).map(|(&id, _)| id).collect();
        for id in tails {
            self.maybe_collect(id);
        }
        while let Some(e) = self.ends.ended.pop() {
            let Some(&id) = self.web.slot_owner.get(&e.slot) else { continue };
            self.web.events.push(json!({ "type": "ended", "node": id }));
            self.retire_node(id);
        }
        let wait = self.wait_decodes;
        while self.web.decoding > 0 {
            let got = if wait { self.web.decode_rx.recv_timeout(std::time::Duration::from_secs(10)).ok() } else { self.web.decode_rx.try_recv().ok() };
            let Some((job, ctx, r)) = got else { break };
            self.web.decoding -= 1;
            let ev = match r {
                Ok(pcm) => {
                    let rate = if self.web.ctxs.contains_key(&ctx) { self.web.rate } else { pcm.rate as f64 };
                    let b = query::planar_resampled(&pcm, rate);
                    let id = self.web.next_buf;
                    self.web.next_buf += 1;
                    let meta = json!({ "type": "decoded", "job": job, "buf": id, "channels": b.chans.len(), "length": b.len, "sampleRate": b.rate });
                    self.web.bufs.insert(id, Arc::new(b));
                    meta
                }
                Err((_, msg)) => json!({ "type": "decodeError", "job": job, "errMsg": format!("decodeAudioData:fail {msg}") }),
            };
            self.web.events.push(ev);
        }
    }

    pub fn take_web_events(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.web.events)
    }

    /// `__game_webaudio` 的分派
    pub fn web_call(&mut self, op: &str, a: &Value, bin: Option<Vec<u8>>) -> Out {
        let u = |k: &str| a.get(k).and_then(Value::as_u64).unwrap_or(0) as u32;
        match op {
            "ctxNew" => self.ctx_new(),
            "suspend" => self.set_state(u("ctx") as usize, State::Suspended),
            "resume" => self.set_state(u("ctx") as usize, State::Running),
            "close" => self.set_state(u("ctx") as usize, State::Closed),
            "time" => ok(json!(self.web.now(u("ctx") as usize))),
            "node" => self.op_node(a),
            "connect" => self.op_connect(a),
            "disconnect" => self.op_disconnect(a),
            "free" => {
                if let Some(n) = self.web.nodes.get_mut(&u("node")) {
                    n.js_alive = false;
                }
                self.maybe_collect(u("node"));
                ok(json!({}))
            }
            "param" => self.op_param(a, bin),
            "pget" => self.op_pget(a),
            "start" => self.op_start(a),
            "stop" => self.op_stop(a),
            "prop" => self.op_prop(a, bin),
            "wave" => self.op_wave(a),
            "bufNew" => self.op_buf_new(a, bin),
            "bufGet" => self.op_buf_get(a),
            "bufFree" => {
                self.web.bufs.remove(&u("buf"));
                ok(json!({}))
            }
            "decode" => self.op_decode(a, bin),
            "analyser" => self.op_analyser(a),
            "freqResponse" => self.op_freq_response(a, bin),
            // 诊断 / 测试：镜像里的节点数、占着的槽位数、AudioBuffer 数
            "stats" => ok(json!({ "nodes": self.web.nodes.len(), "slots": self.web.slot_owner.len(), "bufs": self.web.bufs.len() })),
            _ => err("NotSupportedError", &format!("未知操作 {op}")),
        }
    }

    pub(super) fn web_mode(s: &str) -> Option<Mode> {
        Some(match s {
            "max" => Mode::Max,
            "clamped-max" => Mode::ClampedMax,
            "explicit" => Mode::Explicit,
            _ => return None,
        })
    }

    pub(super) fn web_interp(s: &str) -> Option<Interp> {
        Some(match s {
            "speakers" => Interp::Speakers,
            "discrete" => Interp::Discrete,
            _ => return None,
        })
    }
}
