//! 混音图的单测：直接给 Graph 发命令、渲染，逐样本断言（不经 JS）。
use super::bus::{Interp, Mode};
use super::param::Ev;
use super::*;
use crate::game::audio::ring::{ring, Consumer};

const R: u32 = 48_000;

struct T {
    g: Graph,
    ret: Producer<Retired>,
    _ret_c: Consumer<Retired>,
    ended: Consumer<Ended>,
}

fn spec(n: usize, mode: Mode) -> InSpec {
    InSpec { count: n, mode, interp: Interp::Speakers }
}

fn setup() -> T {
    let clocks: Arc<Clocks> = Arc::new(std::array::from_fn(|_| AtomicU64::new(0)));
    let (ep, ec) = ring(64);
    let (rp, rc) = ring(1024);
    T { g: Graph::new(R, 2, clocks, ep), ret: rp, _ret_c: rc, ended: ec }
}

impl T {
    fn add(&mut self, slot: u32, node: Box<dyn Node>, ins: usize, params: Vec<Param>) {
        let s = Slot::new(node, vec![spec(2, Mode::Max); ins], vec![MAX_CH; ins], vec![MAX_CH], params);
        self.g.apply(GCmd::Add { slot, s }, &mut self.ret);
    }
    fn dest(&mut self, slot: u32) {
        let s = Slot::new(Box::new(effects::Silent), vec![spec(2, Mode::Explicit)], vec![2], vec![1], vec![]);
        self.g.apply(GCmd::Add { slot, s }, &mut self.ret);
    }
    /// 按给的顺序与连线建计划
    fn plan(&mut self, order: &[u32], edges: &[Edge], dest: u32) {
        let mut p = Plan { dest, ..Default::default() };
        for &n in order {
            p.starts.push(p.edges.len() as u32);
            p.order.push(n);
            p.edges.extend(edges.iter().filter(|e| e.dst == n));
        }
        p.starts.push(p.edges.len() as u32);
        self.g.apply(GCmd::Plan { ctx: 0, plan: Box::new(p) }, &mut self.ret);
        self.g.apply(GCmd::Running { ctx: 0, on: true }, &mut self.ret);
    }
    fn msg(&mut self, slot: u32, m: Msg) {
        self.g.apply(GCmd::Msg { slot, m }, &mut self.ret);
    }
    fn render(&mut self, frames: usize) -> Vec<f32> {
        let mut out = vec![0f32; frames * 2];
        for c in out.chunks_mut(QUANTUM * 2) {
            self.g.render(c);
        }
        out
    }
}

fn e(src: u32, dst: u32) -> Edge {
    Edge { dst, port: 0, param: false, src, out: 0 }
}

fn buf(samples: Vec<f32>) -> Arc<AudioBuf> {
    Arc::new(AudioBuf { rate: R as f64, len: samples.len(), chans: vec![samples.into_boxed_slice()].into() })
}

#[test]
fn 缓冲源_起止按帧_播完报一次() {
    let mut t = setup();
    t.add(1, Box::new(source::BufferSource::new()), 0, vec![Param::new(1.0, -3e38, 3e38, true), Param::new(0.0, -3e38, 3e38, true)]);
    t.dest(0);
    t.plan(&[1, 0], &[e(1, 0)], 0);
    t.msg(1, Msg::Buffer(Some(buf((0..1000).map(|i| i as f32 / 1000.0).collect()))));
    // 第 200 帧开始放
    t.msg(1, Msg::Start { when: 200.0 / R as f64, offset: 0.0, duration: None });
    let out = t.render(1408);
    assert_eq!(out[2 * 199], 0.0);
    assert_eq!(out[2 * 200], 0.0, "第一个样本是 0");
    assert_eq!((out[2 * 201], out[2 * 201 + 1]), (0.001, 0.001), "单声道上混到两个声道");
    assert_eq!(out[2 * 1199], 0.999);
    assert_eq!(out[2 * 1200], 0.0);
    assert_eq!(t.ended.pop(), Some(Ended { slot: 1 }));
    assert!(t.ended.pop().is_none());
}

#[test]
fn 增益自动化_直线渐变逐样本() {
    let mut t = setup();
    t.add(1, Box::new(source::Constant::default()), 0, vec![Param::new(1.0, -3e38, 3e38, false)]);
    t.add(2, Box::new(effects::Gain), 1, vec![Param::new(1.0, -3e38, 3e38, false)]);
    t.dest(0);
    t.plan(&[1, 2, 0], &[e(1, 2), e(2, 0)], 0);
    let mut tl = param::Timeline::default();
    tl.insert(Ev::Set { v: 0.0, t: 0.0 });
    tl.insert(Ev::Linear { v: 1.0, t: 256.0 / R as f64 });
    t.g.apply(GCmd::Timeline { slot: 2, param: 0, tl: Box::new(tl), base: 1.0 }, &mut t.ret);
    t.msg(1, Msg::Start { when: 0.0, offset: 0.0, duration: None });
    let out = t.render(512);
    for k in [0usize, 64, 128, 200, 255] {
        assert!((out[2 * k] - k as f32 / 256.0).abs() < 1e-6, "k={k}: {}", out[2 * k]);
    }
    assert_eq!(out[2 * 300], 1.0);
}

#[test]
fn 振荡器_正弦与理想值误差很小_延迟整帧平移() {
    let mut t = setup();
    let w = Arc::new(osc::WaveTables::builtin(osc::Builtin::Sine, R as f64));
    t.add(1, Box::new(osc::Oscillator::new(w)), 0, vec![Param::new(1000.0, -24e3, 24e3, false), Param::new(0.0, -1e5, 1e5, false)]);
    t.add(2, Box::new(effects::Delay::new(1.0, R as f64)), 1, vec![Param::new(100.0 / R as f64, 0.0, 1.0, false)]);
    t.dest(0);
    t.plan(&[1, 2, 0], &[e(1, 2), e(2, 0)], 0);
    t.msg(1, Msg::Start { when: 0.0, offset: 0.0, duration: None });
    let out = t.render(1024);
    for k in 0..100 {
        assert_eq!(out[2 * k], 0.0);
    }
    let mut worst = 0f64;
    for k in 100..1024 {
        let want = (2.0 * std::f64::consts::PI * 1000.0 * (k - 100) as f64 / R as f64).sin();
        worst = worst.max((out[2 * k] as f64 - want).abs());
    }
    assert!(worst < 1e-5, "最大误差 {worst}");
}

#[test]
fn 暂停的上下文不推进时钟() {
    let mut t = setup();
    t.dest(0);
    t.plan(&[0], &[], 0);
    t.render(256);
    assert_eq!(t.g.clocks[0].load(Ordering::Relaxed), 256);
    t.g.apply(GCmd::Running { ctx: 0, on: false }, &mut t.ret);
    t.render(256);
    assert_eq!(t.g.clocks[0].load(Ordering::Relaxed), 256);
}
