//! 整帧渲染与上屏：页面画布 + 覆盖层 + tabBar 的合成，损伤区裁剪，softbuffer 贴图
//!
//! 从 `src/bin/window.rs` 拆出来的一片（`impl crate::MiniAppWindow`）。**纯搬迁**，
//! 一行逻辑没改；判据是 65 张画廊图与逐页整帧快照逐字节不变。
#![allow(clippy::too_many_arguments)]
use super::*;
use crate::*;

impl crate::MiniAppWindow {
    pub(crate) fn render(&mut self) {
        self.render_with_damage(None);
    }

    /// 两个矩形的包围盒
    pub(crate) fn union_of(a: GeoRect, b: GeoRect) -> GeoRect {
        let x0 = a.x.min(b.x);
        let y0 = a.y.min(b.y);
        let x1 = (a.x + a.width).max(b.x + b.width);
        let y1 = (a.y + a.height).max(b.y + b.height);
        GeoRect::new(x0, y0, x1 - x0, y1 - y0)
    }

    /// `damage` 为 `Some(rect)` 时只重绘该矩形（动画帧的局部重绘），
    /// 其余像素保留上一帧结果，并且不更新「已绘制条带」。
    pub(crate) fn render_with_damage(&mut self, damage: Option<GeoRect>) {
        // 调用方给的动画损伤区（可能会被下面并上 setData 的失效范围，先留一份原值）
        let anim_damage_in = damage;
        // 页面数据只在逻辑层 setData 之后才会变，所以只有脏了才做这趟
        // 「JS 侧 JSON.stringify 整份 data → Rust 侧反序列化」往返。
        // 从前每帧都做一次：动画帧里白付一次全量序列化。
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
            // 数据变了 fixed 覆盖层也要重画。这里是所有渲染路径的共同入口，
            // 只在事件循环里标记的话，快照/无窗口路径会漏掉（底部固定栏会整条丢失）。
            self.fixed_dirty = true;
        }
        if self.page_data_dirty {
            self.fixed_dirty = true;
            self.page_data = std::sync::Arc::new(
                self.app
                    .eval("__getRenderData()")
                    .map(|s| serde_json::from_str(&s).unwrap_or(json!({})))
                    .unwrap_or(json!({})),
            );
            self.page_data_dirty = false;
        }
        // Arc 克隆：只加引用计数，不复制整棵 JSON（渲染期同时要可变借用 canvas/renderer）
        let page_data = std::sync::Arc::clone(&self.page_data);
        let page_data = &*page_data;
        let page = match self.page_stack.last() { Some(p) => p, None => return };
        let (current_path, has_tabbar) = (page.path.clone(), self.is_tabbar_page(&page.path));
        let viewport_height = (LOGICAL_HEIGHT - if has_tabbar { tabbar_height() } else { 0 }) as f32;
        let scroll_offset = self.scroll.get_position();
        
        // 只清理「渲染器实际会画的那条带」：视口 ± 裁剪余量。
        // 页面画布是整页高的（首页 6870px），每帧全量 clear 白烧几毫秒。
        let sf = self.scale_factor as f32;
        // 余量是自适应的：滚动时留大一些（少触发重绘，滚动期间大部分帧只做上屏），
        // 静止时留小一些（`setData` 这种偶发整帧重绘只画可见区，帧尖峰更低）。
        let margin = if self.scroll.is_dragging || self.scroll.is_animating() || self.scroll_recent() {
            mini_render::renderer::VIEWPORT_CULL_MARGIN_PX
        } else {
            48.0
        };
        let band_y0 = (scroll_offset * sf - margin).floor() as i32;
        let band_y1 = (scroll_offset * sf + viewport_height * sf + margin).ceil() as i32;

        let t_render_begin = Instant::now();
        // 先把布局算好并问出「这次 setData 只影响哪一块」。必须在清屏之前 ——
        // 清多大是由它决定的。之后的绘制会命中布局缓存，不重复这趟开销。
        // viewport 必须与后面绘制那趟传的完全一致，否则会被判成「视口变了」
        // 而重建第二次布局 —— 既白付一趟开销，算出的失效范围也对不上。
        let plan = self
            .renderer
            .as_mut()
            .map(|r| r.plan_frame(&page.wxml_nodes, page_data, Some((scroll_offset, viewport_height))))
            .unwrap_or(FramePlan::Full);
        // 「有新图到位」这类数据之外的内容变化：本帧强制整帧，不能被增量范围收窄
        let force_full = std::mem::take(&mut self.force_full_redraw);
        // 数据变化的失效范围优先于调用方给的动画损伤区：
        // 这一帧既有 setData 又有动画时，两者的并集才是完整的重绘范围。
        let damage = if force_full { None } else { match plan {
            FramePlan::Unchanged => damage,
            // `MINI_NO_DAMAGE=1` 关掉增量失效，用来做「局部重绘 vs 整帧重绘逐像素一致」对照
            _ if self.no_damage => None,
            FramePlan::Full => None,
            FramePlan::Damage { rect, fixed_changed } => {
                if std::env::var("MINI_LAYOUT_LOG").is_ok() {
                    eprintln!(
                        "🩹 setData 增量重绘 {:.0}x{:.0} @({:.0},{:.0})  覆盖层{}",
                        rect.width, rect.height, rect.x, rect.y,
                        if fixed_changed { "要重画" } else { "复用" }
                    );
                }
                // 覆盖层没变就不用重画它（首页倒计时那种：弹窗内容一动不动，
                // 却要陪着整张覆盖层每秒重画一次，白付 4ms）
                if !fixed_changed {
                    self.fixed_dirty = false;
                }
                if rect.width <= 0.0 || rect.height <= 0.0 {
                    // 渲染结果完全没变（改的字段不参与渲染）
                    damage.or(Some(GeoRect::new(0.0, 0.0, 0.0, 0.0)))
                } else {
                    Some(match damage {
                        None => rect,
                        Some(d) => Self::union_of(d, rect),
                    })
                }
            }
        } };
        // 页面底色以 `page { background-color }` 为准（微信语义），拿不到才退回默认灰。
        // 写死 #F5F5F5 会让在 page 上定义暖色底的应用整体色调都不对。
        let page_bg = self
            .renderer
            .as_ref()
            .and_then(|r| r.page_style().background)
            .unwrap_or(Color::from_hex(0xF5F5F5));
        let mut content_height = 0.0f32;
        if let Some(canvas) = &mut self.canvas {
            match &damage {
                Some(rect) => canvas.clear_area(rect, page_bg),
                None => canvas.clear_band(band_y0, band_y1, page_bg),
            }
            if let Some(renderer) = &mut self.renderer {
                renderer.set_damage_clip(damage);
                content_height = renderer.render_with_scroll_and_viewport(canvas, &page.wxml_nodes, &page_data, &mut self.interaction, scroll_offset, viewport_height);
            }
        }
        if let Some(r) = &mut self.renderer {
            let bounds = r.take_animated_bounds();
            // 只有「这一帧的裁剪范围覆盖了全部动画元素」时，收集到的包围盒才是完整的：
            //   - 整帧重绘：显然完整
            //   - 动画损伤区帧：裁剪范围本来就是由动画包围盒算出来的，完整
            //   - **纯 setData 局部帧**：裁剪范围只是数据变化的那一小块，
            //     落在外面的动画元素这一趟根本没被访问到。此时若拿它覆盖旧记录，
            //     下一个动画帧的损伤区就只剩这一小块 —— 外面的动画会就此冻住。
            let bounds_are_complete = damage.is_none() || anim_damage_in.is_some();
            if bounds_are_complete && (!bounds.is_empty() || damage.is_none()) {
                self.last_animated_bounds = bounds;
            }
        }
        
        if content_height > 0.0 {
            self.scroll.update_content_height(content_height, viewport_height);
            let required_height = (content_height * self.scale_factor as f32).ceil() as u32;
            if self.canvas.as_ref().map(|c| c.height()).unwrap_or(0) != required_height && required_height > 0 {
                self.canvas = Some(Canvas::new((LOGICAL_WIDTH as f64 * self.scale_factor) as u32, required_height));
                if let Some(page) = self.page_stack.last() {
                    if let (Some(canvas), Some(renderer)) = (&mut self.canvas, &mut self.renderer) {
                        canvas.clear_band(band_y0, band_y1, page_bg);
                        renderer.render_with_scroll_and_viewport(canvas, &page.wxml_nodes, &page_data, &mut self.interaction, scroll_offset, viewport_height);
                    }
                }
            }
        }
        
        // `<image bindload>` / `<image binderror>`：图片有结论了就派发到逻辑层。
        // 必须在这里做（每条渲染路径的共同出口），只在事件循环里做的话
        // 快照/无窗口路径会漏掉。
        let img_events = self.renderer.as_mut().map(|r| r.take_image_events()).unwrap_or_default();
        if !img_events.is_empty() {
            let time_ms = self.event_time_ms();
            for e in img_events {
                app_window::dispatch_image_event(&mut self.app, &e, time_ms);
            }
        }

        let t_page_done = Instant::now();
        // 局部重绘帧里 fixed 覆盖层与 tabBar 不会变，直接沿用上一帧的画布 ——
        // 除非覆盖层自己带动画（那时 animation_damage_rect 会拒绝走局部路径）。
        if damage.is_none() && (self.fixed_dirty || self.fixed_layer_animates) {
            self.fixed_dirty = false;
            if let Some(page) = self.page_stack.last() {
                if let (Some(fc), Some(r)) = (&mut self.fixed_canvas, &mut self.renderer) {
                    // 在两趟之间清零，才能分辨「动画在页面里」还是「在 fixed 覆盖层里」
                    let page_anim = r.has_active_animations();
                    r.set_animations_active(false);
                    fc.clear(Color::new(0, 0, 0, 0));
                    r.render_fixed_elements(fc, &page.wxml_nodes, &page_data, &mut self.interaction, viewport_height);
                    let fixed_anim = r.has_active_animations();
                    r.set_animations_active(page_anim || fixed_anim);
                    // 覆盖层自己带动画时，局部重绘会让它停住 —— 禁用局部路径
                    self.fixed_layer_animates = fixed_anim;
                    // 记下覆盖层这一趟产生的事件绑定与遮挡区域：
                    // 事件绑定每帧都会清空重建，而覆盖层在「内容没变」的帧里跳过重绘，
                    // 不缓存的话那些帧里弹窗没有任何绑定，点击会直接穿到下层页面。
                    self.cached_fixed_bindings = r.fixed_event_bindings();
                    self.cached_fixed_regions = r.fixed_hit_regions().to_vec();
                }
                // 覆盖层内容变了才重算它占用的行区间（上屏据此只合成这几行）
                if let Some(fc) = &self.fixed_canvas {
                    self.fixed_rows = app_window::render::opaque_row_span(fc);
                }
            }
        } else if let Some(r) = &mut self.renderer {
            // 这一帧没重绘覆盖层：把上一次的绑定与遮挡区域补回来
            let (b, g) = (self.cached_fixed_bindings.clone(), self.cached_fixed_regions.clone());
            r.restore_fixed_bindings(&b, &g);
        }
        let t_fixed_done = Instant::now();
        
        if has_tabbar && damage.is_none() {
            if self.is_custom_tabbar() { self.render_custom_tabbar(&current_path); }
            else { self.render_native_tabbar(&current_path); }
        }
        // 记下这一帧真正画过的行区间（夹到画布内；画布之外由上屏填背景色）。
        // 局部重绘帧只碰了一个小矩形，不能声称整条带都是新的 ——
        // 否则滚动出旧条带时 viewport_inside_drawn_band 会误判「画过了」，
        // 上屏取到没画过的区域，也就是滑动时露白。
        if damage.is_none() {
            let canvas_h = self.canvas.as_ref().map(|c| c.height() as f32).unwrap_or(0.0);
            self.drawn_band = Some((
                (band_y0 as f32).max(0.0),
                (band_y1 as f32).min(canvas_h),
            ));
        }
        self.render_parts = (
            (t_page_done - t_render_begin).as_secs_f32() * 1000.0,
            (t_fixed_done - t_page_done).as_secs_f32() * 1000.0,
            t_fixed_done.elapsed().as_secs_f32() * 1000.0,
        );
        if std::env::var("MINI_SCROLL_LOG").is_ok() {
            eprintln!(
                "📐 {} 内容高 {:.1} 视口 {:.1} 画布高 {} 滚动 {:.1}/{:.1}",
                current_path, content_height, viewport_height,
                self.canvas.as_ref().map(|c| c.height()).unwrap_or(0),
                self.scroll.get_position(), self.scroll.get_max_scroll()
            );
        }
    }

    pub(crate) fn present(&mut self) {
        let canvas = match &self.canvas { Some(c) => c, None => return };
        let page = match self.page_stack.last() { Some(p) => p, None => return };
        let has_tabbar = self.is_tabbar_page(&page.path);
        let (toast_state, loading_state, modal_state) = (self.toast.clone(), self.loading.clone(), self.modal.clone());
        let picker_state = self.picker_sheet.clone();
        // 画布之外的填充色同样取 page 底色，避免露出突兀的灰边
        let present_bg = self
            .renderer
            .as_ref()
            .and_then(|r| r.page_style().background)
            .map(|c| ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32)
            .unwrap_or(0xF5F5F5);
        
        if let (Some(window), Some(surface)) = (&self.window, &mut self.surface) {
            let size = window.inner_size();
            if let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
                // 只在尺寸真的变了才 resize：每帧都调等于让 softbuffer 反复确认
                // 平台侧后备缓冲的配置，是白付的开销。
                //
                // 注：这**不是**偶发 30~50ms 上屏停顿的原因（改前改后都有）。
                // 那个停顿发生在 `buffer_mut()`/`present()` 内部，即 macOS 合成器侧，
                // 与本引擎的光栅化无关（同一帧的「渲染」一直稳定在 2~3ms）。
                if self.surface_size != Some((size.width, size.height)) {
                    surface.resize(w, h).ok();
                    self.surface_size = Some((size.width, size.height));
                }
                if let Ok(mut buffer) = surface.buffer_mut() {
                    present_to_buffer(&mut buffer, size.width, size.height, canvas, self.fixed_canvas.as_ref(), self.tabbar_canvas.as_ref(),
                        (self.scroll.get_position() * self.scale_factor as f32) as i32, has_tabbar,
                        if has_tabbar { (tabbar_height() as f64 * self.scale_factor) as u32 } else { 0 },
                        self.fixed_rows, present_bg);
                    // 下拉刷新指示器画在页面之上、Toast/Modal 之下
                    let gap = self.scroll.top_gap();
                    if gap > 0.5 || self.pull_refreshing {
                        app_window::render::render_pull_indicator(
                            &mut buffer, size.width, size.height, self.scale_factor as f32,
                            gap, self.pull_refreshing, self.scroll.pull_progress(),
                            // 相位要用「进程启动至今」的时钟：last_frame 每帧都会被重置，
                            // 拿它算 elapsed 恒为 0，三点看起来是静止的。
                            (self.started_at.elapsed().as_secs_f32() * 1.1).fract(),
                        );
                    }
                    render_ui_overlay(&mut buffer, size.width, size.height, self.scale_factor as f32, self.last_frame,
                        &toast_state, &loading_state, &modal_state, self.text_renderer.as_deref());
                    // picker 面板在所有覆盖层之上
                    if let Some(sheet) = &picker_state {
                        picker_sheet::render(&mut buffer, size.width, size.height, self.scale_factor as f32, sheet, self.text_renderer.as_deref());
                    }
                    // 左边缘侧滑返回：整帧右移，左边露出上一页。
                    // 放在最后（覆盖层之上）——被推走的是「整个页面」，页面自己的
                    // 弹窗、Toast 跟着一起走才对；不然弹窗会诡异地钉在原处。
                    if let Some(eb) = &self.edge_back {
                        let off = (eb.offset() * self.scale_factor as f32).round() as u32;
                        app_window::edge_back::compose(
                            &mut buffer, size.width, size.height, off,
                            self.back_shots.last(), eb.progress(),
                        );
                    }
                    buffer.present().ok();
                }
            }
        }
    }
}
