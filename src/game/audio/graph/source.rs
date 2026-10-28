//! 有 start / stop 的源节点：AudioBufferSourceNode、ConstantSourceNode（OscillatorNode 在 osc.rs，共用 [`Schedule`]）。
//!
//! 起止帧按 Chrome 的取法（AudioScheduledSourceHandler::UpdateSchedulingInfo）：时刻 × 采样率**向上取整**到帧，
//! 起点早于当前量子就从量子开头放；停止帧落在本量子里，那一帧起输出零、量子结束时报播完。
//! 子样本精度的起点（Chrome 对 BufferSource 的起点小数部分做插值）没做：夹具的起止时刻都落在整帧上。

use super::bus::{Bus, QUANTUM};
use super::{AudioBuf, Flow, Msg, Node, Param, Quantum, Retired};
use std::sync::Arc;

/// 时刻 → 帧（向上取整；离整帧不到 1e-6 帧的按整帧，避免 0.5 × 48000 这种算出 24000.000001）
pub fn time_to_frame(t: f64, rate: f64) -> u64 {
    let x = (t * rate).max(0.0);
    let r = x.round();
    if (x - r).abs() < 1e-6 { r as u64 } else { x.ceil() as u64 }
}

#[derive(Default)]
pub struct Schedule {
    start: Option<f64>,
    stop: Option<f64>,
    done: bool,
}

/// 本量子里该发声的帧区间 `[a, b)`；`end`：本量子之后就算播完了
pub struct Span {
    pub a: usize,
    pub b: usize,
    pub end: bool,
}

impl Schedule {
    pub fn msg(&mut self, m: &Msg) -> bool {
        match *m {
            Msg::Start { when, .. } if self.start.is_none() => self.start = Some(when),
            Msg::Stop { when } if self.start.is_some() => self.stop = Some(when),
            Msg::Start { .. } | Msg::Stop { .. } => {}
            _ => return false,
        }
        true
    }

    pub fn started_at(&self) -> Option<f64> {
        self.start
    }

    pub fn done(&self) -> bool {
        self.done
    }

    /// 这个量子要不要出声、出哪几帧。返回 None：整个量子静音（还没开始或已经结束）
    pub fn span(&mut self, q: &Quantum) -> Option<Span> {
        if self.done {
            return None;
        }
        let start = self.start?;
        let (q0, q1) = (q.frame, q.frame + QUANTUM as u64);
        let sf = time_to_frame(start, q.rate);
        let ef = self.stop.map(|t| time_to_frame(t, q.rate));
        if ef.is_some_and(|e| e <= q0) {
            self.done = true;
            return Some(Span { a: 0, b: 0, end: true });
        }
        if sf >= q1 {
            return None;
        }
        let a = sf.saturating_sub(q0) as usize;
        let (b, end) = match ef {
            Some(e) if e < q1 => ((e - q0) as usize, true),
            _ => (QUANTUM, false),
        };
        if end {
            self.done = true;
        }
        Some(Span { a, b: b.max(a), end })
    }

    /// 源自己播到头（BufferSource 不循环时）
    pub fn finish(&mut self) {
        self.done = true;
    }
}

/// AudioBufferSourceNode。参数：0 playbackRate、1 detune（都是 k-rate）
pub struct BufferSource {
    sch: Schedule,
    buf: Option<Arc<AudioBuf>>,
    looping: bool,
    loop_start: f64,
    loop_end: f64,
    offset: f64,
    duration: Option<f64>,
    /// 读到缓冲的哪一帧（小数）；None = 还没开始
    pos: Option<f64>,
    /// 已经放了多少帧（输出帧，给 duration 用）
    played: f64,
}

impl BufferSource {
    pub fn new() -> Self {
        Self { sch: Schedule::default(), buf: None, looping: false, loop_start: 0.0, loop_end: 0.0, offset: 0.0, duration: None, pos: None, played: 0.0 }
    }

    /// 循环区间（帧）：规范要求 0 ≤ loopStart < loopEnd ≤ 时长才用，否则整段
    fn loop_frames(&self, b: &AudioBuf) -> (f64, f64) {
        let len = b.len as f64;
        let (s, e) = (self.loop_start * b.rate, self.loop_end * b.rate);
        if self.loop_start >= 0.0 && self.loop_end > 0.0 && s < e && e <= len {
            (s, e)
        } else {
            (0.0, len)
        }
    }
}

impl Node for BufferSource {
    fn process(&mut self, q: &Quantum, _ins: &[Bus], params: &[Param], outs: &mut [Bus]) -> Flow {
        let out = &mut outs[0];
        let Some(b) = self.buf.clone() else {
            out.reset(1);
            return match self.sch.span(q) {
                Some(s) if s.end => Flow::Ended,
                _ => Flow::Live,
            };
        };
        // 播完之后输出关掉：下游看到的是 1 声道静音（Chrome 的 DisableOutputs；Delay 的尾巴因此下混成单声道）
        if self.sch.done() {
            out.reset(1);
            return Flow::Live;
        }
        let nch = b.chans.len().clamp(1, out.cap());
        out.reset(nch);
        let Some(span) = self.sch.span(q) else { return Flow::Live };
        let mut p = self.pos.unwrap_or(self.offset.clamp(0.0, b.len as f64 / b.rate) * b.rate);
        let pos = &mut p;
        let rate = params[0].vals[0] as f64 * 2f64.powf(params[1].vals[0] as f64 / 1200.0) * b.rate / q.rate;
        let len = b.len as f64;
        let (ls, le) = self.loop_frames(&b);
        let grain_end = match (self.duration, self.looping) {
            (Some(d), false) => ((self.offset + d) * b.rate).min(len),
            _ => len,
        };
        let max_out = match (self.duration, self.looping) {
            (Some(d), true) => d * q.rate,
            _ => f64::INFINITY,
        };
        let mut ended = span.end;
        for f in span.a..span.b {
            if self.looping {
                if rate >= 0.0 && *pos >= le {
                    *pos = ls + (*pos - le) % (le - ls).max(1.0);
                } else if rate < 0.0 && *pos < ls {
                    *pos = le - (ls - *pos) % (le - ls).max(1.0);
                }
            } else if *pos >= grain_end || *pos < 0.0 {
                ended = true;
                self.sch.finish();
                break;
            }
            if self.played >= max_out {
                ended = true;
                self.sch.finish();
                break;
            }
            let i = *pos as usize;
            let t = (*pos - i as f64) as f32;
            let j = if i + 1 < le as usize || !self.looping { (i + 1).min(b.len.saturating_sub(1)) } else { ls as usize };
            for c in 0..nch {
                let d = &b.chans[c];
                let s1 = d.get(i).copied().unwrap_or(0.0);
                let v = if t == 0.0 { s1 } else { (1.0 - t) * s1 + t * d.get(j).copied().unwrap_or(0.0) };
                out.chan_mut(c)[f] = v;
            }
            *pos += rate;
            self.played += 1.0;
        }
        // 正好在量子末尾放到头：本量子就算播完（Chrome 同样在这个量子结束时 Finish，下一个量子起输出关掉）
        if !ended && !self.looping && (p >= grain_end || p < 0.0 || self.played >= max_out) {
            ended = true;
            self.sch.finish();
        }
        self.pos = Some(p);
        if ended { Flow::Ended } else { Flow::Live }
    }

    fn msg(&mut self, m: Msg) -> Option<Retired> {
        if let Msg::Start { offset, duration, .. } = m {
            if self.sch.started_at().is_none() {
                self.offset = offset.max(0.0);
                self.duration = duration.filter(|d| *d >= 0.0);
            }
        }
        if self.sch.msg(&m) {
            return None;
        }
        match m {
            Msg::Buffer(b) => std::mem::replace(&mut self.buf, b).map(Retired::Buf),
            Msg::Loop { on, start, end } => {
                (self.looping, self.loop_start, self.loop_end) = (on, start, end);
                None
            }
            _ => None,
        }
    }
}

/// ConstantSourceNode。参数：0 offset（a-rate）
#[derive(Default)]
pub struct Constant {
    sch: Schedule,
}

impl Node for Constant {
    fn process(&mut self, q: &Quantum, _ins: &[Bus], params: &[Param], outs: &mut [Bus]) -> Flow {
        let out = &mut outs[0];
        out.reset(1);
        let Some(s) = self.sch.span(q) else { return Flow::Live };
        out.chan_mut(0)[s.a..s.b].copy_from_slice(&params[0].vals[s.a..s.b]);
        if s.end { Flow::Ended } else { Flow::Live }
    }

    fn msg(&mut self, m: Msg) -> Option<Retired> {
        self.sch.msg(&m);
        None
    }
}
