//! 出帧闸门与帧内推进：这一帧要不要画、画多大范围、画完谁该被通知
//!
//! 从 `src/bin/window.rs` 拆出来的一片（`impl crate::MiniAppWindow`）。**纯搬迁**，
//! 一行逻辑没改；判据是 65 张画廊图与逐页整帧快照逐字节不变。
#![allow(clippy::too_many_arguments)]
use super::*;
use crate::*;

/// scroll-view 滚动带来的重绘范围
pub(crate) enum AreaDamage {
    /// 没有 scroll-view 动过
    Nothing,
    /// 只需重画这块（物理像素，页面画布坐标）
    Rect(GeoRect),
    /// 动的是覆盖层里的、或本帧不知道位置的：只能整帧
    Full,
}

impl crate::MiniAppWindow {
    /// 是否存在持续动画/需要连续帧的状态（滚动、惯性、视频、光标闪烁、定时器、弹层等）。
    /// 用于决定事件循环是「按刷新率连续出帧」还是「空闲休眠（0 CPU）」。
    pub(crate) fn is_animating(&self) -> bool {
        let scrolling = self.scroll.is_animating() || self.scroll.is_dragging;
        let sv_scroll = self.interaction.scroll_controllers.values().any(|c| c.is_animating() || c.is_dragging);
        // CSS @keyframes / switch 拨动过渡：渲染器在上一帧求值时标记，
        // 有动画在跑就继续按刷新率出帧（否则动画只会画出第一帧然后停住）
        let css_anim = self.renderer.as_ref().map(|r| r.has_active_animations()).unwrap_or(false)
            || self.tabbar_renderer.as_ref().map(|r| r.has_active_animations()).unwrap_or(false);
        scrolling
            || sv_scroll
            || css_anim
            // 侧滑返回的收尾动画（推出去/滑回来）要连续出帧
            || self.edge_back.as_ref().map(|e| e.is_settling()).unwrap_or(false)
            || self.interaction.has_focused_input()
            || self.interaction.press_feedback_pending()
            || self.pull_refreshing // 指示器要持续转
            // 有自动播放的 swiper：即使页面没有 JS 定时器也要按刷新率醒着，
            // 否则到点该翻页时没人来推进它的状态
            || mini_render::renderer::components::has_autoplay_swiper()
            || self.app.has_active_timers()
            // 有请求在飞：必须继续出帧，否则响应回来了没人去取（页面永远停在加载态）
            || self.app.has_pending_network()
            // 有远程图片在下载：同理，下完要有一帧把它画出来
            || mini_render::renderer::components::image_net::has_pending()
            || self.toast.as_ref().map(|t| t.visible).unwrap_or(false)
            || self.loading.as_ref().map(|l| l.visible).unwrap_or(false)
            || self.modal.as_ref().map(|m| m.visible).unwrap_or(false)
            // picker 面板入场/退场动画期间要连续出帧
            || self.picker_sheet.as_ref().map(|s| s.animating()).unwrap_or(false)
            || mini_render::renderer::components::has_playing_video()
    }

    /// 光标闪烁的驱动：相位一翻转就安排一次重绘（含 fixed 覆盖层）。
    ///
    /// 只在**翻转的那一帧**标脏，不是每帧都标 —— 覆盖层重画一次要走一遍
    /// `render_fixed_elements`，按刷新率重画纯属浪费（一秒不到两次就够了）。
    pub(crate) fn caret_blink_tick(&mut self) {
        if !self.interaction.has_focused_input() {
            return;
        }
        let now_visible = mini_render::renderer::components::cursor_blink_visible();
        if now_visible != self.caret_visible {
            self.caret_visible = now_visible;
            self.needs_redraw = true;
            // 输入框可能在覆盖层里，那层得跟着重画
            self.fixed_dirty = true;
        }
    }

    /// 告诉系统输入法「光标在哪」，候选词面板才会贴着光标弹出来。
    ///
    /// 不设的话 macOS/Windows 会把候选面板摆在一个默认位置（实测离输入框很远，
    /// 中文输入时候选条飘在输入框下面老远的地方）。
    pub(crate) fn sync_ime_cursor_area(&self) {
        let (Some(window), Some(input)) = (self.window.as_ref(), self.interaction.focused_input.as_ref())
        else { return };
        let sf = self.scale_factor as f32;
        // 光标矩形是绘制期记下的（页面画布的设备像素坐标）；拿不到就退回输入框整体
        let (x, y, w, h) = match mini_render::renderer::components::last_caret_rect() {
            Some((cx, cy, cw, ch)) => (cx / sf, cy / sf, cw.max(1.0) / sf, ch / sf),
            None => (input.bounds.x, input.bounds.y, input.bounds.width, input.bounds.height),
        };
        // 覆盖层钉在视口上，不减滚动；正常流的元素是内容坐标，要减
        let y = if input.is_fixed { y } else { y - self.scroll.get_position() };
        window.set_ime_cursor_area(
            winit::dpi::LogicalPosition::new(x as f64, y as f64),
            winit::dpi::LogicalSize::new(w as f64, h as f64),
        );
    }

    /// 按闸门决定这一帧要不要重绘，以及走整帧还是损伤区。
    ///
    /// - `structural`：数据/滚动/交互引起的变化，必须整帧重绘。
    /// - `css_anim`：只有 CSS/JS 动画在跑，走损伤区：只清并只画动画元素的包围盒。
    ///   一个 `infinite` 的小徽标不该逼着整屏每帧重新光栅化 ——
    ///   浏览器靠图层合成避免这件事，这里用裁剪矩形达到同样效果。
    /// - 正常流的 scroll-view 滚了：它的盒子就是损伤区（再并上动画的损伤区）。
    pub(crate) fn render_frame_if_needed(&mut self, structural: bool, css_anim: bool) {
        let area = if structural { AreaDamage::Nothing } else { self.scroll_area_damage() };
        if !(structural || css_anim) && matches!(area, AreaDamage::Nothing) {
            return;
        }
        let damage = if structural {
            None
        } else {
            match area {
                AreaDamage::Full => None,
                AreaDamage::Rect(r) if css_anim => {
                    self.animation_damage_rect().map(|a| Self::union_of(a, r))
                }
                AreaDamage::Rect(r) => Some(r),
                AreaDamage::Nothing => self.animation_damage_rect(),
            }
        };
        self.render_with_damage(damage);
        self.needs_redraw = false;
        // 光标位置是绘制期记下的，所以放在这一帧画完之后同步给输入法
        self.sync_ime_cursor_area();
    }

    /// 跑一帧「逻辑 + 渲染 + 上屏」，与交互窗体 `RedrawRequested` 走同一套闸门。
    /// 供 `--frames` 用：局部重绘这类只在连续出帧时才暴露的问题，单帧快照测不到。
    pub(crate) fn pump_one_frame(&mut self) {
        self.app.update().ok();
        // 与交互窗体的 RedrawRequested 一致：每帧把逻辑层的输出冲出来。
        // 少了这一句，定时器 / 网络回调里的 console.log 全都攒在缓冲里看不到 ——
        // 排查时会误判成「回调没跑」。
        print_js_output(&self.app);
        if self.app.take_data_dirty() {
            self.needs_redraw = true;
            self.page_data_dirty = true;
        self.fixed_dirty = true;
        }
        let mut pull_req: Option<bool> = None;
        evt::process_ui_events(&mut self.app, &mut self.toast, &mut self.loading, &mut self.modal, &mut pull_req);
        self.apply_pull_down_request(pull_req);
        self.update_scroll();
        if self.scroll.take_pull_trigger() && self.page_enables_pull_down() {
            self.set_pull_refreshing(true);
        }
        // 与交互窗体一致：每帧取一次导航请求并执行。
        // 少了这一步，`--frames` 跑到的「倒计时结束自动进首页」只会在日志里出现，
        // 页面其实没换 —— 快照就测不到这类靠时间驱动的跳转。
        if self.pending_navigation.is_none() {
            self.pending_navigation = app_window::check_navigation(&mut self.app);
        }
        self.process_navigation();
        self.reap_picker_sheet();
        if mini_render::renderer::components::advance_due_swipers() {
            self.needs_redraw = true;
        }
        if self.app.take_network_dirty() {
            self.needs_redraw = true;
            self.page_data_dirty = true;
        }
        if mini_render::renderer::components::image_net::take_dirty() {
            self.needs_redraw = true;
            self.fixed_dirty = true;
            self.force_full_redraw = true;
        }
        // 长按由帧驱动（同 RedrawRequested）：无头链路也要能测到 longpress
        if self.touch.is_active() {
            let clock = self.touch_clock_ms();
            let outs = self.touch.tick(clock);
            let (mx, my) = self.mouse_pos;
            self.dispatch_touch_events(&outs, mx, my);
        }
        if !self.viewport_inside_drawn_band() {
            self.needs_redraw = true;
        }
        self.caret_blink_tick();
        // 页面滚动与 scroll-view 滚动都不进 structural（理由见 RedrawRequested 里的同名判断）
        let css_anim = self.renderer.as_ref().map(|r| r.has_active_animations()).unwrap_or(false);
        let structural = self.needs_redraw
            || mini_render::renderer::components::has_playing_video()
            || self.interaction.has_focused_input()
            || self.interaction.press_feedback_pending()
            || mini_render::renderer::components::swiper_needs_frame();
        self.render_frame_if_needed(structural, css_anim);
    }

    /// 上一帧动画元素包围盒的并集，作为本帧的损伤区。
    ///
    /// 返回 None 表示「不值得做局部重绘」：没有记录、或并集已经占到视口的三成以上
    /// （那时局部重绘省不下多少，还要多付一次裁剪判断）。
    pub(crate) fn animation_damage_rect(&self) -> Option<GeoRect> {
        if self.fixed_layer_animates {
            return None; // 覆盖层在动，必须整帧重绘
        }
        let bounds = &self.last_animated_bounds;
        if bounds.is_empty() {
            return None;
        }
        let (mut x0, mut y0) = (f32::MAX, f32::MAX);
        let (mut x1, mut y1) = (f32::MIN, f32::MIN);
        for r in bounds {
            x0 = x0.min(r.x);
            y0 = y0.min(r.y);
            x1 = x1.max(r.x + r.width);
            y1 = y1.max(r.y + r.height);
        }
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        let sf = self.scale_factor as f32;
        let viewport_area = (LOGICAL_WIDTH as f32 * sf) * (LOGICAL_HEIGHT as f32 * sf);
        if (x1 - x0) * (y1 - y0) > viewport_area * 0.3 {
            return None;
        }
        Some(GeoRect::new(x0, y0, x1 - x0, y1 - y0))
    }

    /// 相对上一次画出时，哪些 scroll-view 的滚动位置变了。
    ///
    /// scroll-view 的内容是按自身偏移画进页面画布的，它一动就得重画 ——
    /// 但只需要重画**它自己的盒子**，外面的像素一点没变。从前拖动/甩动内层列表时
    /// 每帧整条带（视口 ± 400px）重新光栅化，分类页那种左右两栏都是列表的页面
    /// 滑动时每帧 7ms 以上。
    pub(crate) fn scroll_area_damage(&self) -> AreaDamage {
        let sf = self.scale_factor as f32;
        let mut acc: Option<GeoRect> = None;
        let mut any_moved = false;
        for (id, c) in &self.interaction.scroll_controllers {
            let pos = c.get_position();
            let moved = self
                .drawn_area_positions
                .get(id)
                .map(|p| (p - pos).abs() > 0.001)
                .unwrap_or(true);
            if !moved {
                continue;
            }
            any_moved = true;
            // 覆盖层里的要连覆盖层一起重画；本帧没登记（滚出视口了）的不知道画在哪
            let Some((b, false)) = self.interaction.scroll_area_bounds(id) else {
                return AreaDamage::Full;
            };
            let r = GeoRect::new(b.x * sf, b.y * sf, b.width * sf, b.height * sf);
            acc = Some(match acc {
                Some(a) => Self::union_of(a, r),
                None => r,
            });
        }
        // `MINI_NO_DAMAGE=1`：一律整帧，用来做「局部重绘 vs 整帧重绘逐像素一致」对照
        if any_moved && self.no_damage {
            return AreaDamage::Full;
        }
        acc.map(AreaDamage::Rect).unwrap_or(AreaDamage::Nothing)
    }

    /// 最近是否发生过滚动（用于决定裁剪余量留多大）
    pub(crate) fn scroll_recent(&self) -> bool {
        self.last_scroll_at
            .map(|t| t.elapsed() < Duration::from_millis(400))
            .unwrap_or(false)
    }

    /// 上屏要用的那段画布是否已经画过（画布之外的行由上屏填背景色，不算未绘制）
    pub(crate) fn viewport_inside_drawn_band(&self) -> bool {
        let Some((band_top, band_bottom)) = self.drawn_band else { return false };
        let Some(canvas) = &self.canvas else { return false };
        let sf = self.scale_factor as f32;
        let has_tabbar = self
            .page_stack
            .last()
            .map(|p| self.is_tabbar_page(&p.path))
            .unwrap_or(false);
        let viewport = (LOGICAL_HEIGHT - if has_tabbar { tabbar_height() } else { 0 }) as f32;
        let top = (self.scroll.get_position() * sf).max(0.0);
        let bottom = (self.scroll.get_position() * sf + viewport * sf).min(canvas.height() as f32);
        top >= band_top - 0.01 && bottom <= band_bottom + 0.01
    }
}
