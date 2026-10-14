//! [`MiniEngine`] 的出帧：`pump` 推进一帧 → 按需重画页面条带 / 覆盖层 / tabBar → 合成。
//!
//! 从 `engine.rs` 按职责拆出来的一片（`impl MiniEngine`）。**纯搬迁**，一行逻辑没改；
//! 判据是 `tools/sdk-parity.sh` 与 sample / news 逐页快照逐字节不变。
//!
//! 「什么时候只重新合成、什么时候整条带重画」的判断都在这里：视口还落在
//! `drawn_band` 内时滚动只换上屏切片（`compose_current`），滑出才重画。
use super::engine::MiniEngine;
use crate::compositor::{cpu, FrameLayers, OverlayRefs};
use crate::host::tabbar_height;
use crate::{Canvas, Color};

impl MiniEngine {
    /// 跑一帧：逻辑层 → 事件 → 渲染 → 合成。返回是否产出了新画面。
    ///
    /// `now_ms` 用宿主的单调时钟（iOS `CACurrentMediaTime`、Android
    /// `System.nanoTime`）。传 0 表示让引擎自己取。
    pub fn pump(&mut self, now_ms: u64) -> bool {
        let now = if now_ms > 0 { now_ms } else { self.mono_ms() };
        let dt = if self.last_pump_ms == 0 {
            0.016
        } else {
            ((now.saturating_sub(self.last_pump_ms)) as f32 / 1000.0).clamp(0.001, 0.1)
        };
        self.last_pump_ms = now;

        self.app.update().ok();
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
            self.needs_redraw = true;
        }
        self.drain_ui_events();
        // 滚动推进（惯性/回弹），到底事件回调页面
        let (moved, event) = self.scroll.update_with_events(dt);
        if moved {
            self.scroll_moved = true;
        }
        // scroll-view 的惯性与回弹（与桌面 `update_scroll` 一致）：
        // 不推进的话甩一下列表，松手就停在原地。
        for c in self.interaction.scroll_controllers.values_mut() {
            if c.update(dt) {
                self.needs_redraw = true;
            }
        }
        if let Some(crate::ui::scroll_controller::ScrollEvent::ReachBottom) = event {
            self.app
                .eval("if(__currentPage&&__currentPage.onReachBottom)__currentPage.onReachBottom()")
                .ok();
        }
        // 逻辑层发起的路由
        if let Some(nav) = self.take_navigation() {
            self.apply_navigation(nav);
        }
        if crate::renderer::components::advance_due_swipers() {
            self.needs_redraw = true;
        }
        if self.app.take_network_dirty() || crate::renderer::components::image_net::take_dirty() {
            self.page_data_dirty = true;
            self.needs_redraw = true;
        }
        // 长按由帧驱动
        if self.touch.is_active() {
            let outs = self.touch.tick(now);
            if let Some((x, y)) = self.touch.pos() {
                self.dispatch_touch(&outs, x, y);
            }
        }
        // picker 面板的入场/退场动画按帧推进，走完就回收
        if self.picker_sheet.as_ref().map(|s| s.animating()).unwrap_or(false) {
            self.needs_redraw = true;
        }
        if crate::host::picker_sheet::reap(&mut self.picker_sheet) {
            self.needs_redraw = true;
        }
        self.caret_tick();
        if self.interaction.press_feedback_pending() {
            self.needs_redraw = true;
        }
        let scroll_moved = std::mem::take(&mut self.scroll_moved);
        if self.needs_redraw || self.interaction.has_focused_input() {
            self.render();
        } else if scroll_moved {
            // 页面画布用内容坐标：视口还在已绘制条带内时，滚动只是换一段切片上屏
            if self.viewport_inside_drawn_band() {
                self.compose_current();
            } else {
                self.render();
            }
        } else {
            return false;
        }
        self.needs_redraw = false;
        true
    }

    /// 本帧上屏要用的那段页面画布是否已经画过
    fn viewport_inside_drawn_band(&self) -> bool {
        let Some((band_top, band_bottom)) = self.drawn_band else { return false };
        let top = (self.scroll.get_position() * self.dpr).max(0.0);
        let bottom = (self.scroll.get_position() * self.dpr + self.viewport_height() * self.dpr)
            .min(self.canvas.read().height() as f32);
        top >= band_top - 0.01 && bottom <= band_bottom + 0.01
    }

    fn page_background(&self) -> Color {
        self.renderer
            .as_ref()
            .and_then(|r| r.page_style().background)
            .unwrap_or(Color::from_hex(0xF5F5F5))
    }

    /// 不重画任何画布，只按当前滚动位置重新合成一帧
    fn compose_current(&mut self) {
        let has_tabbar = self.is_tabbar_page(&self.current_route());
        let bg = self.page_background();
        self.compose_frame(has_tabbar, bg);
    }

    /// 只清理并重画「视口 ± 裁剪余量」这条带（渲染器的视口裁剪用的是同一个余量），
    /// 返回内容高度。整页画布可能上万像素高，全量 clear 是纯浪费。
    fn render_page_band(&mut self, scroll: f32, vp: f32, bg: Color) -> f32 {
        let margin = crate::renderer::VIEWPORT_CULL_MARGIN_PX;
        let band_y0 = (scroll * self.dpr - margin).floor() as i32;
        let band_y1 = (scroll * self.dpr + vp * self.dpr + margin).ceil() as i32;
        let Self { renderer, canvas, page_stack, page_data, interaction, .. } = self;
        // 声明这一帧写的是哪条带（以后的 GPU 镜像按它补传），再清屏 + 画
        let canvas = canvas.write_band(band_y0, band_y1);
        canvas.clear_band(band_y0, band_y1, bg);
        let (Some(r), Some(page)) = (renderer.as_mut(), page_stack.last()) else { return 0.0 };
        let content_h =
            r.render_with_scroll_and_viewport(canvas, &page.wxml_nodes, page_data, interaction, scroll, vp);
        let canvas_h = self.canvas.read().height() as f32;
        self.drawn_band = Some(((band_y0 as f32).max(0.0), (band_y1 as f32).min(canvas_h)));
        content_h
    }

    fn caret_tick(&mut self) {
        if !self.interaction.has_focused_input() {
            return;
        }
        let vis = crate::renderer::components::cursor_blink_visible();
        if vis != self.caret_visible {
            self.caret_visible = vis;
            self.needs_redraw = true;
        }
    }

    /// 渲染 + 合成到内部缓冲
    pub(crate) fn render(&mut self) {
        let Some(page) = self.page_stack.last() else { return };
        let route = page.path.clone();
        if self.page_data_dirty {
            self.page_data = self
                .app
                .eval("__getRenderData()")
                .map(|s| serde_json::from_str(&s).unwrap_or(serde_json::json!({})))
                .unwrap_or(serde_json::json!({}));
            self.page_data_dirty = false;
        }
        let has_tabbar = self.is_tabbar_page(&route);
        let vp = self.viewport_height();
        let bg = self.page_background();
        let content_h = self.render_page_band(self.scroll.get_position(), vp, bg);
        if content_h > 1.0 {
            self.scroll.update_content_height(content_h, vp);
            // 画布按内容高度分配（与桌面窗体一致）：固定高度的画布装不下长页面，
            // 滚到画布之外的部分上屏时只剩一片底色。
            let required = (content_h * self.dpr).ceil() as u32;
            if required > 0 && required != self.canvas.read().height() {
                let width = self.canvas.read().width();
                self.canvas.replace(Canvas::new(width, required));
                self.render_page_band(self.scroll.get_position(), vp, bg);
            }
        }
        {
            let Self { renderer, fixed_canvas, page_stack, page_data, interaction, .. } = self;
            let fixed_canvas = fixed_canvas.write();
            fixed_canvas.clear(Color::new(0, 0, 0, 0));
            if let (Some(r), Some(page)) = (renderer.as_mut(), page_stack.last()) {
                r.render_fixed_elements(fixed_canvas, &page.wxml_nodes, page_data, interaction, vp);
            }
        }
        // 自定义 tabBar
        if has_tabbar {
            let selected = self
                .config
                .tab_bar
                .as_ref()
                .and_then(|tb| tb.list.iter().position(|i| i.page_path == route));
            // 三个字段各自可变/不可变借用，互不重叠
            if let (Some(ct), Some(tr)) = (self.custom_tabbar.as_ref(), self.tabbar_renderer.as_mut())
            {
                let tabbar_canvas = self.tabbar_canvas.write();
                tabbar_canvas.clear(Color::new(0, 0, 0, 0));
                let mut d = ct.data.clone();
                if let Some(idx) = selected {
                    d["selected"] = serde_json::json!(idx);
                }
                tr.render(tabbar_canvas, &ct.wxml_nodes, &d);
            }
        }
        // 覆盖层真正画过的行区间：`present_to_buffer` 只合成这几行，
        // 传 None 会**整层跳过** —— 底部吸底操作栏、弹窗遮罩就全丢了。
        self.fixed_rows = crate::host::render::opaque_row_span(self.fixed_canvas.read());
        self.compose_frame(has_tabbar, bg);
    }

    fn compose_frame(&mut self, has_tabbar: bool, bg: Color) {
        let (pw, ph) = ((self.width as f32 * self.dpr) as u32, (self.height as f32 * self.dpr) as u32);
        let n = (pw as usize) * (ph as usize);
        if self.compose.len() != n {
            self.compose = vec![0u32; n];
        }
        let bg_u32 = ((bg.r as u32) << 16) | ((bg.g as u32) << 8) | bg.b as u32;
        // 合成序列与桌面窗体同一份（compositor::cpu）。SDK 没有下拉指示器与侧滑返回这两层
        let layers = FrameLayers {
            width: pw,
            height: ph,
            scale: self.dpr,
            page: &self.canvas,
            scroll_px: (self.scroll.get_position() * self.dpr) as i32,
            page_bg: bg_u32,
            has_tabbar,
            tabbar: if has_tabbar { Some(&self.tabbar_canvas) } else { None },
            tabbar_px: if has_tabbar { (tabbar_height() as f32 * self.dpr) as u32 } else { 0 },
            fixed: Some(&self.fixed_canvas),
            fixed_rows: self.fixed_rows,
            pull: None,
            overlays: OverlayRefs {
                last_frame: std::time::Instant::now(),
                toast: &self.toast,
                loading: &self.loading,
                modal: &self.modal,
                // picker 面板画在最上层（与桌面窗体同一份绘制）
                picker: self.picker_sheet.as_ref(),
                text: self.text.as_deref(),
            },
            edge_back: None,
        };
        cpu::compose(&mut self.compose[..], &layers);
        // u32(0xRRGGBB) → RGBA8
        if self.rgba.len() != n * 4 {
            self.rgba = vec![255u8; n * 4];
        }
        for (dst, px) in self.rgba.chunks_exact_mut(4).zip(&self.compose) {
            dst.copy_from_slice(&[(px >> 16) as u8, (px >> 8) as u8, *px as u8, 255]);
        }
    }

    /// 当前帧的 RGBA8 像素（宽 = 逻辑宽 × dpr）
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    pub fn pixel_size(&self) -> (u32, u32) {
        (
            (self.width as f32 * self.dpr) as u32,
            (self.height as f32 * self.dpr) as u32,
        )
    }
}
