//! BiquadFilterNode（规范 §1.13 的系数公式，即 Audio EQ Cookbook 的 Web Audio 版）与 IIRFilterNode。
//!
//! - 计算频率 = frequency × 2^(detune/1200)，钳到 [0, 奈奎斯特]，归一化 w0 = π·f/奈奎斯特；
//! - lowpass / highpass 的 Q 按 dB 解释，bandpass / notch / allpass / peaking 按线性，shelf 不看 Q；
//! - 参数在本量子里没变就整块用一组系数，变了就逐样本算（Chrome 同一个做法）；
//! - 状态与系数用 f64（Chrome 的 Biquad 也是）；
//! - 边界（Chrome 的特判）：截止频率归一化后 = 1 → lowpass 直通 / highpass 全零；= 0 → lowpass 全零 / highpass 直通；
//!   其余类型 f 在 0 或 1 处、或 Q ≤ 0 时按直通（规范没写死，记下）。
//! 参数：0 frequency、1 detune、2 Q、3 gain。

use super::bus::{Bus, MAX_CH, QUANTUM};
use super::{Flow, Msg, Node, Param, Quantum, Retired};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Lowpass,
    Highpass,
    Bandpass,
    Lowshelf,
    Highshelf,
    Peaking,
    Notch,
    Allpass,
}

impl Kind {
    pub const ALL: [&'static str; 8] = ["lowpass", "highpass", "bandpass", "lowshelf", "highshelf", "peaking", "notch", "allpass"];
    pub fn from_u8(i: u8) -> Self {
        [Self::Lowpass, Self::Highpass, Self::Bandpass, Self::Lowshelf, Self::Highshelf, Self::Peaking, Self::Notch, Self::Allpass][(i as usize).min(7)]
    }
}

/// 归一化后的系数 (b0, b1, b2, a1, a2)（a0 已除掉）
pub type Coef = [f64; 5];

const PASS: Coef = [1.0, 0.0, 0.0, 0.0, 0.0];
const ZERO: Coef = [0.0, 0.0, 0.0, 0.0, 0.0];

pub fn coefficients(kind: Kind, freq: f64, detune: f64, q: f64, gain: f64, rate: f64) -> Coef {
    let nyq = rate / 2.0;
    let f = (freq * 2f64.powf(detune / 1200.0)).clamp(0.0, nyq) / nyq;
    let norm = |b0: f64, b1: f64, b2: f64, a0: f64, a1: f64, a2: f64| [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0];
    let w0 = std::f64::consts::PI * f;
    let (s, k) = (w0.sin(), w0.cos());
    let a = 10f64.powf(gain / 40.0);
    match kind {
        Kind::Lowpass | Kind::Highpass => {
            let lp = kind == Kind::Lowpass;
            if f >= 1.0 {
                return if lp { PASS } else { ZERO };
            }
            if f <= 0.0 {
                return if lp { ZERO } else { PASS };
            }
            let alpha = s / (2.0 * 10f64.powf(q / 20.0));
            let beta = if lp { (1.0 - k) / 2.0 } else { (1.0 + k) / 2.0 };
            let b1 = if lp { 2.0 * beta } else { -2.0 * beta };
            norm(beta, b1, beta, 1.0 + alpha, -2.0 * k, 1.0 - alpha)
        }
        Kind::Lowshelf | Kind::Highshelf => {
            if f <= 0.0 || f >= 1.0 {
                return PASS;
            }
            let k2 = 2.0 * a.sqrt() * (s / 2.0 * 2f64.sqrt());
            if kind == Kind::Lowshelf {
                norm(a * ((a + 1.0) - (a - 1.0) * k + k2), 2.0 * a * ((a - 1.0) - (a + 1.0) * k), a * ((a + 1.0) - (a - 1.0) * k - k2),
                    (a + 1.0) + (a - 1.0) * k + k2, -2.0 * ((a - 1.0) + (a + 1.0) * k), (a + 1.0) + (a - 1.0) * k - k2)
            } else {
                norm(a * ((a + 1.0) + (a - 1.0) * k + k2), -2.0 * a * ((a - 1.0) + (a + 1.0) * k), a * ((a + 1.0) + (a - 1.0) * k - k2),
                    (a + 1.0) - (a - 1.0) * k + k2, 2.0 * ((a - 1.0) - (a + 1.0) * k), (a + 1.0) - (a - 1.0) * k - k2)
            }
        }
        _ => {
            if f <= 0.0 || f >= 1.0 || q <= 0.0 {
                return PASS;
            }
            let alpha = s / (2.0 * q);
            match kind {
                Kind::Bandpass => norm(alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * k, 1.0 - alpha),
                Kind::Peaking => norm(1.0 + alpha * a, -2.0 * k, 1.0 - alpha * a, 1.0 + alpha / a, -2.0 * k, 1.0 - alpha / a),
                Kind::Notch => norm(1.0, -2.0 * k, 1.0, 1.0 + alpha, -2.0 * k, 1.0 - alpha),
                _ => norm(1.0 - alpha, -2.0 * k, 1.0 + alpha, 1.0 + alpha, -2.0 * k, 1.0 - alpha),
            }
        }
    }
}

/// H(e^{jω}) 的 (幅度, 相位)，ω = π·f/奈奎斯特。`b` / `a` 是分子 / 分母多项式（a[0] = 1）
pub fn response(b: &[f64], a: &[f64], f: f64, rate: f64) -> (f64, f64) {
    let w = std::f64::consts::PI * f / (rate / 2.0);
    let eval = |c: &[f64]| c.iter().enumerate().fold((0.0, 0.0), |(r, i), (n, v)| (r + v * (n as f64 * w).cos(), i - v * (n as f64 * w).sin()));
    let (nr, ni) = eval(b);
    let (dr, di) = eval(a);
    let d = dr * dr + di * di;
    let (hr, hi) = ((nr * dr + ni * di) / d, (ni * dr - nr * di) / d);
    ((hr * hr + hi * hi).sqrt(), hi.atan2(hr))
}

pub struct Biquad {
    kind: Kind,
    /// 每声道 (x1, x2, y1, y2)
    st: [[f64; 4]; MAX_CH],
}

impl Biquad {
    pub fn new() -> Self {
        Self { kind: Kind::Lowpass, st: [[0.0; 4]; MAX_CH] }
    }
}

impl Node for Biquad {
    fn process(&mut self, q: &Quantum, ins: &[Bus], params: &[Param], outs: &mut [Bus]) -> Flow {
        let (inp, out) = (&ins[0], &mut outs[0]);
        out.reset(inp.ch);
        let per_sample = params.iter().any(|p| p.varies);
        let co = |k: usize| coefficients(self.kind, params[0].vals[k] as f64, params[1].vals[k] as f64, params[2].vals[k] as f64, params[3].vals[k] as f64, q.rate);
        let fixed = co(0);
        for c in 0..inp.ch {
            let [mut x1, mut x2, mut y1, mut y2] = self.st[c];
            let (src, dst) = (inp.chan(c), out.chan_mut(c));
            for k in 0..QUANTUM {
                let [b0, b1, b2, a1, a2] = if per_sample { co(k) } else { fixed };
                let x = src[k] as f64;
                let mut y = b0 * x + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
                if y.abs() < 1e-30 {
                    y = 0.0; // 冲掉非正规数（尾巴衰减到底时 CPU 会慢几十倍）
                }
                dst[k] = y as f32;
                (x2, x1, y2, y1) = (x1, x, y1, y);
            }
            self.st[c] = [x1, x2, y1, y2];
        }
        Flow::Live
    }

    fn msg(&mut self, m: Msg) -> Option<Retired> {
        if let Msg::Kind(k) = m {
            self.kind = Kind::from_u8(k);
        }
        None
    }
}

/// IIRFilterNode：系数在创建时定死（规范：feedforward / feedback 各 ≤ 20 个，feedback[0] ≠ 0）
pub struct Iir {
    b: Box<[f64]>,
    a: Box<[f64]>,
    /// 每声道的 x / y 历史（环形，长度 = 阶数）
    hx: Box<[[f64; 20]]>,
    hy: Box<[[f64; 20]]>,
    pos: usize,
}

impl Iir {
    pub fn new(ff: &[f64], fb: &[f64]) -> Self {
        let a0 = fb.first().copied().filter(|v| *v != 0.0).unwrap_or(1.0);
        let b: Box<[f64]> = ff.iter().take(20).map(|v| v / a0).collect();
        let a: Box<[f64]> = fb.iter().take(20).map(|v| v / a0).collect();
        Self { b, a, hx: vec![[0.0; 20]; MAX_CH].into(), hy: vec![[0.0; 20]; MAX_CH].into(), pos: 0 }
    }
}

impl Node for Iir {
    fn process(&mut self, _q: &Quantum, ins: &[Bus], _params: &[Param], outs: &mut [Bus]) -> Flow {
        let (inp, out) = (&ins[0], &mut outs[0]);
        out.reset(inp.ch);
        let start = self.pos;
        for c in 0..inp.ch {
            let mut pos = start;
            let (hx, hy) = (&mut self.hx[c], &mut self.hy[c]);
            for k in 0..QUANTUM {
                let x = inp.chan(c)[k] as f64;
                let mut y = self.b[0] * x;
                for j in 1..self.b.len().max(self.a.len()) {
                    let idx = (pos + 20 - j) % 20;
                    y += self.b.get(j).copied().unwrap_or(0.0) * hx[idx] - self.a.get(j).copied().unwrap_or(0.0) * hy[idx];
                }
                if y.abs() < 1e-30 {
                    y = 0.0;
                }
                hx[pos] = x;
                hy[pos] = y;
                pos = (pos + 1) % 20;
                out.chan_mut(c)[k] = y as f32;
            }
        }
        self.pos = (start + QUANTUM) % 20;
        Flow::Live
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 低通_直流增益为一_截止处按_q() {
        let c = coefficients(Kind::Lowpass, 1000.0, 0.0, 0.0, 0.0, 48_000.0);
        let (b, a) = ([c[0], c[1], c[2]], [1.0, c[3], c[4]]);
        assert!((response(&b, &a, 0.0, 48_000.0).0 - 1.0).abs() < 1e-9);
        // Q = 0dB：截止频率处 |H| = 1（Web Audio 的 lowpass 定义）
        assert!((response(&b, &a, 1000.0, 48_000.0).0 - 1.0).abs() < 1e-6);
        assert!(response(&b, &a, 10_000.0, 48_000.0).0 < 0.02);
    }

    #[test]
    fn 峰化_中心频率增益等于_gain_db() {
        let c = coefficients(Kind::Peaking, 2000.0, 0.0, 1.0, 6.0, 48_000.0);
        let m = response(&[c[0], c[1], c[2]], &[1.0, c[3], c[4]], 2000.0, 48_000.0).0;
        assert!((20.0 * m.log10() - 6.0).abs() < 1e-6);
    }
}
