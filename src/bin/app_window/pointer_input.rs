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
            if has_tabbar && y >= tabbar_y { return; }

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

            // ── 拖动的归属**不在按下时决定** ──
            //
            // 微信/浏览器都是等第一次明显位移、按主方向定：横向 scroll-view 里竖着划
            // 应该滚页面，纵向列表里横着划则谁也不动。按下就把手势交给命中的容器
            // （旧实现）会让横滑卡片吃掉整页的纵向滑动。
            let candidate = self.scroll_candidate_at(x, y, on_fixed_layer);
            let catch_move = self
                .renderer
                .as_ref()
                .map(|r| {
                    let hy = if on_fixed_layer { y } else { actual_y };
                    r.has_catch_for(x, hy, "touchmove")
                })
                .unwrap_or(false);
            let page_scrollable = !on_fixed_layer
                && !self.interaction.is_dragging_slider()
                && self.scroll.get_max_scroll() > 0.5;
            // 左边缘侧滑返回：起点在触发区、栈里还有上一页、且没盖着覆盖层。
            // 栈底（tab 首页）没有这个手势 —— 微信里在首页往右划什么也不会发生。
            let allow_edge_back = app_window::edge_back::EdgeBack::at_edge(x)
                && self.page_stack.len() > 1
                && !on_fixed_layer
                && self.edge_back.is_none();
            self.gesture = Some(
                app_window::gesture::DragGesture::new((x, y), candidate, catch_move, page_scrollable)
                    .allow_edge_back(allow_edge_back),
            );
    }

    /// 按下点处最内层的可滚区域（id + 它的滚动轴），供方向锁定决策
    pub(crate) fn scroll_candidate_at(
        &self,
        x: f32,
        y: f32,
        on_fixed_layer: bool,
    ) -> Option<(String, app_window::gesture::Axis)> {
        use mini_render::ui::scroll_controller::ScrollDirection;
        let actual_y = y + self.scroll.get_position();
        // 要的是**最内层的可滚区域**，不是最上层的元素：卡片/按钮盖在 scroll-view 上面时
        // 普通命中测试返回的是卡片，于是真正装内容的 scroll-view 永远得不到手势。
        let el = if on_fixed_layer {
            self.interaction.hit_test_scroll_area(x, y, true).cloned()
        } else {
            self.interaction
                .hit_test_scroll_area(x, actual_y, false)
                .or_else(|| self.interaction.hit_test_scroll_area(x, y, false))
                .cloned()
        }?;
        let axis = match self.interaction.get_scroll_controller(&el.id).map(|c| c.get_direction()) {
            Some(ScrollDirection::Horizontal) => app_window::gesture::Axis::Horizontal,
            Some(ScrollDirection::Vertical) => app_window::gesture::Axis::Vertical,
            // 控制器还没建出来时按元素声明的方向
            None if el.is_horizontal => app_window::gesture::Axis::Horizontal,
            None => app_window::gesture::Axis::Vertical,
        };
        Some((el.id, axis))
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
            
            self.interaction.clear_button_pressed();
            self.fixed_dirty = true;
            let was_sel = self.interaction.is_dragging_selection();
            self.interaction.end_text_selection();
            if was_sel { self.needs_redraw = true; if let Some(w) = &self.window { w.request_redraw(); } return; }
            
            if let Some(id) = self.interaction.dragging_scroll_area.take() {
                if let Some(c) = self.interaction.get_scroll_controller_mut(&id) { c.end_drag(); }
                self.needs_redraw = true;
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            
            if let Some(r) = self.interaction.handle_mouse_release() {
                handle_interaction_result(&r, self.window.as_ref(), self.renderer.as_ref(), &mut self.app, &mut self.clipboard, self.scroll.get_position(), self.scale_factor);
            }
            
            let anim = self.scroll.end_drag();
            // 侧滑返回：松手进收尾动画（推到底真返回 / 不够阈值滑回去），
            // 由 tick_edge_back 逐帧推进
            if let Some(eb) = &mut self.edge_back { eb.release(); }
            self.gesture = None;
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
        let mut stopped = false;
        if self.scroll.is_animating() {
            self.scroll.stop();
            stopped = true;
        }
        for c in self.interaction.scroll_controllers.values_mut() {
            if c.is_animating() {
                c.stop();
                stopped = true;
            }
        }
        if stopped {
            self.needs_redraw = true;
        }
        stopped
    }

    /// 把手势仲裁的结果落到滚动控制器上（方向锁定 + 嵌套传递都在这里生效）
    pub(crate) fn apply_gesture_move(&mut self, x: f32, y: f32) {
        use app_window::gesture::{Axis, GestureAction};
        let ts = self.touch_clock_ms();
        let Some(mut g) = self.gesture.take() else { return };
        let before_target = g.target().clone();
        let mut action = g.on_move(x, y);
        if before_target != *g.target() && std::env::var("MINI_SCROLL_LOG").is_ok() {
            eprintln!("🖐 手势归属：{:?} -> {:?}", before_target, g.target());
        }
        loop {
            match action {
                GestureAction::None => break,
                GestureAction::BeginArea { ref id, axis } => {
                    let (sx, sy) = g.start();
                    // 这个 scroll-view 根本没有滚动控制器（内容不足一屏，压根不用滚）：
                    // 直接把手势交给页面，否则手指划在它上面时整页都不动 ——
                    // 「短列表挡住整页滚动」正是这么来的。
                    let cannot_scroll = self
                        .interaction
                        .get_scroll_controller(id)
                        .map(|c| c.get_max_scroll() <= 0.5)
                        .unwrap_or(true);
                    if cannot_scroll {
                        if std::env::var("MINI_SCROLL_LOG").is_ok() {
                            let max = self
                                .interaction
                                .get_scroll_controller(id)
                                .map(|c| c.get_max_scroll());
                            eprintln!("🖐 {} 不可滚（max_scroll={:?}）→ 交给页面", id, max);
                        }
                        action = g.handoff_to_page();
                        continue;
                    }
                    if let Some(c) = self.interaction.get_scroll_controller_mut(id) {
                        // 从**按下点**开始拖：锁定那一刻内容不该跳一下
                        c.begin_drag(if axis == Axis::Horizontal { sx } else { sy }, ts);
                        c.update_drag(if axis == Axis::Horizontal { x } else { y }, ts);
                    }
                    self.interaction.dragging_scroll_area = Some(id.clone());
                    self.needs_redraw = true;
                    break;
                }
                GestureAction::UpdateArea { ref id, axis } => {
                    if let Some(c) = self.interaction.get_scroll_controller_mut(id) {
                        c.update_drag(if axis == Axis::Horizontal { x } else { y }, ts);
                    }
                    // 内层到边界后还在往同一方向推 → 把这次手势交给页面继续
                    // （手指往下推时内容已经到顶、往上推时已经到底）。
                    // 不能用「位置没变」判断：控制器有橡皮筋越界，到边界后位置照样在动。
                    let (_, dy) = g.delta();
                    let keep_pushing = self
                        .interaction
                        .get_scroll_controller(id)
                        .map(|c| {
                            c.get_max_scroll() <= 0.5
                                || (c.is_at_top() && dy > 0.0)
                                || (c.is_at_bottom() && dy < 0.0)
                        })
                        .unwrap_or(true);
                    self.needs_redraw = true;
                    if axis == Axis::Vertical && keep_pushing {
                        if let Some(id) = self.interaction.dragging_scroll_area.take() {
                            if let Some(c) = self.interaction.get_scroll_controller_mut(&id) {
                                c.end_drag();
                            }
                        }
                        action = g.handoff_to_page();
                        continue;
                    }
                    break;
                }
                GestureAction::BeginPage => {
                    let (_, ly) = g.last();
                    self.scroll.begin_drag(ly, ts);
                    self.scroll.update_drag(y, ts);
                    // 注意：**页面滚动不置 needs_redraw**。整页内容已经画在长画布上，
                    // 滚动只是换一条切片上屏；置脏会让每一帧都整页重绘
                    // （实测首页拖动 3.7ms → 6.5ms，等于把「滚动不重绘」这条优化废掉）。
                    self.last_scroll_at = Some(Instant::now());
                    break;
                }
                GestureAction::UpdatePage => {
                    self.scroll.update_drag(y, ts);
                    self.last_scroll_at = Some(Instant::now());
                    break;
                }
                GestureAction::BeginEdgeBack => {
                    // 手势升级成侧滑返回：先把可能已经开始的滚动收掉，
                    // 否则页面会一边被推出去一边继续上下滚。
                    if let Some(id) = self.interaction.dragging_scroll_area.take() {
                        if let Some(c) = self.interaction.get_scroll_controller_mut(&id) { c.end_drag(); }
                    }
                    self.scroll.end_drag();
                    let (sx, _) = g.start();
                    let mut eb = app_window::edge_back::EdgeBack::new(sx, ts, LOGICAL_WIDTH as f32);
                    eb.on_move(x, ts);
                    self.edge_back = Some(eb);
                    // 注意：**不置 needs_redraw**。页面内容一点没变，变的只是上屏时
                    // 整帧往右挪多少（present 每帧都跑），置脏等于每帧白重画一次整页。
                    break;
                }
                GestureAction::UpdateEdgeBack => {
                    if let Some(eb) = &mut self.edge_back { eb.on_move(x, ts); }
                    break;
                }
            }
        }
        self.gesture = Some(g);
    }
}
