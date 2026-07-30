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
use crate::host::picker_sheet;
use crate::host::ui_overlay;
use crate::host::touch::TouchOut;
use crate::ui::interaction::KeyInput;
use std::collections::HashMap;

impl MiniEngine {
    /// 手指按下（逻辑坐标，左上原点）。
    ///
    /// 与桌面窗体走同一条链路（`host::input`）：覆盖层优先、触摸序列先起、
    /// 手势归属留给第一次明显位移决定。从前这里是一条**退化实现** ——
    /// 直接 `scroll.begin_drag()` 把位移全给页面，于是横向卡片列表吞掉整页纵滑、
    /// 内层列表滚到底也不交棒、`catchtouchmove` 的遮罩锁不住页面。
    pub fn pointer_down(&mut self, x: f32, y: f32) {
        // picker 面板在最上层，先于弹窗/页面吃事件
        if picker_sheet::on_press(&mut self.picker_sheet, x, y) {
            self.needs_redraw = true;
            return;
        }
        if self.modal.as_ref().map(|m| m.visible).unwrap_or(false) {
            let vp = self.viewport_logical();
            let sf = self.dpr;
            let Self { modal, text, .. } = self;
            if ui_overlay::modal_press(modal, vp, sf, text.as_deref(), x, y) {
                self.needs_redraw = true;
            }
            return;
        }
        if self.loading.as_ref().map(|l| l.visible).unwrap_or(false) {
            return;
        }

        let now = self.mono_ms();
        let has_tabbar = self.is_tabbar_page(&self.current_route());
        let tabbar_y = if has_tabbar {
            self.height as f32 - crate::host::tabbar_height() as f32
        } else {
            self.height as f32
        };
        if has_tabbar && y >= tabbar_y {
            // tabBar 不属于页面：不建手势、不滚页面、不命中页面里的元素。
            // 但**触摸序列必须照常起来** —— 否则抬手时状态机产不出 Tap，tabBar 点不动。
            self.scroll_pos_at_press = self.scroll.get_position();
            let outs = self.touch.press(x, y, now);
            self.dispatch_touch(&outs, x, y);
            return;
        }

        // 惯性滚动中按下：先停住，并且**这一下不算点击**（iOS/微信一致）
        self.tap_stops_fling = input::stop_running_flings(&mut self.scroll, &mut self.interaction);
        if self.tap_stops_fling {
            self.needs_redraw = true;
        }
        self.scroll_pos_at_press = self.scroll.get_position();
        let outs = self.touch.press(x, y, now);
        self.dispatch_touch(&outs, x, y);

        let on_fixed = self
            .renderer
            .as_ref()
            .map(|r| r.fixed_layer_hit(x, y))
            .unwrap_or(false);
        let actual_y = y + self.scroll.get_position();
        // 命中可点元素 → 进按压态（`:active` / `hover-class` 靠它生效），滚动区除外
        let hit = if on_fixed {
            self.interaction.hit_test_fixed(x, y).cloned()
        } else {
            self.interaction
                .hit_test(x, y)
                .or_else(|| self.interaction.hit_test(x, actual_y))
                .cloned()
        };
        if let Some(el) = hit {
            use crate::ui::interaction::InteractionType;
            if !el.disabled && el.interaction_type != InteractionType::ScrollArea {
                self.interaction.set_button_pressed(el.id.clone(), el.bounds);
                self.needs_redraw = true;
            }
            if el.interaction_type == InteractionType::Slider && !el.disabled {
                let ty = if el.is_fixed { y } else { actual_y };
                if let Some(r) = self.interaction.handle_click(x, ty) {
                    self.apply_interaction(r);
                }
                return;
            }
        }

        self.gesture = Some(input::begin_gesture(
            &self.interaction,
            &self.scroll,
            self.renderer.as_ref(),
            x,
            y,
            on_fixed,
            self.page_stack.len(),
            // 侧滑返回还没接（要给被覆盖的页留视口图），先不开
            true,
        ));
        self.needs_redraw = true;
    }

    /// 手指移动
    pub fn pointer_move(&mut self, x: f32, y: f32) {
        let now = self.mono_ms();
        // 顺序要紧：**手势推进不能挂在「触摸序列还活着」上**。滚动接管的那一刻我们会
        // 补一次 touchcancel，状态机随即不活跃；要是先判 is_active 再往下走，
        // 后面所有移动都不会再滚 —— 一次 240px 的拖动只滚了第一步的 30px。
        if self.touch.is_active() {
            let outs = self.touch.move_to(x, y, now);
            self.dispatch_touch(&outs, x, y);
        }
        // 手势仲裁决定这次位移归谁：内层 scroll-view / 页面 / 谁也不动
        let mut edge_back = None;
        let eff = input::apply_gesture_move(
            &mut self.gesture,
            &mut self.interaction,
            &mut self.scroll,
            &mut edge_back,
            self.width as f32,
            x,
            y,
            now,
        );
        if eff.needs_redraw {
            self.needs_redraw = true;
        }
        if eff.page_scrolled {
            // 页面滚动只换上屏切片，不重绘（见 GestureEffect 注释）；
            // 但 SDK 目前每帧整帧重绘，所以这里仍要出帧
            self.needs_redraw = true;
        }
        // 被滚动接管的那一刻补一次 touchcancel（手势竞争的失败方要能收尾）
        if self.touch.is_active()
            && input::scroll_took_over(&self.scroll, &self.interaction, self.scroll_pos_at_press)
        {
            let outs = self.touch.cancel();
            self.dispatch_touch(&outs, x, y);
        }
    }

    /// 手指抬起
    pub fn pointer_up(&mut self, x: f32, y: f32) {
        if picker_sheet::on_release(&mut self.picker_sheet, &mut self.app, x, y) {
            self.page_data_dirty = true;
            self.needs_redraw = true;
            return;
        }
        if self.modal.as_ref().map(|m| m.visible).unwrap_or(false) {
            let vp = self.viewport_logical();
            let sf = self.dpr;
            let Self { modal, app, text, .. } = self;
            ui_overlay::modal_release(modal, app, vp, sf, text.as_deref(), x, y);
            self.needs_redraw = true;
            return;
        }

        let now = self.mono_ms();
        if let Some(r) = self.interaction.handle_mouse_release() {
            self.apply_interaction(r);
        }
        input::end_drags(
            &mut self.interaction,
            &mut self.scroll,
            &mut None,
            &mut self.gesture,
        );
        let outs = self.touch.release(x, y, now);
        self.touch.finish();
        self.dispatch_touch(&outs, x, y);
        // tap 只在「没被判成滑动、没被滚动接管、也不是用来停惯性」时才有
        let stopped_fling = std::mem::take(&mut self.tap_stops_fling);
        if outs.contains(&TouchOut::Tap) && !stopped_fling {
            self.handle_click(x, y);
        }
        self.needs_redraw = true;
    }

    /// 手势被系统打断（来电、返回手势接管…）
    pub fn pointer_cancel(&mut self) {
        let outs = self.touch.cancel();
        if let Some((x, y)) = self.touch.start_pos() {
            self.dispatch_touch(&outs, x, y);
        }
        input::end_drags(
            &mut self.interaction,
            &mut self.scroll,
            &mut None,
            &mut self.gesture,
        );
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
        if self.picker_sheet.as_ref().map(|s| s.visible).unwrap_or(false) {
            return;
        }
        if self.modal.as_ref().map(|m| m.visible).unwrap_or(false) {
            return;
        }
        if self.loading.as_ref().map(|l| l.visible).unwrap_or(false) {
            return;
        }
        // 点在 `<picker>` 上：弹面板，不再走普通内容点击
        if let Some(sheet) =
            picker_sheet::open_if_hit(self.renderer.as_ref(), self.scroll.get_position(), x, y)
        {
            self.picker_sheet = Some(sheet);
            self.needs_redraw = true;
            return;
        }
        let route = self.current_route();
        let has_tabbar = self.is_tabbar_page(&route);
        let tabbar_y = self.height as f32 - crate::host::tabbar_height() as f32;
        // 底部 tabBar 命中优先（它盖在页面之上）：自定义组件式与原生配置式都要认
        if has_tabbar && y >= tabbar_y {
            let custom = if self.custom_tabbar.is_some() {
                self.tabbar_renderer.as_ref()
            } else {
                None
            };
            if let Some(nav) =
                crate::host::tabbar::nav_at(x, y - tabbar_y, custom, self.config.tab_bar.as_ref(), &route)
            {
                self.apply_navigation(nav);
            }
            return;
        }
        let scroll = self.scroll.get_position();
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

    /// 视口的逻辑尺寸（弹窗等覆盖层的居中算式要用）
    pub(crate) fn viewport_logical(&self) -> (f32, f32) {
        (self.width as f32, self.height as f32)
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
