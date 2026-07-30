//! [`MiniEngine`] 的输入与路由入口。
//!
//! 宿主只送**指针三段**（按下/移动/抬起），`touchstart/touchmove/touchend/
//! longpress/tap` 由引擎的触摸状态机产出 —— 与桌面窗体走同一份 `host::touch`。
//! 千万不要让宿主自己合成「点击」再调过来：那样会绕过状态机，
//! 「按下时提前 return 导致抬手没有 tap」这类 bug 在无头测试里全绿、真机上点不动
//! （这个仓库踩过一次，见 `doc/引擎测试说明.md` T2-6）。

use super::engine::MiniEngine;
use super::navigation::{parse_url, NavigationRequest};
use crate::host::input;
use crate::host::touch::TouchOut;
use crate::ui::interaction::KeyInput;
use std::collections::HashMap;

impl MiniEngine {
    /// 手指按下（逻辑坐标，左上原点）
    pub fn pointer_down(&mut self, x: f32, y: f32) {
        let now = self.mono_ms();
        if self.modal_press(x, y) {
            return;
        }
        // 内容区先记下按压位置：滚动拖拽与 tap 都从这里起算
        self.scroll.begin_drag(y, now);
        let outs = self.touch.press(x, y, now);
        self.dispatch_touch(&outs, x, y);
        self.needs_redraw = true;
    }

    /// 手指移动
    pub fn pointer_move(&mut self, x: f32, y: f32) {
        if !self.touch.is_active() {
            return;
        }
        let now = self.mono_ms();
        let outs = self.touch.move_to(x, y, now);
        self.dispatch_touch(&outs, x, y);
        // 手势没被内层吃掉时，纵向位移驱动页面滚动
        self.scroll.update_drag(y, now);
        self.needs_redraw = true;
    }

    /// 手指抬起
    pub fn pointer_up(&mut self, x: f32, y: f32) {
        let now = self.mono_ms();
        let moved = self.touch.moved();
        let outs = self.touch.release(x, y, now);
        self.dispatch_touch(&outs, x, y);
        self.scroll.end_drag();
        // 没有构成滑动才算点击（与微信一致：手指挪动超过阈值就不再是 tap）
        if !moved {
            self.handle_click(x, y);
        }
        self.touch.finish();
        self.needs_redraw = true;
    }

    /// 手势被系统打断（来电、返回手势接管…）
    pub fn pointer_cancel(&mut self) {
        let outs = self.touch.cancel();
        if let Some((x, y)) = self.touch.start_pos() {
            self.dispatch_touch(&outs, x, y);
        }
        self.scroll.end_drag();
        self.touch.finish();
    }

    /// 滚轮 / 触控板（桌面与带鼠标的平板）
    pub fn wheel(&mut self, delta_y: f32, precise: bool) {
        self.scroll.handle_scroll(delta_y, precise);
        self.needs_redraw = true;
    }

    /// 往当前聚焦的输入框送文字（输入法提交的整串也走这里）
    pub fn text_input(&mut self, text: &str) {
        if !self.interaction.has_focused_input() {
            return;
        }
        for c in text.chars() {
            if let Some(r) = self.interaction.handle_key_input(KeyInput::Char(c)) {
                self.apply_interaction(r);
            }
        }
        self.needs_redraw = true;
    }

    /// 功能键：`enter` / `backspace` / `delete` / `left` / `right` / `home` /
    /// `end` / `selectall` / `escape`
    pub fn key(&mut self, name: &str) {
        let input = match name {
            "enter" | "confirm" => KeyInput::Enter,
            "backspace" => KeyInput::Backspace,
            "delete" => KeyInput::Delete,
            "left" => KeyInput::Left,
            "right" => KeyInput::Right,
            "home" => KeyInput::Home,
            "end" => KeyInput::End,
            "selectall" => KeyInput::SelectAll,
            "escape" => KeyInput::Escape,
            _ => return,
        };
        if let Some(r) = self.interaction.handle_key_input(input) {
            self.apply_interaction(r);
        }
        self.needs_redraw = true;
    }

    /// 系统返回键 / 侧滑返回。
    ///
    /// 返回 `false` 表示**页面栈已经只剩一页**，宿主该关掉这个小程序
    /// （Android 的 `onBackPressed` 要在这时才调 `finish()`）。
    pub fn back(&mut self) -> bool {
        if self.page_stack.len() <= 1 {
            return false;
        }
        self.pop_page();
        true
    }

    pub(crate) fn pop_page(&mut self) {
        // 离开当前页：先 onUnload，再让逻辑层出栈（**不重载**上一页的 js，
        // 否则用户在上一页的筛选/表单/滚动位置全丢，还会重复触发「仅首次进入」的逻辑）
        self.app
            .eval("if(__currentPage&&__currentPage.onUnload)__currentPage.onUnload()")
            .ok();
        self.page_stack.pop();
        self.app.eval("__popPage(1)").ok();
        self.app.eval("if(__currentPage&&__currentPage.onShow)__currentPage.onShow()").ok();
        self.interaction.clear_page_state();
        let vp = self.viewport_height();
        self.scroll = crate::ui::ScrollController::new(vp, vp);
        self.page_data_dirty = true;
        self.update_renderers();
        self.needs_redraw = true;
    }

    /// 取一次逻辑层挂起的路由请求
    pub(crate) fn take_navigation(&mut self) -> Option<NavigationRequest> {
        let raw = self.app.eval("JSON.stringify(__pendingNavigation||null)").ok()?;
        let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
        let obj = v.as_object()?;
        let kind = obj.get("type")?.as_str()?.to_string();
        let url = obj.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string();
        self.app.eval("__pendingNavigation=null").ok();
        Some(match kind.as_str() {
            "navigateTo" => NavigationRequest::NavigateTo { url },
            "navigateBack" => NavigationRequest::NavigateBack,
            "switchTab" => NavigationRequest::SwitchTab { url },
            "redirectTo" => NavigationRequest::RedirectTo { url },
            "reLaunch" => NavigationRequest::ReLaunch { url },
            _ => return None,
        })
    }

    pub(crate) fn apply_navigation(&mut self, nav: NavigationRequest) {
        match nav {
            NavigationRequest::NavigateTo { url } => {
                let (p, q) = parse_url(&url);
                self.navigate_to(&p, q).ok();
            }
            NavigationRequest::NavigateBack => {
                self.back();
            }
            NavigationRequest::SwitchTab { url } => {
                let (p, _) = parse_url(&url);
                self.reset_stack_and_open(&p);
            }
            NavigationRequest::RedirectTo { url } => {
                // 替换当前页：栈深不变
                let (p, q) = parse_url(&url);
                self.app
                    .eval("if(__currentPage&&__currentPage.onUnload)__currentPage.onUnload()")
                    .ok();
                self.page_stack.pop();
                self.app.eval("__popPage(1)").ok();
                self.interaction.clear_page_state();
                self.navigate_to(&p, q).ok();
            }
            NavigationRequest::ReLaunch { url } => {
                let (p, _) = parse_url(&url);
                self.reset_stack_and_open(&p);
            }
        }
    }

    fn reset_stack_and_open(&mut self, path: &str) {
        self.app
            .eval("if(__currentPage&&__currentPage.onUnload)__currentPage.onUnload()")
            .ok();
        self.page_stack.clear();
        self.app.eval("__resetPageStack()").ok();
        self.interaction.clear_page_state();
        self.navigate_to(path, HashMap::new()).ok();
    }

    /// 主动跳页（宿主侧的深链：扫码进来直接落到某个页面）
    pub fn navigate(&mut self, url: &str) -> Result<(), String> {
        let (p, q) = parse_url(url);
        self.navigate_to(&p, q)
    }

    // ───────────────────────── 内部：派发 ─────────────────────────

    /// 触摸派发：与桌面窗体走同一份 [`crate::host::input::dispatch_touch_sequence`]，
    /// 这里只补「派发之后引擎要做什么」（取 setData 脏标记）。
    pub(crate) fn dispatch_touch(&mut self, outs: &[TouchOut], x: f32, y: f32) {
        let id = self.touch.identifier();
        let time_ms = self.mono_ms();
        let scroll_pos = self.scroll.get_position();
        let Self {
            app,
            renderer,
            page_data_dirty,
            needs_redraw,
            ..
        } = self;
        input::dispatch_touch_sequence(
            app,
            renderer.as_ref(),
            outs,
            (x, y),
            scroll_pos,
            id,
            time_ms,
            |app| {
                if app.take_data_dirty() {
                    *page_data_dirty = true;
                    *needs_redraw = true;
                }
            },
        );
    }

    fn handle_click(&mut self, x: f32, y: f32) {
        if self.modal.as_ref().map(|m| m.visible).unwrap_or(false) {
            self.modal_release(x, y);
            return;
        }
        // 底部 tabBar 命中优先（它盖在页面之上）
        if let Some(nav) = self.tabbar_hit(x, y) {
            self.apply_navigation(nav);
            return;
        }
        let scroll = self.scroll.get_position();
        let has_tabbar = self.is_tabbar_page(&self.current_route());
        let tap_ctx = (self.touch.identifier(), self.mono_ms());
        let text = self.text.clone();
        let result = crate::host::tap::handle_content_click(
            x,
            y,
            scroll,
            has_tabbar,
            &mut self.interaction,
            self.renderer.as_ref(),
            &mut self.app,
            self.dpr as f64,
            text.as_deref(),
            tap_ctx,
        );
        if let Some(res) = result {
            self.apply_interaction(res);
        }
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
        }
        self.needs_redraw = true;
    }

    fn tabbar_hit(&self, x: f32, y: f32) -> Option<NavigationRequest> {
        let tb = self.config.tab_bar.as_ref()?;
        if !self.is_tabbar_page(&self.current_route()) {
            return None;
        }
        let h = crate::host::tabbar_height() as f32;
        if y < self.height as f32 - h {
            return None;
        }
        let n = tb.list.len().max(1);
        let idx = ((x / (self.width as f32 / n as f32)) as usize).min(n - 1);
        let target = tb.list.get(idx)?.page_path.clone();
        if target == self.current_route() {
            return None;
        }
        Some(NavigationRequest::SwitchTab { url: target })
    }

    fn modal_press(&mut self, _x: f32, _y: f32) -> bool {
        self.modal.as_ref().map(|m| m.visible).unwrap_or(false)
    }

    fn modal_release(&mut self, x: f32, y: f32) {
        let Some(modal) = self.modal.clone() else { return };
        if !modal.visible {
            return;
        }
        // 按钮区在弹窗底部：左取消右确定（单按钮时整条都是确定）
        let confirm = if modal.show_cancel {
            x > self.width as f32 / 2.0
        } else {
            true
        };
        let _ = y;
        self.app
            .eval(&format!("__handleModalResult({})", if confirm { "true" } else { "false" }))
            .ok();
        self.modal = None;
        self.needs_redraw = true;
    }

    fn apply_interaction(&mut self, res: crate::ui::interaction::InteractionResult) {
        use crate::ui::interaction::InteractionResult as R;
        let (id, kind, value) = match &res {
            R::Focus { id, value, .. } => (id.clone(), "focus", value.clone()),
            R::InputChange { id, value } => (id.clone(), "input", value.clone()),
            R::InputBlur { id, value } => (id.clone(), "blur", value.clone()),
            R::InputConfirm { id, value } => (id.clone(), "confirm", value.clone()),
            _ => return,
        };
        if kind == "focus" {
            crate::renderer::components::reset_cursor_blink();
        }
        // 与桌面窗体同一条派发（`host::tap::dispatch_input_event`）
        let r = self.renderer.take();
        crate::host::tap::dispatch_input_event(&mut self.app, r.as_ref(), &id, kind, &value);
        self.renderer = r;
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
            self.needs_redraw = true;
        }
    }
}
