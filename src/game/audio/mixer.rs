//! 混音器（音频线程上跑，game-doc/02 §8.1）：Voice 表 + 线性插值重采样 + 增益平滑。
//!
//! 只在 `render` 里被调用；**不分配、不加锁、不做 I/O**。命令从 SPSC 队列取（ring.rs），回报（播放位置、播完）放回另一条；
//! 不再用的 PCM（`Arc`）也经队列送回引擎线程去释放 —— 最后一个引用在这里掉了就会在音频线程上 free。
//! 离线模式（无头 / 设备打不开）同一个混音器由引擎线程按帧调用，结果逐样本相同。

use super::graph::{GCmd, Graph, Retired};
use super::ring::{Consumer, Producer};
use crate::audio_ctx::decode::Pcm;
use std::sync::Arc;

/// 同时发声的上限（game-doc/02 §8.1；微信的真实上限待取证）
pub const MAX_VOICES: usize = 32;

pub enum Cmd {
    Start { slot: usize, gen: u32, pcm: Arc<Pcm>, at: f64, gain: f32, rate: f32, looping: bool },
    Pause { slot: usize, gen: u32 },
    Resume { slot: usize, gen: u32 },
    Stop { slot: usize, gen: u32 },
    Seek { slot: usize, gen: u32, at: f64 },
    Gain { slot: usize, gen: u32, gain: f32 },
    Rate { slot: usize, gen: u32, rate: f32 },
    Loop { slot: usize, gen: u32, looping: bool },
    /// 进后台 / 打断：整体静音暂停（Voice 原样留着）
    Suspend(bool),
    /// WebAudio 混音图的命令（graph/mod.rs）
    Graph(GCmd),
}

/// 每次 render 之后，每个在发声的 Voice 一条：(槽, 代数, 源里的位置（秒）, 是否播完)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Report {
    pub slot: usize,
    pub gen: u32,
    pub pos: f64,
    pub ended: bool,
}

struct Voice {
    gen: u32,
    pcm: Arc<Pcm>,
    /// 源帧位置（小数）
    pos: f64,
    gain: f32,
    /// 当前实际增益（向 gain 平滑靠拢，避免改音量时咔哒一声）
    cur: f32,
    rate: f32,
    looping: bool,
    playing: bool,
}

pub struct Mixer {
    out_rate: u32,
    channels: usize,
    voices: [Option<Voice>; MAX_VOICES],
    suspended: bool,
    cmds: Consumer<Cmd>,
    reports: Producer<Report>,
    retire: Producer<Retired>,
    /// WebAudio 的图：与 Voice 叠在同一个输出上
    graph: Graph,
    /// 已经输出的帧数（播放时钟）
    pub frames: u64,
}

impl Mixer {
    pub fn new(out_rate: u32, channels: usize, cmds: Consumer<Cmd>, reports: Producer<Report>, retire: Producer<Retired>, graph: Graph) -> Self {
        Self { out_rate: out_rate.max(1), channels: channels.max(1), voices: Default::default(), suspended: false, cmds, reports, retire, graph, frames: 0 }
    }

    fn drop_voice(&mut self, slot: usize) {
        if let Some(v) = self.voices[slot].take() {
            // 送回引擎线程释放；队列满了就只能在这里放（极少，计数器以后在 MINI_AUDIO_LOG 里报）
            let _ = self.retire.push(Retired::Pcm(v.pcm));
        }
    }

    fn voice(&mut self, slot: usize, gen: u32) -> Option<&mut Voice> {
        self.voices.get_mut(slot)?.as_mut().filter(|v| v.gen == gen)
    }

    fn apply(&mut self, c: Cmd) {
        match c {
            Cmd::Start { slot, gen, pcm, at, gain, rate, looping } if slot < MAX_VOICES => {
                self.drop_voice(slot);
                let pos = at.max(0.0) * pcm.rate as f64;
                self.voices[slot] = Some(Voice { gen, pcm, pos, gain, cur: gain, rate, looping, playing: true });
            }
            Cmd::Pause { slot, gen } => {
                if let Some(v) = self.voice(slot, gen) {
                    v.playing = false;
                }
            }
            Cmd::Resume { slot, gen } => {
                if let Some(v) = self.voice(slot, gen) {
                    v.playing = true;
                }
            }
            Cmd::Stop { slot, gen } => {
                if self.voice(slot, gen).is_some() {
                    self.drop_voice(slot);
                }
            }
            Cmd::Seek { slot, gen, at } => {
                if let Some(v) = self.voice(slot, gen) {
                    v.pos = at.max(0.0) * v.pcm.rate as f64;
                }
            }
            Cmd::Gain { slot, gen, gain } => {
                if let Some(v) = self.voice(slot, gen) {
                    v.gain = gain;
                }
            }
            Cmd::Rate { slot, gen, rate } => {
                if let Some(v) = self.voice(slot, gen) {
                    v.rate = rate;
                }
            }
            Cmd::Loop { slot, gen, looping } => {
                if let Some(v) = self.voice(slot, gen) {
                    v.looping = looping;
                }
            }
            Cmd::Suspend(s) => self.suspended = s,
            Cmd::Graph(g) => self.graph.apply(g, &mut self.retire),
            Cmd::Start { .. } => {}
        }
    }

    /// 填满 `out`（交错，`channels` 声道）。音频线程每个回调调一次
    pub fn render(&mut self, out: &mut [f32]) {
        while let Some(c) = self.cmds.pop() {
            self.apply(c);
        }
        out.fill(0.0);
        let ch = self.channels;
        let frames = out.len() / ch;
        if !self.suspended {
            for slot in 0..MAX_VOICES {
                let Some(v) = self.voices[slot].as_mut() else { continue };
                if !v.playing {
                    continue;
                }
                let ended = mix_voice(v, out, ch, frames, self.out_rate);
                let pos = v.pos / v.pcm.rate.max(1) as f64;
                let _ = self.reports.push(Report { slot, gen: v.gen, pos, ended });
                if ended {
                    self.drop_voice(slot);
                }
            }
            self.graph.render(out);
        }
        for s in out.iter_mut() {
            *s = s.clamp(-1.0, 1.0);
        }
        self.frames += frames as u64;
    }
}

/// 把一个 Voice 叠进 `out`；返回是否播到头（不循环时）
fn mix_voice(v: &mut Voice, out: &mut [f32], ch: usize, frames: usize, out_rate: u32) -> bool {
    let pcm = &v.pcm;
    let src_ch = pcm.channels.max(1) as usize;
    let len = pcm.samples.len() / src_ch;
    if len == 0 {
        return true;
    }
    let step = pcm.rate as f64 / out_rate as f64 * v.rate as f64;
    // 增益在一块里线性过渡到目标
    let (g0, g1) = (v.cur, v.gain);
    let dg = (g1 - g0) / frames.max(1) as f32;
    for f in 0..frames {
        if v.pos >= len as f64 {
            if v.looping {
                v.pos -= len as f64;
            } else {
                v.cur = g1;
                return true;
            }
        }
        let i = v.pos as usize;
        let t = (v.pos - i as f64) as f32;
        let j = if i + 1 < len { i + 1 } else if v.looping { 0 } else { i };
        let g = g0 + dg * f as f32;
        for c in 0..ch.min(2) {
            let sc = if src_ch == 1 { 0 } else { c.min(src_ch - 1) };
            let a = pcm.samples[i * src_ch + sc];
            let b = pcm.samples[j * src_ch + sc];
            out[f * ch + c] += (a + (b - a) * t) * g;
        }
        v.pos += step;
    }
    v.cur = g1;
    false
}

#[cfg(test)]
mod tests {
    use super::super::ring::ring;
    use super::*;

    fn mixer() -> (Mixer, Producer<Cmd>, Consumer<Report>, Consumer<Retired>) {
        let (cp, cc) = ring(64);
        let (rp, rc) = ring(1024);
        let (tp, tc) = ring(64);
        let (ep, _) = ring(8);
        let clocks = Arc::new(std::array::from_fn(|_| std::sync::atomic::AtomicU64::new(0)));
        (Mixer::new(48_000, 2, cc, rp, tp, Graph::new(48_000, 2, clocks, ep)), cp, rc, tc)
    }

    fn ramp(n: usize, rate: u32) -> Arc<Pcm> {
        Arc::new(Pcm { samples: Arc::new((0..n).map(|i| i as f32 / n as f32).collect()), rate, channels: 1 })
    }

    #[test]
    fn 同采样率逐样本原样_播完回报并还回_pcm() {
        let (mut m, mut cmd, mut rep, mut ret) = mixer();
        let pcm = ramp(100, 48_000);
        cmd.push(Cmd::Start { slot: 3, gen: 7, pcm: pcm.clone(), at: 0.0, gain: 1.0, rate: 1.0, looping: false }).ok();
        let mut out = vec![0f32; 2 * 64];
        m.render(&mut out);
        assert_eq!((out[0], out[1], out[2 * 10]), (0.0, 0.0, 0.1));
        assert_eq!(out[2 * 10 + 1], 0.1, "单声道铺到两个声道");
        m.render(&mut out);
        assert_eq!(out[2 * 35], 0.99, "第二块接着第 64 帧");
        assert_eq!(out[2 * 36], 0.0, "播完之后是静音");
        let r: Vec<Report> = std::iter::from_fn(|| rep.pop()).collect();
        assert_eq!(r.last().map(|r| (r.slot, r.gen, r.ended)), Some((3, 7, true)));
        drop(pcm);
        let Some(Retired::Pcm(back)) = ret.pop() else { panic!("PCM 送回引擎线程") };
        assert_eq!(Arc::strong_count(&back), 1, "音频线程不留引用");
    }

    #[test]
    fn 倍速_重采样_循环_代数不对的命令不生效() {
        let (mut m, mut cmd, _rep, _ret) = mixer();
        // 24k 的源在 48k 输出：每个输出帧走半个源帧
        cmd.push(Cmd::Start { slot: 0, gen: 1, pcm: ramp(10, 24_000), at: 0.0, gain: 1.0, rate: 1.0, looping: true }).ok();
        cmd.push(Cmd::Gain { slot: 0, gen: 99, gain: 0.0 }).ok();
        let mut out = vec![0f32; 2 * 30];
        m.render(&mut out);
        assert!((out[2] - 0.05).abs() < 1e-6, "半帧处线性插值：{}", out[2]);
        assert_eq!(out[2 * 20], 0.0, "20 个输出帧 = 10 个源帧：循环回到开头");
        assert!(out[2 * 22] > 0.0, "代数不对的静音命令被忽略");
    }

    #[test]
    fn 挂起时静音但位置不动() {
        let (mut m, mut cmd, mut rep, _ret) = mixer();
        cmd.push(Cmd::Start { slot: 1, gen: 1, pcm: ramp(48_000, 48_000), at: 0.5, gain: 1.0, rate: 1.0, looping: false }).ok();
        cmd.push(Cmd::Suspend(true)).ok();
        let mut out = vec![1f32; 2 * 128];
        m.render(&mut out);
        assert!(out.iter().all(|&s| s == 0.0));
        assert!(rep.pop().is_none(), "挂起时不回报");
        cmd.push(Cmd::Suspend(false)).ok();
        m.render(&mut out);
        assert!((out[0] - 0.5).abs() < 1e-6, "从 0.5s 接着放：{}", out[0]);
    }
}
