//! 指针输入层：把窗口/脚本的指针事件变成微信语义的触摸序列、手势与侧滑返回。
//!
//! 这一层从 `window.rs` 里拆出来，原因不只是行数：**无头输入与交互窗体必须共用
//! 同一条链路**。此前两边各写一套，结果无头链路驱动的是 `scroll-view`、
//! 真机驱动的是页面滚动 —— 无头全绿而手上不对，测的根本不是同一套东西。
//! 现在窗体的 `MouseInput`/`CursorMoved` 与 CLI 的 `--touch/--swipe/--drag`
//! 都只走 `on_pointer_press` / `on_pointer_move` / `on_pointer_release`。
//!
//! 判定逻辑本身都在纯逻辑模块里，这里只负责把它们接到宿主状态上：
//! - [`super::touch`]：触摸状态机与事件对象（slop / longpress / touchcancel）
//! - [`super::gesture`]：拖动归属仲裁（方向锁定 / 嵌套传递 / catchtouchmove）
//! - [`super::edge_back`]：左边缘侧滑返回（跟手位移 / 阈值判定 / 上一页合成）

use super::*;
use crate::app_window;
use crate::app_window::event_handler as evt;
use mini_render::host::input;
use std::time::Instant;

impl crate::MiniAppWindow {
    /// 模拟按下（与交互窗体同一条链路）
    pub(crate) fn sim_press(&mut self, x: f32, y: f32) {
        self.mouse_pos = (x, y);
        self.on_pointer_press(x, y);
    }

    /// 模拟移动
    pub(crate) fn sim_move(&mut self, x: f32, y: f32) {
        self.on_pointer_move(x, y);
    }

    /// 模拟抬起
    pub(crate) fn sim_release(&mut self, x: f32, y: f32) {
        self.mouse_pos = (x, y);
        self.on_pointer_release(x, y);
    }

    /// 指针按下：交互窗体与无头模拟共用这一条链路（此前两边各写一套，
    /// 于是「无头能过、真机不一样」）。
    pub(crate) fn on_pointer_press(&mut self, x: f32, y: f32) {
        let ts = self.touch_clock_ms();
        let _ = ts;

            self.click_start_pos = self.mouse_pos;
            self.click_start_time = Instant::now();
            
            // picker 面板在最上层，先于弹窗/页面吃事件
            if self.handle_picker_sheet_press(x, y) { return; }
            if self.modal.as_ref().map(|m| m.visible).unwrap_or(false) { self.handle_modal_press(x, y); return; }
            if self.loading.as_ref().map(|l| l.visible).unwrap_or(false) { return; }
            
            let has_tabbar = self.page_stack.last().map(|p| self.is_tabbar_page(&p.path)).unwrap_or(false);
            let tabbar_y = if has_tabbar { (LOGICAL_HEIGHT - tabbar_height()) as f32 } else { LOGICAL_HEIGHT as f32 };
            if has_tabbar && y >= tabbar_y {
                // tabBar 不属于页面：不建手势、不滚页面、不去命中页面里的元素。
                //
                // 但**触摸序列必须照常起来**。从前这里直接 `return`，`self.touch.press()`
                // 根本没执行，于是抬手时 `touch.release()` 产不出 `Tap`，
                // `on_pointer_release` 里那句 `if Tap { self.handle_click(..) }` 也就不会走
                // —— 真机上 tabBar 完全点不动（sample-app / news-app 都是 app.json 里
                // 声明 tabBar 的页面，命中这条；tea-app 的 tabBar 是页面内的自定义组件，
                // `is_tabbar_page` 为假，所以那边一直正常，看着像只有这两个 app 有问题）。
                //
                // 这条 bug 无头测不出来：`--click` 直接调 `handle_click`，绕过了整个
                // 指针层。tabBar 的点击回归必须用 `--touch`（走 press/release）才测得到。
                self.scroll_pos_at_press = self.scroll.get_position();
                let clock = self.touch_clock_ms();
                let outs = self.touch.press(x, y, clock);
                self.dispatch_touch_events(&outs, x, y);
                return;
            }

            // 惯性滚动中按下：先把它停住，并且**这一下不算点击** ——
            // iOS/微信里滑动的列表点一下只是停住，不会激活那一项。
            self.tap_stops_fling = self.stop_running_flings();
            // 触摸序列先起来：`touchstart` 要在宿主决定怎么处理这次按下**之前**
            // 就派发出去（微信语义），页面自己实现的手势才拿得到起点。
            self.scroll_pos_at_press = self.scroll.get_position();
            let clock = self.touch_clock_ms();
            let outs = self.touch.press(x, y, clock);
            self.dispatch_touch_events(&outs, x, y);

            let actual_y = y + self.scroll.get_position();
            // 落在 `position: fixed` 覆盖层上（弹窗/遮罩）：按压与拖动都不允许穿透。
            // 覆盖层内部自己的可交互元素（fixed 的滚动区、按钮）由下面的
            // hit_test 分支处理 —— 它本来就先查 fixed 元素。
            let on_fixed_layer = self
                .renderer
                .as_ref()
                .map(|r| r.fixed_layer_hit(x, y))
                .unwrap_or(false);
            
            // 输入框内点击
            if let Some(focused) = &self.interaction.focused_input {
                let b = focused.bounds;
                if (x >= b.x && x <= b.x + b.width && y >= b.y - self.scroll.get_position() && y <= b.y + b.height - self.scroll.get_position()) ||
                   (x >= b.x && x <= b.x + b.width && actual_y >= b.y && actual_y <= b.y + b.height) {
                    if let Some(tr) = &self.text_renderer {
                        let sf = self.scale_factor as f32;
                        let cw: Vec<f32> = focused.value.chars().map(|c| tr.measure_text(&c.to_string(), 16.0 * sf)).collect();
                        let cp = mini_render::ui::interaction::calculate_cursor_position(&focused.value, &cw, (x - b.x) * sf, 12.0 * sf, focused.text_offset);
                        self.interaction.prepare_text_selection(cp);
                        self.needs_redraw = true;
                        if let Some(w) = &self.window { w.request_redraw(); }
                        return;
                    }
                }
            }
            
            // 交互元素
            // 覆盖层之上：只允许命中覆盖层自己的元素（is_fixed），
            // 下层页面的元素一律不参与，避免「弹窗弹着还能按到底下的商品」
            let hit = if on_fixed_layer {
                // 只在覆盖层自己的元素里找（不能全局 hit_test 再过滤 —— 见 hit_test_fixed）
                self.interaction.hit_test_fixed(x, y).cloned()
            } else {
                self.interaction.hit_test(x, y).or_else(|| self.interaction.hit_test(x, actual_y)).cloned()
            };
            if let Some(el) = hit {
                use mini_render::ui::interaction::InteractionType;
                // 任何可点元素都进入按压态（`:active` / `hover-class` 靠它生效），
                // 滚动区域除外 —— 那是拖动不是按压
                if !el.disabled && el.interaction_type != InteractionType::ScrollArea {
                    self.interaction.set_button_pressed(el.id.clone(), el.bounds);
                    self.needs_redraw = true;
                    self.fixed_dirty = true; // 按压的可能是覆盖层里的元素
                }
                match el.interaction_type {
                    InteractionType::Slider if !el.disabled => {
                        let ty = if el.is_fixed { y } else { actual_y };
                        if let Some(r) = self.interaction.handle_click(x, ty) {
                            handle_interaction_result(&r, self.window.as_ref(), self.renderer.as_ref(), &mut self.app, &mut self.clipboard, self.scroll.get_position(), self.scale_factor);
                            self.needs_redraw = true;
                            if let Some(w) = &self.window { w.request_redraw(); }
                        }
                        return;
                    }
                    InteractionType::Button if !el.disabled => {
                        self.interaction.set_button_pressed(el.id.clone(), el.bounds);
                        self.needs_redraw = true;
                        if let Some(w) = &self.window { w.request_redraw(); }
                    }
                    _ => {}
                }
            }

            // 拖动的归属**不在按下时决定**，等第一次明显位移再按主方向锁定
            // （判定与移动端 SDK 共用 host::input::begin_gesture）
            self.gesture = Some(input::begin_gesture(
                &self.interaction,
                &self.scroll,
                self.renderer.as_ref(),
                x,
                y,
                on_fixed_layer,
                self.page_stack.len(),
                self.edge_back.is_some(),
            ));
    }

    /// 指针抬起
    pub(crate) fn on_pointer_release(&mut self, x: f32, y: f32) {

            // Released
            // picker 面板在最上层，先于弹窗/页面吃事件
            if self.handle_picker_sheet_release(x, y) { return; }
            if self.modal.as_ref().map(|m| m.visible && m.pressed_button.is_some()).unwrap_or(false) {
                self.handle_modal_release(x, y);
                return;
            }
            
            self.fixed_dirty = true;
            let was_sel = self.interaction.is_dragging_selection();
            self.interaction.end_text_selection();
            if was_sel {
                self.interaction.clear_button_pressed();
                self.needs_redraw = true;
                if let Some(w) = &self.window { w.request_redraw(); }
                return;
            }

            if let Some(r) = self.interaction.handle_mouse_release() {
                handle_interaction_result(&r, self.window.as_ref(), self.renderer.as_ref(), &mut self.app, &mut self.clipboard, self.scroll.get_position(), self.scale_factor);
            }

            // 拖动收尾（按压态 / scroll-view / 页面 / 侧滑）与 SDK 共用 host::input
            let anim = input::end_drags(
                &mut self.interaction,
                &mut self.scroll,
                &mut self.edge_back,
                &mut self.gesture,
            );
            // 触摸序列收尾：`touchend` 照常派发；tap 只在「没移动过、没被滚动接管、
            // 也不是用来停惯性」时才有 —— **不再有 300ms 上限**
            // （微信里按住两秒再松手同样是一次 tap，旧实现按久一点就点不动，
            // 手上就是「点了没反应」）。
            let clock = self.touch_clock_ms();
            let outs = self.touch.release(x, y, clock);
            self.touch.finish();
            self.dispatch_touch_events(&outs, x, y);
            let stopped_fling = std::mem::take(&mut self.tap_stops_fling);
            if outs.contains(&app_window::touch::TouchOut::Tap) && !stopped_fling {
                self.handle_click(x, y);
            }
            
            self.needs_redraw = true;
            if let Some(w) = &self.window { w.request_redraw(); }
            if anim { if let Some(w) = &self.window { w.request_redraw(); } }
    }

    /// 指针移动：文本选择/滑块等交给交互层，滚动交给手势仲裁，再派发 touchmove。
    pub(crate) fn on_pointer_move(&mut self, x: f32, y: f32) {
        self.mouse_pos = (x, y);
        // 文本选择、滑块拖动这些「非滚动」的拖拽仍由交互层处理；
        // 滚动不再由它推进（改由手势仲裁决定谁滚、往哪滚）
        if evt::handle_cursor_moved(
            x, y, &mut self.interaction, &mut self.scroll, self.text_renderer.as_deref(),
            self.window.as_ref(), self.renderer.as_ref(), &mut self.app, &mut self.clipboard,
            self.scale_factor,
        ) {
            self.needs_redraw = true;
        }
        self.apply_gesture_move(x, y);
        // 触摸序列：先 `touchmove`，如果这一下已经被滚动接管，再补一次
        // `touchcancel`（手势竞争的失败方要收到 cancel，页面自己的拖拽才知道收尾）
        if self.touch.is_active() {
            let clock = self.touch_clock_ms();
            let outs = self.touch.move_to(x, y, clock);
            self.dispatch_touch_events(&outs, x, y);
            if self.scroll_took_over() {
                let outs = self.touch.cancel();
                self.dispatch_touch_events(&outs, x, y);
            }
        }
    }

    /// 停掉正在跑的惯性/回弹（页面与所有 scroll-view）。返回是否真的停了什么。
    pub(crate) fn stop_running_flings(&mut self) -> bool {
        let stopped = input::stop_running_flings(&mut self.scroll, &mut self.interaction);
        if stopped {
            self.needs_redraw = true;
        }
        stopped
    }

    /// 把手势仲裁的结果落到滚动控制器上（算法在 host::input，两端共用）
    pub(crate) fn apply_gesture_move(&mut self, x: f32, y: f32) {
        let ts = self.touch_clock_ms();
        let eff = input::apply_gesture_move(
            &mut self.gesture,
            &mut self.interaction,
            &mut self.scroll,
            &mut self.edge_back,
            LOGICAL_WIDTH as f32,
            x,
            y,
            ts,
        );
        // 只推动了 scroll-view：不置整帧重绘，出帧闸门按位置变化只重画那一块
        if eff.needs_redraw && !eff.area_scrolled {
            self.needs_redraw = true;
        }
        if eff.page_scrolled {
            // 页面滚动**不置 needs_redraw**（见 GestureEffect 的注释），只记时刻：
            // 裁剪余量按「最近是否在滚」自适应
            self.last_scroll_at = Some(Instant::now());
        }
    }
}
