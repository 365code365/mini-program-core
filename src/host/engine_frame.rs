//! [`MiniEngine`] 的出帧：推进一帧 → 按需重画页面条带 / 覆盖层 / tabBar → 合成。
//!
//! 从 `engine.rs` 按职责拆出来的一片（`impl MiniEngine`）。拆分本身是纯搬迁；
//! 之后唯一的改动是 `pump` 的外壳挪去了 `engine_backend.rs`（加帧标志位与统计），
//! 判据是 `tools/sdk-parity.sh` 与 sample / news 逐页快照逐字节不变。
//!
//! 「这一帧重画多少」的判断在 `engine_damage.rs`（只合成 / 局部 / 补条带 / 整帧）；
//! 这里是推进一帧的逻辑，以及按它给的 [`RenderScope`] 真正去画。
use super::engine::MiniEngine;
use super::engine_damage::{PagePass, RenderScope};
use crate::compositor::{cpu, FrameLayers, OverlayRefs};
use crate::host::tabbar_height;
use crate::{Canvas, Color};

impl MiniEngine {
    /// 一帧的本体（逻辑层 → 事件 → 渲染 → 合成），返回是否产出了新画面。
    ///
    /// 对外的入口是 `engine_backend.rs` 的 `pump` / `pump_ex`：它们在这外面套了
    /// 帧标志位与 `MINI_FPS` 统计。
    pub(super) fn pump_frame(&mut self, now_ms: u64) -> bool {
        // 宿主给了时间就以它为准，并让引擎的时钟（触摸、长按、事件 timeStamp）跟上它
        if now_ms > 0 {
            self.host_clock = Some((now_ms, std::time::Instant::now()));
        }
        let now = if now_ms > 0 { now_ms } else { self.mono_ms() };
        let dt = if self.last_pump_ms == 0 {
            0.016
        } else {
            ((now.saturating_sub(self.last_pump_ms)) as f32 / 1000.0).clamp(0.001, 0.1)
        };
        self.last_pump_ms = now;

        self.app.update().ok();
        // setData：只标数据脏，重画多大由失效分析定（engine_damage.rs）。
        // 从前这里连带 `needs_redraw` —— 每秒一次的倒计时 setData 就是每秒一次整帧
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
        }
        self.drain_ui_events();
        // Toast 到时收起（桌面窗体由 update_toast_timeout 做，SDK 从前从不收）
        if self.expire_toast() {
            self.dmg.compose_dirty = true;
        }
        // 提示框里的圈转到了新的一步：重新合成（页面不动）。从前没有这一条，
        // 圈只在「别的原因」重新合成时才挪一下 —— 加上时钟传错，Loading 一直是静止的
        if crate::host::ui_overlay::tip_needs_recompose(self.overlay_clock(), &self.toast, &self.loading) {
            self.dmg.compose_dirty = true;
        }
        // 滚动推进（惯性/回弹），到底事件回调页面
        let (moved, event) = self.scroll.update_with_events(dt);
        if moved {
            self.scroll_moved = true;
        }
        // scroll-view 的惯性与回弹（与桌面 `update_scroll` 一致）：
        // 不推进的话甩一下列表，松手就停在原地。位置变了的那几个由 engine_damage
        // 按「上次画进画布时的位置」比出来，只重画它们自己的盒子
        for c in self.interaction.scroll_controllers.values_mut() {
            c.update(dt);
        }
        // scroll-view 的事件、wx.pageScrollTo、onPageScroll（与桌面共用 host::scroll_view）
        if self.tick_scroll_views(now) {
            self.needs_redraw = true;
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
        // 画布被画过（旧接口的 draw()、type="2d" 的上下文）：像素在页面绘制时拷进去，要重画
        // （scroll-view 里的 canvas 在离屏缓存里，缓存也要作废）
        if crate::renderer::components::take_canvas_dirty() {
            self.needs_redraw = true;
            if let Some(r) = self.renderer.as_mut() {
                r.invalidate_offscreen_caches();
            }
        }
        // 新字体登记好了（@font-face / wx.loadFontFace）：文字宽度变了，整棵树重排
        if crate::text_faces::take_changed() {
            for r in [self.renderer.as_mut(), self.tabbar_renderer.as_mut()].into_iter().flatten() {
                r.invalidate_layout();
            }
            self.needs_redraw = true;
        }
        let image_arrived = crate::renderer::components::image_net::take_dirty();
        if self.app.take_network_dirty() || image_arrived {
            self.page_data_dirty = true;
            self.needs_redraw = true;
            // scroll-view 的离屏缓存也要作废：数据没变它就一直「干净」，贴回去的还是占位图（G-1）
            if image_arrived {
                for r in [self.renderer.as_mut(), self.tabbar_renderer.as_mut()].into_iter().flatten() {
                    r.invalidate_offscreen_caches();
                }
            }
        }
        // 侧滑返回的收尾动画（推出去 / 滑回来），走完判定为返回就退栈
        if self.tick_edge_back() {
            self.dmg.compose_dirty = true;
        }
        // 页面转场（route_anim.rs）：位移变了只要重新合成，页面一个像素不用重画
        if self.route.tick(now) != crate::host::route_anim::Tick::Idle {
            self.dmg.compose_dirty = true;
        }
        // 导航栏的标题 / 颜色 / loading 转圈 / 按压态变了（engine_navbar.rs）：重画那一小条再合成
        if self.nav_needs_paint() {
            self.dmg.compose_dirty = true;
        }
        // 长按由帧驱动
        if self.touch.is_active() {
            let outs = self.touch.tick(now);
            if let Some((x, y)) = self.touch.pos() {
                self.dispatch_touch(&outs, x, y);
            }
        }
        // picker 面板的入场/退场动画按帧推进，走完就回收。面板画在合成那一步，
        // 页面一个像素不用动
        if self.picker_sheet.as_ref().map(|s| s.animating()).unwrap_or(false)
            || crate::host::ui_overlay::sheet_overlay_animating()
        {
            self.dmg.compose_dirty = true;
        }
        if crate::host::picker_sheet::reap(&mut self.picker_sheet) {
            self.dmg.compose_dirty = true;
        }
        self.caret_tick();
        if self.interaction.press_feedback_pending() {
            self.needs_redraw = true;
        }
        // 与桌面窗体的出帧闸门对齐：CSS / JS 动画、swiper 翻页的滑动过程、播放中的视频
        // 都要逐帧重画。从前 SDK 只认 needs_redraw，于是 `@keyframes` 只画出第一帧、
        // swiper 自动翻页没有 300ms 的滑动（直接跳过去）。
        // 动画标记是上一次绘制时求出来的，所以「有动画」本身就是重画的理由 ——
        // 但只重画动画那一块（engine_damage.rs），不再逼着整条带陪着重画。
        let tabbar_anim = self.tabbar_renderer.as_ref().map(|r| r.has_active_animations()).unwrap_or(false);
        let media = crate::renderer::components::swiper_needs_frame()
            || (crate::renderer::components::has_playing_video() && self.opts.native_components.iter().all(|t| t != "video"));
        let scroll_moved = std::mem::take(&mut self.scroll_moved);
        // 聚焦的输入框**不再**逼着每帧重画：光标闪烁由 caret_tick 在翻转那一帧标脏，
        // 打字/移光标由 text_input / key 标脏。从前这里多一个 `|| has_focused_input()`，
        // 键盘弹着的整段时间里每次 pump 都整条带重画并回报「有新画面」。
        // 页面画布用内容坐标：视口还在已绘制条带内时，滚动只是换一段切片上屏
        self.redraw_as_needed(tabbar_anim, media, scroll_moved)
    }

    /// 本帧上屏要用的那段页面画布是否已经画过
    pub(super) fn viewport_inside_drawn_band(&self) -> bool {
        let Some((band_top, band_bottom)) = self.drawn_band else { return false };
        let top = (self.scroll.get_position() * self.dpr).max(0.0);
        let bottom = (self.scroll.get_position() * self.dpr + self.viewport_height() * self.dpr)
            .min(self.canvas.read().height() as f32);
        top >= band_top - 0.01 && bottom <= band_bottom + 0.01
    }

    fn page_background(&self) -> Color {
        // 页面 CSS 背景 > wx.setBackgroundColor > #F5F5F5
        super::window_api::background(self.renderer.as_ref().and_then(|r| r.page_style().background))
    }

    /// 不重画任何画布，只按当前滚动位置重新合成一帧
    pub(super) fn compose_current(&mut self) {
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

    /// setData 之后取一次页面数据（JS 侧 JSON.stringify → 反序列化），顺带对账组件实例
    pub(super) fn refresh_page_data(&mut self) {
        if !self.page_data_dirty {
            return;
        }
        let Some(page) = self.page_stack.last() else { return };
        // 与桌面窗体共用一个入口（每处使用一个组件实例）
        self.page_data =
            crate::host::component_mount::render_data(&self.app, &page.wxml_nodes, &page.component_templates);
        self.page_data_dirty = false;
    }

    /// 渲染 + 合成到内部缓冲（三层全画）
    pub(crate) fn render(&mut self) {
        self.render_scoped(RenderScope::FULL);
    }

    /// 按范围重画再合成：页面画布整条带 / 一块矩形 / 不动，覆盖层与 tabBar 各自画或沿用
    pub(super) fn render_scoped(&mut self, scope: RenderScope) {
        let Some(page) = self.page_stack.last() else { return };
        let route = page.path.clone();
        self.refresh_page_data();
        let has_tabbar = self.is_tabbar_page(&route);
        let vp = self.viewport_height();
        let bg = self.page_background();
        let scroll = self.scroll.get_position();
        match scope.page {
            PagePass::Band => {
                let content_h = self.render_page_band(scroll, vp, bg);
                if content_h > 1.0 {
                    self.scroll.update_content_height(content_h, vp);
                    // 画布按内容高度分配（与桌面窗体一致）：固定高度的画布装不下长页面，
                    // 滚到画布之外的部分上屏时只剩一片底色。
                    let required = (content_h * self.dpr).ceil() as u32;
                    if required > 0 && required != self.canvas.read().height() {
                        let width = self.canvas.read().width();
                        self.canvas.replace(Canvas::new(width, required));
                        self.render_page_band(scroll, vp, bg);
                    }
                }
                self.after_page_pass(true, scroll);
            }
            PagePass::Rects { ref rects, anim_complete } => {
                let Self { renderer, canvas, page_stack, page_data, interaction, .. } = self;
                if let (Some(r), Some(page)) = (renderer.as_mut(), page_stack.last()) {
                    // 一块一趟：每一趟都完整走一遍裁剪窗里的节点树（事件绑定、动画标记照常
                    // 重建），只有落在这块里的像素被清掉重画
                    for &rect in rects {
                        // 声明只写了这块（GPU 镜像按它补传），再清 + 裁剪着画
                        let canvas = canvas.write_rect(rect);
                        canvas.clear_area(&rect, bg);
                        r.set_damage_clip(Some(rect));
                        r.render_with_scroll_and_viewport(canvas, &page.wxml_nodes, page_data, interaction, scroll, vp);
                    }
                }
                self.after_page_pass(anim_complete, scroll);
            }
            PagePass::Skip => {}
        }
        if scope.fixed {
            self.render_fixed_layer(vp);
        } else if scope.page != PagePass::Skip {
            // 页面那一趟清空了全部事件绑定：把覆盖层上次登记的补回来，否则弹窗点不动
            if let Some(r) = self.renderer.as_mut() {
                r.restore_fixed_bindings(&self.dmg.cached_fixed_bindings, &self.dmg.cached_fixed_regions);
            }
        }
        if scope.tabbar && has_tabbar {
            self.render_tabbar(&route);
        }
        self.compose_frame(has_tabbar, bg);
    }

    /// 页面画布画完之后：动画包围盒、页面动画标记、scroll-view 位置、布局偏移的记账
    fn after_page_pass(&mut self, anim_complete: bool, scroll: f32) {
        if let Some(r) = self.renderer.as_mut() {
            let bounds = r.take_animated_bounds();
            // 只有这一趟的裁剪范围覆盖了全部动画节点，收集到的包围盒才是完整的（与桌面
            // 窗体同一条规则）。纯 setData 的局部帧只碰了一小块，拿它覆盖旧记录、或拿它的
            // 「有没有动画」覆盖旧结论，都可能让外面的动画就此冻住 —— 那种帧只增不减
            if anim_complete {
                self.dmg.last_animated_bounds = bounds;
                self.dmg.page_animates = r.has_active_animations();
            } else {
                self.dmg.page_animates |= r.has_active_animations();
            }
            // 覆盖层这一帧可能不重画：渲染器的「有动画」要把它那份并回来，宿主据此续帧
            r.set_animations_active(self.dmg.page_animates || self.dmg.fixed_layer_animates);
        }
        self.dmg.drawn_area_positions.clear();
        for (id, c) in &self.interaction.scroll_controllers {
            self.dmg.drawn_area_positions.insert(id.clone(), c.get_position());
        }
        self.dmg.layout_scroll = Some(scroll);
    }

    /// 重画 fixed 覆盖层，并记下它的动画标记、事件绑定与遮挡区域（跳过它的帧要补回来）
    fn render_fixed_layer(&mut self, vp: f32) {
        // fixed 层是页面视口坐标：引擎画了导航栏时它比画面矮一截（engine_navbar.rs）
        self.ensure_fixed_canvas();
        // 右上角胶囊画在这一层的最上面（为什么放这里见 capsule.rs）。有引擎画的导航栏时
        // 胶囊在导航栏画布里（paint_nav），这里不画
        let capsule = if self.nav_rows() > 0 {
            None
        } else {
            self.capsule_spec().map(|(r, dark)| (r, dark, self.capsule_pressed()))
        };
        let dpr = self.dpr;
        let Self { renderer, fixed_canvas, page_stack, page_data, interaction, dmg, .. } = self;
        let fixed_canvas = fixed_canvas.write();
        fixed_canvas.clear(Color::new(0, 0, 0, 0));
        if let (Some(r), Some(page)) = (renderer.as_mut(), page_stack.last()) {
            // 在两趟之间清零，才能分辨「动画在页面里」还是「在覆盖层里」
            r.set_animations_active(false);
            r.render_fixed_elements(fixed_canvas, &page.wxml_nodes, page_data, interaction, vp);
            dmg.fixed_layer_animates = r.has_active_animations();
            r.set_animations_active(dmg.page_animates || dmg.fixed_layer_animates);
            dmg.cached_fixed_bindings = r.fixed_event_bindings();
            dmg.cached_fixed_regions = r.fixed_hit_regions().to_vec();
        }
        let had_capsule = capsule.is_some();
        if let Some((rect, dark, pressed)) = capsule {
            fixed_canvas.reset_clip();
            crate::host::capsule::draw(fixed_canvas, rect, dpr, dark, pressed);
        }
        // 有导航栏时胶囊的「已画」由 paint_nav 记（换前景色会让导航栏的签名变，必然重画）
        if had_capsule || self.nav_rows() == 0 {
            self.capsule_painted();
        }
        self.dmg.fixed_dirty = false;
        // 覆盖层真正画过的行区间：`present_to_buffer` 只合成这几行，
        // 传 None 会**整层跳过** —— 底部吸底操作栏、弹窗遮罩就全丢了。
        self.fixed_rows = crate::host::render::opaque_row_span(self.fixed_canvas.read());
    }

    /// 重画 tabBar（自定义组件式 / app.json 配置式）
    fn render_tabbar(&mut self, route: &str) {
        self.dmg.tabbar_route = Some(route.to_string());
        let selected = self
            .config
            .tab_bar
            .as_ref()
            .and_then(|tb| tb.list.iter().position(|i| i.page_path == route));
        // 三个字段各自可变/不可变借用，互不重叠
        if let (Some(ct), Some(tr)) = (self.custom_tabbar.as_ref(), self.tabbar_renderer.as_mut()) {
            let tabbar_canvas = self.tabbar_canvas.write();
            tabbar_canvas.clear(Color::new(0, 0, 0, 0));
            let mut d = ct.data.clone();
            if let Some(idx) = selected {
                d["selected"] = serde_json::json!(idx);
            }
            tr.render(tabbar_canvas, &ct.wxml_nodes, &d);
        } else if let (Some(tb), Some(text)) = (self.config.tab_bar.as_ref(), self.text.as_deref()) {
            // app.json 配置式的原生 tabBar（没有自定义组件）。SDK 从前只画自定义的，
            // 这种小程序底部是一条空白（命中却照常 —— 点得动、看不见）
            crate::host::tabbar::render_native_tabbar_w(
                self.tabbar_canvas.write(),
                text,
                tb,
                route,
                self.dpr as f64,
                self.width as f32,
            );
        }
    }

    pub(super) fn compose_frame(&mut self, has_tabbar: bool, bg: Color) {
        let (pw, ph) = ((self.width as f32 * self.dpr) as u32, (self.height as f32 * self.dpr) as u32);
        let n = (pw as usize) * (ph as usize);
        if self.compose.len() != n {
            self.compose = vec![0u32; n];
        }
        let bg_u32 = ((bg.r as u32) << 16) | ((bg.g as u32) << 8) | bg.b as u32;
        // 引擎画的导航栏（engine_navbar.rs）：内容变了才重画它那块小画布
        self.paint_nav();
        let top_rows = self.nav_rows();
        // 合成序列与桌面窗体同一份（compositor::cpu）。SDK 没有下拉指示器这一层
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
                anim_secs: self.overlay_clock(),
                toast: &self.toast,
                loading: &self.loading,
                modal: &self.modal,
                // picker 面板画在最上层（与桌面窗体同一份绘制）
                picker: self.picker_sheet.as_ref(),
                text: self.text.as_deref(),
            },
            // 页面转场优先于侧滑（转场期间不接受触摸，两者不会同时有）
            edge_back: self
                .route
                .frame(&self.back_shots, pw)
                .or_else(|| Self::edge_back_frame(&self.edge_back, &self.back_shots, self.dpr)),
            top: self.nav.band(top_rows),
        };
        // GPU 上屏（宿主挂了原生 surface 时）：画到 surface 上就不做 CPU 合成
        #[cfg(feature = "gpu")]
        match super::engine_gpu::present_on(&mut self.gpu, &layers) {
            Ok(super::engine_gpu::GpuFrame::Presented) => {
                self.frame_presented = true;
                self.present_pending = false;
                // 合成缓冲没更新：`mr_app_pixels*` 要像素时再补（engine_gpu::sync_cpu_frame）
                self.cpu_frame_pending = Some((has_tabbar, bg));
                return;
            }
            Ok(super::engine_gpu::GpuFrame::Skipped) => {
                self.present_pending = true;
                return;
            }
            Ok(super::engine_gpu::GpuFrame::NotActive) => {}
            Err((why, frame)) => {
                // 当帧回退：这一帧照样要出来，改走 CPU
                drop(layers);
                self.gpu_fell_back(&why, frame);
                return self.compose_frame(has_tabbar, bg);
            }
        }
        cpu::compose(&mut self.compose[..], &layers);
        #[cfg(feature = "gpu")]
        {
            self.cpu_frame_pending = None;
        }
        // RGBA 不在这里转：要的人（`rgba()` / `mr_app_pixels`）再转，
        // `pixels_into` 直接从合成缓冲写进平台缓冲（见 engine_pixels.rs）
        self.rgba_stale = true;
    }

    /// 宿主是否该在下一个 vsync 再 pump 一次（`MR_FRAME_WANTS_MORE`）。
    ///
    /// 与桌面窗体 `is_animating` 同一组条件：滚动 / 惯性、scroll-view、CSS 动画、
    /// swiper 自动播放、JS 定时器、网络请求与远程图片在飞、逻辑层没办完的宿主活
    /// （画布指令、没答的 SelectorQuery）、Toast（要到时收起）、Loading（圈在转）、
    /// picker 面板动画、播放中的视频、看得见的动图、聚焦的输入框（光标闪烁），外加**按住不动的手指**
    /// （长按靠帧驱动，手指不动就没有输入事件进来）。
    ///
    /// 返回 false 时宿主可以停掉 vsync 回调，等下一个输入事件再 pump —— 静止页面零耗电。
    /// 从前没有这个信号：Android 的 View 只在 `pump` 返回「有新画面」时续帧，
    /// `setTimeout` / `wx.request` 回包 / swiper 自动播放在静止页面上全部停住。
    pub fn wants_next_frame(&self) -> bool {
        use crate::renderer::components as c;
        self.scroll.is_animating()
            || self.scroll.is_dragging
            || self.interaction.scroll_controllers.values().any(|c| c.is_animating() || c.is_dragging)
            || self.renderer.as_ref().map(|r| r.has_active_animations()).unwrap_or(false)
            || self.tabbar_renderer.as_ref().map(|r| r.has_active_animations()).unwrap_or(false)
            || self.interaction.has_focused_input()
            || self.interaction.press_feedback_pending()
            || self.touch.is_active()
            || c::has_autoplay_swiper()
            || c::swiper_needs_frame()
            || self.app.has_active_timers()
            || self.app.has_pending_network()
            // 逻辑层发了、还没办完的：画布指令、没答的 SelectorQuery（type="2d" 画布靠它拿节点）
            || self.app.has_pending_host_work()
            || c::image_net::has_pending()
            || self.toast.as_ref().map(|t| t.visible).unwrap_or(false)
            // Loading 的圈要一直转（`wx.showLoading` 没有时长，等逻辑层 hideLoading）
            || crate::host::ui_overlay::tip_spinning(&self.toast, &self.loading)
            || self.picker_sheet.as_ref().map(|s| s.animating()).unwrap_or(false)
            || crate::host::ui_overlay::sheet_overlay_animating()
            || self.edge_back.as_ref().map(|e| e.is_settling()).unwrap_or(false)
            || c::has_playing_video()
            // 看得见的动图：醒着等它换帧（多数帧问下来不用重画，见 engine_damage::plan_redraw）
            || self.animated_images_visible()
            // surface 这次没取到：宿主不续帧的话 PENDING 那一帧永远补不上
            || self.present_pending
            // 导航栏前景色在这一帧末尾才改：胶囊换配色要再来一帧
            || self.capsule_wants_frame()
            // 页面转场在走、引擎画的导航栏在转圈或还欠一次重画
            || self.route.active()
            || self.nav_wants_frame()
    }

    pub fn pixel_size(&self) -> (u32, u32) {
        (
            (self.width as f32 * self.dpr) as u32,
            (self.height as f32 * self.dpr) as u32,
        )
    }
}
