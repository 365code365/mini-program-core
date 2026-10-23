//! 同层渲染的 `<video>`：宿主的系统播放器解码，**帧交给引擎画进页面**（创建选项 `sameLayer` 含 `video`）。
//!
//! ## 为什么不再让宿主把控件盖在画面上
//! 盖上去的控件与引擎画面走两条上屏路（Android 上引擎画到自己的 SurfaceView，控件要经事件、主线程、
//! 宿主的下一帧），两边每帧差几个 vsync 不固定：真机上手指一拖视频框就跟页面错开、一顿一顿，
//! 按 vsync 推算也只能把惯性那段推准（拖动时引擎画面自己每帧挪 0 / 2.8 / 0 / 4.3px，推不准）。
//! 画进页面以后视频就是页面画布里的一块像素，滚动时与文字、卡片在同一张图里挪 —— 微信同层渲染的结果。
//!
//! ## 分工
//! - 宿主：按 `videos` 事件建 / 换 / 放播放器，解出的帧调 [`push_yuv`] / [`push_rgba`]（任何线程都行），
//!   播放器的状态变化照旧当组件事件回传（`play` / `pause` / `timeupdate` / `ended` …，`mr_app_native_view_event`），
//!   执行引擎发来的 `videoCommand`（点了画面 / 控制条、`wx.createVideoContext`）。
//! - 引擎：把最新一帧按 `object-fit` 画进节点的框，画封面、中间的播放键、底部控制条（与内置播放器同一套，
//!   `chrome.rs` / `ui.rs`）、全屏那一层（`fullscreen.rs`）；新帧或状态变了只重画这个框
//!   （走动图那套「换帧就重画那一块」，`wxml_renderer/animated_image.rs`，版本号见 `ui::mark_seq`）。
//!
//! 键：节点的 `id`，没有就用 `src`（与引擎自己的播放器一样按 src 认；两个没写 id、src 又相同的视频共用一路画面）。
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// 一帧画面（RGBA，紧密排列）
pub struct HostFrame {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// 引擎画控制条要的播放状态（宿主回传的事件里来）
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HostVideoState {
    pub playing: bool,
    /// 起播过（封面只在没起播时盖着）
    pub started: bool,
    pub ended: bool,
    /// 秒
    pub current: f64,
    pub duration: f64,
    /// 画面尺寸（`loadedmetadata`；全屏不给方向时按它判断横竖）
    pub width: u32,
    pub height: u32,
}

#[derive(Default)]
struct Feed {
    frame: Option<Arc<HostFrame>>,
    /// 画面或状态变一次加一（引擎按它判断这个框要不要重画）
    seq: u32,
    state: HostVideoState,
    /// 上一次画它的框（设备像素）：大帧先按它降采样再转 RGBA，省掉一大半转换与缩放
    box_px: (u32, u32),
}

fn feeds() -> &'static Mutex<HashMap<String, Feed>> {
    static F: OnceLock<Mutex<HashMap<String, Feed>>> = OnceLock::new();
    F.get_or_init(|| Mutex::new(HashMap::new()))
}

thread_local! {
    /// 这个引擎（线程）里 `<video>` 走同层渲染。thread_local 的理由同 `HOST_OWNS_VIDEO`
    static ON: Cell<bool> = const { Cell::new(false) };
    /// 引擎线程上攒的、要发给宿主的播放命令（点击 / VideoContext），`MiniEngine` 每帧取走
    static COMMANDS: RefCell<Vec<serde_json::Value>> = const { RefCell::new(Vec::new()) };
}

/// `MiniEngine` 换页重建渲染器时设
pub fn set_host_feeds_video(on: bool) {
    ON.with(|c| c.set(on));
}

pub fn host_feeds_video() -> bool {
    ON.with(|c| c.get())
}

/// 节点的键：`id`，没有就是 `src`
pub fn video_key(attrs: &HashMap<String, String>) -> String {
    match attrs.get("id").map(|s| s.trim()).filter(|s| !s.is_empty()) {
        Some(id) => id.to_string(),
        None => attrs.get("src").map(|s| s.trim().to_string()).unwrap_or_default(),
    }
}

/// 当前一帧
pub fn frame(key: &str) -> Option<Arc<HostFrame>> {
    feeds().lock().ok()?.get(key).and_then(|f| f.frame.clone())
}

pub fn state(key: &str) -> HostVideoState {
    feeds().lock().ok().and_then(|m| m.get(key).map(|f| f.state)).unwrap_or_default()
}

/// 画面 / 状态的版本号。同层渲染开着时总是 Some（还没来过帧就是 0）：
/// 第一帧到了版本号变成 1，引擎才知道要重画这个框
pub fn seq(key: &str) -> Option<u32> {
    if !host_feeds_video() {
        return None;
    }
    Some(feeds().lock().ok().and_then(|m| m.get(key).map(|f| f.seq)).unwrap_or(0))
}

/// 绘制时记下框的设备像素尺寸（降采样用）
pub fn note_box(key: &str, w: f32, h: f32) {
    if let Ok(mut m) = feeds().lock() {
        m.entry(key.to_string()).or_default().box_px = (w.max(1.0) as u32, h.max(1.0) as u32);
    }
}

fn store(key: &str, frame: HostFrame) {
    if let Ok(mut m) = feeds().lock() {
        let f = m.entry(key.to_string()).or_default();
        f.frame = Some(Arc::new(frame));
        f.seq = f.seq.wrapping_add(1);
    }
}

fn box_of(key: &str) -> (u32, u32) {
    feeds().lock().ok().and_then(|m| m.get(key).map(|f| f.box_px)).unwrap_or((0, 0))
}

/// 降采样的整数步长：帧比框大一倍以上才降（降完仍不小于框，留给绘制时的缩放去做最后一步）
fn step_for(w: u32, h: u32, (bw, bh): (u32, u32)) -> u32 {
    if bw == 0 || bh == 0 {
        return 1;
    }
    (w / bw).min(h / bh).max(1)
}

/// 宿主解出的一帧（RGBA，`stride` 字节一行）。任何线程都可以调。返回是否收下
pub fn push_rgba(key: &str, w: u32, h: u32, stride: usize, data: &[u8]) -> bool {
    let row = w as usize * 4;
    if w == 0 || h == 0 || stride < row || data.len() < stride * (h as usize - 1) + row {
        return false;
    }
    let mut rgba = Vec::with_capacity(row * h as usize);
    for y in 0..h as usize {
        rgba.extend_from_slice(&data[y * stride..y * stride + row]);
    }
    store(key, HostFrame { rgba, width: w, height: h });
    true
}

/// YUV 4:2:0 的三个平面（Android `ImageReader` 的 `YUV_420_888`：U / V 可能交错，`uv_pixel_stride` 为 2）
pub struct Yuv420<'a> {
    pub width: u32,
    pub height: u32,
    pub y: &'a [u8],
    pub y_stride: usize,
    pub u: &'a [u8],
    pub v: &'a [u8],
    pub uv_stride: usize,
    pub uv_pixel_stride: usize,
}

/// 宿主解出的一帧（YUV 4:2:0）。转 RGBA 在调用线程上做（宿主的解码线程，不占引擎线程）。返回是否收下
pub fn push_yuv(key: &str, f: &Yuv420) -> bool {
    let Some(frame) = yuv_to_rgba(f, step_for(f.width, f.height, box_of(key))) else { return false };
    store(key, frame);
    true
}

/// YUV 4:2:0（视频范围 16~235）→ RGBA。高清（高 ≥ 720）按 BT.709，标清按 BT.601 ——
/// 系统解码器不报矩阵时播放器的通行做法。`step` > 1 时每隔 step 个像素取一个（先降采样再转换）
pub fn yuv_to_rgba(f: &Yuv420, step: u32) -> Option<HostFrame> {
    let (w, h) = (f.width as usize, f.height as usize);
    let ps = f.uv_pixel_stride.max(1);
    let (cw, ch) = (w.div_ceil(2), h.div_ceil(2));
    let need = |len: usize, stride: usize, rows: usize, last: usize| rows > 0 && len >= stride * (rows - 1) + last;
    if w == 0
        || h == 0
        || !need(f.y.len(), f.y_stride, h, w)
        || !need(f.u.len(), f.uv_stride, ch, (cw - 1) * ps + 1)
        || !need(f.v.len(), f.uv_stride, ch, (cw - 1) * ps + 1)
    {
        return None;
    }
    let step = step.max(1) as usize;
    let (ow, oh) = (w.div_ceil(step), h.div_ceil(step));
    // 定点系数（×1024）：R = Y' + rv·V，G = Y' − gu·U − gv·V，B = Y' + bu·U，Y' = 1.164·(Y−16)
    let (rv, gu, gv, bu) = if h >= 720 { (1836, 218, 546, 2163) } else { (1634, 401, 832, 2066) };
    let mut out = vec![0u8; ow * oh * 4];
    for oy in 0..oh {
        let y = oy * step;
        let (yrow, crow) = (y * f.y_stride, (y / 2) * f.uv_stride);
        let line = &mut out[oy * ow * 4..(oy + 1) * ow * 4];
        for ox in 0..ow {
            let x = ox * step;
            let luma = (f.y[yrow + x] as i32 - 16).max(0) * 1192;
            let ci = crow + (x / 2) * ps;
            let (u, v) = (f.u[ci] as i32 - 128, f.v[ci] as i32 - 128);
            let clamp = |c: i32| ((c + 512) >> 10).clamp(0, 255) as u8;
            let p = &mut line[ox * 4..ox * 4 + 4];
            p[0] = clamp(luma + rv * v);
            p[1] = clamp(luma - gu * u - gv * v);
            p[2] = clamp(luma + bu * u);
            p[3] = 255;
        }
    }
    Some(HostFrame { rgba: out, width: ow as u32, height: oh as u32 })
}

/// 宿主回传的组件事件：按它更新引擎画控制条用的状态。返回画面上的东西变没变（变了就重画这个框）
pub fn note_event(key: &str, name: &str, detail: &serde_json::Value) -> bool {
    let num = |k: &str| detail.get(k).and_then(|v| v.as_f64());
    let Ok(mut m) = feeds().lock() else { return false };
    let f = m.entry(key.to_string()).or_default();
    let before = f.state;
    let s = &mut f.state;
    match name {
        "play" => {
            s.playing = true;
            s.started = true;
            s.ended = false;
        }
        "pause" => s.playing = false,
        "ended" => {
            s.playing = false;
            s.ended = true;
        }
        "timeupdate" => {
            if let Some(t) = num("currentTime") {
                s.current = t;
            }
            if let Some(d) = num("duration") {
                s.duration = d;
            }
        }
        "loadedmetadata" => {
            if let Some(d) = num("duration") {
                s.duration = d;
            }
            s.width = num("width").unwrap_or(0.0).max(0.0) as u32;
            s.height = num("height").unwrap_or(0.0).max(0.0) as u32;
        }
        "seekcomplete" => {
            if let Some(p) = num("position") {
                s.current = p;
                s.ended = false;
            }
        }
        _ => {}
    }
    let changed = f.state != before;
    if changed {
        f.seq = f.seq.wrapping_add(1);
    }
    changed
}

/// 页面上已经没有的视频：帧与状态一起丢掉（一帧整屏 RGBA 好几 MB）
pub fn retain(keys: &[String]) {
    if let Ok(mut m) = feeds().lock() {
        m.retain(|k, _| keys.iter().any(|x| x == k));
    }
}

/// 攒一条发给宿主的播放命令（`{key, command, position?, rate?}`）
pub fn queue_command(cmd: serde_json::Value) {
    COMMANDS.with(|c| c.borrow_mut().push(cmd));
}

/// 取走攒下的播放命令
pub fn take_commands() -> Vec<serde_json::Value> {
    COMMANDS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane(w: usize, h: usize, v: u8) -> Vec<u8> {
        vec![v; w * h]
    }

    #[test]
    fn 视频范围的黑白与红绿蓝() {
        // 4×2 的一帧，左半 / 右半两种颜色（色度 2×1 个样本）
        let y = [16u8, 16, 235, 235, 16, 16, 235, 235];
        let (u, v) = ([128u8, 128], [128u8, 128]);
        let f = Yuv420 { width: 4, height: 2, y: &y, y_stride: 4, u: &u, v: &v, uv_stride: 2, uv_pixel_stride: 1 };
        let o = yuv_to_rgba(&f, 1).unwrap();
        assert_eq!(&o.rgba[0..4], &[0, 0, 0, 255], "Y=16 是黑");
        assert_eq!(&o.rgba[8..12], &[255, 255, 255, 255], "Y=235 是白");
        // BT.601 的红：Y=81 U=90 V=240
        let (y, u, v) = (plane(2, 2, 81), [90u8], [240u8]);
        let f = Yuv420 { width: 2, height: 2, y: &y, y_stride: 2, u: &u, v: &v, uv_stride: 1, uv_pixel_stride: 1 };
        let p = &yuv_to_rgba(&f, 1).unwrap().rgba[0..3];
        assert!(p[0] > 250 && p[1] < 5 && p[2] < 5, "红 {p:?}");
    }

    #[test]
    fn 交错的色度平面_与降采样() {
        // Android 常见：U / V 交错（pixel stride 2），行宽带对齐填充
        let (w, h, ys, cs) = (8usize, 4usize, 16usize, 16usize);
        let y = vec![126u8; ys * h];
        let mut uv = vec![0u8; cs * (h / 2)];
        for r in 0..h / 2 {
            for c in 0..w / 2 {
                uv[r * cs + c * 2] = 128;
                uv[r * cs + c * 2 + 1] = 128;
            }
        }
        let f = Yuv420 { width: w as u32, height: h as u32, y: &y, y_stride: ys, u: &uv, v: &uv[1..], uv_stride: cs, uv_pixel_stride: 2 };
        let o = yuv_to_rgba(&f, 2).unwrap();
        assert_eq!((o.width, o.height), (4, 2), "隔一个取一个");
        assert!(o.rgba.chunks(4).all(|p| p[0] == p[1] && p[1] == p[2]), "灰色：三个通道相等");
        // 平面短了（宿主给错了步长）：不收，不越界
        let short = Yuv420 { y: &y[..10], ..f };
        assert!(yuv_to_rgba(&short, 1).is_none());
    }

    #[test]
    fn 事件更新状态_变了才加版本号() {
        let k = "__host_feed_test";
        set_host_feeds_video(true);
        let s0 = seq(k).unwrap();
        assert!(note_event(k, "play", &serde_json::json!({})));
        assert!(!note_event(k, "play", &serde_json::json!({})), "已经在播：不算变化");
        assert!(note_event(k, "timeupdate", &serde_json::json!({"currentTime": 1.5, "duration": 10})));
        let st = state(k);
        assert!(st.playing && st.started && st.current == 1.5 && st.duration == 10.0);
        assert_eq!(seq(k).unwrap(), s0 + 2);
        assert!(push_rgba(k, 2, 1, 8, &[1, 2, 3, 255, 4, 5, 6, 255]));
        assert_eq!(seq(k).unwrap(), s0 + 3);
        retain(&[]);
        assert!(frame(k).is_none());
        set_host_feeds_video(false);
        assert!(seq(k).is_none(), "没开同层渲染：不报版本号（节点不算动图）");
    }
}
