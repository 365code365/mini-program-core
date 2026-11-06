//! `<camera>` / `<live-pusher>` / `<voip-room>` 在引擎这一侧的事（官方 component/camera、live-pusher、voip-room）。
//!
//! 三者都走**通用同层渲染**（创建选项 `sameLayer` 里点名，`components::host_view`）：宿主开相机、按框裁好画面交 RGBA，
//! 引擎画进页面；`bindinitdone` / `binderror` / `bindstop` / `bindstatechange` 由宿主经 `nativeViewEvent` 回传；
//! `CameraContext.*` / `LivePusherContext.*` 走 hostCall（宿主按 contextClass 找实例）。这里只管与平台无关、引擎手里就有的三件：
//!
//! - **onCameraFrame**（`__sameLayerCamera` 时）：宿主交来的预览画面就在引擎手里，按 `frame-size` 缩好、
//!   经 `bin_stash`（种类 3）直通成 `ArrayBuffer` 派给监听（`__cameraFrameArrived`），不经宿主、不过 base64。
//!   官方 tip：帧尺寸「由系统决定」、与组件展示的图像不同 —— 这里取预览画面按长边缩（small 360 / medium 640 / large 1280，不放大）；
//! - **扫码**（`mode="scanCode"`）：在预览画面上解码（二维码与各种条形码，rxing：ZXing 的纯 Rust 移植），后台线程解、同一时刻只解一张、至少隔 [`SCAN_GAP_MS`]；
//!   解出来派 `bindscancode`，同一个结果 [`SCAN_REPEAT_MS`] 内不重复发；
//! - **voip-room**：房间要 `wx.joinVoIPChat`（微信后台）。宿主没在 `hostApis` 里接 `joinVoIPChat` 时进不了房：`mode="video"`
//!   （别人的画面）派一次 `binderror`；接了的（宿主自己的通话后台，SDK 的 `MiniVoip`）不派，别人那一格的画面由宿主交。
//!   `mode="camera"`（自己）由宿主当相机预览画。Skyline 本身不渲染 voip-room（官方「渲染框架支持情况：WebView」）。
//!
//! 页面上有没有这几个组件按布局代数缓存（换了布局才重新查一遍），没有相机的页面每帧只多一次比较。
use super::engine::MiniEngine;
use crate::renderer::components::host_view;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::mpsc::{Receiver, TryRecvError};

/// 两次解码之间至少隔这么久（扫码不需要每帧解：一张 1000 像素宽的灰度图要解十几到几十毫秒，在后台线程上）
const SCAN_GAP_MS: u64 = 250;
/// 同一个结果在这段时间内只发一次（镜头一直对着同一个码时不刷屏）
const SCAN_REPEAT_MS: u64 = 2000;
/// 解码前把画面缩到这个长边以内
const SCAN_MAX_SIDE: u32 = 960;
/// 相机帧（bin_stash 的种类）
pub const BIN_CAMERA: u8 = 3;

struct Cam {
    key: String,
    component_id: String,
    scan: bool,
    frame_side: u32,
}

#[derive(Default)]
struct State {
    generation: Option<u64>,
    cams: Vec<Cam>,
    /// voip-room（mode=video）：(key, component_id)
    voips: Vec<(String, String)>,
    /// 已经派过 binderror 的 voip-room
    voip_errored: HashSet<String>,
    listening: bool,
    frame_seq: Option<u32>,
    scan_seq: Option<u32>,
    scan_at: u64,
    scanning: Option<Receiver<Option<(String, Vec<u8>, String)>>>,
    last_result: Option<(String, u64)>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// `frame-size` → 帧的长边
fn frame_side(attrs: &std::collections::HashMap<String, String>) -> u32 {
    match attrs.get("frame-size").map(|s| s.trim()) {
        Some("small") => 360,
        Some("large") => 1280,
        _ => 640,
    }
}

/// 最近邻缩放（RGBA）：帧给逻辑层做识别 / 取色用，不追求滤波
pub fn scale_rgba(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    let mut out = vec![0u8; (dw * dh * 4) as usize];
    for y in 0..dh {
        let sy = (y as u64 * sh as u64 / dh as u64) as u32;
        let row = (sy * sw) as usize * 4;
        for x in 0..dw {
            let sx = (x as u64 * sw as u64 / dw as u64) as usize;
            let s = row + sx * 4;
            let d = ((y * dw + x) * 4) as usize;
            out[d..d + 4].copy_from_slice(&src[s..s + 4]);
        }
    }
    out
}

/// 长边缩到 `side` 以内（不放大）
pub fn fit(w: u32, h: u32, side: u32) -> (u32, u32) {
    let long = w.max(h);
    if long <= side || long == 0 {
        return (w, h);
    }
    ((w as u64 * side as u64 / long as u64).max(1) as u32, (h as u64 * side as u64 / long as u64).max(1) as u32)
}

/// 在一张 RGBA 上找码：二维码、条形码（EAN-13 / EAN-8 / UPC / Code 128 / 39 / 93 / Codabar / ITF）、
/// Data Matrix、PDF417、Aztec 都认（rxing，ZXing 的移植）。返回 (文字, 原始字节, 码制名)。后台线程上跑。
/// 码制名与 wx.scanCode 的 `scanType` 同一套写法（`QR_CODE`、`EAN_13`、`CODE_128` …）
pub fn decode_code(rgba: &[u8], w: u32, h: u32) -> Option<(String, Vec<u8>, String)> {
    let (dw, dh) = fit(w, h, SCAN_MAX_SIDE);
    let small;
    let (px, pw, ph) = if (dw, dh) != (w, h) {
        small = scale_rgba(rgba, w, h, dw, dh);
        (&small[..], dw, dh)
    } else {
        (rgba, w, h)
    };
    // 整数近似的亮度（ITU-R BT.601）
    let luma: Vec<u8> = px.chunks_exact(4).map(|p| ((p[0] as u32 * 77 + p[1] as u32 * 150 + p[2] as u32 * 29) >> 8) as u8).collect();
    let r = rxing::helpers::detect_in_luma(luma, pw, ph, None).ok()?;
    let text = r.getText().to_string();
    let raw = if r.getRawBytes().is_empty() { text.as_bytes().to_vec() } else { r.getRawBytes().to_vec() };
    Some((text, raw, format!("{:?}", r.getBarcodeFormat())))
}

impl MiniEngine {
    /// 每帧一次（`tick_scroll_views`，挨着直播的命令）
    pub(super) fn tick_cameras(&mut self) {
        let reqs = self.app.take_host_requests("cameraFrame");
        for req in &reqs {
            let on = req.get("op").and_then(Value::as_str) == Some("start");
            let seq = req.get("seq").and_then(Value::as_u64).unwrap_or(0);
            let ok = host_view::is_same_layer("camera");
            STATE.with(|s| {
                let mut s = s.borrow_mut();
                s.listening = on && ok;
                s.frame_seq = None;
            });
            let res = if ok {
                json!({ "errMsg": format!("{}:ok", if on { "start" } else { "stop" }) })
            } else {
                json!({ "errMsg": "start:fail 当前宿主没有接相机" })
            };
            self.app.eval(&format!("__cameraReply({seq}, {ok}, {res})")).ok();
        }
        let watch = ["camera", "voip-room"].iter().any(|t| host_view::is_same_layer(t));
        if !watch && reqs.is_empty() {
            return;
        }
        let Some(r) = self.renderer.take() else { return };
        self.refresh_cameras(&r);
        let now = self.mono_ms();
        let mut events: Vec<(String, &'static str, Value)> = Vec::new();
        let mut frame: Option<(u32, u32)> = None;
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            let Some(cam) = s.cams.first() else { return };
            let (key, cid, scan, side) = (cam.key.clone(), cam.component_id.clone(), cam.scan, cam.frame_side);
            let seq = host_view::seq("camera", &key);
            if std::env::var("MINI_CAMERA_LOG").is_ok() {
                eprintln!("📷 tick listening={} seq={seq:?} last={:?} scan={scan}", s.listening, s.frame_seq);
            }
            if s.listening && seq.is_some() && seq != s.frame_seq {
                s.frame_seq = seq;
                if let Some(f) = host_view::frame(&key) {
                    let (w, h) = fit(f.width, f.height, side);
                    let px = if (w, h) == (f.width, f.height) { f.rgba.clone() } else { scale_rgba(&f.rgba, f.width, f.height, w, h) };
                    // 只留最新一帧：逻辑层慢了就丢帧，不攒
                    crate::net::bin_stash::clear(BIN_CAMERA, 0);
                    crate::net::bin_stash::stash(BIN_CAMERA, 0, px);
                    frame = Some((w, h));
                }
            }
            if scan {
                if let Some(rx) = &s.scanning {
                    match rx.try_recv() {
                        Ok(hit) => {
                            s.scanning = None;
                            if let Some((text, raw, kind)) = hit {
                                let repeat = s.last_result.as_ref().is_some_and(|(t, at)| *t == text && now.saturating_sub(*at) < SCAN_REPEAT_MS);
                                if !repeat {
                                    s.last_result = Some((text.clone(), now));
                                    events.push((cid.clone(), "scancode", scancode_detail(&text, &raw, &kind)));
                                }
                            }
                        }
                        Err(TryRecvError::Disconnected) => s.scanning = None,
                        Err(TryRecvError::Empty) => {}
                    }
                }
                if s.scanning.is_none() && seq.is_some() && seq != s.scan_seq && now.saturating_sub(s.scan_at) >= SCAN_GAP_MS {
                    if let Some(f) = host_view::frame(&key) {
                        s.scan_seq = seq;
                        s.scan_at = now;
                        let (tx, rx) = std::sync::mpsc::channel();
                        s.scanning = Some(rx);
                        std::thread::Builder::new()
                            .name("mini-qr".into())
                            .spawn(move || {
                                tx.send(decode_code(&f.rgba, f.width, f.height)).ok();
                            })
                            .ok();
                    }
                }
            }
        });
        // 宿主接了通话后台（hostApis 里有 joinVoIPChat）：进房与否由宿主管，这里不替它报错
        let host_voip = self.opts.host_apis.iter().any(|a| a == "joinVoIPChat");
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            let voips = if host_voip { Vec::new() } else { s.voips.clone() };
            for (key, cid) in voips {
                if s.voip_errored.insert(key) {
                    events.push((cid, "error", json!({
                        "errCode": -1,
                        "errMsg": "voip-room:fail 没有加入音视频通话（wx.joinVoIPChat 要微信后台，宿主里进不了房）",
                    })));
                }
            }
        });
        let mut any = false;
        for (cid, name, detail) in events {
            any |= super::own_events::dispatch_own(&mut self.app, &r, &cid, name, detail, now);
        }
        self.renderer = Some(r);
        if let Some((w, h)) = frame {
            self.app.eval(&format!("__cameraFrameArrived({w}, {h})")).ok();
            any = true;
        }
        if any || !reqs.is_empty() {
            self.app.pump_jobs();
            if self.app.take_data_dirty() {
                self.page_data_dirty = true;
            }
        }
    }

    /// 换了布局才重新数页面上的相机与 voip-room
    fn refresh_cameras(&self, r: &crate::renderer::WxmlRenderer) {
        let generation = r.layout_generation();
        if STATE.with(|s| s.borrow().generation == Some(generation)) {
            return;
        }
        let cams: Vec<Cam> = if host_view::is_same_layer("camera") {
            r.query_selector("camera", true, &self.interaction)
                .into_iter()
                .map(|h| Cam {
                    key: host_view::key("camera", &h.attrs),
                    scan: h.attrs.get("mode").map(|m| m.trim() == "scanCode").unwrap_or(false),
                    frame_side: frame_side(&h.attrs),
                    component_id: h.component_id,
                })
                .collect()
        } else {
            Vec::new()
        };
        let voips: Vec<(String, String)> = r
            .query_selector("voip-room", true, &self.interaction)
            .into_iter()
            .filter(|h| h.attrs.get("mode").map(|m| m.trim() == "video").unwrap_or(false))
            .map(|h| (host_view::key("voip-room", &h.attrs), h.component_id))
            .collect();
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.generation = Some(generation);
            s.cams = cams;
            let keys: HashSet<String> = voips.iter().map(|(k, _)| k.clone()).collect();
            s.voip_errored.retain(|k| keys.contains(k));
            s.voips = voips;
            // 监听不跟着清：第一次布局之前就可能已经 start 了（onLoad 里），换页由 reset() 清
            if s.cams.is_empty() {
                s.scanning = None;
            }
        });
    }
}

/// bindscancode 的 detail：与 wx.scanCode 的成功结果同一组字段（官方 camera 文档没列 detail，按扫码接口给；待验）
fn scancode_detail(text: &str, raw: &[u8], kind: &str) -> Value {
    use base64::Engine as _;
    json!({
        "type": kind,
        "result": text,
        "charSet": "utf-8",
        "rawData": base64::engine::general_purpose::STANDARD.encode(raw),
    })
}

/// 换页：监听与扫码状态清掉（下一页的相机重新开始）
pub(super) fn reset() {
    STATE.with(|s| *s.borrow_mut() = State::default());
    crate::net::bin_stash::clear(BIN_CAMERA, 0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 帧按长边缩_不放大() {
        assert_eq!(fit(1270, 900, 640), (640, 453));
        assert_eq!(fit(300, 200, 640), (300, 200));
        let src: Vec<u8> = (0..4 * 4 * 4).map(|i| i as u8).collect();
        let d = scale_rgba(&src, 4, 4, 2, 2);
        assert_eq!(&d[0..4], &src[0..4]);
        assert_eq!(&d[12..16], &src[(2 * 4 + 2) * 4..(2 * 4 + 2) * 4 + 4], "右下取源 (2,2)");
    }
}
