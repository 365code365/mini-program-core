//! 桌面窗体侧的触摸派发胶水：只负责「时钟」与「派发之后宿主要做什么」。
//!
//! 派发本身（覆盖层 vs 正常流的两套坐标、事件名映射、longpress 的双名字）在
//! [`mini_render::host::input::dispatch_touch_sequence`] —— 与移动端 SDK 同一份。

use crate::app_window;
use crate::print_js_output;
use mini_render::host::input;

impl crate::MiniAppWindow {
    /// 事件对象的 `timeStamp`：页面打开到现在的毫秒数（微信语义）
    pub(crate) fn event_time_ms(&self) -> u64 {
        self.page_opened_at.elapsed().as_millis() as u64
    }

    /// 状态机的时钟（进程启动至今毫秒），与 `timeStamp` 分开：
    /// 换页会重置 `timeStamp`，但触摸序列的计时不能因此错乱。
    pub(crate) fn touch_clock_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }

    /// 把状态机产出的事件派发给逻辑层，并在**每次**派发之后取一次脏标记与导航请求
    /// （挪到批次外面的话，同一批事件里第二个处理函数的导航会盖掉第一个）。
    pub(crate) fn dispatch_touch_events(
        &mut self,
        outs: &[app_window::touch::TouchOut],
        x: f32,
        y: f32,
    ) {
        let id = self.touch.identifier();
        let time_ms = self.event_time_ms();
        let scroll_pos = self.scroll.get_position();
        // 逐字段借出：`app` 要可变借给派发，其余字段在回调里更新
        let Self {
            app,
            renderer,
            needs_redraw,
            page_data_dirty,
            fixed_dirty,
            pending_navigation,
            ..
        } = self;
        let sent = input::dispatch_touch_sequence(
            app,
            renderer.as_ref(),
            outs,
            (x, y),
            scroll_pos,
            id,
            time_ms,
            |app| {
                print_js_output(app);
                // 处理函数里可能 setData / 导航
                if app.take_data_dirty() {
                    *needs_redraw = true;
                    *page_data_dirty = true;
                    *fixed_dirty = true;
                }
                if pending_navigation.is_none() {
                    *pending_navigation = app_window::check_navigation(app);
                }
            },
        );
        // 长按派发到了处理函数：抬手不再有 tap（与 SDK 同一条规则，见 TouchTracker::note_long_handled）
        if sent && outs.contains(&app_window::touch::TouchOut::LongPress) {
            self.touch.note_long_handled();
        }
    }

    /// 拖着的 slider 换了值：派发 `changing`（派发本身与 SDK 共用 `host::slider_input`）
    pub(crate) fn dispatch_slider_changing(&mut self) {
        let (id, time_ms) = (self.touch.identifier(), self.event_time_ms());
        mini_render::host::slider_input::dispatch_changing(
            &mut self.app, self.renderer.as_ref(), &self.interaction, id, time_ms,
        );
        self.after_slider_event();
    }

    /// 松手 / 取消：结束 slider 拖动（拖过就发 `change`）。返回这次触摸是不是一次滑块拖动。
    pub(crate) fn finish_slider_drag(&mut self) -> bool {
        let (id, time_ms) = (self.touch.identifier(), self.event_time_ms());
        let dragged = mini_render::host::slider_input::finish(
            &mut self.app, self.renderer.as_ref(), &mut self.interaction, id, time_ms,
        );
        self.after_slider_event();
        dragged
    }

    /// 处理函数里可能 setData / 导航（与 `dispatch_touch_events` 的回调同一套收尾）
    fn after_slider_event(&mut self) {
        print_js_output(&self.app);
        if self.app.take_data_dirty() {
            self.needs_redraw = true;
            self.page_data_dirty = true;
            self.fixed_dirty = true;
        }
        if self.pending_navigation.is_none() {
            self.pending_navigation = app_window::check_navigation(&mut self.app);
        }
    }

    /// 滚动是否已经接管这次触摸 —— 接管方要给触摸序列补一次 `touchcancel`
    pub(crate) fn scroll_took_over(&self) -> bool {
        input::scroll_took_over(&self.scroll, &self.interaction, self.scroll_pos_at_press)
    }
}
