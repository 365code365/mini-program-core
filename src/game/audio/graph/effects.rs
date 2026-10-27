//! 一进一出的简单节点：GainNode、DelayNode、WaveShaperNode，以及降级用的直通 / 静音节点与 destination。
//!
//! - Delay：每声道一个环形缓冲（容量 = maxDelayTime × 采样率 + 一个量子，引擎线程建好），先写后读，
//!   读位置 = 写位置 − delayTime × 采样率，两点线性插值（Chrome DelayDSPKernel 同一个做法；delayTime = 0 时输出就是输入）。
//!   缓冲只建 2 个声道：多声道输入的第 3 声道起直接丢（记在验收文档）；输入从立体声变成单声道时（源播完），
//!   尾巴下混成单声道放（与 Chrome 一致，见 Delay::process）；
//! - WaveShaper：曲线按规范插值，`oversample` 的 2x / 4x 没做，按 none 处理；没设曲线时直通；
//! - DynamicsCompressor 用直通（[`Pass`]）降级，ScriptProcessor 用静音（[`Silent`]）降级（webaudio.js 文件头写明）。

use super::bus::{Bus, QUANTUM};
use super::{Flow, Msg, Node, Param, Quantum, Retired};

/// 参数：0 gain（a-rate）
pub struct Gain;

impl Node for Gain {
    fn process(&mut self, _q: &Quantum, ins: &[Bus], params: &[Param], outs: &mut [Bus]) -> Flow {
        let (inp, out) = (&ins[0], &mut outs[0]);
        out.reset(inp.ch);
        let g = &params[0].vals;
        let flat = !params[0].varies;
        for c in 0..inp.ch {
            let (s, d) = (inp.chan(c), out.chan_mut(c));
            if flat {
                let g0 = g[0];
                d.iter_mut().zip(s.iter()).for_each(|(o, x)| *o = x * g0);
            } else {
                for k in 0..QUANTUM {
                    d[k] = s[k] * g[k];
                }
            }
        }
        Flow::Live
    }
}

/// 参数：0 delayTime（a-rate，钳到 [0, maxDelayTime]）
pub struct Delay {
    buf: [Box<[f32]>; 2],
    w: usize,
    max: f64,
}

impl Delay {
    pub fn new(max_delay: f64, rate: f64) -> Self {
        let len = (max_delay * rate).ceil() as usize + QUANTUM + 2;
        Self { buf: [vec![0.0; len].into(), vec![0.0; len].into()], w: 0, max: max_delay }
    }
}

impl Node for Delay {
    fn process(&mut self, q: &Quantum, ins: &[Bus], params: &[Param], outs: &mut [Bus]) -> Flow {
        let (inp, out) = (&ins[0], &mut outs[0]);
        // 延迟线总是存两个声道（单声道输入两边写同一个值）；输出声道数跟着**当前**输入走（规范：输出声道数 = 输入声道数）。
        // 源播完、输入变成 1 声道静音之后，立体声的尾巴按 0.5(L+R) 下混成单声道放完 —— Chrome 实测同样如此
        let mono = inp.ch <= 1;
        out.reset(if mono { 1 } else { 2 });
        let len = self.buf[0].len();
        let mut w = self.w;
        let rate = q.rate as f32;
        for k in 0..QUANTUM {
            let (l, r) = if mono { (inp.chan(0)[k], inp.chan(0)[k]) } else { (inp.chan(0)[k], inp.chan(1)[k]) };
            self.buf[0][w] = l;
            self.buf[1][w] = r;
            // 帧数按 f32 算（Chrome 同样）：0.01 × 48000 在 f32 里正好是 480，按 f64 算是 479.99999，会漏进相邻样本一点点
            let d = (params[0].vals[k].clamp(0.0, self.max as f32) * rate) as f64;
            let mut rp = w as f64 - d;
            if rp < 0.0 {
                rp += len as f64;
            }
            let i1 = rp as usize % len;
            let i2 = (i1 + 1) % len;
            let t = (rp - rp.floor()) as f32;
            let read = |b: &[f32]| (1.0 - t) * b[i1] + t * b[i2];
            let (ol, or) = (read(&self.buf[0]), read(&self.buf[1]));
            if mono {
                out.chan_mut(0)[k] = 0.5 * (ol + or);
            } else {
                out.chan_mut(0)[k] = ol;
                out.chan_mut(1)[k] = or;
            }
            w = (w + 1) % len;
        }
        self.w = w;
        Flow::Live
    }
}

#[derive(Default)]
pub struct Shaper {
    curve: Option<Box<[f32]>>,
}

impl Node for Shaper {
    fn process(&mut self, _q: &Quantum, ins: &[Bus], _params: &[Param], outs: &mut [Bus]) -> Flow {
        let (inp, out) = (&ins[0], &mut outs[0]);
        out.reset(inp.ch);
        for c in 0..inp.ch {
            let (s, d) = (inp.chan(c), out.chan_mut(c));
            match self.curve.as_deref() {
                Some(cv) if !cv.is_empty() => {
                    let n = cv.len();
                    for k in 0..QUANTUM {
                        let v = (n - 1) as f32 / 2.0 * (s[k] + 1.0);
                        d[k] = if v <= 0.0 {
                            cv[0]
                        } else if v >= (n - 1) as f32 {
                            cv[n - 1]
                        } else {
                            let i = v as usize;
                            let f = v - i as f32;
                            (1.0 - f) * cv[i] + f * cv[i + 1]
                        };
                    }
                }
                _ => d.copy_from_slice(s),
            }
        }
        Flow::Live
    }

    fn msg(&mut self, m: Msg) -> Option<Retired> {
        match m {
            Msg::Curve(c) => std::mem::replace(&mut self.curve, c).map(Retired::Curve),
            _ => None,
        }
    }
}

/// 直通（DynamicsCompressor 的降级）
pub struct Pass;

impl Node for Pass {
    fn process(&mut self, _q: &Quantum, ins: &[Bus], _params: &[Param], outs: &mut [Bus]) -> Flow {
        let (inp, out) = (&ins[0], &mut outs[0]);
        out.reset(inp.ch);
        for c in 0..inp.ch {
            out.chan_mut(c).copy_from_slice(inp.chan(c));
        }
        Flow::Live
    }
}

/// 静音输出（ScriptProcessor 的降级）；destination 也用它：图直接读 destination 的输入
pub struct Silent;

impl Node for Silent {
    fn process(&mut self, _q: &Quantum, _ins: &[Bus], _params: &[Param], outs: &mut [Bus]) -> Flow {
        for o in outs.iter_mut() {
            o.reset(1);
        }
        Flow::Live
    }
}
