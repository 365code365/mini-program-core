//! 同层渲染组件**自己的手势**：地图要跟手平移、双指捏合缩放（微信里在地图上拖动拖的是地图，不是页面）。
//!
//! 宿主在创建选项 `sameLayerGestures` 里点名的标签（同时得在 `sameLayer` 里），按下点落在它的框里、
//! 而且那一点没被页面元素盖住时（地图上的 cover-view 按钮先收，见 `wxml_renderer/cover_hit.rs`）：
//! 这一串触摸（含第二根手指）**整串**交给宿主，页面不建手势、不滚、不派 touchstart / tap，
//! 每一段发一条 `nativeViewTouch` 事件：
//!
//! ```json
//! {"type":"nativeViewTouch","key":"map1","tag":"map","phase":"start","pointer":0,"x":120.5,"y":80,"t":1234}
//! ```
//! - `phase`：`start` / `move` / `end` / `cancel`；`pointer` 0 是第一根手指，1 是第二根（捏合）。
//! - `x` / `y`：框里的**逻辑**坐标（左上原点），按下那一刻定的原点 —— 这串触摸期间页面不滚，原点不变。
//! - `t`：引擎时钟毫秒（宿主算速度 / 双击用）。
//!
//! 点击也在这串里（宿主自己判：位移小、时间短就是点），不再另发 `nativeViewTap`。
//!
//! ## 为什么不让宿主自己拦触摸
//! 宿主只知道组件的尺寸（`sameLayerViews` 不带位置：同层组件跟着页面滚，位置每帧在变），
//! 也不知道它上面有没有盖着弹窗、fixed 元素。按下点该归谁只有引擎说得清 —— 与覆盖层、导航栏、
//! picker 面板在 `pointer_down` 里先吃一样，排在它们之后、页面手势之前。
use super::engine::MiniEngine;
use serde_json::json;

/// 这串触摸归了哪个同层组件
#[derive(Clone, Debug)]
pub struct NativeTouch {
    key: String,
    tag: String,
    /// 框左上角在页面视口里的位置（逻辑像素）
    ox: f32,
    oy: f32,
    /// 第二根手指按着
    second: bool,
}

impl MiniEngine {
    fn native_touch_emit(&mut self, phase: &str, pointer: u8, x: f32, y: f32) {
        let Some(c) = self.native_touch.as_ref() else { return };
        let ev = json!({
            "type": "nativeViewTouch", "key": c.key, "tag": c.tag, "phase": phase, "pointer": pointer,
            "x": x - c.ox, "y": y - c.oy, "t": self.mono_ms(),
        });
        self.events.push(ev);
    }

    /// `pointer_down` 里、页面建手势之前调（画面坐标，与指针三段的入参一样）。归了同层组件返回 true（调用方到此为止）
    pub(super) fn native_touch_down(&mut self, x: f32, y: f32) -> bool {
        self.native_touch = None;
        if crate::renderer::components::host_view::tags().is_empty() {
            return false;
        }
        let y = self.unshift_y(y) - self.nav_height();
        // tabBar 盖在页面上：按在 tabBar 上的不归它下面的地图
        if self.is_tabbar_page(&self.current_route()) && y >= self.page_window_height() - self.tabbar_h() {
            return false;
        }
        let scroll = self.scroll.get_position();
        let hit = self
            .renderer
            .as_ref()
            .and_then(|r| super::engine_host_views::gesture_view_at(r, &self.interaction, x, y, scroll));
        let Some((key, tag, ox, oy)) = hit else { return false };
        // 惯性滚动中按在地图上：页面先停住（与按在别处一致），这一下照样交给地图
        if crate::host::input::stop_running_flings(&mut self.scroll, &mut self.interaction) {
            self.dmg.compose_dirty = true;
        }
        self.native_touch = Some(NativeTouch { key, tag, ox, oy, second: false });
        self.native_touch_emit("start", 0, x, y);
        true
    }

    /// 移动（画面坐标）。这串触摸归了同层组件返回 true
    pub(super) fn native_touch_move(&mut self, x: f32, y: f32) -> bool {
        if self.native_touch.is_none() {
            return false;
        }
        let y = self.unshift_y(y) - self.nav_height();
        self.native_touch_emit("move", 0, x, y);
        true
    }

    /// 抬起（画面坐标）
    pub(super) fn native_touch_up(&mut self, x: f32, y: f32) -> bool {
        if self.native_touch.is_none() {
            return false;
        }
        let y = self.unshift_y(y) - self.nav_height();
        self.native_touch_emit("end", 0, x, y);
        self.native_touch = None;
        true
    }

    /// 被系统打断
    pub(super) fn native_touch_cancel(&mut self) -> bool {
        if self.native_touch.is_none() {
            return false;
        }
        self.native_touch_emit("cancel", 0, 0.0, 0.0);
        self.native_touch = None;
        true
    }

    /// 第二根手指（`pointer2` 的 phase：0 按下 / 1 移动 / 2 抬起 / 其它 取消；画面坐标）
    pub(super) fn native_touch_pointer2(&mut self, phase: i32, x: f32, y: f32) -> bool {
        let Some(second) = self.native_touch.as_ref().map(|c| c.second) else { return false };
        let y = self.unshift_y(y) - self.nav_height();
        let name = match phase {
            0 => "start",
            1 if second => "move",
            2 if second => "end",
            1 | 2 => return true,
            _ => "cancel",
        };
        if let Some(c) = self.native_touch.as_mut() {
            c.second = phase <= 1;
        }
        self.native_touch_emit(name, 1, x, y);
        true
    }
}
