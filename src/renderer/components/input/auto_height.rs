//! textarea 的 `auto-height`（随行数长高）与 `bindlinechange`。
//!
//! 官方（component/textarea.html）：`auto-height` 「是否自动增高，设置 auto-height 时，style.height 不生效」；
//! `bind:linechange` 「输入框行数变化时调用，event.detail = {height: 0, heightRpx: 0, lineCount: 0}」。
//!
//! 为什么是「画出来的行数驱动下一次布局」而不是在布局里量：折行只有绘制端知道（`layout_textarea_lines`
//! 要量字宽、按内容区宽折，布局时 taffy 还没给出宽度），而且打字不经 setData 时节点树里的 value 是旧的，
//! 布局量不到用户刚打的字。所以：建树按上一次画出来的行数定高（第一次按 value 里的换行数），
//! 绘制时行数变了就记下新行数、请宿主整树重排一次（`take_relayout`，与换字体同一条路）。
//! 代价：只有行数变的那一下多一次重排，打字不换行时不重排；首帧按换行数估，长初值多一帧校正。
//!
//! 行数按「稳定键」记（`id` 属性 > wx:key 身份 > `name` > 占位符）：组件 id 里含坐标，长高之后坐标就变了。
//! 同一页两个都没写 id / name、占位符又相同的 auto-height 会共用一个行数（已知取舍）。
use super::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

/// 建树时用的行数写在节点上（绘制端据此判断「画出来的行数与布局时的不同」）
pub const AUTO_HEIGHT_LINES: &str = "__ah_lines";

type Events = Vec<(String, &'static str, serde_json::Value)>;

thread_local! {
    /// 稳定键 → 上一次画出来的行数（auto-height 的定高依据）
    static LINES: RefCell<HashMap<String, usize>> = RefCell::new(HashMap::new());
    static RELAYOUT: Cell<bool> = const { Cell::new(false) };
    static EVENTS: RefCell<Events> = const { RefCell::new(Vec::new()) };
}

fn key_of(attrs: &HashMap<String, String>) -> String {
    let get = |k: &str| attrs.get(k).filter(|s| !s.is_empty());
    if let Some(id) = get("id") {
        return format!("#{id}");
    }
    if let Some(k) = get(crate::parser::template::KEYED_ID_ATTR) {
        return format!("k{k}");
    }
    if let Some(n) = get("name") {
        return format!("n{n}");
    }
    format!("p{}", attrs.get("placeholder").map(String::as_str).unwrap_or(""))
}

pub fn is_auto_height(attrs: &HashMap<String, String>) -> bool {
    attrs.get("auto-height").is_some_and(|s| crate::renderer::components::attr_is_true(s))
}

/// 建树：这一次按几行定高（上一次画出来的，没有就按 value 里的换行数）
pub(super) fn lines_for_build(attrs: &HashMap<String, String>, value: &str) -> usize {
    let key = key_of(attrs);
    LINES.with(|m| m.borrow().get(&key).copied()).unwrap_or_else(|| value.matches('\n').count() + 1).max(1)
}

/// 绘制之后（`wxml_renderer/interactive.rs` 调）：`prev` 是这个 textarea 上一次画出来的行数（没画过为 None），
/// `lines` 是这一次的。行数变了发 `linechange`；auto-height 的与布局时不同就请求重排。
/// `line_h` 物理 px，`sf` 缩放，`screen_w` 逻辑宽（heightRpx 换算）
pub fn after_draw(node: &RenderNode, component_id: &str, prev: Option<usize>, lines: usize, line_h: f32, sf: f32, screen_w: f32) {
    let lines = lines.max(1);
    if prev.is_some_and(|p| p != lines) && node.events.iter().any(|e| e.event_type == "linechange") {
        // height：内容的高（行数 × 行高，逻辑 px），与 WebView 版一致不含内边距（待真机核对）
        let height = (lines as f32 * line_h / sf * 100.0).round() / 100.0;
        let rpx = if screen_w > 0.0 { (height * 750.0 / screen_w * 100.0).round() / 100.0 } else { height * 2.0 };
        let detail = serde_json::json!({ "height": height, "heightRpx": rpx, "lineCount": lines });
        EVENTS.with(|e| e.borrow_mut().push((component_id.to_string(), "linechange", detail)));
    }
    if !is_auto_height(&node.attrs) {
        return;
    }
    let built = node.attrs.get(AUTO_HEIGHT_LINES).and_then(|s| s.parse::<usize>().ok());
    if built != Some(lines) {
        LINES.with(|m| m.borrow_mut().insert(key_of(&node.attrs), lines));
        RELAYOUT.with(|r| r.set(true));
    }
}

/// 宿主每帧：有 auto-height 的 textarea 行数变了，整树重排一次
pub fn take_relayout() -> bool {
    RELAYOUT.with(|r| r.replace(false))
}

/// 取走待派发的 `linechange`：(组件 id, 事件名, detail)
pub fn take_events() -> Events {
    EVENTS.with(|e| std::mem::take(&mut *e.borrow_mut()))
}

/// 换页：行数表跟着页面走
pub fn clear() {
    LINES.with(|m| m.borrow_mut().clear());
    EVENTS.with(|e| e.borrow_mut().clear());
}
