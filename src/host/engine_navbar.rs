//! SDK 引擎画的默认导航栏（`navigationBar` 选项）：哪些页画、多高、什么时候重画、指针。
//! 画法与几何在 `navbar.rs`（为什么要引擎画也在那里）。
//!
//! ## 坐标
//! 画面（宿主的纹理 / surface）是整屏，导航栏占最上面 `nav_height()` 逻辑像素（含状态栏）。
//! 页面、fixed 层、tabBar 仍在**页面视口坐标**里（导航栏下沿为 y=0）——布局、命中、
//! 触摸事件的 `clientY`、SelectorQuery 都不用改。换算只在三处边界做：
//! - 合成：页面与 fixed 层整体下移（`FrameLayers::top`，`present_to_buffer` 的 `top_px`）；
//! - 指针：浮层与胶囊用画面坐标，之后 `y - nav_height()` 才交给页面（`engine_input.rs`）；
//! - 报给宿主的矩形（输入框、原生组件）加回 `nav_height()`（`engine_events.rs`）。
//!
//! 浮层（Toast / Modal / picker / 操作菜单）本来就画在整块画面上，遮罩自然盖住导航栏。
use super::capsule::{self, Half};
use super::engine::MiniEngine;
use super::navbar::{self, NavBar};
use crate::compositor::{LayerCanvas, TopBand};
use crate::{Canvas, Rect};

/// 导航栏的画布与交互状态
#[derive(Default)]
pub struct NavChrome {
    canvas: Option<LayerCanvas>,
    /// 上一次画进画布的内容（变了才重画）
    painted: Option<NavSig>,
    /// 这串触摸按在导航栏上（整串归导航栏，不进页面）
    down: bool,
    /// 按下落在返回键上
    back_down: bool,
    /// 手指还在返回键里（按压态）
    back_inside: bool,
    /// 逻辑层系统信息里装的 windowHeight（换页时导航栏有无变了要重装）
    installed_window_h: Option<f32>,
}

impl NavChrome {
    /// 本帧合成要的顶部那一条（`rows` = `MiniEngine::nav_rows()`；没有导航栏给 None）。
    /// 只借这一个字段：合成时 `gpu` / `compose` 还要可变借用
    pub(super) fn band(&self, rows: u32) -> Option<TopBand<'_>> {
        let canvas = self.canvas.as_ref()?;
        (rows > 0 && self.painted.is_some()).then_some(TopBand { rows, canvas })
    }
}

/// 导航栏这一帧画成什么样（变了才重画）
#[derive(Clone, Debug, PartialEq)]
struct NavSig {
    bar: NavBar,
    px: (u32, u32),
    back_pressed: bool,
    spin: u32,
    capsule: Option<(Rect, bool, Option<Half>)>,
}

impl MiniEngine {
    /// 当前页顶上引擎画的导航栏高度（逻辑像素，含状态栏）。没开选项、custom 导航栏的页面、
    /// 还没有页面时为 0 —— 此时一切与从前逐字节相同
    pub fn nav_height(&self) -> f32 {
        if !self.opts.navigation_bar || self.page_stack.is_empty() || self.page_is_custom_nav() {
            return 0.0;
        }
        self.opts.status_bar_height() + navbar::bar_height(self.opts.platform())
    }

    /// 导航栏占的设备像素行数（合成用；与 `nav_height` 同源，四舍五入到整行）
    pub(crate) fn nav_rows(&self) -> u32 {
        let (_, ph) = self.pixel_size();
        ((self.nav_height() * self.dpr).round() as u32).min(ph)
    }

    /// 页面窗口的逻辑高（画面高减导航栏）：tabBar 的上沿、`100vh`、fixed 层的视口都按它算
    pub(crate) fn page_window_height(&self) -> f32 {
        (self.height as f32 - self.nav_height()).max(1.0)
    }

    fn page_is_custom_nav(&self) -> bool {
        self.page_window().get("navigationStyle").and_then(|v| v.as_str()) == Some("custom")
    }

    /// 某个路由的页面有没有引擎画的导航栏（解析它的样式表时要知道：有导航栏的页面
    /// `env(safe-area-inset-top)` 是 0，画面不与状态栏重叠 —— 与微信一致）
    pub(crate) fn route_has_nav(&self, route: &str) -> bool {
        if !self.opts.navigation_bar {
            return false;
        }
        let style = |v: &serde_json::Value| v.get("navigationStyle").and_then(|s| s.as_str()).map(str::to_string);
        let page = self.pages.get(route).and_then(|p| style(&p.window));
        page.or_else(|| style(&self.app_window)).as_deref() != Some("custom")
    }

    fn nav_sig(&self) -> Option<NavSig> {
        let rows = self.nav_rows();
        if rows == 0 {
            return None;
        }
        let bar = NavBar::from_window(&self.page_window(), self.page_stack.len());
        let spin = if bar.loading { navbar::spin_step(self.overlay_clock()) } else { 0 };
        let capsule = self.capsule_spec().map(|(r, dark)| (r, dark, self.capsule_pressed()));
        Some(NavSig {
            bar,
            px: (self.pixel_size().0, rows),
            back_pressed: self.nav.back_down && self.nav.back_inside,
            spin,
            capsule,
        })
    }

    /// 导航栏要不要重画（标题 / 颜色 / loading 转到下一步 / 按压态 / 胶囊）
    pub(super) fn nav_needs_paint(&self) -> bool {
        self.nav_sig() != self.nav.painted
    }

    /// 导航栏在转圈或还欠一次重画：宿主要再续一帧
    pub(super) fn nav_wants_frame(&self) -> bool {
        self.nav.painted.as_ref().map(|s| s.bar.loading).unwrap_or(false) || self.nav_needs_paint()
    }

    /// 合成前调：内容变了才重画导航栏画布
    pub(super) fn paint_nav(&mut self) {
        let sig = self.nav_sig();
        if sig == self.nav.painted {
            return;
        }
        if let Some(s) = &sig {
            let (pw, rows) = s.px;
            let stale = self.nav.canvas.as_ref().is_none_or(|c| c.read().width() != pw || c.read().height() != rows);
            if stale {
                self.nav.canvas = Some(LayerCanvas::new(Canvas::new(pw, rows)));
            }
            let top = self.opts.status_bar_height();
            let bar_h = navbar::bar_height(self.opts.platform());
            let width = self.width as f32;
            let title_max = navbar::title_max_width(width, s.capsule.map(|c| c.0.x));
            let text = self.text.clone();
            let dpr = self.dpr;
            if let Some(c) = self.nav.canvas.as_mut() {
                let c = c.write();
                navbar::draw(c, &s.bar, width, top, bar_h, dpr, text.as_deref(), title_max, s.back_pressed, s.spin);
                if let Some((rect, dark, pressed)) = s.capsule {
                    c.reset_clip();
                    capsule::draw(c, rect, dpr, dark, pressed);
                }
            }
            self.capsule_painted();
        }
        self.nav.painted = sig;
    }



    /// fixed 覆盖层的画布高（设备像素）：画面高减导航栏。导航栏有无随页面变，变了就重分配
    pub(super) fn ensure_fixed_canvas(&mut self) {
        let (pw, ph) = self.pixel_size();
        let want = ph.saturating_sub(self.nav_rows()).max(1);
        let c = self.fixed_canvas.read();
        if c.width() != pw || c.height() != want {
            self.fixed_canvas = LayerCanvas::new(Canvas::new(pw, want));
            self.dmg.fixed_dirty = true;
        }
    }

    /// 逻辑层的系统信息：`windowHeight` 是这一页的窗口高（有导航栏的页面减掉它）
    pub(crate) fn page_system_info(&self) -> serde_json::Value {
        self.system_info_with_window(self.page_window_height())
    }

    fn system_info_with_window(&self, window_h: f32) -> serde_json::Value {
        let mut info = super::engine_options::system_info(self.width, self.height, self.dpr, &self.opts);
        if self.opts.navigation_bar {
            info["windowHeight"] = serde_json::json!(window_h);
        }
        info
    }

    /// 导航栏有无变了，`windowHeight` 跟着变（重装系统信息，不派发 onResize）。
    /// 每次 pump 末尾调一次（返回、切 tab 之后对齐）
    pub(super) fn sync_window_height(&mut self) {
        let h = self.page_window_height();
        self.install_window_height(h);
    }

    /// 进入 `route` 之前调：页面 js（onLoad 里读 `windowHeight` 算布局是常态）看到的
    /// 就是这一页的窗口高，而不是上一页的
    pub(super) fn sync_window_height_for(&mut self, route: &str) {
        let nav = if self.route_has_nav(route) {
            self.opts.status_bar_height() + navbar::bar_height(self.opts.platform())
        } else {
            0.0
        };
        self.install_window_height((self.height as f32 - nav).max(1.0));
    }

    fn install_window_height(&mut self, h: f32) {
        if !self.opts.navigation_bar || self.nav.installed_window_h == Some(h) {
            return;
        }
        self.nav.installed_window_h = Some(h);
        super::engine_options::install_system_info(&self.app, &self.system_info_with_window(h));
    }

    /// 解析 `route` 的样式表时用的选项：有引擎画的导航栏的页面，`env(safe-area-inset-top)` 是 0
    /// （画面在导航栏下面，不与状态栏重叠 —— 与微信一致；custom 页才是状态栏高）
    pub(crate) fn env_opts_for(&self, route: &str) -> std::borrow::Cow<'_, super::engine_options::EngineOptions> {
        match self.opts.safe_area {
            Some(s) if s.top > 0.0 && self.route_has_nav(route) => {
                let mut o = self.opts.clone();
                o.safe_area = Some(super::engine_options::SafeAreaInsets { top: 0.0, ..s });
                std::borrow::Cow::Owned(o)
            }
            _ => std::borrow::Cow::Borrowed(&self.opts),
        }
    }

    /// 尺寸 / 安全区变了，系统信息整个重装过：下次对齐时要重装 windowHeight
    pub(super) fn forget_window_height(&mut self) {
        self.nav.installed_window_h = None;
        self.nav.painted = None;
    }

    // ── 指针（画面坐标）。浮层与胶囊在这之前；返回 true 表示这串触摸归导航栏 ──

    pub(super) fn nav_pointer_down(&mut self, x: f32, y: f32) -> bool {
        let h = self.nav_height();
        if h <= 0.0 || y >= h {
            return false;
        }
        let (top, bar_h) = (self.opts.status_bar_height(), navbar::bar_height(self.opts.platform()));
        self.nav.down = true;
        self.nav.back_down = self.page_stack.len() > 1 && navbar::back_hit(x, y, top, bar_h);
        self.nav.back_inside = self.nav.back_down;
        true
    }

    pub(super) fn nav_pointer_move(&mut self, x: f32, y: f32) -> bool {
        if !self.nav.down {
            return false;
        }
        if self.nav.back_down {
            let (top, bar_h) = (self.opts.status_bar_height(), navbar::bar_height(self.opts.platform()));
            self.nav.back_inside = navbar::back_hit(x, y, top, bar_h);
        }
        true
    }

    /// 抬起：还在返回键里就返回上一页（与系统返回键同一条路：带转场、派 onUnload / onShow）
    pub(super) fn nav_pointer_up(&mut self, x: f32, y: f32) -> bool {
        if !std::mem::take(&mut self.nav.down) {
            return false;
        }
        let back = std::mem::take(&mut self.nav.back_down);
        self.nav.back_inside = false;
        let (top, bar_h) = (self.opts.status_bar_height(), navbar::bar_height(self.opts.platform()));
        if back && navbar::back_hit(x, y, top, bar_h) {
            self.back();
        }
        true
    }

    pub(super) fn nav_pointer_cancel(&mut self) -> bool {
        if !std::mem::take(&mut self.nav.down) {
            return false;
        }
        self.nav.back_down = false;
        self.nav.back_inside = false;
        true
    }
}
