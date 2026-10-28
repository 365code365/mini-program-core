//! 二进制往来与只读查询：AudioBuffer 的上传 / 取回、decodeAudioData、AnalyserNode 的数据、getFrequencyResponse，
//! 以及 JS 入口 `__game_webaudio(op, json, bytes?) → [json, ArrayBuffer | null]`。
//!
//! decodeAudioData 在后台线程解码（与 InnerAudioContext 同一个解码器），结果重采样到上下文的采样率（规范要求）。
//! 重采样用线性插值（Chrome 用 sinc 重采样器）：采样率不同的素材与 Chrome 有可听不出的小差别，AU5 用 48k 素材对拍。

use super::super::graph::analyser::{byte_of_db, byte_of_sample, db, MAX_FFT};
use super::super::graph::filter::{coefficients, response, Kind as FKind};
use super::super::graph::AudioBuf;
use super::kinds::Kind;
use super::{err, ok, Out};
use crate::audio_ctx::decode::Pcm;
use crate::game::audio::GameAudio;
use serde_json::{json, Value};
use std::sync::Arc;

fn planar_bytes(b: &AudioBuf) -> Vec<u8> {
    let mut out = Vec::with_capacity(b.len * b.chans.len() * 4);
    for c in b.chans.iter() {
        for v in c.iter() {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
    out
}

/// 交错 PCM → 平面，并重采样到 `rate`
pub fn planar_resampled(p: &Pcm, rate: f64) -> AudioBuf {
    let ch = p.channels.max(1) as usize;
    let n = p.samples.len() / ch;
    let src_rate = p.rate.max(1) as f64;
    let out_len = if (src_rate - rate).abs() < 0.5 { n } else { (n as f64 * rate / src_rate).round() as usize };
    let step = src_rate / rate;
    let chans = (0..ch)
        .map(|c| {
            (0..out_len)
                .map(|i| {
                    if out_len == n {
                        return p.samples[i * ch + c];
                    }
                    let x = i as f64 * step;
                    let k = x as usize;
                    let t = (x - k as f64) as f32;
                    let a = p.samples.get(k * ch + c).copied().unwrap_or(0.0);
                    let b = p.samples.get((k + 1) * ch + c).copied().unwrap_or(a);
                    a + (b - a) * t
                })
                .collect::<Vec<f32>>()
                .into_boxed_slice()
        })
        .collect();
    AudioBuf { rate, len: out_len, chans }
}

impl GameAudio {
    /// createBuffer / 改过数据的 AudioBuffer：平面 f32 一次传过来
    pub(super) fn op_buf_new(&mut self, a: &Value, bin: Option<Vec<u8>>) -> Out {
        let ch = a.get("channels").and_then(Value::as_u64).unwrap_or(0) as usize;
        let len = a.get("length").and_then(Value::as_u64).unwrap_or(0) as usize;
        let rate = a.get("sampleRate").and_then(Value::as_f64).unwrap_or(0.0);
        if !(1..=32).contains(&ch) || len == 0 || !(3000.0..=768_000.0).contains(&rate) {
            return err("NotSupportedError", "numberOfChannels 1~32、length > 0、sampleRate 3000~768000");
        }
        let bytes = bin.unwrap_or_default();
        let f = |i: usize| bytes.get(i * 4..i * 4 + 4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).unwrap_or(0.0);
        let chans = (0..ch).map(|c| (0..len).map(|i| f(c * len + i)).collect::<Vec<_>>().into_boxed_slice()).collect();
        let id = self.web.next_buf;
        self.web.next_buf += 1;
        self.web.bufs.insert(id, Arc::new(AudioBuf { rate, len, chans }));
        ok(json!({ "buf": id }))
    }

    pub(super) fn op_buf_get(&mut self, a: &Value) -> Out {
        let id = a.get("buf").and_then(Value::as_u64).unwrap_or(0) as u32;
        match self.web.bufs.get(&id) {
            Some(b) => (json!({}).to_string(), Some(planar_bytes(b))),
            None => err("InvalidStateError", "AudioBuffer 不存在"),
        }
    }

    pub(super) fn op_decode(&mut self, a: &Value, bin: Option<Vec<u8>>) -> Out {
        let ctx = a.get("ctx").and_then(Value::as_u64).unwrap_or(0) as usize;
        let Some(bytes) = bin.filter(|b| !b.is_empty()) else { return err("TypeError", "decodeAudioData 要 ArrayBuffer") };
        let job = self.web.next_job;
        self.web.next_job += 1;
        self.web.decoding += 1;
        let tx = self.web.decode_tx.clone();
        std::thread::Builder::new()
            .name("mini-game-webaudio-dec".into())
            .spawn(move || {
                let r = std::panic::catch_unwind(|| crate::audio_ctx::decode::decode_bytes(bytes, "")).unwrap_or_else(|_| Err((10004, "解码器内部错误".into())));
                let _ = tx.send((job, ctx, r));
            })
            .ok();
        ok(json!({ "job": job }))
    }

    /// `what`：floatTime / byteTime / floatFreq / byteFreq；`n` = JS 数组的长度
    pub(super) fn op_analyser(&mut self, a: &Value) -> Out {
        let id = a.get("node").and_then(Value::as_u64).unwrap_or(0) as u32;
        let what = a.get("what").and_then(Value::as_str).unwrap_or("");
        let len = a.get("n").and_then(Value::as_u64).unwrap_or(0) as usize;
        let Some(n) = self.web.nodes.get(&id) else { return ok(json!({})) };
        let now = self.web.now(n.ctx);
        let n = self.web.nodes.get_mut(&id).unwrap();
        let g = |k: &str, d: f64| n.props.get(k).and_then(Value::as_f64).unwrap_or(d);
        let fft_size = (g("fftSize", 2048.0) as usize).clamp(32, MAX_FFT);
        let (smooth, min_db, max_db) = (g("smoothingTimeConstant", 0.8), g("minDecibels", -100.0), g("maxDecibels", -30.0));
        let Some((sh, spec)) = n.analyser.as_mut() else { return ok(json!({})) };
        let bytes: Vec<u8> = match what {
            "floatTime" | "byteTime" => {
                let x = sh.latest(fft_size);
                let x = &x[..len.min(fft_size)];
                if what == "floatTime" {
                    x.iter().flat_map(|v| v.to_le_bytes()).collect()
                } else {
                    x.iter().map(|&v| byte_of_sample(v)).collect()
                }
            }
            _ => {
                let m = spec.analyse(sh, fft_size, smooth, now);
                let m = &m[..len.min(m.len())];
                if what == "floatFreq" {
                    m.iter().flat_map(|&v| db(v).to_le_bytes()).collect()
                } else {
                    m.iter().map(|&v| byte_of_db(db(v), min_db, max_db)).collect()
                }
            }
        };
        (json!({}).to_string(), Some(bytes))
    }

    /// getFrequencyResponse(freqs) → [幅度 × n, 相位 × n]（f32）。Biquad 取当前时刻的参数值
    pub(super) fn op_freq_response(&mut self, a: &Value, bin: Option<Vec<u8>>) -> Out {
        let id = a.get("node").and_then(Value::as_u64).unwrap_or(0) as u32;
        let Some(n) = self.web.nodes.get(&id) else { return ok(json!({})) };
        let rate = self.web.rate;
        let now = self.web.now(n.ctx);
        let (b, a2): (Vec<f64>, Vec<f64>) = match n.kind {
            Kind::Biquad => {
                let v = |i: usize| { let p = &n.params[i]; (p.tl.value_at(now, p.base) as f32).clamp(p.spec.min, p.spec.max) as f64 };
                let ty = n.props.get("type").and_then(Value::as_str).and_then(|t| FKind::ALL.iter().position(|x| *x == t)).unwrap_or(0);
                let c = coefficients(FKind::from_u8(ty as u8), v(0), v(1), v(2), v(3), rate);
                (vec![c[0], c[1], c[2]], vec![1.0, c[3], c[4]])
            }
            Kind::Iir => {
                let (ff, fb) = n.iir.clone().unwrap_or_default();
                let a0 = fb.first().copied().unwrap_or(1.0);
                (ff.iter().map(|x| x / a0).collect(), fb.iter().map(|x| x / a0).collect())
            }
            _ => return ok(json!({})),
        };
        let freqs: Vec<f32> = bin.unwrap_or_default().chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        let (mut mag, mut ph) = (Vec::new(), Vec::new());
        for &f in &freqs {
            // 规范：频率在 [0, 奈奎斯特] 之外的给 NaN
            if !(0.0..=rate / 2.0).contains(&(f as f64)) {
                mag.push(f32::NAN);
                ph.push(f32::NAN);
                continue;
            }
            let (m, p) = response(&b, &a2, f as f64, rate);
            mag.push(m as f32);
            ph.push(p as f32);
        }
        let out: Vec<u8> = mag.iter().chain(ph.iter()).flat_map(|v| v.to_le_bytes()).collect();
        (json!({}).to_string(), Some(out))
    }
}

/// 节点的尾巴有多长（秒）：上游没了之后还要响多久。0 = 没有尾巴
pub fn tail_time(w: &super::WebAudio, id: u32) -> f64 {
    let Some(n) = w.nodes.get(&id) else { return 0.0 };
    match n.kind {
        Kind::Delay => n.max_delay + 128.0 / w.rate,
        Kind::Iir => 1.0,
        Kind::Biquad => {
            let now = w.now(n.ctx);
            let v = |i: usize| { let p = &n.params[i]; (p.tl.value_at(now, p.base) as f32).clamp(p.spec.min, p.spec.max) as f64 };
            let ty = n.props.get("type").and_then(Value::as_str).and_then(|t| FKind::ALL.iter().position(|x| *x == t)).unwrap_or(0);
            let c = coefficients(FKind::from_u8(ty as u8), v(0), v(1), v(2), v(3), w.rate);
            // z² + a1·z + a2 的根的最大模
            let (a1, a2) = (c[3], c[4]);
            let disc = a1 * a1 - 4.0 * a2;
            let r = if disc < 0.0 { a2.abs().sqrt() } else { ((-a1 + disc.sqrt()) / 2.0).abs().max(((-a1 - disc.sqrt()) / 2.0).abs()) };
            if r <= 0.0 {
                return 128.0 / w.rate;
            }
            if r >= 1.0 {
                return 10.0;
            }
            ((1e-4f64).ln() / r.ln() / w.rate + 128.0 / w.rate).min(10.0)
        }
        _ => 0.0,
    }
}

/// JS 入口：`__game_webaudio(op, json, bytes?)` → `[json, ArrayBuffer | null]`
pub fn register(rt: &crate::js::JsRuntime, audio: std::rc::Rc<std::cell::RefCell<GameAudio>>) -> Result<(), String> {
    use rquickjs::{Array, Value as JsValue};
    rt.register_raw("__game_webaudio", move |ctx, a| {
        let text = |v: Option<&JsValue>| v.and_then(|x| x.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default();
        let op = text(a.0.first());
        let args: Value = serde_json::from_str(&text(a.0.get(1))).unwrap_or(Value::Null);
        let bin = a.0.get(2).filter(|v| !v.is_undefined() && !v.is_null()).and_then(crate::js::binary::bytes_of);
        let (out, bytes) = audio.borrow_mut().web_call(&op, &args, bin);
        let arr = Array::new(ctx.clone())?;
        arr.set(0, out)?;
        match bytes {
            Some(b) => arr.set(1, crate::js::binary::array_buffer(&ctx, b)?)?,
            None => arr.set(1, JsValue::new_null(ctx.clone()))?,
        }
        Ok(arr.into_value())
    })
}
