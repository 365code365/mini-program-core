//! AudioParam 的自动化时间线（Web Audio 规范 §1.6「Computation of Value」）。
//!
//! 同一份求值在两个线程用：音频线程按样本时刻填 128 帧的参数值；引擎线程用它回答 JS 的 `param.value`、
//! 给「没有前一个事件的 ramp」补隐式起点、剪掉过去的事件。两边拿的是同一张事件表（引擎线程每改一次就整张发过去），
//! 所以结果一致。
//!
//! 约定（与规范一致的地方不再一一注明）：
//! - 事件按时间排序，同一时刻的按插入顺序排在已有事件之后；
//! - ramp 由**结束**时刻定义，起点是前一个事件的 (时刻, 值)；前面没有事件时引擎线程先补一条
//!   `Set(当前值, currentTime)`（规范对 linearRamp / exponentialRamp 的要求）；
//! - exponentialRamp 起止值异号或起点为 0：整段保持起点值；
//! - setTarget 的起点值是「它之前的事件在它那一刻给出的值」；
//! - setValueCurve 之后引擎线程补一条 `Set(最后一个值, T + D)`（规范要求）；
//! - `cancelAndHoldAtTime` 在引擎线程改表（见 [`Timeline::cancel_and_hold`]）。

/// 一个自动化事件
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ev {
    Set { v: f64, t: f64 },
    Linear { v: f64, t: f64 },
    Exp { v: f64, t: f64 },
    Target { v: f64, t: f64, tau: f64 },
    /// 曲线的值在 `Timeline::curve[at .. at + len]`；`cut` 是 cancelAndHold 截断的时刻（没截断就是 t + dur）
    Curve { t: f64, dur: f64, at: u32, len: u32, cut: f64 },
}

impl Ev {
    pub fn time(&self) -> f64 {
        match *self {
            Ev::Set { t, .. } | Ev::Linear { t, .. } | Ev::Exp { t, .. } | Ev::Target { t, .. } | Ev::Curve { t, .. } => t,
        }
    }
    fn is_ramp(&self) -> bool {
        matches!(self, Ev::Linear { .. } | Ev::Exp { .. })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Timeline {
    pub ev: Vec<Ev>,
    pub curve: Vec<f32>,
}

impl Timeline {
    /// 按规范的位置插入（同一时刻排在已有事件之后）
    pub fn insert(&mut self, e: Ev) {
        let t = e.time();
        let at = self.ev.partition_point(|x| x.time() <= t);
        self.ev.insert(at, e);
    }

    pub fn push_curve(&mut self, values: &[f32], t: f64, dur: f64) {
        let at = self.curve.len() as u32;
        self.curve.extend_from_slice(values);
        self.insert(Ev::Curve { t, dur, at, len: values.len() as u32, cut: t + dur });
        if let Some(&last) = values.last() {
            self.insert(Ev::Set { v: last as f64, t: t + dur });
        }
    }

    /// 时刻 `t` 的值（`base`：没有任何事件生效时的固有值）
    pub fn value_at(&self, t: f64, base: f64) -> f64 {
        let n = self.ev.partition_point(|x| x.time() <= t);
        self.eval(n, t, base)
    }

    /// 只看前 `n` 个事件（时刻都 ≤ t）与第 n 个（> t，可能是正在进行的 ramp）求 t 处的值
    fn eval(&self, n: usize, t: f64, base: f64) -> f64 {
        if let Some(next) = self.ev.get(n).filter(|e| e.is_ramp()) {
            let (t0, v0) = match n.checked_sub(1) {
                Some(i) => (self.ev[i].time(), self.start_value(i, base)),
                // 前面没有事件：引擎线程本该补过起点；补漏了就按固有值从 0 时刻起
                None => (0.0, base),
            };
            return ramp(next, t0, v0, t);
        }
        let Some(i) = n.checked_sub(1) else { return base };
        match self.ev[i] {
            Ev::Set { v, .. } | Ev::Linear { v, .. } | Ev::Exp { v, .. } => v,
            Ev::Target { v, t: t0, tau } => {
                let v0 = self.eval(i, t0, base);
                if tau <= 0.0 {
                    return v;
                }
                v + (v0 - v) * (-(t - t0) / tau).exp()
            }
            Ev::Curve { t: t0, dur, at, len, cut } => {
                let tt = t.min(cut);
                self.curve_value(at, len, t0, dur, tt)
            }
        }
    }

    /// 事件 i 后面接 ramp 时的起点值。Set / ramp 是它自己的值；setTarget 是它开始前一刻的值（规范：没开始的
    /// setTarget 被 ramp 取代；已经开始的由引擎线程在插入 ramp 时先补一条 `Set(当前值, now)`，到不了这里）；
    /// 曲线后面总跟着一条 Set（push_curve / cancel_and_hold 补的），这里取截断处的值兜底
    fn start_value(&self, i: usize, base: f64) -> f64 {
        match self.ev[i] {
            Ev::Set { v, .. } | Ev::Linear { v, .. } | Ev::Exp { v, .. } => v,
            Ev::Target { t, .. } => self.eval(i, t, base),
            Ev::Curve { t, dur, at, len, cut } => self.curve_value(at, len, t, dur, cut),
        }
    }

    fn curve_value(&self, at: u32, len: u32, t0: f64, dur: f64, t: f64) -> f64 {
        let c = &self.curve[at as usize..(at + len) as usize];
        if c.is_empty() {
            return 0.0;
        }
        if dur <= 0.0 || t >= t0 + dur || c.len() == 1 {
            return *c.last().unwrap() as f64;
        }
        let pos = (c.len() - 1) as f64 * (t - t0) / dur;
        let k = pos.floor() as usize;
        let k = k.min(c.len() - 2);
        let (a, b) = (c[k] as f64, c[k + 1] as f64);
        a + (b - a) * (pos - k as f64)
    }

    /// 从 t0 起每隔 dt 填 `out`（音频线程的 a-rate 参数）。全段落在一个常量区间里时直接整段填
    pub fn fill(&self, out: &mut [f32], t0: f64, dt: f64, base: f64) {
        let t1 = t0 + dt * out.len() as f64;
        let n = self.ev.partition_point(|x| x.time() <= t0);
        let constant = self.ev.get(n).is_none_or(|e| e.time() > t1 && !e.is_ramp())
            && n.checked_sub(1).is_none_or(|i| matches!(self.ev[i], Ev::Set { .. } | Ev::Linear { .. } | Ev::Exp { .. }));
        if constant {
            out.fill(self.eval(n, t0, base) as f32);
            return;
        }
        let mut n = n;
        for (k, o) in out.iter_mut().enumerate() {
            let t = t0 + dt * k as f64;
            while n < self.ev.len() && self.ev[n].time() <= t {
                n += 1;
            }
            *o = self.eval(n, t, base) as f32;
        }
    }

    /// 时刻 `t` 及以后的事件全删（cancelScheduledValues）
    pub fn cancel(&mut self, t: f64) {
        self.ev.retain(|e| e.time() < t);
    }

    /// cancelAndHoldAtTime：t 处的值保持住。进行中的 ramp 改成「在 t 结束、终值为 v(t)」（直线 / 指数曲线上
    /// 截一段仍是同一条线）；进行中的 setTarget 保留、再在 t 补一条 Set；进行中的曲线截断在 t
    pub fn cancel_and_hold(&mut self, t: f64, base: f64) {
        let v = self.value_at(t, base);
        let n = self.ev.partition_point(|x| x.time() <= t);
        let next = self.ev.get(n).copied();
        self.ev.truncate(n);
        match next {
            Some(Ev::Linear { .. }) => self.ev.push(Ev::Linear { v, t }),
            Some(Ev::Exp { .. }) => self.ev.push(Ev::Exp { v, t }),
            _ => {
                if let Some(Ev::Curve { cut, .. }) = self.ev.last_mut() {
                    *cut = cut.min(t);
                }
                self.ev.push(Ev::Set { v, t });
            }
        }
    }

    /// 剪掉 `now` 之前已经不影响 ≥ now 求值的事件：取最后一个时刻 ≤ now 的事件 k，
    /// 前面的换成一条 `Set(k 之前的事件在 T_k 给出的值, T_k)` 放在 k 前面（同一时刻、排在前）。
    /// 对 t ≥ T_k 的求值逐位相同：ramp 的起点、setTarget 的起点值都只看「k 之前在 T_k 的值」
    pub fn prune(&mut self, now: f64, base: f64) {
        let n = self.ev.partition_point(|x| x.time() <= now);
        if n < 2 {
            return;
        }
        let k = n - 1;
        let tk = self.ev[k].time();
        let before = self.eval(k, tk, base);
        let keep_curve = matches!(self.ev[k], Ev::Curve { .. });
        self.ev.drain(..k);
        self.ev.insert(0, Ev::Set { v: before, t: tk });
        if !keep_curve && !self.ev.iter().any(|e| matches!(e, Ev::Curve { .. })) {
            self.curve.clear();
        }
    }
}

fn ramp(next: &Ev, t0: f64, v0: f64, t: f64) -> f64 {
    match *next {
        Ev::Linear { v, t: t1 } => {
            if t1 <= t0 {
                return v;
            }
            v0 + (v - v0) * (t - t0) / (t1 - t0)
        }
        Ev::Exp { v, t: t1 } => {
            if v0 == 0.0 || v0 * v < 0.0 {
                return v0;
            }
            if t1 <= t0 {
                return v;
            }
            v0 * (v / v0).powf((t - t0) / (t1 - t0))
        }
        _ => v0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn 置值_直线_指数_目标值_逐段() {
        let mut tl = Timeline::default();
        tl.insert(Ev::Set { v: 0.0, t: 1.0 });
        tl.insert(Ev::Linear { v: 1.0, t: 2.0 });
        tl.insert(Ev::Exp { v: 4.0, t: 3.0 });
        tl.insert(Ev::Target { v: 0.0, t: 4.0, tau: 0.5 });
        assert!(close(tl.value_at(0.5, 7.0), 7.0), "第一个事件之前是固有值");
        assert!(close(tl.value_at(1.5, 7.0), 0.5));
        assert!(close(tl.value_at(2.5, 7.0), 2.0), "指数：1→4 中点 = 2");
        assert!(close(tl.value_at(3.5, 7.0), 4.0), "ramp 结束后保持");
        assert!(close(tl.value_at(4.5, 7.0), 4.0 * (-1.0f64).exp()));
    }

    #[test]
    fn 指数_起点为零或异号整段保持() {
        let mut tl = Timeline::default();
        tl.insert(Ev::Set { v: 0.0, t: 0.0 });
        tl.insert(Ev::Exp { v: 1.0, t: 1.0 });
        assert_eq!(tl.value_at(0.5, 0.0), 0.0);
        assert_eq!(tl.value_at(1.0, 0.0), 1.0);
    }

    #[test]
    fn 曲线_线性插值_之后保持最后一个() {
        let mut tl = Timeline::default();
        tl.push_curve(&[0.0, 1.0, 0.0], 1.0, 2.0);
        assert!(close(tl.value_at(1.5, 9.0), 0.5));
        assert!(close(tl.value_at(2.0, 9.0), 1.0));
        assert!(close(tl.value_at(5.0, 9.0), 0.0));
    }

    #[test]
    fn 取消与保持() {
        let mut tl = Timeline::default();
        tl.insert(Ev::Set { v: 0.0, t: 0.0 });
        tl.insert(Ev::Linear { v: 10.0, t: 10.0 });
        let mut c = tl.clone();
        c.cancel_and_hold(4.0, 0.0);
        assert!(close(c.value_at(2.0, 0.0), 2.0), "截断前的斜线不变");
        assert!(close(c.value_at(8.0, 0.0), 4.0), "之后保持 v(4)");
        tl.cancel(5.0);
        assert!(close(tl.value_at(8.0, 0.0), 0.0), "cancelScheduledValues 把 ramp 整个删掉");
    }

    #[test]
    fn 剪枝前后求值相同() {
        let mut tl = Timeline::default();
        let mut t = 0.0;
        for i in 0..50 {
            tl.insert(Ev::Target { v: (i % 3) as f64, t, tau: 0.05 });
            t += 0.02;
            tl.insert(Ev::Linear { v: i as f64 * 0.1, t });
            t += 0.01;
        }
        let full = tl.clone();
        tl.prune(0.7, 1.0);
        // 0.7s 及以前的事件只留最后一个，前面补 1 条 Set
        let before = full.ev.iter().filter(|e| e.time() <= 0.7).count();
        assert!(before > 40);
        assert_eq!(tl.ev.len(), full.ev.len() - before + 2);
        for k in 0..200 {
            let x = 0.7 + k as f64 * 0.005;
            assert!(close(tl.value_at(x, 1.0), full.value_at(x, 1.0)), "t={x}");
        }
    }

    #[test]
    fn 填充_常量区间与逐样本一致() {
        let mut tl = Timeline::default();
        tl.insert(Ev::Set { v: 0.25, t: 0.0 });
        tl.insert(Ev::Linear { v: 1.0, t: 0.01 });
        let mut a = [0f32; 128];
        for q in 0..8 {
            let t0 = q as f64 * 128.0 / 48_000.0;
            tl.fill(&mut a, t0, 1.0 / 48_000.0, 0.0);
            for (k, &x) in a.iter().enumerate() {
                assert_eq!(x, tl.value_at(t0 + k as f64 / 48_000.0, 0.0) as f32);
            }
        }
    }
}
