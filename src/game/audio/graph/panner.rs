//! PannerNode（equalpower）。规范 §1.30 / §6：方位角与仰角、距离衰减、锥形衰减、等功率声像。
//!
//! 参数：0~2 positionX/Y/Z、3~5 orientationX/Y/Z，6~14 是所属上下文的 listener
//! （positionX/Y/Z、forwardX/Y/Z、upX/Y/Z）—— listener 是上下文级的，引擎线程改它时给每个 PannerNode 发同一张时间线。
//! 位置类参数在本量子里没变就整块算一次增益，变了就逐样本算。
//! `panningModel = 'HRTF'` 没做（要 HRTF 数据库），按 equalpower 处理（记在验收文档）。

use super::bus::{Bus, QUANTUM};
use super::{Flow, Msg, Node, Param, Quantum, Retired};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Distance {
    Linear,
    Inverse,
    Exponential,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub distance: Distance,
    pub ref_distance: f64,
    pub max_distance: f64,
    pub rolloff: f64,
    pub cone_inner: f64,
    pub cone_outer: f64,
    pub cone_outer_gain: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self { distance: Distance::Inverse, ref_distance: 1.0, max_distance: 10000.0, rolloff: 1.0, cone_inner: 360.0, cone_outer: 360.0, cone_outer_gain: 0.0 }
    }
}

type V3 = [f64; 3];
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn len(a: V3) -> f64 {
    dot(a, a).sqrt()
}
fn norm(a: V3) -> V3 {
    let l = len(a);
    if l > 0.0 { [a[0] / l, a[1] / l, a[2] / l] } else { a }
}

/// 规范的「Azimuth and Elevation」：返回方位角（度）
pub fn azimuth(src: V3, lpos: V3, fwd: V3, up: V3) -> f64 {
    let sl = sub(src, lpos);
    if len(sl) == 0.0 {
        return 0.0;
    }
    let sl = norm(sl);
    let right = norm(cross(fwd, up));
    let fwd_n = norm(fwd);
    let up2 = cross(right, fwd_n);
    let up_proj = dot(sl, up2);
    let proj = norm(sub(sl, [up2[0] * up_proj, up2[1] * up_proj, up2[2] * up_proj]));
    let mut az = dot(proj, right).clamp(-1.0, 1.0).acos().to_degrees();
    if dot(proj, fwd_n) < 0.0 {
        az = 360.0 - az;
    }
    if (0.0..=270.0).contains(&az) { 90.0 - az } else { 450.0 - az }
}

pub fn distance_gain(c: &Config, d: f64) -> f64 {
    let r = c.ref_distance;
    match c.distance {
        Distance::Linear => {
            let (mn, mx) = (r.min(c.max_distance), r.max(c.max_distance));
            let dd = d.clamp(mn, mx);
            if mx == mn {
                return 1.0;
            }
            1.0 - c.rolloff.clamp(0.0, 1.0) * (dd - mn) / (mx - mn)
        }
        Distance::Inverse => {
            if r <= 0.0 {
                return 1.0;
            }
            r / (r + c.rolloff * (d.max(r) - r))
        }
        Distance::Exponential => {
            if r <= 0.0 {
                return 1.0;
            }
            (d.max(r) / r).powf(-c.rolloff)
        }
    }
}

pub fn cone_gain(c: &Config, src: V3, orient: V3, lpos: V3) -> f64 {
    if len(orient) == 0.0 || (c.cone_inner == 360.0 && c.cone_outer == 360.0) {
        return 1.0;
    }
    let to_l = norm(sub(lpos, src));
    let ang = dot(to_l, norm(orient)).clamp(-1.0, 1.0).acos().to_degrees().abs();
    let (inner, outer) = (c.cone_inner.abs() / 2.0, c.cone_outer.abs() / 2.0);
    if ang <= inner {
        1.0
    } else if ang >= outer {
        c.cone_outer_gain
    } else {
        let x = (ang - inner) / (outer - inner);
        (1.0 - x) + c.cone_outer_gain * x
    }
}

/// 等功率声像的 (左增益, 右增益, 是否把另一声道混过来)：单声道输入只用前两个
fn pan(az: f64, stereo_in: bool) -> (f32, f32, bool) {
    let mut a = az.clamp(-180.0, 180.0);
    if a < -90.0 {
        a = -180.0 - a;
    } else if a > 90.0 {
        a = 180.0 - a;
    }
    let h = std::f64::consts::FRAC_PI_2;
    if !stereo_in {
        let x = (a + 90.0) / 180.0;
        return ((x * h).cos() as f32, (x * h).sin() as f32, false);
    }
    if a <= 0.0 {
        let x = (a + 90.0) / 90.0;
        ((x * h).cos() as f32, (x * h).sin() as f32, true)
    } else {
        let x = a / 90.0;
        ((x * h).cos() as f32, (x * h).sin() as f32, false)
    }
}

pub struct Panner {
    pub cfg: Config,
}

impl Panner {
    fn gains(&self, p: &[Param], k: usize, stereo: bool) -> (f32, f32, bool, f32) {
        let v = |i: usize| p[i].vals[k] as f64;
        let src = [v(0), v(1), v(2)];
        let orient = [v(3), v(4), v(5)];
        let (lpos, fwd, up) = ([v(6), v(7), v(8)], [v(9), v(10), v(11)], [v(12), v(13), v(14)]);
        let az = azimuth(src, lpos, fwd, up);
        let g = distance_gain(&self.cfg, len(sub(src, lpos))) * cone_gain(&self.cfg, src, orient, lpos);
        let (l, r, left_side) = pan(az, stereo);
        (l, r, left_side, g as f32)
    }
}

impl Node for Panner {
    fn process(&mut self, _q: &Quantum, ins: &[Bus], params: &[Param], outs: &mut [Bus]) -> Flow {
        let (inp, out) = (&ins[0], &mut outs[0]);
        out.reset(2);
        let stereo = inp.ch >= 2;
        let per_sample = params.iter().any(|p| p.varies);
        let mut g = self.gains(params, 0, stereo);
        for k in 0..QUANTUM {
            if per_sample && k > 0 {
                g = self.gains(params, k, stereo);
            }
            let (gl, gr, left_side, dg) = g;
            let (l, r) = if !stereo {
                let x = inp.chan(0)[k];
                (x * gl, x * gr)
            } else {
                let (il, ir) = (inp.chan(0)[k], inp.chan(1)[k]);
                // 声源在左：右声道按 cos 混一部分进左；在右：左声道按 sin 混一部分进右（规范 §6.2.2）
                if left_side { (il + ir * gl, ir * gr) } else { (il * gl, ir + il * gr) }
            };
            out.chan_mut(0)[k] = l * dg;
            out.chan_mut(1)[k] = r * dg;
        }
        Flow::Live
    }

    fn msg(&mut self, m: Msg) -> Option<Retired> {
        if let Msg::Panner(c) = m {
            self.cfg = c;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 方位角_左右前后() {
        let (o, f, u) = ([0.0; 3], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]);
        assert!((azimuth([1.0, 0.0, 0.0], o, f, u) - 90.0).abs() < 1e-9, "右边 = +90");
        assert!((azimuth([-1.0, 0.0, 0.0], o, f, u) + 90.0).abs() < 1e-9);
        assert!(azimuth([0.0, 0.0, -1.0], o, f, u).abs() < 1e-9, "正前方 = 0");
        let (l, r, _) = pan(90.0, false);
        assert!(l.abs() < 1e-6 && (r - 1.0).abs() < 1e-6);
        let (l, r, _) = pan(0.0, false);
        assert!((l - r).abs() < 1e-6 && (l - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
    }

    #[test]
    fn 距离衰减三种模型() {
        let mut c = Config::default();
        assert!((distance_gain(&c, 3.0) - 1.0 / 3.0).abs() < 1e-12);
        c.distance = Distance::Linear;
        c.max_distance = 11.0;
        assert!((distance_gain(&c, 6.0) - 0.5).abs() < 1e-12);
        c.distance = Distance::Exponential;
        c.rolloff = 2.0;
        assert!((distance_gain(&c, 2.0) - 0.25).abs() < 1e-12);
    }
}
