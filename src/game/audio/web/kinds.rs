//! 节点种类表：每种节点几路输入 / 输出、有哪些参数（默认值、名义范围、a / k-rate）、默认混音规则，
//! 以及在引擎线程上建好它的音频线程实例（缓冲在这里分配）。参数的默认值与范围按规范各节点的 IDL。

use super::super::graph::bus::{Interp, Mode, MAX_CH};
use super::super::graph::{analyser, channels, effects, filter, osc, panner, source, InSpec, Node, Param, Slot};
use std::sync::Arc;

const BIG: f32 = 3.402_823_5e38;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Destination,
    Buffer,
    Oscillator,
    Constant,
    Gain,
    Delay,
    Biquad,
    Iir,
    Shaper,
    Compressor,
    Panner,
    Merger,
    Splitter,
    Analyser,
    Script,
}

impl Kind {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "buffer" => Self::Buffer,
            "oscillator" => Self::Oscillator,
            "constant" => Self::Constant,
            "gain" => Self::Gain,
            "delay" => Self::Delay,
            "biquad" => Self::Biquad,
            "iir" => Self::Iir,
            "waveshaper" => Self::Shaper,
            "compressor" => Self::Compressor,
            "panner" => Self::Panner,
            "merger" => Self::Merger,
            "splitter" => Self::Splitter,
            "analyser" => Self::Analyser,
            "script" => Self::Script,
            _ => return None,
        })
    }

    pub fn is_source(self) -> bool {
        matches!(self, Self::Buffer | Self::Oscillator | Self::Constant)
    }
}

/// 参数的说明
#[derive(Clone, Copy)]
pub struct PSpec {
    pub name: &'static str,
    pub base: f64,
    pub min: f32,
    pub max: f32,
    pub krate: bool,
}

const fn p(name: &'static str, base: f64, min: f32, max: f32, krate: bool) -> PSpec {
    PSpec { name, base, min, max, krate }
}

/// listener 的九个参数（PannerNode 的第 6~14 个参数）
pub const LISTENER: [PSpec; 9] = [
    p("positionX", 0.0, -BIG, BIG, false),
    p("positionY", 0.0, -BIG, BIG, false),
    p("positionZ", 0.0, -BIG, BIG, false),
    p("forwardX", 0.0, -BIG, BIG, false),
    p("forwardY", 0.0, -BIG, BIG, false),
    p("forwardZ", -1.0, -BIG, BIG, false),
    p("upX", 0.0, -BIG, BIG, false),
    p("upY", 1.0, -BIG, BIG, false),
    p("upZ", 0.0, -BIG, BIG, false),
];

/// 建节点时 JS 给的参数（只有建的那一刻用得到的）
#[derive(Default)]
pub struct Opts {
    pub max_delay: f64,
    pub inputs: usize,
    pub outputs: usize,
    pub ff: Vec<f64>,
    pub fb: Vec<f64>,
}

pub struct Built {
    pub slot: Box<Slot>,
    pub params: Vec<PSpec>,
    pub n_in: usize,
    pub n_out: usize,
    /// 第 0 路输入的混音规则（JS 的 channelCount / Mode / Interpretation 读回用）
    pub spec: InSpec,
    pub analyser: Option<Arc<analyser::Shared>>,
}

pub fn build(kind: Kind, rate: f64, o: &Opts, sine: impl FnOnce() -> Arc<osc::WaveTables>) -> Built {
    let nyq = (rate / 2.0) as f32;
    let sp = |count: usize, mode: Mode, interp: Interp| InSpec { count, mode, interp };
    let std2 = sp(2, Mode::Max, Interp::Speakers);
    let mut analyser_shared = None;
    let (node, params, ins, outs): (Box<dyn Node>, Vec<PSpec>, Vec<(InSpec, usize)>, Vec<usize>) = match kind {
        Kind::Destination => (Box::new(effects::Silent), vec![], vec![(sp(2, Mode::Explicit, Interp::Speakers), MAX_CH)], vec![]),
        Kind::Buffer => (
            Box::new(source::BufferSource::new()),
            vec![p("playbackRate", 1.0, -BIG, BIG, true), p("detune", 0.0, -BIG, BIG, true)],
            vec![],
            vec![MAX_CH],
        ),
        Kind::Oscillator => (
            Box::new(osc::Oscillator::new(sine())),
            vec![p("frequency", 440.0, -nyq, nyq, false), p("detune", 0.0, -153_600.0, 153_600.0, false)],
            vec![],
            vec![1],
        ),
        Kind::Constant => (Box::new(source::Constant::default()), vec![p("offset", 1.0, -BIG, BIG, false)], vec![], vec![1]),
        Kind::Gain => (Box::new(effects::Gain), vec![p("gain", 1.0, -BIG, BIG, false)], vec![(std2, MAX_CH)], vec![MAX_CH]),
        Kind::Delay => (
            Box::new(effects::Delay::new(o.max_delay, rate)),
            vec![p("delayTime", 0.0, 0.0, o.max_delay as f32, false)],
            vec![(std2, MAX_CH)],
            vec![2],
        ),
        Kind::Biquad => (
            Box::new(filter::Biquad::new()),
            vec![p("frequency", 350.0, 0.0, nyq, false), p("detune", 0.0, -153_600.0, 153_600.0, false), p("Q", 1.0, -BIG, BIG, false), p("gain", 0.0, -BIG, 1541.0, false)],
            vec![(std2, MAX_CH)],
            vec![MAX_CH],
        ),
        Kind::Iir => (Box::new(filter::Iir::new(&o.ff, &o.fb)), vec![], vec![(std2, MAX_CH)], vec![MAX_CH]),
        Kind::Shaper => (Box::new(effects::Shaper::default()), vec![], vec![(std2, MAX_CH)], vec![MAX_CH]),
        Kind::Compressor => (
            Box::new(effects::Pass),
            vec![p("threshold", -24.0, -100.0, 0.0, true), p("knee", 30.0, 0.0, 40.0, true), p("ratio", 12.0, 1.0, 20.0, true),
                p("attack", 0.003, 0.0, 1.0, true), p("release", 0.25, 0.0, 1.0, true)],
            vec![(sp(2, Mode::ClampedMax, Interp::Speakers), MAX_CH)],
            vec![MAX_CH],
        ),
        Kind::Panner => {
            let mut ps = vec![p("positionX", 0.0, -BIG, BIG, false), p("positionY", 0.0, -BIG, BIG, false), p("positionZ", 0.0, -BIG, BIG, false),
                p("orientationX", 1.0, -BIG, BIG, false), p("orientationY", 0.0, -BIG, BIG, false), p("orientationZ", 0.0, -BIG, BIG, false)];
            ps.extend(LISTENER);
            (Box::new(panner::Panner { cfg: Default::default() }), ps, vec![(sp(2, Mode::ClampedMax, Interp::Speakers), 2)], vec![2])
        }
        Kind::Merger => {
            let n = o.inputs.clamp(1, 32);
            (Box::new(channels::Merger), vec![], vec![(sp(1, Mode::Explicit, Interp::Speakers), 1); n], vec![n])
        }
        Kind::Splitter => {
            let n = o.outputs.clamp(1, 32);
            (Box::new(channels::Splitter), vec![], vec![(sp(n, Mode::Explicit, Interp::Discrete), n)], vec![1; n])
        }
        Kind::Analyser => {
            let sh = analyser::Shared::new();
            analyser_shared = Some(sh.clone());
            (Box::new(analyser::Analyser::new(sh)), vec![], vec![(std2, MAX_CH)], vec![MAX_CH])
        }
        Kind::Script => (Box::new(effects::Silent), vec![], vec![(std2, MAX_CH)], vec![2]),
    };
    let (n_in, n_out) = (ins.len(), outs.len());
    let spec = ins.first().map(|x| x.0).unwrap_or(std2);
    let ps: Vec<Param> = params.iter().map(|s| Param::new(s.base, s.min, s.max, s.krate)).collect();
    let slot = Slot::new(node, ins.iter().map(|x| x.0).collect(), ins.iter().map(|x| x.1).collect(), outs, ps);
    Built { slot, params, n_in, n_out, spec, analyser: analyser_shared }
}
