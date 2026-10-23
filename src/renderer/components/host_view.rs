//! 同层渲染的**通用**原生组件：创建选项 `sameLayer` 里除 `video` 以外的标签（典型是 `<map>`）。
//!
//! ## 为什么要它
//! 宿主把原生控件**盖**在引擎画面上（`nativeComponents`）时，控件与引擎画面走两条上屏路：Android 上引擎
//! 画到自己的 SurfaceView、系统合成器直接取；控件要经事件队列 → 主线程 → 宿主下一帧，事件到得忽早忽晚
//! （一加 PLY110 上晚 2~4 个 vsync）。按 vsync 推算只能把惯性那段推准，拖动时引擎画面自己每帧挪
//! 0 / 2.8 / 0 / 4.3px，推不准 —— 滑动时地图框跟页面错开、一顿一顿（与视频当初同一个病，见
//! `video/host_feed.rs`）。画进页面以后它就是页面画布里的一块像素，滚动时与文字在同一张图里挪。
//!
//! ## 分工
//! - 宿主：按 `sameLayerViews` 事件（`key` / `tag` / `attrs` / 框的设备像素 `box`）把控件画进一块缓冲
//!   （离屏），内容变了才调 [`push_rgba`]（任何线程；FFI `mr_native_view_frame_rgba`，JNI `nativeViewFrameRgba`）；
//!   收 `nativeViewTap`（点在框里的视图本地坐标）自己判点到了什么，事件照旧 `mr_app_native_view_event` 回传。
//! - 引擎：框的背景照常画，最新一帧按框拉满画在上面（圆角照节点的）；新帧到了只重画这一块
//!   （与动图换帧同一套，`wxml_renderer/animated_image.rs`）。滚动一个像素都不用宿主重画。
//!
//! 取舍：拖动从框里开始照样滚页面（与同层渲染的视频一致）；地图自己的平移 / 缩放手势没有转给宿主，
//! 只转点击。键：节点 `id`，没有就是标签名（两个都没写 id 的同名组件共用一路画面）。
use super::base::RenderNode;
use super::host_feed::HostFrame;
use crate::Canvas;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Default)]
struct Feed {
    frame: Option<Arc<HostFrame>>,
    /// 来一帧加一（引擎按它判这个框要不要重画）
    seq: u32,
}

fn feeds() -> &'static Mutex<HashMap<String, Feed>> {
    static F: OnceLock<Mutex<HashMap<String, Feed>>> = OnceLock::new();
    F.get_or_init(|| Mutex::new(HashMap::new()))
}

thread_local! {
    /// 这个引擎（线程）里走同层渲染的标签（不含 video）。thread_local 的理由同 `host_feed::ON`：
    /// 渲染器的绘制函数拿不到引擎选项
    static TAGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// 引擎线程上攒的点击（`nativeViewTap`），`MiniEngine` 每帧取走
    static TAPS: RefCell<Vec<serde_json::Value>> = const { RefCell::new(Vec::new()) };
    /// 换了渲染器：下一帧重新数一遍页面上的同层组件
    static STALE: Cell<bool> = const { Cell::new(false) };
}

/// `MiniEngine` 换页重建渲染器时设（`sameLayer` 去掉 video）
pub fn set_tags(tags: Vec<String>) {
    TAGS.with(|t| *t.borrow_mut() = tags);
    STALE.with(|s| s.set(true));
}

pub fn tags() -> Vec<String> {
    TAGS.with(|t| t.borrow().clone())
}

pub fn is_same_layer(tag: &str) -> bool {
    TAGS.with(|t| t.borrow().iter().any(|x| x == tag))
}

/// 换了渲染器之后第一次问返回 true（只返回一次）
pub fn take_stale() -> bool {
    STALE.with(|s| s.replace(false))
}

/// 节点的键：`id`，没有就是标签名
pub fn key(tag: &str, attrs: &HashMap<String, String>) -> String {
    match attrs.get("id").map(|s| s.trim()).filter(|s| !s.is_empty()) {
        Some(id) => id.to_string(),
        None => tag.to_string(),
    }
}

/// 画面的版本号。这个标签走同层渲染时总是 Some（还没来过帧就是 0）
pub fn seq(tag: &str, key: &str) -> Option<u32> {
    if !is_same_layer(tag) {
        return None;
    }
    Some(feeds().lock().ok().and_then(|m| m.get(key).map(|f| f.seq)).unwrap_or(0))
}

/// 同上，只按键问（动图登记里只存了键；登记时已确认过标签）
pub fn seq_of(key: &str) -> Option<u32> {
    if TAGS.with(|t| t.borrow().is_empty()) {
        return None;
    }
    Some(feeds().lock().ok().and_then(|m| m.get(key).map(|f| f.seq)).unwrap_or(0))
}

pub fn frame(key: &str) -> Option<Arc<HostFrame>> {
    feeds().lock().ok()?.get(key).and_then(|f| f.frame.clone())
}

/// 宿主画好的一帧（RGBA，`stride` 字节一行）。任何线程都可以调。返回是否收下
pub fn push_rgba(key: &str, w: u32, h: u32, stride: usize, data: &[u8]) -> bool {
    let row = w as usize * 4;
    if key.is_empty() || w == 0 || h == 0 || stride < row || data.len() < stride * (h as usize - 1) + row {
        return false;
    }
    let mut rgba = Vec::with_capacity(row * h as usize);
    for y in 0..h as usize {
        rgba.extend_from_slice(&data[y * stride..y * stride + row]);
    }
    let Ok(mut m) = feeds().lock() else { return false };
    let f = m.entry(key.to_string()).or_default();
    f.frame = Some(Arc::new(HostFrame { rgba, width: w, height: h }));
    f.seq = f.seq.wrapping_add(1);
    true
}

/// 页面上已经没有的：帧一起丢掉（一帧几 MB）
pub fn retain(keys: &[String]) {
    if let Ok(mut m) = feeds().lock() {
        m.retain(|k, _| keys.iter().any(|x| x == k));
    }
}

/// 点在同层组件的框里（视图本地坐标，逻辑像素）
pub fn queue_tap(key: &str, tag: &str, x: f32, y: f32) {
    let ev = serde_json::json!({ "type": "nativeViewTap", "key": key, "tag": tag, "x": x, "y": y });
    TAPS.with(|t| t.borrow_mut().push(ev));
}

pub fn take_taps() -> Vec<serde_json::Value> {
    TAPS.with(|t| std::mem::take(&mut *t.borrow_mut()))
}

/// 框的背景已经画好之后调：宿主交来的最新一帧按框拉满画上去（圆角照节点的）。没来过帧就只有背景
pub fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32) {
    let Some(f) = frame(&key(&node.tag, &node.attrs)) else { return };
    let radius = node.style.border_radius;
    // 宿主按 `box` 画、尺寸刚好：整行拷贝；框刚变、宿主还没按新尺寸重画：逐像素缩放
    if !canvas.blit_image_exact(&f.rgba, f.width, f.height, x, y, w, h, "scaleToFill", radius) {
        canvas.draw_image(&f.rgba, f.width, f.height, x, y, w, h, "scaleToFill", radius);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 只认登记了的标签_版本号随帧走() {
        set_tags(vec!["map".into()]);
        assert!(take_stale() && !take_stale(), "换了标签只报一次");
        assert_eq!(seq("view", "m"), None);
        assert_eq!(seq("map", "hv-test-a"), Some(0), "还没来过帧也是 Some(0)：第一帧到了才知道要重画");
        let px = [255u8; 2 * 2 * 4 + 8];
        assert!(push_rgba("hv-test-a", 2, 2, 12, &px), "带行尾填充的 stride");
        assert!(!push_rgba("hv-test-a", 2, 2, 4, &px), "stride 比一行还短");
        assert_eq!(seq("map", "hv-test-a"), Some(1));
        let f = frame("hv-test-a").unwrap();
        assert_eq!((f.width, f.height, f.rgba.len()), (2, 2, 16), "去掉了行尾填充");
        let mut a = HashMap::new();
        assert_eq!(key("map", &a), "map");
        a.insert("id".into(), " map1 ".into());
        assert_eq!(key("map", &a), "map1");
        set_tags(vec![]);
        assert_eq!(seq("map", "hv-test-a"), None, "关掉之后不算");
    }
}
