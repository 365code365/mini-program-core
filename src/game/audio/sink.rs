//! 给**小程序**的 InnerAudioContext / BackgroundAudioManager 出声用的设备混音器（`audio_ctx/output.rs` 在移动端用它）。
//!
//! 为什么在这里：移动端构建不带 `audio`（rodio）特性，从前 `audio_ctx` 的输出是空实现 —— 播放时钟照走、事件照发，
//! 手机上就是没声音（静心播放页进度条在走、听不见）。小游戏那一套（混音器 + Android AAudio / iOS cpal）已经在移动端
//! 验过，这里只借它的混音器与设备输出：一个上下文占一个槽，开播 / 停 / 改音量三件事。
//! 位置与事件仍由 `audio_ctx` 按时钟算（与桌面 rodio 那条一致），这里不回报位置，混音器的回报只取走丢掉。
use super::graph::{self, Graph};
use super::mixer::{Cmd, Mixer, MAX_VOICES};
use super::output::{self, Output};
use super::ring::{ring, Consumer, Producer};
use crate::audio_ctx::decode::Pcm;
use std::collections::HashMap;
use std::sync::Arc;

struct Ends {
    cmds: Producer<Cmd>,
    reports: Consumer<super::mixer::Report>,
    retire: Consumer<graph::Retired>,
    _ended: Consumer<graph::Ended>,
}

pub struct Sink {
    out: Output,
    ends: Ends,
    /// 上下文 id → (槽, 代数)
    voices: HashMap<u32, (usize, u32)>,
    next_gen: u32,
    /// 打开时的说明（MINI_AUDIO_LOG 打出来）
    pub note: String,
}

impl Sink {
    /// 开设备；打不开（模拟器没声卡、故障注入）时落到离线模式，照样能调，只是不出声
    pub fn open() -> Self {
        let clocks: Arc<graph::Clocks> = Arc::new(std::array::from_fn(|_| Default::default()));
        let (opened, ends) = output::open(false, None, |rate, channels| {
            let (cp, cc) = ring(256);
            let (rp, rc) = ring(4096);
            let (tp, tc) = ring(256);
            let (ep, ec) = ring(16);
            let g = Graph::new(rate, channels, clocks.clone(), ep);
            (Mixer::new(rate, channels, cc, rp, tp, g), Ends { cmds: cp, reports: rc, retire: tc, _ended: ec })
        });
        Self { out: opened.out, ends, voices: HashMap::new(), next_gen: 1, note: opened.note }
    }

    fn send(&mut self, c: Cmd) {
        // 队列 256 条，引擎线程每帧取空回报 / 回收；满了说明音频线程停了，丢掉这条不比卡住引擎线程差
        let _ = self.ends.cmds.push(c);
    }

    /// 从 `from_sec` 开始放（同一个 id 已在放就先停掉，与 rodio 那条一致）
    pub fn start(&mut self, id: u32, samples: Arc<Vec<f32>>, rate: u32, channels: u16, from_sec: f64, volume: f32, speed: f32) {
        self.stop(id);
        let used: Vec<usize> = self.voices.values().map(|v| v.0).collect();
        let Some(slot) = (0..MAX_VOICES).find(|s| !used.contains(s)) else { return };
        let gen = self.next_gen;
        self.next_gen = self.next_gen.wrapping_add(1).max(1);
        let pcm = Arc::new(Pcm { samples, rate, channels });
        self.voices.insert(id, (slot, gen));
        self.send(Cmd::Start { slot, gen, pcm, at: from_sec, gain: volume, rate: speed, looping: false });
    }

    pub fn stop(&mut self, id: u32) {
        if let Some((slot, gen)) = self.voices.remove(&id) {
            self.send(Cmd::Stop { slot, gen });
        }
    }

    pub fn set_volume(&mut self, id: u32, gain: f32) {
        if let Some(&(slot, gen)) = self.voices.get(&id) {
            self.send(Cmd::Gain { slot, gen, gain });
        }
    }

    /// 全部停掉（小程序实例销毁时）
    pub fn stop_all(&mut self) {
        let ids: Vec<u32> = self.voices.keys().copied().collect();
        for id in ids {
            self.stop(id);
        }
    }

    /// 引擎线程每帧调：取空回报、回收不用的 PCM、设备断开就重开；播完的槽让出来。返回要记的日志
    pub fn tick(&mut self) -> Option<String> {
        while let Some(r) = self.ends.reports.pop() {
            if r.ended {
                self.voices.retain(|_, v| !(v.0 == r.slot && v.1 == r.gen));
            }
        }
        while self.ends.retire.pop().is_some() {}
        self.out.poll(self.voices.is_empty())
    }
}
