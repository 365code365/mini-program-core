//! 预览里当前那张的缩放与平移（对齐微信）：双指捏合缩放（以两指中点为锚）、双击放大 / 还原（以点的位置为锚）、
//! 放大后单指拖动平移（到边有阻力）、松手回到合法范围（缩放 1~[`MAX`]、平移不露出图外的黑边）。
//!
//! 坐标约定：`(px, py)` 是图的中心相对屏幕中心的位移（逻辑像素），`z` 是相对 aspectFit 大小的倍数。
//! 屏幕上一点 `s` 落在图的「fit 坐标」`c = (s - 屏幕中心 - p) / z` 上；缩放时让锚点下面那个 `c` 不动，
//! 就是 `p' = s - 屏幕中心 - c·z'` —— 捏合与双击都按这一条算。
//!
//! 动画同样按时间的闭式解求值（`ease_out_cubic`），同一时刻画几次结果都一样。
use super::motion::ease_out_cubic;
use std::time::Instant;

/// 最大倍数（松手时超过的回弹到这里；捏的过程中可以再多一点，有阻力）
pub const MAX: f32 = 4.0;
/// 双击放大到的倍数
pub const DOUBLE: f32 = 2.5;
/// 两次轻点间隔不超过它、距离不超过 [`DOUBLE_TAP_DIST`] 算双击。单击的关闭要等它过去（微信也是这样）
pub const DOUBLE_TAP_SECS: f32 = 0.25;
pub const DOUBLE_TAP_DIST: f32 = 40.0;
/// 回到合法范围 / 双击的动画时长
const ANIM_SECS: f32 = 0.24;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub z: f32,
    pub px: f32,
    pub py: f32,
}

impl View {
    pub const FIT: View = View { z: 1.0, px: 0.0, py: 0.0 };

    pub fn zoomed(&self) -> bool {
        self.z > 1.01
    }
}

/// 缩放状态：停着的那份 + 正在走的动画
#[derive(Clone, Copy, Debug)]
pub struct Zoom {
    view: View,
    anim: Option<(Instant, View, View)>,
}

impl Default for Zoom {
    fn default() -> Self {
        Self { view: View::FIT, anim: None }
    }
}

impl Zoom {
    /// 这一刻的样子
    pub fn at(&self, now: Instant) -> View {
        let Some((t0, a, b)) = self.anim else { return self.view };
        let k = ease_out_cubic(now.saturating_duration_since(t0).as_secs_f32() / ANIM_SECS);
        View { z: a.z + (b.z - a.z) * k, px: a.px + (b.px - a.px) * k, py: a.py + (b.py - a.py) * k }
    }

    pub fn animating(&self) -> bool {
        self.anim.is_some()
    }

    /// 动画走完了就落定（每帧调一次）。返回是否刚落定
    pub fn tick(&mut self, now: Instant) -> bool {
        match self.anim {
            Some((t0, _, b)) if now.saturating_duration_since(t0).as_secs_f32() >= ANIM_SECS => {
                self.view = b;
                self.anim = None;
                true
            }
            _ => false,
        }
    }

    /// 直接设（跟手的时候）：动画停在这一刻
    pub fn set(&mut self, v: View) {
        self.view = v;
        self.anim = None;
    }

    pub fn animate_to(&mut self, now: Instant, to: View) {
        let from = self.at(now);
        self.view = to;
        self.anim = (from != to).then_some((now, from, to));
    }

    /// 回到合法范围（缩放 1~MAX、平移不露黑边），有动画
    pub fn settle(&mut self, now: Instant, fit: (f32, f32), vp: (f32, f32)) {
        let v = self.at(now);
        let z = v.z.clamp(1.0, MAX);
        // 缩小回 1 的时候平移也归零（不然还原后图是歪的）
        let (px, py) = if z <= 1.0 { (0.0, 0.0) } else { clamp_pan(z, v.px, v.py, fit, vp) };
        self.animate_to(now, View { z, px, py });
    }

    /// 双击：没放大就以 `(sx, sy)`（屏幕逻辑坐标）为锚放大到 [`DOUBLE`]，放大着就还原
    pub fn double_tap(&mut self, now: Instant, sx: f32, sy: f32, fit: (f32, f32), vp: (f32, f32)) {
        let v = self.at(now);
        if v.zoomed() {
            return self.animate_to(now, View::FIT);
        }
        // 点在图外（上下的黑边）就按最近的图边放大：锚点 fit 坐标夹到图里
        let (cx, cy) = (((sx - vp.0 / 2.0 - v.px) / v.z).clamp(-fit.0 / 2.0, fit.0 / 2.0), ((sy - vp.1 / 2.0 - v.py) / v.z).clamp(-fit.1 / 2.0, fit.1 / 2.0));
        let z = DOUBLE;
        let (px, py) = clamp_pan(z, sx - vp.0 / 2.0 - cx * z, sy - vp.1 / 2.0 - cy * z, fit, vp);
        self.animate_to(now, View { z, px, py });
    }
}

/// 平移的合法范围：图比屏宽 / 高的那一边可以挪到边缘对齐，不够宽 / 高的那一边居中
pub fn pan_limit(z: f32, fit: (f32, f32), vp: (f32, f32)) -> (f32, f32) {
    (((fit.0 * z - vp.0) / 2.0).max(0.0), ((fit.1 * z - vp.1) / 2.0).max(0.0))
}

pub fn clamp_pan(z: f32, px: f32, py: f32, fit: (f32, f32), vp: (f32, f32)) -> (f32, f32) {
    let (mx, my) = pan_limit(z, fit, vp);
    (px.clamp(-mx, mx), py.clamp(-my, my))
}

/// 超出范围的那一截给阻力（越拉越紧）：返回实际画出来的位置
pub fn rubber_pan(z: f32, px: f32, py: f32, fit: (f32, f32), vp: (f32, f32)) -> (f32, f32) {
    let (mx, my) = pan_limit(z, fit, vp);
    let r = |v: f32, m: f32, span: f32| {
        let over = v.abs() - m;
        if over <= 0.0 {
            v
        } else {
            v.signum() * (m + span * 0.35 * over / (over + span * 0.35))
        }
    };
    (r(px, mx, vp.0), r(py, my, vp.1))
}

/// 捏合中的缩放：低于 1、高于 MAX 的那一截有阻力
pub fn rubber_zoom(z: f32) -> f32 {
    if z < 1.0 {
        1.0 - (1.0 - z) * 0.45
    } else if z > MAX {
        MAX + (z - MAX) * 0.3 / (1.0 + (z - MAX))
    } else {
        z
    }
}

/// 一次捏合：两指按下那一刻的距离、缩放与锚点下的 fit 坐标
#[derive(Clone, Copy, Debug)]
pub struct Pinch {
    d0: f32,
    z0: f32,
    c: (f32, f32),
}

impl Pinch {
    /// 两指按下：`a`、`b` 屏幕逻辑坐标，`v` 这一刻的样子
    pub fn begin(a: (f32, f32), b: (f32, f32), v: View, vp: (f32, f32)) -> Self {
        let m = mid(a, b);
        let c = ((m.0 - vp.0 / 2.0 - v.px) / v.z, (m.1 - vp.1 / 2.0 - v.py) / v.z);
        Self { d0: dist(a, b).max(1.0), z0: v.z, c }
    }

    /// 两指挪到 `a`、`b`：锚点（fit 坐标 `c`）始终落在两指中点下面 —— 缩放与双指一起拖动都跟手
    pub fn update(&self, a: (f32, f32), b: (f32, f32), vp: (f32, f32)) -> View {
        let z = rubber_zoom(self.z0 * dist(a, b) / self.d0);
        let m = mid(a, b);
        View { z, px: m.0 - vp.0 / 2.0 - self.c.0 * z, py: m.1 - vp.1 / 2.0 - self.c.1 * z }
    }
}

fn mid(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
}

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VP: (f32, f32) = (375.0, 667.0);
    /// 横图：fit 后 375×250
    const FIT: (f32, f32) = (375.0, 250.0);

    #[test]
    fn 捏合时两指中点下面的那一点不动() {
        let (a, b) = ((100.0, 300.0), (200.0, 300.0));
        let p = Pinch::begin(a, b, View::FIT, VP);
        // 两指拉开一倍：放大 2 倍，中点 (150,300) 下面还是同一点
        let v = p.update((50.0, 300.0), (250.0, 300.0), VP);
        assert!((v.z - 2.0).abs() < 1e-4);
        let c = ((150.0 - VP.0 / 2.0 - v.px) / v.z, (300.0 - VP.1 / 2.0 - v.py) / v.z);
        let c0 = (150.0 - VP.0 / 2.0, 300.0 - VP.1 / 2.0);
        assert!((c.0 - c0.0).abs() < 1e-3 && (c.1 - c0.1).abs() < 1e-3, "{c:?} vs {c0:?}");
    }

    #[test]
    fn 捏得过小过大有阻力_松手回到合法范围() {
        assert!(rubber_zoom(0.5) > 0.5 && rubber_zoom(0.5) < 1.0);
        assert!(rubber_zoom(8.0) < MAX + 0.3);
        let mut z = Zoom::default();
        let t0 = Instant::now();
        z.set(View { z: 0.7, px: 30.0, py: 0.0 });
        z.settle(t0, FIT, VP);
        z.tick(t0 + std::time::Duration::from_secs(1));
        assert_eq!(z.at(t0 + std::time::Duration::from_secs(1)), View::FIT, "缩小过头：回到原大、居中");
    }

    #[test]
    fn 平移不露黑边_不够高的那一边居中() {
        // 放大 2 倍：750×500，横向可挪 ±187.5，纵向 500 < 667 只能居中
        assert_eq!(clamp_pan(2.0, 500.0, 80.0, FIT, VP), (187.5, 0.0));
        let (x, _) = rubber_pan(2.0, 300.0, 0.0, FIT, VP);
        assert!(x > 187.5 && x < 300.0, "超出的那一截有阻力：{x}");
    }

    #[test]
    fn 双击放大以点的位置为锚_再双击还原() {
        let t0 = Instant::now();
        let later = t0 + std::time::Duration::from_secs(1);
        let mut z = Zoom::default();
        // 点在图的左边四分之一处：放大后往右挪（让那一块到屏幕中间附近），而且不越界
        z.double_tap(t0, 100.0, 333.5, FIT, VP);
        z.tick(later);
        let v = z.at(later);
        assert_eq!(v.z, DOUBLE);
        assert!(v.px > 0.0 && v.px <= pan_limit(DOUBLE, FIT, VP).0, "{v:?}");
        z.double_tap(later, 100.0, 333.5, FIT, VP);
        z.tick(later + std::time::Duration::from_secs(1));
        assert_eq!(z.at(later + std::time::Duration::from_secs(1)), View::FIT);
    }
}
