//! AnalyserNode（规范 §1.8，按 Chrome RealtimeAnalyser 的算法）。
//!
//! 音频线程：输入原样送到输出，同时下混成单声道写进一个共享环（f32 的位存进 AtomicU32，写完再推进写位置）。
//! 引擎线程：`get*Data` 时从环里取最近 fftSize 个样本 —— 时域直接给；频域加 Blackman 窗（α = 0.16）、FFT、
//! 幅度 / fftSize、与上一次的结果按 smoothingTimeConstant 平滑，再换算 dB。与 Chrome 一样，currentTime 没变就不重算
//! （同一个量子里连调两次，第二次不再平滑一遍）。
//! 环比 fftSize 上限（32768）多留 4096：引擎线程读的时候音频线程最多再写几个量子，不会读到正在覆盖的那段。

use super::bus::{Bus, Interp, QUANTUM};
use super::fft::fft;
use super::{Flow, Node, Param, Quantum};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

pub const MAX_FFT: usize = 32768;
const RING: usize = MAX_FFT + 4096;

pub struct Shared {
    buf: Box<[AtomicU32]>,
    /// 已写入的样本总数
    written: AtomicU64,
}

impl Shared {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { buf: (0..RING).map(|_| AtomicU32::new(0)).collect(), written: AtomicU64::new(0) })
    }

    /// 最近 n 个样本（不够就前面补零）
    pub fn latest(&self, n: usize) -> Vec<f32> {
        let w = self.written.load(Ordering::Acquire) as usize;
        let mut out = vec![0f32; n];
        for (i, o) in out.iter_mut().enumerate() {
            let back = n - i;
            if back <= w {
                *o = f32::from_bits(self.buf[(w - back) % RING].load(Ordering::Relaxed));
            }
        }
        out
    }
}

pub struct Analyser {
    shared: Arc<Shared>,
    mono: Bus,
}

impl Analyser {
    pub fn new(shared: Arc<Shared>) -> Self {
        Self { shared, mono: Bus::new(1) }
    }
}

impl Node for Analyser {
    fn process(&mut self, _q: &Quantum, ins: &[Bus], _params: &[Param], outs: &mut [Bus]) -> Flow {
        let (inp, out) = (&ins[0], &mut outs[0]);
        out.reset(inp.ch);
        for c in 0..inp.ch {
            out.chan_mut(c).copy_from_slice(inp.chan(c));
        }
        self.mono.reset(1);
        self.mono.sum_from(inp, Interp::Speakers);
        let w = self.shared.written.load(Ordering::Relaxed) as usize;
        for (k, v) in self.mono.chan(0).iter().enumerate() {
            self.shared.buf[(w + k) % RING].store(v.to_bits(), Ordering::Relaxed);
        }
        self.shared.written.store((w + QUANTUM) as u64, Ordering::Release);
        Flow::Live
    }
}

/// 引擎线程那一侧：频谱的平滑状态与上一次分析的时刻
pub struct Spectrum {
    pub smoothed: Vec<f64>,
    pub last_time: f64,
}

impl Default for Spectrum {
    fn default() -> Self {
        Self { smoothed: Vec::new(), last_time: -1.0 }
    }
}

impl Spectrum {
    /// 需要时重算一次，返回各频点的幅度（线性，已平滑），长度 fftSize / 2
    pub fn analyse(&mut self, sh: &Shared, fft_size: usize, smoothing: f64, now: f64) -> &[f64] {
        let half = fft_size / 2;
        if self.smoothed.len() != half {
            self.smoothed = vec![0.0; half];
            self.last_time = -1.0;
        }
        if now <= self.last_time {
            return &self.smoothed;
        }
        self.last_time = now;
        let x = sh.latest(fft_size);
        let (a0, a1, a2) = (0.42, 0.5, 0.08);
        let n = fft_size as f64;
        let mut re: Vec<f64> = x
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let t = 2.0 * std::f64::consts::PI * i as f64 / n;
                v as f64 * (a0 - a1 * t.cos() + a2 * (2.0 * t).cos())
            })
            .collect();
        let mut im = vec![0.0; fft_size];
        fft(&mut re, &mut im, false);
        let k = smoothing.clamp(0.0, 1.0);
        for i in 0..half {
            let mag = (re[i] * re[i] + im[i] * im[i]).sqrt() / n;
            let s = k * self.smoothed[i] + (1.0 - k) * mag;
            self.smoothed[i] = if s.is_finite() { s } else { 0.0 };
        }
        &self.smoothed
    }
}

/// 线性幅度 → dB（0 → -inf，与 Chrome 一致）
pub fn db(m: f64) -> f32 {
    (20.0 * m.log10()) as f32
}

/// getByte*FrequencyData 的换算：[minDecibels, maxDecibels] 映射到 0~255，截断取整
pub fn byte_of_db(d: f32, min_db: f64, max_db: f64) -> u8 {
    let range = if max_db == min_db { 1.0 } else { 1.0 / (max_db - min_db) };
    let v = 255.0 * (d as f64 - min_db) * range;
    if v.is_nan() { 0 } else { v.clamp(0.0, 255.0) as u8 }
}

/// getByteTimeDomainData：128 × (1 + x)，钳到 0~255，截断
pub fn byte_of_sample(x: f32) -> u8 {
    (128.0 * (x as f64 + 1.0)).clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 正弦的峰值落在对应频点() {
        let sh = Shared::new();
        let n = 2048;
        let rate = 48_000.0;
        let bin = 64; // 64 × 48000 / 2048 = 1500 Hz
        let f = bin as f64 * rate / n as f64;
        for (i, v) in sh.buf.iter().enumerate().take(n) {
            v.store(((2.0 * std::f64::consts::PI * f * i as f64 / rate).sin() as f32).to_bits(), Ordering::Relaxed);
        }
        sh.written.store(n as u64, Ordering::Release);
        let mut s = Spectrum::default();
        let m = s.analyse(&sh, n, 0.0, 1.0).to_vec();
        let peak = m.iter().enumerate().fold((0, 0.0), |b, (i, &v)| if v > b.1 { (i, v) } else { b }).0;
        assert_eq!(peak, bin);
        // Blackman 窗的相干增益 0.42：幅度 ≈ 0.5 × 0.42
        assert!((m[bin] - 0.21).abs() < 0.01, "{}", m[bin]);
        assert_eq!(s.analyse(&sh, n, 0.5, 1.0)[bin], m[bin], "时刻没变不重算");
        assert_eq!(byte_of_sample(0.0), 128);
    }
}
