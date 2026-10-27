//! 改图的操作：建节点、连线 / 断线、AudioParam 自动化、start / stop、节点属性。
//! 参数校验与抛的异常名按规范（RangeError / InvalidStateError / IndexSizeError / InvalidAccessError / NotSupportedError）。

use super::super::graph::param::Ev;
use super::super::graph::{filter, osc, panner, GCmd, InSpec, Msg};
use super::kinds::{Kind, Opts};
use super::{err, ok, Conn, Out, PMirror};
use crate::game::audio::GameAudio;
use serde_json::{json, Value};

fn f(a: &Value, k: &str) -> Option<f64> {
    a.get(k).and_then(Value::as_f64)
}

fn floats(bin: &Option<Vec<u8>>) -> Vec<f32> {
    bin.as_deref().unwrap_or(&[]).chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

impl GameAudio {
    pub(super) fn op_node(&mut self, a: &Value) -> Out {
        let ctx = a.get("ctx").and_then(Value::as_u64).unwrap_or(u64::MAX) as usize;
        if !self.web.ctxs.contains_key(&ctx) {
            return err("InvalidStateError", "上下文已经关闭");
        }
        let Some(kind) = a.get("kind").and_then(Value::as_str).and_then(Kind::parse) else {
            return err("NotSupportedError", "未知的节点种类");
        };
        let o = a.get("opts").cloned().unwrap_or(Value::Null);
        let list = |k: &str| o.get(k).and_then(Value::as_array).map(|v| v.iter().filter_map(Value::as_f64).collect::<Vec<_>>()).unwrap_or_default();
        let opts = Opts {
            max_delay: f(&o, "maxDelayTime").unwrap_or(1.0),
            inputs: f(&o, "numberOfInputs").unwrap_or(6.0) as usize,
            outputs: f(&o, "numberOfOutputs").unwrap_or(6.0) as usize,
            ff: list("feedforward"),
            fb: list("feedback"),
        };
        match kind {
            Kind::Delay if !(opts.max_delay > 0.0 && opts.max_delay < 180.0) => return err("NotSupportedError", "maxDelayTime 必须在 (0, 180) 之间"),
            Kind::Merger | Kind::Splitter if !(1..=32).contains(&(if kind == Kind::Merger { opts.inputs } else { opts.outputs })) => {
                return err("IndexSizeError", "声道数必须在 1~32 之间")
            }
            Kind::Iir if opts.ff.is_empty() || opts.ff.len() > 20 || opts.fb.is_empty() || opts.fb.len() > 20 => {
                return err("NotSupportedError", "feedforward / feedback 长度必须在 1~20 之间")
            }
            Kind::Iir if opts.ff.iter().all(|v| *v == 0.0) || opts.fb[0] == 0.0 => return err("InvalidStateError", "feedforward 不能全零、feedback[0] 不能为 0"),
            _ => {}
        }
        match self.node_new(ctx, kind, &opts) {
            Ok(id) => {
                self.flush_plan(ctx);
                let n = &self.web.nodes[&id];
                let names: Vec<&str> = n.params.iter().map(|p| p.spec.name).collect();
                let defaults: Vec<Value> = n.params.iter().map(|p| json!([p.spec.base, p.spec.min, p.spec.max])).collect();
                ok(json!({ "node": id, "inputs": n.n_in, "outputs": n.n_out, "params": names, "ranges": defaults,
                    "channelCount": n.spec.count, "channelCountMode": mode_name(n.spec.mode), "channelInterpretation": interp_name(n.spec.interp) }))
            }
            Err(e) => e,
        }
    }

    pub(super) fn op_connect(&mut self, a: &Value) -> Out {
        let u = |k: &str| a.get(k).and_then(Value::as_u64).unwrap_or(0) as u32;
        let (from, to, out, port) = (u("from"), u("to"), u("out") as u16, u("input") as u16);
        let param = a.get("param").and_then(Value::as_str);
        let (Some(src), Some(dst)) = (self.web.nodes.get(&from), self.web.nodes.get(&to)) else {
            return err("InvalidAccessError", "节点不存在");
        };
        if src.ctx != dst.ctx {
            return err("InvalidAccessError", "不能跨上下文连线");
        }
        if out as usize >= src.n_out {
            return err("IndexSizeError", "output 越界");
        }
        let (port, is_param) = match param {
            Some(name) => match dst.params.iter().position(|p| p.spec.name == name) {
                // listener 参数（PannerNode 的 6~14）不接受连线：它们属于上下文，不是这个节点
                Some(i) if !(dst.kind == Kind::Panner && i >= 6) => (i as u16, true),
                _ => return err("InvalidAccessError", "参数不存在"),
            },
            None if port as usize >= dst.n_in => return err("IndexSizeError", "input 越界"),
            None => (port, false),
        };
        let ctx = src.ctx;
        let c = Conn { to, out, port, param: is_param };
        let n = self.web.nodes.get_mut(&from).unwrap();
        if !n.outs.contains(&c) && n.slot.is_some() {
            n.outs.push(c);
            self.flush_plan(ctx);
        }
        ok(json!({}))
    }

    /// disconnect()、disconnect(output)、disconnect(dest[, output[, input]])、disconnect(param[, output])
    pub(super) fn op_disconnect(&mut self, a: &Value) -> Out {
        let from = a.get("from").and_then(Value::as_u64).unwrap_or(0) as u32;
        let to = a.get("to").and_then(Value::as_u64).map(|v| v as u32);
        let out = a.get("out").and_then(Value::as_u64).map(|v| v as u16);
        let input = a.get("input").and_then(Value::as_u64).map(|v| v as u16);
        let param = a.get("param").and_then(Value::as_str).map(str::to_string);
        let Some(n) = self.web.nodes.get(&from) else { return ok(json!({})) };
        if out.is_some_and(|o| o as usize >= n.n_out) {
            return err("IndexSizeError", "output 越界");
        }
        let pidx = match (&param, to) {
            (Some(name), Some(t)) => self.web.nodes.get(&t).and_then(|d| d.params.iter().position(|p| &p.spec.name == name)),
            _ => None,
        };
        let ctx = n.ctx;
        let before = n.outs.len();
        let keep = |c: &Conn| {
            let hit = to.is_none_or(|t| c.to == t)
                && out.is_none_or(|o| c.out == o)
                && match (&param, pidx) {
                    (Some(_), Some(i)) => c.param && c.port == i as u16,
                    (Some(_), None) => false,
                    (None, _) => to.is_none() || (!c.param && input.is_none_or(|i| c.port == i)),
                };
            !hit
        };
        let n = self.web.nodes.get_mut(&from).unwrap();
        let downstream: Vec<u32> = n.outs.iter().filter(|c| !keep(c)).map(|c| c.to).collect();
        n.outs.retain(keep);
        if to.is_some() && n.outs.len() == before {
            return err("InvalidAccessError", "没有这条连线");
        }
        self.flush_plan(ctx);
        for d in downstream {
            self.maybe_collect(d);
        }
        ok(json!({}))
    }

    /// 参数的镜像（listener 参数在上下文上，按名字找）
    fn pmirror(&mut self, a: &Value) -> Option<(Option<u32>, usize, &mut PMirror)> {
        let name = a.get("name").and_then(Value::as_str)?;
        if let Some(ctx) = a.get("listener").and_then(Value::as_u64) {
            let c = self.web.ctxs.get_mut(&(ctx as usize))?;
            let i = c.listener.iter().position(|p| p.spec.name == name)?;
            return Some((None, ctx as usize, &mut c.listener[i]));
        }
        let id = a.get("node").and_then(Value::as_u64)? as u32;
        let n = self.web.nodes.get_mut(&id)?;
        let i = n.params.iter().position(|p| p.spec.name == name)?;
        let ctx = n.ctx;
        Some((Some(id), ctx, &mut n.params[i]))
    }

    pub(super) fn op_pget(&mut self, a: &Value) -> Out {
        let Some(ctx) = self.pmirror(a).map(|x| x.1) else { return ok(json!(0)) };
        let now = self.web.now(ctx);
        let (_, _, p) = self.pmirror(a).unwrap();
        let v = p.tl.value_at(now, p.base);
        ok(json!((v as f32).clamp(p.spec.min, p.spec.max)))
    }

    /// 自动化：`op` = value / set / linear / exp / target / curve / cancel / hold
    pub(super) fn op_param(&mut self, a: &Value, bin: Option<Vec<u8>>) -> Out {
        let op = a.get("op").and_then(Value::as_str).unwrap_or("").to_string();
        let (v, t) = (f(a, "v").unwrap_or(0.0), f(a, "t").unwrap_or(0.0));
        if !v.is_finite() || !t.is_finite() {
            return err("TypeError", "参数必须是有限数");
        }
        if t < 0.0 && op != "value" {
            return err("RangeError", "时刻不能为负");
        }
        let ctx_of = |s: &mut Self| s.pmirror(a).map(|x| x.1);
        let Some(ctx) = ctx_of(self) else { return err("InvalidAccessError", "参数不存在") };
        let now = self.web.now(ctx);
        let curve = floats(&bin);
        let (id, _, p) = self.pmirror(a).unwrap();
        let base = p.base;
        // ramp 前面没有事件、或前面是已经开始的 setTarget：先补一条 Set(当前值, now)（规范 §1.6.3）
        let anchor = |p: &mut PMirror, at: f64| {
            let before = p.tl.ev.iter().rposition(|e| e.time() < at);
            let need = match before {
                None => true,
                Some(i) => matches!(p.tl.ev[i], Ev::Target { t, .. } if t <= now),
            };
            if need {
                let cur = p.tl.value_at(now, p.base);
                p.tl.insert(Ev::Set { v: cur, t: now });
            }
        };
        match op.as_str() {
            "value" => {
                if p.tl.ev.is_empty() {
                    p.base = v;
                } else {
                    p.tl.insert(Ev::Set { v, t: now });
                }
            }
            "set" => p.tl.insert(Ev::Set { v, t }),
            "linear" => {
                anchor(p, t);
                p.tl.insert(Ev::Linear { v, t });
            }
            "exp" => {
                if v == 0.0 {
                    return err("RangeError", "exponentialRampToValueAtTime 的目标值不能为 0");
                }
                anchor(p, t);
                p.tl.insert(Ev::Exp { v, t });
            }
            "target" => {
                let tau = f(a, "tau").unwrap_or(0.0);
                if tau < 0.0 {
                    return err("RangeError", "timeConstant 不能为负");
                }
                p.tl.insert(Ev::Target { v, t, tau });
            }
            "curve" => {
                let dur = f(a, "dur").unwrap_or(0.0);
                if curve.len() < 2 {
                    return err("InvalidStateError", "曲线至少要 2 个值");
                }
                if dur <= 0.0 {
                    return err("RangeError", "duration 必须为正");
                }
                let (s, e) = (t, t + dur);
                if p.tl.ev.iter().any(|x| x.time() > s && x.time() < e) {
                    return err("NotSupportedError", "曲线的时间段里不能有别的事件");
                }
                p.tl.push_curve(&curve, t, dur);
            }
            "cancel" => p.tl.cancel(t),
            "hold" => p.tl.cancel_and_hold(t, base),
            _ => return err("NotSupportedError", "未知的自动化操作"),
        }
        p.tl.prune(now, p.base);
        let listener = a.get("listener").is_some();
        match (id, listener) {
            (Some(id), _) => {
                let i = self.web.nodes[&id].params.iter().position(|x| Some(x.spec.name) == a.get("name").and_then(Value::as_str)).unwrap();
                self.send_timeline(id, i);
            }
            (None, true) => {
                // listener 改了：同一个上下文里每个 PannerNode 的 6~14 号参数跟着换
                let i = self.web.ctxs[&ctx].listener.iter().position(|x| Some(x.spec.name) == a.get("name").and_then(Value::as_str)).unwrap();
                let (tl, b) = { let m = &self.web.ctxs[&ctx].listener[i]; (m.tl.clone(), m.base) };
                let panners: Vec<u32> = self.web.ctxs[&ctx].nodes.iter().copied().filter(|n| self.web.nodes.get(n).is_some_and(|x| x.kind == Kind::Panner)).collect();
                for pn in panners {
                    let m = &mut self.web.nodes.get_mut(&pn).unwrap().params[6 + i];
                    (m.tl, m.base) = (tl.clone(), b);
                    self.send_timeline(pn, 6 + i);
                }
            }
            _ => {}
        }
        ok(json!({}))
    }

    pub(super) fn op_start(&mut self, a: &Value) -> Out {
        let id = a.get("node").and_then(Value::as_u64).unwrap_or(0) as u32;
        let (when, offset) = (f(a, "when").unwrap_or(0.0), f(a, "offset").unwrap_or(0.0));
        let duration = f(a, "duration");
        if when < 0.0 || offset < 0.0 || duration.is_some_and(|d| d < 0.0) {
            return err("RangeError", "when / offset / duration 不能为负");
        }
        let Some(n) = self.web.nodes.get_mut(&id) else { return ok(json!({})) };
        if n.started {
            return err("InvalidStateError", "start 只能调一次");
        }
        n.started = true;
        if let Some(slot) = n.slot {
            self.web_send(GCmd::Msg { slot, m: Msg::Start { when, offset, duration } });
        }
        ok(json!({}))
    }

    pub(super) fn op_stop(&mut self, a: &Value) -> Out {
        let id = a.get("node").and_then(Value::as_u64).unwrap_or(0) as u32;
        let when = f(a, "when").unwrap_or(0.0);
        if when < 0.0 {
            return err("RangeError", "when 不能为负");
        }
        let Some(n) = self.web.nodes.get_mut(&id) else { return ok(json!({})) };
        if !n.started {
            return err("InvalidStateError", "还没 start");
        }
        n.stopped = true;
        if let Some(slot) = n.slot {
            self.web_send(GCmd::Msg { slot, m: Msg::Stop { when } });
        }
        ok(json!({}))
    }

    /// 节点属性：buffer / loop* / type / curve / panner 的各项 / channelCount* / analyser 的各项
    pub(super) fn op_prop(&mut self, a: &Value, bin: Option<Vec<u8>>) -> Out {
        let id = a.get("node").and_then(Value::as_u64).unwrap_or(0) as u32;
        let k = a.get("k").and_then(Value::as_str).unwrap_or("").to_string();
        let v = a.get("v").cloned().unwrap_or(Value::Null);
        let Some(n) = self.web.nodes.get(&id) else { return ok(json!({})) };
        let (kind, slot) = (n.kind, n.slot);
        let mut msg: Option<Msg> = None;
        match (kind, k.as_str()) {
            // bufferRefresh：JS 在 start 时发现 AudioBuffer 的数据被改过，重新上传后换上（不算第二次设 buffer）
            (Kind::Buffer, "buffer" | "bufferRefresh") => {
                let b = v.as_u64().and_then(|b| self.web.bufs.get(&(b as u32)).cloned());
                if k == "buffer" && b.is_some() && n.has_buffer {
                    return err("InvalidStateError", "buffer 只能设一次");
                }
                self.web.nodes.get_mut(&id).unwrap().has_buffer |= b.is_some();
                msg = Some(Msg::Buffer(b));
            }
            (Kind::Buffer, "loop" | "loopStart" | "loopEnd") => {
                let n = self.web.nodes.get_mut(&id).unwrap();
                n.props.insert(k.clone(), v.clone());
                let g = |x: &str| n.props.get(x).cloned().unwrap_or(Value::Null);
                msg = Some(Msg::Loop { on: g("loop").as_bool().unwrap_or(false), start: g("loopStart").as_f64().unwrap_or(0.0), end: g("loopEnd").as_f64().unwrap_or(0.0) });
            }
            (Kind::Oscillator, "type") => match v.as_str().and_then(osc::Builtin::parse) {
                Some(b) => msg = Some(Msg::Wave(self.web.wave(b))),
                None if v.as_str() == Some("custom") => return err("InvalidStateError", "custom 要用 setPeriodicWave"),
                None => return ok(json!({})),
            },
            (Kind::Oscillator, "periodicWave") => match v.as_u64().and_then(|w| self.web.waves.get(&(w as u32)).cloned()) {
                Some(w) => msg = Some(Msg::Wave(w)),
                None => return err("InvalidAccessError", "PeriodicWave 不存在"),
            },
            (Kind::Biquad, "type") => match filter::Kind::ALL.iter().position(|x| Some(*x) == v.as_str()) {
                Some(i) => msg = Some(Msg::Kind(i as u8)),
                None => return ok(json!({})),
            },
            (Kind::Shaper, "curve") => {
                let c = floats(&bin);
                if bin.is_some() && c.len() < 2 {
                    return err("InvalidStateError", "curve 至少要 2 个值");
                }
                msg = Some(Msg::Curve(bin.map(|_| c.into_boxed_slice())));
            }
            (Kind::Panner, _) if PANNER_PROPS.contains(&k.as_str()) => {
                let n = self.web.nodes.get_mut(&id).unwrap();
                n.props.insert(k.clone(), v.clone());
                msg = Some(Msg::Panner(panner_cfg(&n.props)));
            }
            (_, "channelCount" | "channelCountMode" | "channelInterpretation") => return self.op_channels(id, &k, &v),
            _ => {}
        }
        self.web.nodes.get_mut(&id).unwrap().props.insert(k, v);
        if let (Some(slot), Some(m)) = (slot, msg) {
            self.web_send(GCmd::Msg { slot, m });
        }
        ok(json!({}))
    }

    fn op_channels(&mut self, id: u32, k: &str, v: &Value) -> Out {
        let n = self.web.nodes.get_mut(&id).unwrap();
        let mut s: InSpec = n.spec;
        let fixed = matches!(n.kind, Kind::Merger | Kind::Splitter | Kind::Destination);
        match k {
            "channelCount" => {
                let c = v.as_f64().unwrap_or(0.0) as usize;
                if c == 0 || c > 32 {
                    return err("NotSupportedError", "channelCount 必须在 1~32 之间");
                }
                if fixed && c != s.count || n.kind == Kind::Panner && c > 2 {
                    return err("InvalidStateError", "这个节点不能改 channelCount");
                }
                s.count = c;
            }
            "channelCountMode" => match v.as_str().and_then(Self::web_mode) {
                Some(m) if fixed && m != s.mode => return err("InvalidStateError", "这个节点不能改 channelCountMode"),
                Some(m) if n.kind == Kind::Panner && m == super::super::graph::bus::Mode::Max => return err("NotSupportedError", "PannerNode 不支持 max"),
                Some(m) => s.mode = m,
                None => return ok(json!({})),
            },
            _ => match v.as_str().and_then(Self::web_interp) {
                Some(i) if fixed && i != s.interp => return err("InvalidStateError", "这个节点不能改 channelInterpretation"),
                Some(i) => s.interp = i,
                None => return ok(json!({})),
            },
        }
        n.spec = s;
        if let Some(slot) = n.slot {
            self.web_send(GCmd::Channels { slot, input: 0, spec: s });
        }
        ok(json!({}))
    }

    pub(super) fn op_wave(&mut self, a: &Value) -> Out {
        let list = |k: &str| a.get(k).and_then(Value::as_array).map(|v| v.iter().map(|x| x.as_f64().unwrap_or(0.0)).collect::<Vec<_>>());
        let (re, im) = (list("real"), list("imag"));
        let (re, im) = match (re, im) {
            (Some(r), Some(i)) if r.len() != i.len() => return err("IndexSizeError", "real 与 imag 长度必须相同"),
            (Some(r), Some(i)) => (r, i),
            (Some(r), None) => { let n = r.len(); (r, vec![0.0; n]) }
            (None, Some(i)) => (vec![0.0; i.len()], i),
            (None, None) => (vec![0.0, 0.0], vec![0.0, 1.0]),
        };
        if re.len() < 2 {
            return err("IndexSizeError", "real / imag 至少 2 个");
        }
        let no_norm = a.get("disableNormalization").and_then(Value::as_bool).unwrap_or(false);
        let id = self.web.next_buf;
        self.web.next_buf += 1;
        self.web.waves.insert(id, std::sync::Arc::new(osc::WaveTables::from_coeffs(&re, &im, self.web.rate, no_norm)));
        ok(json!({ "wave": id }))
    }
}

const PANNER_PROPS: [&str; 7] = ["distanceModel", "refDistance", "maxDistance", "rolloffFactor", "coneInnerAngle", "coneOuterAngle", "coneOuterGain"];

fn panner_cfg(p: &serde_json::Map<String, Value>) -> panner::Config {
    let d = panner::Config::default();
    let g = |k: &str, def: f64| p.get(k).and_then(Value::as_f64).unwrap_or(def);
    panner::Config {
        distance: match p.get("distanceModel").and_then(Value::as_str) {
            Some("linear") => panner::Distance::Linear,
            Some("exponential") => panner::Distance::Exponential,
            _ => panner::Distance::Inverse,
        },
        ref_distance: g("refDistance", d.ref_distance),
        max_distance: g("maxDistance", d.max_distance),
        rolloff: g("rolloffFactor", d.rolloff),
        cone_inner: g("coneInnerAngle", d.cone_inner),
        cone_outer: g("coneOuterAngle", d.cone_outer),
        cone_outer_gain: g("coneOuterGain", d.cone_outer_gain),
    }
}

pub fn mode_name(m: super::super::graph::bus::Mode) -> &'static str {
    use super::super::graph::bus::Mode::*;
    match m {
        Max => "max",
        ClampedMax => "clamped-max",
        Explicit => "explicit",
    }
}

pub fn interp_name(i: super::super::graph::bus::Interp) -> &'static str {
    match i {
        super::super::graph::bus::Interp::Speakers => "speakers",
        super::super::graph::bus::Interp::Discrete => "discrete",
    }
}
