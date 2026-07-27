//! 把触摸状态机产出的事件派发给逻辑层，并判断「滚动是否已经接管这次触摸」。
//!
//! 事件对象由 **Rust 侧**构造后交给 `__dispatchEvent`，而不是在 JS 里拼：
//! `touches` 的坐标、`target` 与 `currentTarget` 的区分只有渲染层知道
//! （谁被摸到、谁挂着处理函数，是两棵不同的节点）。
//!
//! 覆盖层与正常流是两套坐标，所以派发也要分开问：覆盖层用视口坐标且命中即止
//! （弹窗遮罩之上的触摸不许穿透），正常流要加上页面滚动偏移。

use crate::app_window;
use crate::print_js_output;
use serde_json::json;

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

    /// 把状态机产出的事件派发给逻辑层。
    ///
    /// 覆盖层与正常流是两套坐标：覆盖层用视口坐标且**命中即到此为止**
    /// （弹窗遮罩之上的触摸不许穿透到下层页面），正常流要加上滚动偏移。
    pub(crate) fn dispatch_touch_events(&mut self, outs: &[app_window::touch::TouchOut], x: f32, y: f32) {
        use app_window::touch::TouchOut;
        if outs.is_empty() {
            return;
        }
        let on_fixed = self
            .renderer
            .as_ref()
            .map(|r| r.fixed_layer_hit(x, y))
            .unwrap_or(false);
        let (hit_y, scope) = if on_fixed {
            (y, Some(true))
        } else {
            (y + self.scroll.get_position(), Some(false))
        };
        let id = self.touch.identifier();
        let time_ms = self.event_time_ms();
        for out in outs {
            // `tap` 由既有的点击链路处理（它还要管按压态、picker、输入框等）
            let names: &[&str] = match out {
                TouchOut::Start => &["touchstart"],
                TouchOut::Move => &["touchmove"],
                TouchOut::End => &["touchend"],
                TouchOut::Cancel => &["touchcancel"],
                // 微信同时派发新旧两个名字
                TouchOut::LongPress => &["longpress", "longtap"],
                TouchOut::Tap => &[],
            };
            for name in names {
                let detail = if *name == "longpress" || *name == "longtap" {
                    json!({ "x": x, "y": y })
                } else {
                    json!({})
                };
                let dispatched = match self.renderer.as_ref() {
                    Some(r) => app_window::touch::dispatch_to_js(
                        &mut self.app, r, name, (x, hit_y), (x, y), scope, id, time_ms, detail,
                    ),
                    None => false,
                };
                if dispatched {
                    print_js_output(&self.app);
                    // 处理函数里可能 setData / 导航
                    if self.app.take_data_dirty() {
                        self.needs_redraw = true;
                        self.page_data_dirty = true;
                        self.fixed_dirty = true;
                    }
                    if self.pending_navigation.is_none() {
                        self.pending_navigation = app_window::check_navigation(&mut self.app);
                    }
                }
            }
        }
    }

    /// 滚动是否已经接管这次触摸（页面滚动位置变了，或某个 scroll-view 正在被拖）。
    /// 接管方要给触摸序列发 `touchcancel` —— 与浏览器/Skyline 的手势竞争一致。
    pub(crate) fn scroll_took_over(&self) -> bool {
        if (self.scroll.get_position() - self.scroll_pos_at_press).abs() > 0.5 {
            return true;
        }
        self.interaction
            .dragging_scroll_area
            .as_ref()
            .and_then(|id| self.interaction.get_scroll_controller(id))
            .map(|c| c.is_dragging && c.get_position() > 0.0 || c.is_animating())
            .unwrap_or(false)
    }
}
