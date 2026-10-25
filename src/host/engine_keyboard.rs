//! 键盘顶起画面（输入框 `adjust-position`，默认开）：宿主算出「该顶多高」交给引擎，引擎自己按帧推进、
//! 合成时导航栏以下整体上移（`FrameLayers::content_shift`，render_shift.rs / compose.wgsl），触摸坐标加回去。
//!
//! 为什么不让宿主挪视图：从前 Android SDK 给 SurfaceView 做 200ms 的 `translationY` 动画 ——
//! ① 引擎画的导航栏跟着整个画面被推出屏幕，页面内容顶到状态栏底下（转账页「附言」实测）；
//! ② SurfaceView 的位置更新跟 App 的 RenderThread 走，与引擎出帧不同步，顶起那 200ms 一顿一顿。
//! 放进引擎：上移只是合成时换一个行偏移（不重画页面），每帧按 vsync 时刻取样，导航栏留在原处。
//!
//! 宿主接口：`MiniEngine::set_keyboard_shift(逻辑像素)`（目标值，0 = 收回）。
use super::engine::MiniEngine;

/// 顶起 / 收回各用多久（与系统键盘动画差不多：微信实测顶起约 250ms）
const UP_MS: f64 = 220.0;
const DOWN_MS: f64 = 180.0;

#[derive(Default)]
pub struct KeyboardShift {
    /// 目标、起点（设备像素）与这一段开始的时刻（引擎时钟毫秒）
    to: f32,
    from: f32,
    start_ms: Option<u64>,
    /// 当前上移量（设备像素）
    cur: f32,
    /// 键盘是为哪个输入框弹的（组件 id、所在页的栈深）：失焦之后键盘落下的那次高度变化
    /// 也要送到它的 `bindkeyboardheightchange`（页面靠它把列表恢复原高）
    input: Option<(String, usize)>,
}

impl MiniEngine {
    /// 宿主：键盘顶起画面的目标上移量（逻辑像素，≥0；0 = 收回）。引擎从当前位置缓动过去
    pub fn set_keyboard_shift(&mut self, logical: f32) {
        let to = (logical.max(0.0) * self.dpr).round();
        if (to - self.kb.to).abs() < 0.5 {
            return;
        }
        self.kb.from = self.kb.cur;
        self.kb.to = to;
        self.kb.start_ms = None; // 下一次 pump 取它的时刻作起点（与出帧同一个钟）
        self.dmg.compose_dirty = true;
    }

    /// 每次 pump：推进上移量。变了只要重新合成（页面一个像素不用重画）
    pub(super) fn tick_keyboard_shift(&mut self, now_ms: u64) {
        if (self.kb.cur - self.kb.to).abs() < 0.01 {
            return;
        }
        let start = *self.kb.start_ms.get_or_insert(now_ms);
        let dur = if self.kb.to > self.kb.from { UP_MS } else { DOWN_MS };
        let t = ((now_ms.saturating_sub(start)) as f64 / dur).clamp(0.0, 1.0);
        // 先快后慢（ease-out cubic），与系统键盘的曲线接近
        let e = 1.0 - (1.0 - t).powi(3);
        self.kb.cur = if t >= 1.0 { self.kb.to } else { self.kb.from + (self.kb.to - self.kb.from) * e as f32 };
        self.dmg.compose_dirty = true;
    }

    /// 还在顶起 / 收回的路上（宿主要续帧）
    pub(super) fn keyboard_shift_animating(&self) -> bool {
        (self.kb.cur - self.kb.to).abs() >= 0.01
    }

    /// 合成用的整数行数
    pub(super) fn keyboard_shift_px(&self) -> u32 {
        self.kb.cur.round().max(0.0) as u32
    }

    /// 输入框聚焦（engine_events.rs `emit_input_focus` 调）：之后的键盘高度变化归它
    pub(super) fn note_keyboard_input(&mut self, id: &str) {
        self.kb.input = Some((id.to_string(), self.page_stack.len()));
    }

    /// 宿主推了键盘高度（`dispatch_event("keyboardHeightChange", {height, duration})` 转进来）：
    /// 除了 `wx.onKeyboardHeightChange`，键盘所属输入框的 `bindkeyboardheightchange` 也收一份
    /// `{height, duration}`（微信 input / textarea 的同名事件）。聊天页就靠它：`adjust-position="{{false}}"`
    /// 不让画面整体上移，按高度把消息列表改矮、输入条贴着键盘 —— 像原生聊天那样最新消息还在眼前。
    ///
    /// `height` 是键盘挡住小程序视图的那一截（逻辑像素，宿主算好：视图下面另有东西时先扣掉），
    /// 页面拿 `windowHeight - height` 就是键盘上方还看得见的高度。
    pub(super) fn keyboard_height_to_input(&mut self, payload: &str) {
        let v: serde_json::Value = serde_json::from_str(payload).unwrap_or_default();
        let height = v.get("height").and_then(|h| h.as_f64()).unwrap_or(0.0).max(0.0);
        let duration = v.get("duration").and_then(|d| d.as_f64()).unwrap_or(if height > 0.0 { UP_MS } else { DOWN_MS });
        let focused = self.interaction.focused_input.as_ref().map(|f| f.id.clone());
        let depth = self.page_stack.len();
        // 聚焦中的优先；没有聚焦的（已失焦、键盘正在落下）只认同一页上刚才那个
        let id = focused.clone().or_else(|| self.kb.input.as_ref().filter(|(_, d)| *d == depth).map(|(id, _)| id.clone()));
        if height <= 0.0 && focused.is_none() {
            self.kb.input = None;
        }
        let Some(id) = id else { return };
        let detail = serde_json::json!({ "height": height, "duration": duration });
        let r = self.renderer.take();
        crate::host::tap::dispatch_input_detail(&mut self.app, r.as_ref(), &id, "keyboardheightchange", detail);
        self.renderer = r;
    }

    /// 画面上导航栏以下的点（逻辑像素）对应「没顶起时」的位置：触摸、命中都按没顶起的坐标算
    pub(super) fn unshift_y(&self, y: f32) -> f32 {
        if y >= self.nav_height() {
            y + self.keyboard_shift_px() as f32 / self.dpr
        } else {
            y
        }
    }
}
