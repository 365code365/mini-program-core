//! OscillatorNode 与 PeriodicWave 波表（按 Chrome 的 PeriodicWave 做法，规范 §1.29 只定义了理想波形）。
//!
//! - 波表长 4096（采样率 ≤ 88.2k；≤ 24k 用 2048、更高用 16384），分 `round(3·log2(表长))` 个音区，每个音区保留的
//!   谐波数 = 表长/2 × 2^(-音区/3)，频率越高谐波越少（限带，防混叠）；
//! - 只按第 0 个音区（谐波最全）的峰值归一化，所有音区乘同一个系数（disableNormalization 时不乘）；
//! - 发声时按频率在相邻两个音区之间线性插值，每个音区内按读位置线性插值；
//! - 内置波形的系数：sine b1 = 1；square b_k = 4/(kπ)（奇次）；sawtooth b_k = (-1)^(k+1)·2/(kπ)；
//!   triangle b_k = 8·sin(kπ/2)/(kπ)²。
//!
//! 波表在引擎线程生成（36 次 4096 点 IFFT），同一个（波形, 采样率）共用一份（`Arc`），音频线程只读。
//! 参数：0 frequency、1 detune（a-rate）。

use super::bus::{Bus, QUANTUM};
use super::fft::fft;
use super::source::Schedule;
use super::{Flow, Msg, Node, Param, Quantum, Retired};
use std::sync::Arc;

pub struct WaveTables {
    pub size: usize,
    pub tables: Vec<Box<[f32]>>,
    rate_scale: f64,
    lowest: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Builtin {
    Sine,
    Square,
    Sawtooth,
    Triangle,
}

impl Builtin {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "sine" => Self::Sine,
            "square" => Self::Square,
            "sawtooth" => Self::Sawtooth,
            "triangle" => Self::Triangle,
            _ => return None,
        })
    }
}

fn wave_size(rate: f64) -> usize {
    if rate <= 24_000.0 {
        2048
    } else if rate <= 88_200.0 {
        4096
    } else {
        16384
    }
}

impl WaveTables {
    pub fn builtin(kind: Builtin, rate: f64) -> Self {
        let half = wave_size(rate) / 2;
        let (mut a, mut b) = (vec![0.0; half], vec![0.0; half]);
        let pi = std::f64::consts::PI;
        for k in 1..half {
            let kf = k as f64;
            b[k] = match kind {
                Builtin::Sine => if k == 1 { 1.0 } else { 0.0 },
                Builtin::Square => if k % 2 == 1 { 4.0 / (kf * pi) } else { 0.0 },
                Builtin::Sawtooth => if k % 2 == 1 { 2.0 / (kf * pi) } else { -2.0 / (kf * pi) },
                Builtin::Triangle => 8.0 * (kf * pi / 2.0).sin() / (kf * pi).powi(2),
            };
        }
        a[0] = 0.0;
        Self::from_coeffs(&a, &b, rate, false)
    }

    /// createPeriodicWave(real, imag, {disableNormalization})
    pub fn from_coeffs(real: &[f64], imag: &[f64], rate: f64, no_norm: bool) -> Self {
        let size = wave_size(rate);
        let half = size / 2;
        let ranges = (3.0 * (size as f64).log2()).round() as usize;
        let comps = real.len().min(imag.len()).min(half);
        let mut tables = Vec::with_capacity(ranges);
        let mut scale = 1.0;
        for r in 0..ranges {
            let partials = (2f64.powf(-(r as f64 * 400.0) / 1200.0) * half as f64) as usize;
            let (mut re, mut im) = (vec![0.0; size], vec![0.0; size]);
            for k in 1..comps.min(partials + 1) {
                // x[n] = Σ a_k cos(2πkn/N) + b_k sin(2πkn/N)：X[k] = (a - ib)/2，X[N-k] 取共轭
                re[k] = real[k] / 2.0;
                im[k] = -imag[k] / 2.0;
                re[size - k] = real[k] / 2.0;
                im[size - k] = imag[k] / 2.0;
            }
            fft(&mut re, &mut im, true);
            if r == 0 && !no_norm {
                let peak = re.iter().fold(0f64, |m, v| m.max(v.abs()));
                if peak > 0.0 {
                    scale = 1.0 / peak;
                }
            }
            tables.push(re.iter().map(|v| (v * scale) as f32).collect());
        }
        Self { size, tables, rate_scale: size as f64 / rate, lowest: (rate / 2.0) / half as f64 }
    }

    /// 按频率选两张相邻音区的表与它们之间的插值系数：(谐波多的, 谐波少的, 系数)
    fn pick(&self, f: f64) -> (usize, usize, f32) {
        let n = self.tables.len();
        let ratio = if f > 0.0 { f / self.lowest } else { 0.5 };
        let range = (1.0 + ratio.log2() * 1200.0 / 400.0).clamp(0.0, (n - 1) as f64);
        let r1 = range as usize;
        let r2 = if r1 < n - 1 { r1 + 1 } else { r1 };
        (r1, r2, (range - r1 as f64) as f32)
    }
}

pub struct Oscillator {
    sch: Schedule,
    wave: Arc<WaveTables>,
    phase: f64,
}

impl Oscillator {
    pub fn new(wave: Arc<WaveTables>) -> Self {
        Self { sch: Schedule::default(), wave, phase: 0.0 }
    }
}

impl Node for Oscillator {
    fn process(&mut self, q: &Quantum, _ins: &[Bus], params: &[Param], outs: &mut [Bus]) -> Flow {
        let out = &mut outs[0];
        out.reset(1);
        let Some(s) = self.sch.span(q) else { return Flow::Live };
        let w = &*self.wave;
        let mask = w.size - 1;
        let nyq = q.rate / 2.0;
        let per_sample = params[0].varies || params[1].varies;
        let freq = |k: usize| (params[0].vals[k] as f64 * 2f64.powf(params[1].vals[k] as f64 / 1200.0)).clamp(-nyq, nyq);
        let mut f0 = freq(0);
        let mut pick = w.pick(f0.abs());
        let o = out.chan_mut(0);
        for k in s.a..s.b {
            if per_sample {
                f0 = freq(k);
                pick = w.pick(f0.abs());
            }
            let (hi, lo, tif) = (&w.tables[pick.0], &w.tables[pick.1], pick.2);
            let i = self.phase.floor();
            let fr = (self.phase - i) as f32;
            let (i1, i2) = (i as i64 as usize & mask, (i as i64 as usize).wrapping_add(1) & mask);
            let sh = (1.0 - fr) * hi[i1] + fr * hi[i2];
            let sl = (1.0 - fr) * lo[i1] + fr * lo[i2];
            o[k] = (1.0 - tif) * sh + tif * sl;
            self.phase += f0 * w.rate_scale;
            let sz = w.size as f64;
            self.phase -= (self.phase / sz).floor() * sz;
        }
        let _ = QUANTUM;
        if s.end { Flow::Ended } else { Flow::Live }
    }

    fn msg(&mut self, m: Msg) -> Option<Retired> {
        if self.sch.msg(&m) {
            return None;
        }
        match m {
            Msg::Wave(w) => Some(Retired::Wave(std::mem::replace(&mut self.wave, w))),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 正弦表就是正弦_方波限带且峰值归一() {
        let s = WaveTables::builtin(Builtin::Sine, 48_000.0);
        assert_eq!((s.size, s.tables.len()), (4096, 36));
        for n in [0usize, 512, 1024, 3000] {
            let want = (2.0 * std::f64::consts::PI * n as f64 / 4096.0).sin();
            assert!((s.tables[20][n] as f64 - want).abs() < 1e-6);
        }
        let sq = WaveTables::builtin(Builtin::Square, 48_000.0);
        let peak = sq.tables[0].iter().fold(0f32, |m, v| m.max(v.abs()));
        assert!((peak - 1.0).abs() < 1e-6);
        // 440Hz 选的两个音区都不含超过奈奎斯特的谐波
        let (r1, r2, _) = sq.pick(440.0);
        let partials = |r: usize| (2f64.powf(-(r as f64) / 3.0) * 2048.0) as usize;
        assert!(partials(r1) as f64 * 440.0 <= 24_000.0 && partials(r2) <= partials(r1));
    }
}
