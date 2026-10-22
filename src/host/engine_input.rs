//! [`MiniEngine`] 的输入与路由入口。
//!
//! 宿主只送**指针三段**（按下/移动/抬起），`touchstart/touchmove/touchend/
//! longpress/tap` 由引擎的触摸状态机产出 —— 与桌面窗体走同一份 `host::touch`。
//! 千万不要让宿主自己合成「点击」再调过来：那样会绕过状态机，
//! 「按下时提前 return 导致抬手没有 tap」这类 bug 在无头测试里全绿、真机上点不动
//! （这个仓库踩过一次，见 `doc/引擎测试说明.md` T2-6）。

use super::engine::MiniEngine;
use crate::host::input;
use crate::host::picker_sheet;
use crate::host::touch::TouchOut;
use crate::ui::interaction::KeyInput;

impl MiniEngine {
    /// 手指按下（逻辑坐标，左上原点）。
    ///
    /// 与桌面窗体走同一条链路（`host::input`）：覆盖层优先、触摸序列先起、
    /// 手势归属留给第一次明显位移决定。从前这里是一条**退化实现** ——
    /// 直接 `scroll.begin_drag()` 把位移全给页面，于是横向卡片列表吞掉整页纵滑、
    /// 内层列表滚到底也不交棒、`catchtouchmove` 的遮罩锁不住页面。
    pub fn pointer_down(&mut self, x: f32, y: f32) {
        // 页面转场中：这串触摸整串吞掉（微信同样在转场里不响应点击，route_anim.rs）
        if self.route.swallow_down() {
            return;
        }
        // 宿主浮层（picker 面板、全屏视频、带蒙层的提示框、Modal）先吃，见 engine_overlay_input.rs
        if self.overlay_pointer_down(x, y) {
            return;
        }
        // 引擎画的导航栏（返回键）：画面坐标，之后一律换成页面视口坐标（engine_navbar.rs）
        if self.nav_pointer_down(x, y) {
            return;
        }
        let y = y - self.nav_height();

        let now = self.mono_ms();
        let has_tabbar = self.is_tabbar_page(&self.current_route());
        let tabbar_y = if has_tabbar {
            self.page_window_height() - self.tabbar_h()
        } else {
            self.page_window_height()
        };
        if has_tabbar && y >= tabbar_y {
            // tabBar 不属于页面：不建手势、不滚页面、不命中页面里的元素。
            // 但**触摸序列必须照常起来** —— 否则抬手时状态机产不出 Tap，tabBar 点不动。
            self.scroll_pos_at_press = self.scroll.get_position();
            let outs = self.touch.press(x, y, now);
            self.dispatch_touch(&outs, x, y);
            return;
        }

        self.resample_release();
        // 惯性滚动中按下：先停住，并且**这一下不算点击**（iOS/微信一致）
        self.tap_stops_fling = input::stop_running_flings(&mut self.scroll, &mut self.interaction);
        if self.tap_stops_fling {
            // 停住只是位置不再变：按当前位置重新合成就够（停住的 scroll-view 由 engine_damage
            // 按位置比出来）。从前这里整页重画 —— 惯性里按一下接着拖，按下那一帧就卡
            self.dmg.compose_dirty = true;
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
            // slider 不在按下时抢走触摸：横向锁定才交给它（手势仲裁），竖着划照样滚页面
            if !el.disabled && el.interaction_type != InteractionType::ScrollArea {
                // 点击态要等 hover-start-time 才出现，出现那一帧只重画这个元素（engine_press.rs）
                self.interaction.set_button_pressed(el.id.clone(), el.bounds);
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
            // 侧滑返回（engine_edge_back.rs）：上一次的还在收尾时不再起新的
            self.edge_back.is_some(),
        ));
        // 建手势本身不改任何像素：按压态、停惯性、touchstart 里的 setData 上面各自标过脏。
        // 从前这里无条件 `needs_redraw` —— 每一次按下都整帧重画一次，滑动的第一帧必卡
    }

    /// 手指移动
    pub fn pointer_move(&mut self, x: f32, y: f32) {
        // 不带时间戳的移动：不重采样（`pointer_move_at` 转进来的除外，engine_resample.rs）
        self.resample_untimed_move();
        if self.route.swallowing() || self.overlay_pointer_move(x, y) || self.nav_pointer_move(x, y) {
            return;
        }
        let y = y - self.nav_height();
        let now = self.mono_ms();
        // 移出元素就撤点击态；重画只画它自己（engine_press.rs 下一帧比对出来）
        self.interaction.update_press_position(x, y, y + self.scroll.get_position());
        // 顺序要紧：**手势推进不能挂在「触摸序列还活着」上**。滚动接管的那一刻我们会
        // 补一次 touchcancel，状态机随即不活跃；要是先判 is_active 再往下走，
        // 后面所有移动都不会再滚 —— 一次 240px 的拖动只滚了第一步的 30px。
        if self.touch.is_active() {
            let outs = self.touch.move_to(x, y, now);
            self.dispatch_touch(&outs, x, y);
        }
        // 手势仲裁决定这次位移归谁：内层 scroll-view / 页面 / 谁也不动
        let eff = input::apply_gesture_move(
            &mut self.gesture,
            &mut self.interaction,
            &mut self.scroll,
            &mut self.edge_back,
            self.width as f32,
            x,
            y,
            now,
        );
        // 只推动了 scroll-view：不置整帧，出帧时按位置变化只重画它自己的盒子（engine_damage）
        if eff.needs_redraw && !eff.area_scrolled {
            self.mark_full();
        }
        if eff.slider_changed {
            let (tid, t) = (self.touch.identifier(), self.mono_ms());
            crate::host::slider_input::dispatch_changing(&mut self.app, self.renderer.as_ref(), &self.interaction, tid, t);
            if self.app.take_data_dirty() {
                self.page_data_dirty = true;
            }
        }
        if eff.page_scrolled {
            // 页面滚动只换上屏切片，不重绘（见 GestureEffect 注释）
            self.scroll_moved = true;
        }
        // 侧滑返回跟手：页面一个像素不变，只是上屏时整帧右移，重新合成就够
        if self.edge_back.is_some() {
            self.dmg.compose_dirty = true;
        }
        // 被滚动接管的那一刻补一次 touchcancel（手势竞争的失败方要能收尾）
        if self.touch.is_active()
            && input::scroll_took_over(&self.scroll, &self.interaction, self.scroll_pos_at_press)
        {
            let outs = self.touch.cancel();
            self.dispatch_touch(&outs, x, y);
            // 页面动了，这一下不再是点击：点击态立刻撤（不等手指移出元素，也不留 hover-stay-time）
            self.interaction.cancel_press();
        }
    }

    /// 手指抬起
    pub fn pointer_up(&mut self, x: f32, y: f32) {
        if self.route.swallow_end() || self.overlay_pointer_up(x, y) || self.nav_pointer_up(x, y) {
            return;
        }
        let y = y - self.nav_height();

        // 松手要重画的只有按压态（`:active` / `hover-class` 要褪掉）与 scroll-view 拖拽收尾。
        // 从前这里无条件 `needs_redraw`：每一次滑动松手都整帧重画，正好砸在惯性的第一帧上
        // 正在拖的 swiper 松手要起吸附 / 翻页动画：动画由绘制推进，这一帧不画它就停在半路
        // 点击态褪去不在这里整页重画：engine_press.rs 在它真正褪掉的那一帧只重画那个元素
        let had_press = self.interaction.dragging_scroll_area.is_some()
            || self.interaction.dragging_slider.is_some()
            || crate::renderer::components::swiper_is_dragging();
        let now = self.mono_ms();
        // 拖过 slider：松手发 change，并且这一下不再算 tap（见 slider_input::finish）
        let slider_dragged = crate::host::slider_input::finish(
            &mut self.app, self.renderer.as_ref(), &mut self.interaction, self.touch.identifier(), now,
        );
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
        }
        // 页面停在屏上那一帧的位置再松手（重采样落后手指约一帧，见 engine_resample.rs）
        self.resample_release();
        input::end_drags(
            &mut self.interaction,
            &mut self.scroll,
            &mut self.edge_back,
            &mut self.gesture,
        );
        let outs = self.touch.release(x, y, now);
        self.touch.finish();
        self.dispatch_touch(&outs, x, y);
        // tap 只在「没被判成滑动、没被滚动接管、也不是用来停惯性」时才有
        let stopped_fling = std::mem::take(&mut self.tap_stops_fling);
        if outs.contains(&TouchOut::Tap) && !stopped_fling && !slider_dragged {
            self.handle_click(x, y);
        }
        if had_press {
            self.mark_full();
        }
    }

    /// 手势被系统打断（来电、返回手势接管…）
    pub fn pointer_cancel(&mut self) {
        if self.route.swallow_end() || self.nav_pointer_cancel() { return; } // 转场中 / 导航栏上的那串
        if self.capsule_cancel() { return; } // 按在胶囊上的那串（engine_capsule.rs）
        // 与按下/抬起对称：面板弹着时这串触摸根本没进页面的状态机
        if picker_sheet::on_cancel(&mut self.picker_sheet) {
            self.dmg.compose_dirty = true;
            return;
        }
        let outs = self.touch.cancel();
        if let Some((x, y)) = self.touch.start_pos() {
            self.dispatch_touch(&outs, x, y);
        }
        // 拖到一半被打断：值已经变了，照样发 change 让页面知道
        let (tid, now) = (self.touch.identifier(), self.mono_ms());
        crate::host::slider_input::finish(&mut self.app, self.renderer.as_ref(), &mut self.interaction, tid, now);
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
        }
        // 页面停在屏上那一帧的位置再松手（重采样落后手指约一帧，见 engine_resample.rs）
        self.resample_release();
        input::end_drags(
            &mut self.interaction,
            &mut self.scroll,
            &mut self.edge_back,
            &mut self.gesture,
        );
        self.touch.finish();
    }

    /// 滚轮 / 触控板（桌面与带鼠标的平板）
    pub fn wheel(&mut self, delta_y: f32, precise: bool) {
        self.scroll.handle_scroll(delta_y, precise);
        self.scroll_moved = true;
    }

    /// 往当前聚焦的输入框送文字（输入法提交的整串也走这里）。
    ///
    /// 整串恰好是一个换行（`"\n"` / `"\r\n"`）时当作回车键：textarea 换行、input 触发 confirm ——
    /// iOS 的 `insertText("\n")`、Android 硬件键盘与 Flutter 多行输入的回车都是这样送进来的。
    /// 其余控制字符丢掉（画出来是豆腐块），多行里的换行保留。
    pub fn text_input(&mut self, text: &str) {
        if !self.interaction.has_focused_input() {
            return;
        }
        for r in self.interaction.insert_text(text) {
            self.apply_interaction(r);
        }
        self.mark_full();
    }

    /// 功能键，名字见 [`crate::ui::interaction::KEY_NAMES`]：`enter` `newline` `backspace`
    /// `delete` `left` `right` `up` `down` `home` `end`、`shift+…` 扩展选区、`word-left` /
    /// `word-right`、`doc-start` / `doc-end`、`delete-word-back`、`selectall` `copy` `cut`
    /// `undo` `redo` `escape`。不认识的名字忽略。
    pub fn key(&mut self, name: &str) {
        let Some(input) = KeyInput::from_name(name) else {
            return;
        };
        if let Some(r) = self.interaction.handle_key_input(input) {
            self.apply_interaction(r);
        }
        self.mark_full();
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
            ..
        } = self;
        // touchmove 里 setData（跟手的拖拽条、滑块）每一步都会走到这里：只标数据脏，
        // 重画多大交给失效分析，不再每一步整帧
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
        if self.tip_blocks_input() {
            return;
        }
        // 点在 `<picker>` 上：弹面板，不再走普通内容点击
        if let Some(mut sheet) =
            picker_sheet::open_if_hit(self.renderer.as_ref(), self.scroll.get_position(), x, y)
        {
            sheet.viewport = (self.width as f32, self.height as f32);
            self.picker_sheet = Some(sheet);
            self.mark_full();
            return;
        }
        let route = self.current_route();
        let has_tabbar = self.is_tabbar_page(&route);
        // 页面视口坐标（导航栏下沿为 0）
        let tabbar_y = self.page_window_height() - self.tabbar_h();
        // 底部 tabBar 命中优先（它盖在页面之上）：自定义组件式与原生配置式都要认
        if has_tabbar && y >= tabbar_y {
            let custom = if self.custom_tabbar.is_some() {
                self.tabbar_renderer.as_ref()
            } else {
                None
            };
            if let Some(nav) =
                crate::host::tabbar::nav_at_w(
                    x,
                    y - tabbar_y,
                    custom,
                    self.config.tab_bar.as_ref(),
                    &route,
                    self.width as f32,
                )
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
        // 整页重画只留给「组件自己的画法变了」的点击：开关 / 单选 / 滑块 / 输入框聚焦。
        // 普通 bindtap 不用：处理函数里的 setData 走失效分析（只画变了的那块），点击态由
        // engine_press.rs 只重画那个元素，弹 ActionSheet / Modal 只重新合成。从前每一下点击都整页重画，
        // 一加 PLY110 上点「ActionSheet」先卡两帧整页（37~41ms）面板才开始往上滑
        let redraw = match &result {
            Some(crate::ui::interaction::InteractionResult::ButtonClick { .. }) | None => {
                // 引擎自己画的 <video>：点一下起播 / 暂停 / 拖进度，控制条的样子变了却没有 setData
                !self.interaction.click_animations.is_empty() || self.tap_on_engine_video(x, y, scroll)
            }
            Some(_) => true,
        };
        if let Some(res) = result {
            self.apply_interaction(res);
        }
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
        }
        if redraw {
            self.mark_full();
        }
    }

    /// 点在引擎自己画的 `<video>` 上（宿主承载的不算：那是宿主的控件）
    fn tap_on_engine_video(&self, x: f32, y: f32, scroll: f32) -> bool {
        let Some(r) = self.renderer.as_ref().filter(|r| !r.is_host_native("video")) else { return false };
        r.query_selector("video", true, &self.interaction).iter().any(|v| {
            let y = if v.is_fixed { y } else { y + scroll };
            v.rect.contains(&crate::Point::new(x, y))
        })
    }

    /// 视口的逻辑尺寸（弹窗等覆盖层的居中算式要用）
    pub(crate) fn viewport_logical(&self) -> (f32, f32) {
        (self.width as f32, self.height as f32)
    }

    fn apply_interaction(&mut self, res: crate::ui::interaction::InteractionResult) {
        use crate::ui::interaction::InteractionResult as R;
        // 复制 / 剪切：引擎里存一份，再发 `clipboard` 事件让宿主写系统剪贴板
        // （与 `wx.setClipboardData` 同一条事件）。从前这两种结果在这里直接丢掉，
        // 剪切还会让值变了却不发 `bindinput`。
        if let R::CopyText { text } | R::CutText { text, .. } = &res {
            self.emit_clipboard(text);
        }
        let (id, kind, value) = match &res {
            R::Focus { id, value, .. } => (id.clone(), "focus", value.clone()),
            R::InputChange { id, value } | R::CutText { id, value, .. } => (id.clone(), "input", value.clone()),
            R::InputBlur { id, value } => (id.clone(), "blur", value.clone()),
            R::InputConfirm { id, value, .. } => (id.clone(), "confirm", value.clone()),
            _ => return,
        };
        if kind == "focus" {
            crate::renderer::components::reset_cursor_blink();
        }
        // 告诉宿主弹 / 收软键盘（事件队列，见 engine_events.rs）。
        // `confirm-hold` 的确认不收键盘：输入框还聚焦着
        match &res {
            R::Focus { id, value, bounds, is_fixed, .. } => self.emit_input_focus(id, value, *bounds, *is_fixed),
            R::InputBlur { id, value } | R::InputConfirm { id, value, keep_focus: false } => {
                self.emit_input_blur(id, value)
            }
            _ => {}
        }
        // 与桌面窗体同一条派发（`host::tap::dispatch_input_event`）
        let r = self.renderer.take();
        crate::host::tap::dispatch_input_event(&mut self.app, r.as_ref(), &id, kind, &value);
        self.renderer = r;
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
            self.mark_full();
        }
    }
}
