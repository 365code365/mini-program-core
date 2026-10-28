//! 总线（一个节点的一路输出 / 输入）：每声道 128 帧，容量在引擎线程建节点时定好，音频线程不再分配。
//! 混音规则按 Web Audio 规范 §4「Channel Up-Mixing and Down-Mixing」：`speakers` 解释下 1 / 2 / 4 / 6 声道之间
//! 按规范的表混，其余组合与 `discrete` 一样（多的丢、少的补零）。

pub const QUANTUM: usize = 128;
/// 普通节点总线的声道上限。规范允许 32；8 已经盖住 7.1，合并 / 拆分节点按自己的路数单独建
pub const MAX_CH: usize = 8;

pub type Chan = [f32; QUANTUM];

#[derive(Clone)]
pub struct Bus {
    data: Box<[Chan]>,
    /// 当前有效声道数（0 = 静音）
    pub ch: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Max,
    ClampedMax,
    Explicit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interp {
    Speakers,
    Discrete,
}

impl Bus {
    pub fn new(cap: usize) -> Self {
        Self { data: vec![[0f32; QUANTUM]; cap.max(1)].into_boxed_slice(), ch: 0 }
    }

    pub fn cap(&self) -> usize {
        self.data.len()
    }

    /// 设声道数并清零
    pub fn reset(&mut self, ch: usize) {
        self.ch = ch.min(self.cap());
        for c in &mut self.data[..self.ch] {
            c.fill(0.0);
        }
    }

    pub fn chan(&self, c: usize) -> &Chan {
        &self.data[c]
    }

    pub fn chan_mut(&mut self, c: usize) -> &mut Chan {
        &mut self.data[c]
    }

    /// 把 `src` 按规则混进自己（自己的声道数已定）
    pub fn sum_from(&mut self, src: &Bus, interp: Interp) {
        let (n, m) = (self.ch, src.ch);
        if m == 0 || n == 0 {
            return;
        }
        if n == m || interp == Interp::Discrete {
            for c in 0..n.min(m) {
                add(&mut self.data[c], src.chan(c), 1.0);
            }
            return;
        }
        let s = |c: usize| src.chan(c);
        const H: f32 = std::f32::consts::FRAC_1_SQRT_2;
        match (m, n) {
            // 上混
            (1, 2) | (1, 4) => {
                add(&mut self.data[0], s(0), 1.0);
                add(&mut self.data[1], s(0), 1.0);
            }
            (1, 6) => add(&mut self.data[2], s(0), 1.0),
            (2, 4) | (2, 6) => {
                add(&mut self.data[0], s(0), 1.0);
                add(&mut self.data[1], s(1), 1.0);
            }
            (4, 6) => {
                add(&mut self.data[0], s(0), 1.0);
                add(&mut self.data[1], s(1), 1.0);
                add(&mut self.data[4], s(2), 1.0);
                add(&mut self.data[5], s(3), 1.0);
            }
            // 下混
            (2, 1) => {
                add(&mut self.data[0], s(0), 0.5);
                add(&mut self.data[0], s(1), 0.5);
            }
            (4, 1) => {
                for c in 0..4 {
                    add(&mut self.data[0], s(c), 0.25);
                }
            }
            (6, 1) => {
                add(&mut self.data[0], s(0), H);
                add(&mut self.data[0], s(1), H);
                add(&mut self.data[0], s(2), 1.0);
                add(&mut self.data[0], s(4), 0.5);
                add(&mut self.data[0], s(5), 0.5);
            }
            (4, 2) => {
                add(&mut self.data[0], s(0), 0.5);
                add(&mut self.data[0], s(2), 0.5);
                add(&mut self.data[1], s(1), 0.5);
                add(&mut self.data[1], s(3), 0.5);
            }
            (6, 2) => {
                add(&mut self.data[0], s(0), 1.0);
                add(&mut self.data[0], s(2), H);
                add(&mut self.data[0], s(4), H);
                add(&mut self.data[1], s(1), 1.0);
                add(&mut self.data[1], s(2), H);
                add(&mut self.data[1], s(5), H);
            }
            (6, 4) => {
                add(&mut self.data[0], s(0), 1.0);
                add(&mut self.data[0], s(2), H);
                add(&mut self.data[1], s(1), 1.0);
                add(&mut self.data[1], s(2), H);
                add(&mut self.data[2], s(4), 1.0);
                add(&mut self.data[3], s(5), 1.0);
            }
            _ => {
                for c in 0..n.min(m) {
                    add(&mut self.data[c], s(c), 1.0);
                }
            }
        }
    }
}

fn add(dst: &mut Chan, src: &Chan, g: f32) {
    for (d, s) in dst.iter_mut().zip(src.iter()) {
        *d += s * g;
    }
}

/// 规范的「computedNumberOfChannels」
pub fn computed_channels(mode: Mode, count: usize, max_in: usize) -> usize {
    match mode {
        Mode::Max => max_in.max(1),
        Mode::ClampedMax => max_in.clamp(1, count.max(1)),
        Mode::Explicit => count.max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bus(vals: &[f32]) -> Bus {
        let mut b = Bus::new(vals.len());
        b.reset(vals.len());
        for (c, &v) in vals.iter().enumerate() {
            b.chan_mut(c).fill(v);
        }
        b
    }

    #[test]
    fn 单声道上混与立体声下混() {
        let mut st = Bus::new(2);
        st.reset(2);
        st.sum_from(&bus(&[0.5]), Interp::Speakers);
        assert_eq!((st.chan(0)[0], st.chan(1)[5]), (0.5, 0.5));
        let mut mono = Bus::new(1);
        mono.reset(1);
        mono.sum_from(&bus(&[1.0, 0.2]), Interp::Speakers);
        assert!((mono.chan(0)[0] - 0.6).abs() < 1e-6);
        let mut d = Bus::new(2);
        d.reset(2);
        d.sum_from(&bus(&[0.5]), Interp::Discrete);
        assert_eq!((d.chan(0)[0], d.chan(1)[0]), (0.5, 0.0), "discrete 不铺到右声道");
    }

    #[test]
    fn 五点一下混立体声() {
        let mut st = Bus::new(2);
        st.reset(2);
        st.sum_from(&bus(&[1.0, 0.0, 1.0, 9.0, 0.0, 1.0]), Interp::Speakers);
        let h = std::f32::consts::FRAC_1_SQRT_2;
        assert!((st.chan(0)[0] - (1.0 + h)).abs() < 1e-6);
        assert!((st.chan(1)[0] - 2.0 * h).abs() < 1e-6, "LFE 丢掉");
    }

    #[test]
    fn 声道数计算() {
        assert_eq!(computed_channels(Mode::Max, 2, 6), 6);
        assert_eq!(computed_channels(Mode::ClampedMax, 2, 6), 2);
        assert_eq!(computed_channels(Mode::ClampedMax, 2, 0), 1);
        assert_eq!(computed_channels(Mode::Explicit, 1, 6), 1);
    }
}
