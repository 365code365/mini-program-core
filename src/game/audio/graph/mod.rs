//! WebAudio 的真混音图（game-doc/02 §8.1、04 G5.3）：音频线程上按 128 帧的渲染量子逐节点处理。
//!
//! 分工：引擎线程（`audio/web`）维护节点表的镜像、连线、参数时间线，算好每个上下文的处理顺序（[`Plan`]），
//! 连同建好的节点（[`Slot`]，缓冲在引擎线程分配）一起经命令队列发过来。音频线程只做三件事：换表、按顺序处理、
//! 把不再用的东西经回收队列送回去释放 —— **不分配、不加锁**（与 mixer.rs 同一条规矩）。
//!
//! 处理顺序是拓扑序；环上的边读的是源节点**上一个量子**的输出（多出一个量子的延迟）。规范要求环里必须有
//! DelayNode 且把它拆成读写两半、延迟钳到 ≥ 1 个量子；这里没拆，带延迟的反馈环总延迟多 128 帧（写在验收文档）。
//! 没有 DelayNode 的环规范要求静音，这里照常算（同样多一个量子的延迟）。

pub mod bus;
pub mod channels;
pub mod effects;
pub mod fft;
pub mod filter;
pub mod osc;
pub mod panner;
pub mod param;
pub mod source;
pub mod analyser;

use super::ring::Producer;
use bus::{computed_channels, Bus, Chan, Interp, Mode, MAX_CH, QUANTUM};
use param::Timeline;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// 节点槽位上限（所有上下文合计）
pub const MAX_NODES: usize = 4096;
/// 上下文上限
pub const MAX_CTX: usize = 32;

/// 一个渲染量子的时刻
#[derive(Clone, Copy)]
pub struct Quantum {
    /// 本量子第一帧在所属上下文里的帧号
    pub frame: u64,
    pub rate: f64,
}

impl Quantum {
    pub fn time(&self) -> f64 {
        self.frame as f64 / self.rate
    }
}

/// 节点处理一个量子之后的状态
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Live,
    /// 源节点播完 / 停了（只报一次）
    Ended,
}

/// 发给某个节点的消息（引擎线程 → 音频线程）
pub enum Msg {
    Start { when: f64, offset: f64, duration: Option<f64> },
    Stop { when: f64 },
    Buffer(Option<Arc<AudioBuf>>),
    Loop { on: bool, start: f64, end: f64 },
    /// 振荡器的波表 / 滤波器的类型（各自解释）
    Wave(Arc<osc::WaveTables>),
    Kind(u8),
    Curve(Option<Box<[f32]>>),
    Panner(panner::Config),
}

/// 送回引擎线程释放的东西。字段不读：拿着它们只为了让最后一次 drop 发生在引擎线程上
#[allow(dead_code)]
pub enum Retired {
    Pcm(Arc<crate::audio_ctx::decode::Pcm>),
    Slot(Box<Slot>),
    Plan(Box<Plan>),
    Timeline(Box<Timeline>),
    Buf(Arc<AudioBuf>),
    Wave(Arc<osc::WaveTables>),
    Curve(Box<[f32]>),
}

pub trait Node: Send {
    /// `ins` 已按输入规则混好；`params` 是本量子各参数逐帧的值（含调制输入）。输出声道数由节点自己定
    fn process(&mut self, q: &Quantum, ins: &[Bus], params: &[Param], outs: &mut [Bus]) -> Flow;
    /// 处理消息；换下来的旧东西返回去释放
    fn msg(&mut self, _m: Msg) -> Option<Retired> {
        None
    }
}

/// 平面 PCM（AudioBuffer 的原生副本）
pub struct AudioBuf {
    pub rate: f64,
    pub len: usize,
    pub chans: Box<[Box<[f32]>]>,
}

/// 一个 AudioParam 在音频线程上的状态
pub struct Param {
    pub tl: Box<Timeline>,
    pub base: f64,
    pub min: f32,
    pub max: f32,
    /// k-rate：一个量子只取第一帧的值
    pub krate: bool,
    pub vals: Chan,
    /// 本量子里值变过（滤波器 / 声像据此决定逐样本还是整块算系数）
    pub varies: bool,
    input: Bus,
}

impl Param {
    pub fn new(base: f64, min: f32, max: f32, krate: bool) -> Self {
        Self { tl: Box::default(), base, min, max, krate, vals: [base as f32; QUANTUM], varies: false, input: Bus::new(1) }
    }

    fn compute(&mut self, q: &Quantum, has_input: bool) {
        let dt = 1.0 / q.rate;
        if self.krate {
            let v = self.tl.value_at(q.time(), self.base) as f32 + if has_input { self.input.chan(0)[0] } else { 0.0 };
            self.vals.fill(v.clamp(self.min, self.max));
            self.varies = false;
            return;
        }
        self.tl.fill(&mut self.vals, q.time(), dt, self.base);
        if has_input {
            for (v, i) in self.vals.iter_mut().zip(self.input.chan(0).iter()) {
                *v += i;
            }
        }
        for v in self.vals.iter_mut() {
            *v = v.clamp(self.min, self.max);
        }
        self.varies = self.vals[0] != self.vals[QUANTUM - 1] || self.vals.iter().any(|&v| v != self.vals[0]);
    }
}

/// 一路输入的混音规则
#[derive(Clone, Copy)]
pub struct InSpec {
    pub count: usize,
    pub mode: Mode,
    pub interp: Interp,
}

/// 音频线程上的一个节点
pub struct Slot {
    pub node: Box<dyn Node>,
    pub spec: Box<[InSpec]>,
    pub ins: Box<[Bus]>,
    pub outs: Box<[Bus]>,
    pub params: Box<[Param]>,
    ended: bool,
}

impl Slot {
    pub fn new(node: Box<dyn Node>, spec: Vec<InSpec>, in_cap: Vec<usize>, out_cap: Vec<usize>, params: Vec<Param>) -> Box<Self> {
        let ins = in_cap.into_iter().map(Bus::new).collect();
        let outs = out_cap.into_iter().map(Bus::new).collect();
        Box::new(Self { node, spec: spec.into(), ins, outs, params: params.into(), ended: false })
    }
}

/// 一条连线：`src` 的第 `out` 路输出 → `dst` 的第 `port` 路输入（`param` 时是第 port 个参数）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    pub dst: u32,
    pub port: u16,
    pub param: bool,
    pub src: u32,
    pub out: u16,
}

/// 一个上下文的处理计划：`order[i]` 的入边是 `edges[starts[i]..starts[i + 1]]`
#[derive(Default, Debug)]
pub struct Plan {
    pub order: Vec<u32>,
    pub starts: Vec<u32>,
    pub edges: Vec<Edge>,
    pub dest: u32,
}

/// 混音图的命令
pub enum GCmd {
    Add { slot: u32, s: Box<Slot> },
    Remove { slot: u32 },
    Plan { ctx: usize, plan: Box<Plan> },
    /// 上下文开始 / 暂停渲染（closed 也走这条：false 且不再改回来）
    Running { ctx: usize, on: bool },
    Timeline { slot: u32, param: u16, tl: Box<Timeline>, base: f64 },
    Msg { slot: u32, m: Msg },
    Channels { slot: u32, input: u16, spec: InSpec },
}

/// 回报：源节点播完
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ended {
    pub slot: u32,
}

/// 每个上下文已经渲染的帧数（引擎线程读 currentTime）
pub type Clocks = [AtomicU64; MAX_CTX];

struct CtxState {
    plan: Option<Box<Plan>>,
    running: bool,
}

pub struct Graph {
    rate: f64,
    out_ch: usize,
    slots: Vec<Option<Box<Slot>>>,
    ctxs: Vec<CtxState>,
    clocks: Arc<Clocks>,
    ended: Producer<Ended>,
    /// 已渲染还没交给设备的一个量子（交错前的平面形式）与读到哪一帧
    fifo: Bus,
    fifo_pos: usize,
    scratch: Bus,
    any_running: bool,
}

impl Graph {
    pub fn new(rate: u32, out_ch: usize, clocks: Arc<Clocks>, ended: Producer<Ended>) -> Self {
        let out_ch = out_ch.clamp(1, MAX_CH);
        let mut slots = Vec::with_capacity(MAX_NODES);
        slots.resize_with(MAX_NODES, || None);
        let ctxs = (0..MAX_CTX).map(|_| CtxState { plan: None, running: false }).collect();
        Self { rate: rate.max(1) as f64, out_ch, slots, ctxs, clocks, ended, fifo: Bus::new(out_ch), fifo_pos: QUANTUM, scratch: Bus::new(out_ch), any_running: false }
    }

    pub fn apply(&mut self, c: GCmd, retire: &mut Producer<Retired>) {
        let mut rt = |r: Retired| {
            let _ = retire.push(r);
        };
        match c {
            GCmd::Add { slot, s } => {
                if let Some(old) = self.slots.get_mut(slot as usize).and_then(|x| x.replace(s)) {
                    rt(Retired::Slot(old));
                }
            }
            GCmd::Remove { slot } => {
                if let Some(old) = self.slots.get_mut(slot as usize).and_then(Option::take) {
                    rt(Retired::Slot(old));
                }
            }
            GCmd::Plan { ctx, plan } => {
                if let Some(c) = self.ctxs.get_mut(ctx) {
                    if let Some(old) = c.plan.replace(plan) {
                        rt(Retired::Plan(old));
                    }
                }
            }
            GCmd::Running { ctx, on } => {
                if let Some(c) = self.ctxs.get_mut(ctx) {
                    c.running = on;
                }
                self.any_running = self.ctxs.iter().any(|c| c.running);
            }
            GCmd::Timeline { slot, param, tl, base } => {
                if let Some(p) = self.slot_mut(slot).and_then(|s| s.params.get_mut(param as usize)) {
                    p.base = base;
                    rt(Retired::Timeline(std::mem::replace(&mut p.tl, tl)));
                } else {
                    rt(Retired::Timeline(tl));
                }
            }
            GCmd::Msg { slot, m } => {
                if let Some(s) = self.slot_mut(slot) {
                    if let Some(old) = s.node.msg(m) {
                        rt(old);
                    }
                }
            }
            GCmd::Channels { slot, input, spec } => {
                if let Some(x) = self.slot_mut(slot).and_then(|s| s.spec.get_mut(input as usize)) {
                    *x = spec;
                }
            }
        }
    }

    fn slot_mut(&mut self, slot: u32) -> Option<&mut Slot> {
        self.slots.get_mut(slot as usize).and_then(|s| s.as_deref_mut())
    }

    /// 把图的输出叠进交错的设备缓冲 `out`（声道数 = 建图时给的 out_ch）
    pub fn render(&mut self, out: &mut [f32]) {
        if !self.any_running && self.fifo_pos >= QUANTUM {
            return;
        }
        let ch = self.out_ch;
        let frames = out.len() / ch;
        let mut f = 0;
        while f < frames {
            if self.fifo_pos >= QUANTUM {
                if !self.any_running {
                    return;
                }
                self.render_quantum();
                self.fifo_pos = 0;
            }
            let n = (QUANTUM - self.fifo_pos).min(frames - f);
            for c in 0..self.fifo.ch.min(ch) {
                let src = &self.fifo.chan(c)[self.fifo_pos..self.fifo_pos + n];
                for (k, s) in src.iter().enumerate() {
                    out[(f + k) * ch + c] += s;
                }
            }
            self.fifo_pos += n;
            f += n;
        }
    }

    fn render_quantum(&mut self) {
        self.fifo.reset(self.out_ch);
        for ci in 0..MAX_CTX {
            if !self.ctxs[ci].running {
                continue;
            }
            let Some(plan) = self.ctxs[ci].plan.take() else {
                self.clocks[ci].fetch_add(QUANTUM as u64, Ordering::Relaxed);
                continue;
            };
            let q = Quantum { frame: self.clocks[ci].load(Ordering::Relaxed), rate: self.rate };
            for (i, &slot) in plan.order.iter().enumerate() {
                let (a, b) = (plan.starts[i] as usize, plan.starts[i + 1] as usize);
                self.process(slot, &plan.edges[a..b], &q);
            }
            let Self { slots, fifo, scratch, out_ch, .. } = self;
            if let Some(b) = slots.get(plan.dest as usize).and_then(|s| s.as_deref()).and_then(|d| d.ins.first()) {
                scratch.reset(*out_ch);
                scratch.sum_from(b, Interp::Speakers);
                for c in 0..*out_ch {
                    let s = scratch.chan(c);
                    fifo.chan_mut(c).iter_mut().zip(s.iter()).for_each(|(x, y)| *x += y);
                }
            }
            self.ctxs[ci].plan = Some(plan);
            self.clocks[ci].fetch_add(QUANTUM as u64, Ordering::Relaxed);
        }
    }

    /// 处理一个节点：混输入 → 算参数 → 节点自己处理
    fn process(&mut self, slot: u32, edges: &[Edge], q: &Quantum) {
        let si = slot as usize;
        let Some(mut me) = self.slots.get_mut(si).and_then(Option::take) else { return };
        // 输入：每一路先按所有来源的最大声道数定 computedNumberOfChannels，再逐个混进来
        for (port, bus) in me.ins.iter_mut().enumerate() {
            let spec = me.spec[port];
            let max_in = edges
                .iter()
                .filter(|e| !e.param && e.port as usize == port)
                .filter_map(|e| self.out_bus(e))
                .map(|b| b.ch)
                .max()
                .unwrap_or(0);
            let n = if max_in == 0 { 1 } else { computed_channels(spec.mode, spec.count, max_in) };
            bus.reset(n);
            for e in edges.iter().filter(|e| !e.param && e.port as usize == port) {
                if let Some(src) = self.out_bus(e) {
                    bus.sum_from(src, spec.interp);
                }
            }
        }
        for (pi, p) in me.params.iter_mut().enumerate() {
            let mut has = false;
            p.input.reset(1);
            for e in edges.iter().filter(|e| e.param && e.port as usize == pi) {
                if let Some(src) = self.out_bus(e) {
                    p.input.sum_from(src, Interp::Speakers);
                    has = true;
                }
            }
            p.compute(q, has);
        }
        let Slot { node, ins, outs, params, ended, .. } = &mut *me;
        if node.process(q, ins, params, outs) == Flow::Ended && !*ended {
            *ended = true;
            let _ = self.ended.push(Ended { slot });
        }
        self.slots[si] = Some(me);
    }

    fn out_bus(&self, e: &Edge) -> Option<&Bus> {
        self.slots.get(e.src as usize)?.as_deref()?.outs.get(e.out as usize)
    }
}

#[cfg(test)]
mod tests;
