//! SDK 引擎的胶囊按钮：哪些页面画、画在哪、深色浅色，以及按下 / 移动 / 抬起 / 取消。
//! 几何与画法在 `capsule.rs`（为什么画进 fixed 覆盖层也在那里）。
//!
//! 只在宿主打开 `menuButton` 选项、且当前页 `navigationStyle: custom` 时出现。C ABI 的默认是关
//! （老宿主不传选项，画面一个像素不变）；Flutter 插件与 iOS / Android 的 `MiniProgramView` 默认都开。
//! 桌面窗体在 custom 页面上也画（`src/bin/app_window/capsule_host.rs`），`sdk-parity` 的 SDK 一侧
//! 因此开着它（`examples/sdk_parity.rs`），两边仍逐像素一致。
//!
//! 点击不走页面的触摸状态机：胶囊不属于页面，按下就整串吃掉（与 Modal 的按钮同一套，
//! 由 `engine_overlay_input.rs` 在 Modal 之后、页面之前调）。抬手还在**按下的那一半**里才算点了，
//! 发 `menuButton` 事件：`menu`（「•••」查看关于）/ `close`（「⊙」返回宿主），做什么由宿主定；
//! 滑出去再松手什么也不发（与原生按钮一致）。
use super::capsule::{self, Half};
use super::engine::MiniEngine;
use crate::Rect;
use serde_json::json;

/// 胶囊的交互状态
#[derive(Default)]
pub struct CapsuleState {
    /// 按下落在哪一半（整串触摸归胶囊）
    down: Option<Half>,
    /// 手指还在按下的那一半里（画不画按压态）
    inside: bool,
    /// 导航栏前景色改了（`wx.setNavigationBarColor`）：要换一套配色重画。
    /// 这条事件在一帧的末尾才收到，宿主据此再续一帧（`wants_next_frame`）
    repaint: bool,
}

impl MiniEngine {
    /// 当前页画着的胶囊（视口逻辑坐标）；不画时为 None。与这一页
    /// `wx.getMenuButtonBoundingClientRect()` 返回的是同一个矩形
    pub fn menu_button_rect(&self) -> Option<Rect> {
        self.capsule_spec().map(|(r, _)| r)
    }

    /// 画不画、画在哪、是不是深色导航栏（白图标）
    pub(super) fn capsule_spec(&self) -> Option<(Rect, bool)> {
        // 分栏时左栏没有胶囊（engine_frameset.rs `capsule_in_this_pane`）
        if !self.capsule_in_this_pane() {
            return None;
        }
        self.capsule_spec_raw()
    }

    /// 不看分栏：这一页按它的导航栏该不该有胶囊
    pub(super) fn capsule_spec_raw(&self) -> Option<(Rect, bool)> {
        if !self.opts.menu_button || self.page_stack.is_empty() {
            return None;
        }
        let w = self.page_window();
        let custom = w.get("navigationStyle").and_then(|v| v.as_str()) == Some("custom");
        // 普通页面：只有引擎自己画导航栏（`navigationBar` 选项）时胶囊才归引擎，画在导航栏里
        if !custom && self.nav_height() <= 0.0 {
            return None;
        }
        let style = w.get("navigationBarTextStyle").and_then(|v| v.as_str());
        // custom 页沿用老规则（明确写 white 才是深色，与桌面窗体一致）；引擎画的导航栏
        // 跟着导航栏前景走，缺省就是 white（app.json 文档的默认值）
        let dark = if custom { style == Some("white") } else { style != Some("black") };
        Some((capsule::rect(&self.page_system_info()), dark))
    }

    /// 按压态（手指按着、还在那一半里）
    pub(super) fn capsule_pressed(&self) -> Option<Half> {
        self.capsule.down.filter(|_| self.capsule.inside)
    }

    /// 前景色改了：下一帧换配色重画（`drain_js_host_events` 收到 `frontColor` 时调）
    pub(super) fn capsule_restyle(&mut self) {
        if self.opts.menu_button {
            self.capsule.repaint = true;
            self.dmg.fixed_dirty = true;
        }
    }

    /// 还欠一次重画（宿主要再续一帧）
    pub(super) fn capsule_wants_frame(&self) -> bool {
        self.capsule.repaint
    }

    /// fixed 层重画过了：配色已是最新
    pub(super) fn capsule_painted(&mut self) {
        self.capsule.repaint = false;
    }

    /// 按下：落在胶囊上返回 true（这串触摸归胶囊）
    pub(super) fn capsule_down(&mut self, x: f32, y: f32) -> bool {
        let Some(half) = self.capsule_spec().and_then(|(r, _)| capsule::half_at(r, x, y)) else {
            return false;
        };
        self.capsule.down = Some(half);
        self.capsule.inside = true;
        self.dmg.fixed_dirty = true;
        true
    }

    /// 移动：这串触摸归胶囊时返回 true。移出按下的那一半就褪掉按压态，移回来再显示
    pub(super) fn capsule_move(&mut self, x: f32, y: f32) -> bool {
        let Some(half) = self.capsule.down else { return false };
        let inside = self.capsule_spec().and_then(|(r, _)| capsule::half_at(r, x, y)) == Some(half);
        if inside != self.capsule.inside {
            self.capsule.inside = inside;
            self.dmg.fixed_dirty = true;
        }
        true
    }

    /// 抬起：这串触摸归胶囊时返回 true；还在按下的那一半里就发 `menuButton`
    pub(super) fn capsule_up(&mut self, x: f32, y: f32) -> bool {
        let Some(half) = self.capsule.down.take() else { return false };
        self.capsule.inside = false;
        self.dmg.fixed_dirty = true;
        if self.capsule_spec().and_then(|(r, _)| capsule::half_at(r, x, y)) == Some(half) {
            self.events.push(json!({ "type": "menuButton", "action": half.action() }));
        }
        true
    }

    /// 触摸被系统打断：按压态褪掉，什么也不发
    pub(super) fn capsule_cancel(&mut self) -> bool {
        if self.capsule.down.take().is_none() {
            return false;
        }
        self.capsule.inside = false;
        self.dmg.fixed_dirty = true;
        true
    }
}
