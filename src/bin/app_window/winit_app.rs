//! winit 事件 → 窗体调用的适配层：这里是**唯一**认识 winit 类型的地方。
//!
//! 判定与状态迁移都在下面这些层里，本文件只做「翻译 + 转发」：
//! - 指针三段 → [`super::pointer_input`]（再往下是 `host::input` 的手势仲裁）
//! - 键盘/输入法 → [`super::events`]
//! - 滚轮 → `host::input` 的同一套嵌套滚动仲裁
//! - 帧节奏（`about_to_wait` / `new_events`）→ [`super::frame`] 的闸门
//!
//! 从 `src/bin/window.rs` 拆出来，**纯搬迁**。
use super::*;
use crate::*;

impl ApplicationHandler for crate::MiniAppWindow {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let window = Arc::new(event_loop.create_window(WindowAttributes::default()
            .with_title("Mini App").with_inner_size(winit::dpi::LogicalSize::new(LOGICAL_WIDTH, LOGICAL_HEIGHT)).with_resizable(false)).unwrap());
        window.set_ime_allowed(true);
        // 依据显示器刷新率设定每帧间隔，适配 60/120/144Hz 等高刷屏
        let millihertz = window.current_monitor()
            .and_then(|m| m.refresh_rate_millihertz())
            .filter(|&hz| hz > 0)
            .unwrap_or(60_000);
        self.frame_interval = Duration::from_secs_f64(1000.0 / millihertz as f64);
        println!("🖥️  显示器刷新率: {:.1}Hz -> 每帧 {:.2}ms", millihertz as f64 / 1000.0, self.frame_interval.as_secs_f64() * 1000.0);
        self.setup_canvas(window.scale_factor());
        self.update_renderers();
        let ctx = softbuffer::Context::new(window.clone()).unwrap();
        self.surface = Some(softbuffer::Surface::new(&ctx, window.clone()).unwrap());
        self.window = Some(window);
        self.render();
        self.present();
        println!("\n🎮 Ready!\n");
    }
    
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::ModifiersChanged(m) => self.modifiers = m.state(),
            
            WindowEvent::KeyboardInput { event, .. } => {
                let (nr, pn, ex) = evt::handle_keyboard_event(event, self.modifiers, &mut self.interaction, &mut self.clipboard,
                    self.window.as_ref(), self.renderer.as_ref(), &mut self.app, &mut self.scroll, self.scale_factor);
                if ex { event_loop.exit(); }
                if let Some(n) = pn { self.pending_navigation = Some(n); }
                if nr { self.needs_redraw = true; if let Some(w) = &self.window { w.request_redraw(); } }
            }
            
            WindowEvent::Ime(ime) => {
                if evt::handle_ime_event(ime, &mut self.interaction, self.window.as_ref(), self.renderer.as_ref(),
                    &mut self.app, &mut self.clipboard, self.scroll.get_position(), self.scale_factor) {
                    self.needs_redraw = true;
                    if let Some(w) = &self.window { w.request_redraw(); }
                }
            }
            
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.setup_canvas(scale_factor);
                self.update_renderers();
                self.render();
                self.needs_redraw = false;
            }
            
            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = (position.x as f32 / self.scale_factor as f32, position.y as f32 / self.scale_factor as f32);
                self.on_pointer_move(x, y);
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            
            WindowEvent::MouseWheel { delta, phase, .. } => {
                use app_window::events::wheel::WheelStep;
                let step = self.wheel_filter.classify(phase, Instant::now());
                if step == WheelStep::Ignore {
                    return;
                }
                // 手指重新放上触控板：和触摸一样，先停住还在跑的惯性
                if step == WheelStep::FingerDown && self.stop_running_flings() {
                    if let Some(w) = &self.window { w.request_redraw(); }
                }
                // 指针停在 fixed 覆盖层（弹窗遮罩）上时锁住页面滚动，与微信一致；
                // 覆盖层内部自己的 scroll-view 仍然可滚（下面按元素命中处理）
                let over_fixed = self
                    .renderer
                    .as_ref()
                    .map(|r| r.fixed_layer_hit(self.mouse_pos.0, self.mouse_pos.1))
                    .unwrap_or(false);
                let out = evt::handle_mouse_wheel_gated(delta, self.mouse_pos, &mut self.interaction, &mut self.scroll, self.scale_factor, over_fixed);
                if out.redraw && !out.area_scrolled {
                    self.needs_redraw = true;
                }
                if out.redraw || out.page_scrolled {
                    self.last_scroll_at = Some(Instant::now());
                }
                // 滚的是覆盖层里的 scroll-view：覆盖层画布得重画，否则位置动了画面不动
                if out.fixed_dirty {
                    self.fixed_dirty = true;
                }
                // 触控板抬手/取消：越界立即回弹，否则进入控制器自己的惯性
                // （鼠标滚轮没有这个阶段，由控制器的静默计时兜底）
                if step == WheelStep::FingerUp {
                    if self.scroll.end_wheel_gesture() { self.needs_redraw = true; }
                    for c in self.interaction.scroll_controllers.values_mut() {
                        c.end_wheel_gesture();
                    }
                    self.fixed_dirty = true;
                }
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                let (x, y) = self.mouse_pos;
                if state == ElementState::Pressed {
                    self.on_pointer_press(x, y);
                } else {
                    self.on_pointer_release(x, y);
                }
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            
            WindowEvent::RedrawRequested => {
                let frame_begin = Instant::now();
                if self.fps_log {
                    if let Some(prev) = self.last_frame_begin {
                        let gap = (frame_begin - prev).as_secs_f32() * 1000.0;
                        self.frame_gap_max_ms = self.frame_gap_max_ms.max(gap);
                        self.frame_gap_min_ms = self.frame_gap_min_ms.min(gap);
                    }
                    self.last_frame_begin = Some(frame_begin);
                }
                self.app.update().ok();
                print_js_output(&self.app);
                // 逻辑层这一帧改过数据就必须重绘。没有这一步的话，定时器/网络回调
                // 里的 setData 只在「刚好还有 CSS 动画在跑」时才顺带上屏，
                // 纯 JS 驱动的页面（秒杀倒计时、轮询刷新）会一直显示旧值。
                if self.app.take_data_dirty() {
                    self.needs_redraw = true;
                    self.page_data_dirty = true;
                    self.fixed_dirty = true;
                }
                // 网络响应回来了：即使这一帧逻辑层没有 setData（比如回调里只存了缓存），
                // 也要重绘一次 —— 图片 src 之类可能已经跟着变了
                if self.app.take_network_dirty() {
                    self.needs_redraw = true;
                    self.page_data_dirty = true;
                }
                // 远程图片下载完了：重绘一次让它出现（异步加载的必要配套 ——
                // 不置脏的话图片只会在下一次别的原因触发重绘时才显示）
                if mini_render::renderer::components::image_net::take_dirty() {
                    self.needs_redraw = true;
                    self.fixed_dirty = true;
                    self.force_full_redraw = true;
                }
                // 长按要由帧驱动：手指按住不动时没有任何输入事件进来，
                // 只在 move/up 里判时间的话那一下永远等不到
                if self.touch.is_active() {
                    let clock = self.touch_clock_ms();
                    let outs = self.touch.tick(clock);
                    let (mx, my) = self.mouse_pos;
                    self.dispatch_touch_events(&outs, mx, my);
                }
                
                let mut pull_req: Option<bool> = None;
                if evt::process_ui_events(&mut self.app, &mut self.toast, &mut self.loading, &mut self.modal, &mut pull_req) { self.needs_redraw = true; }
                self.apply_pull_down_request(pull_req);
                // 每帧轮询一次导航请求：此前只在「内容区被点击」时检查，
                // 于是 setTimeout / 网络回调里发起的 wx.navigateTo / navigateBack
                // 永远不会被宿主取走（登录成功 800ms 后自动返回就是这么失效的）。
                if self.pending_navigation.is_none() {
                    if let Some(nav) = app_window::check_navigation(&mut self.app) {
                        self.pending_navigation = Some(nav);
                        self.needs_redraw = true;
                    }
                }
                if evt::update_toast_timeout(&mut self.toast) { self.needs_redraw = true; }
                
                self.update_scroll();
                // 下拉到位并松手：只有页面声明了 enablePullDownRefresh 才进入刷新态
                // （微信里回弹一直有，指示器与回调由这个开关决定）
                if self.scroll.take_pull_trigger() && self.page_enables_pull_down() {
                    self.set_pull_refreshing(true);
                }
                self.process_navigation();
                self.reap_picker_sheet();
                // swiper 的自动播放由宿主推进，而不是等它被画到才走时钟 ——
                // 滚出视口或被局部重绘剪掉的 swiper 会永远"到点"，把整页拖成每帧重绘
                if mini_render::renderer::components::advance_due_swipers() {
                    self.needs_redraw = true;
                }
                // 不变量：上屏要用的那段画布必须是已经画过的。
                // 页面画布只画「视口 ± 裁剪余量」，所以滚动出这条带就必须重画，
                // 否则上屏取到没画过的区域（表现为滑动时一片空白，停下才出现内容）。
                //
                // 反过来说：只要还在这条带内，滚动**不需要重绘** —— 页面画布用的是
                // 内容坐标，滚动只是取不同的切片，上屏本身就是一次逐行拷贝。
                // 这是滑动流畅度的关键：滚动期间大部分帧只花上屏的 3~4ms，
                // 而不是每帧重画整屏的 8~14ms（帧时间抖动就是「卡卡的」来源）。
                if !self.viewport_inside_drawn_band() {
                    self.needs_redraw = true;
                }
                
                // 页面级滚动**不进** structural：整页画布用内容坐标，滚动只是让 present()
                // 按新偏移取不同切片（present 每帧都会跑）。只有滚出「已绘制条带」时，
                // 上面的 viewport_inside_drawn_band 才置 needs_redraw 触发一次重绘。
                // 从前把 `self.scroll.is_animating()||is_dragging` 也算进 structural，
                // 于是滚动的每一帧都整条带重画（6~7ms），刚好卡在 144Hz 的 6.9ms 预算边缘，
                // 时不时超一点就丢帧 —— 这正是「滑动像抖动、高刷没体现」的根因。
                // scroll-view 内滚动仍要重画（它的内容是按自身偏移画进整页画布的，没有独立切片），
                // 但只重画位置变了的那个 scroll-view 的盒子 —— 由 render_frame_if_needed 按
                // 位置变化求出损伤区，所以也不进 structural。
                self.caret_blink_tick();
                let sv_scroll = self.interaction.scroll_controllers.values().any(|c| c.is_animating() || c.is_dragging);
                let css_anim = self.renderer.as_ref().map(|r| r.has_active_animations()).unwrap_or(false);
                let t_logic = frame_begin.elapsed();
                // swiper 自动播放到点/正在滑动时也要重绘（它的状态在绘制期推进）
                let swiper_frame = mini_render::renderer::components::swiper_needs_frame();
                let structural = self.needs_redraw
                    || mini_render::renderer::components::has_playing_video()
                    || self.interaction.has_focused_input()
                    || self.interaction.press_feedback_pending()
                    || swiper_frame;
                // 整帧重绘的归因统计（MINI_FPS=1 时每秒汇总）。
                // 「为什么这一帧又整屏重画了」如果只能靠猜，性能问题就没法收敛。
                if self.fps_log {
                    if self.needs_redraw { self.gate_counts.0 += 1; }
                    if swiper_frame { self.gate_counts.1 += 1; }
                    if sv_scroll { self.gate_counts.2 += 1; }
                    if css_anim { self.gate_counts.3 += 1; }
                    if !structural && css_anim && self.animation_damage_rect().is_none() {
                        self.gate_counts.4 += 1; // 动画帧被迫走整帧（损伤区被否）
                    }
                }
                self.render_frame_if_needed(structural, css_anim);
                let t_render = frame_begin.elapsed();
                self.present();
                if self.fps_log {
                    let total = frame_begin.elapsed();
                    if total.as_secs_f32() * 1000.0 > self.fps_worst_ms {
                        self.fps_worst_parts = (
                            t_logic.as_secs_f32() * 1000.0,
                            (t_render - t_logic).as_secs_f32() * 1000.0,
                            (total - t_render).as_secs_f32() * 1000.0,
                        );
                        self.fps_worst_render_parts = self.render_parts;
                    }
                }
                self.last_present = Instant::now();
                // 本帧真正花在「逻辑 + 渲染 + 上屏」上的时间（不含为限速而睡的时间）
                let work_ms = frame_begin.elapsed().as_secs_f32() * 1000.0;
                // 动画期间自己续订下一帧：只靠 about_to_wait 的定时唤醒时，
                // 每个周期要多绕一次事件循环，实测在 144Hz 屏上只能跑到 ~47FPS
                // （单帧渲染其实只要 4ms）。
                // 续帧前睡到下一个刷新时点 —— softbuffer 的 present 不阻塞垂直同步，
                // 不限速会以 2~4 倍刷新率空转，白烧一个核画没人看得见的帧。
                // 时点要从**帧开始**算：从 present 之后算的话，每帧会多出一整个
                // 渲染耗时（16.7ms 节拍变成 21ms，只剩 46FPS）。
                if self.is_animating() {
                    // 固定节拍：下一帧时点是「上一个时点 + 一个刷新周期」的累加，
                    // 不是「本帧开始 + 一个周期」。后者每帧都把 sleep 的过冲（macOS 上
                    // 约 0.5~2ms）算进新的起点，节拍会越走越偏且忽快忽慢 ——
                    // 平均帧率看着达标，实际帧间隔在抖，这正是「高刷没体现出来」的手感。
                    let now = Instant::now();
                    let mut target = self.next_frame_at + self.frame_interval;
                    // 落后超过一整帧（大重绘、系统抢占）就重新对齐，避免追帧追出一串挤压帧。
                    // 对齐到「现在」而不是「现在 + 一个周期」：一帧干了 10ms 活本来就已经
                    // 超预算了，再补睡 6.9ms 会把帧间隔顶到 17ms —— 手上感觉到的顿挫是
                    // **帧间隔**，不是干活时间，白睡的这一下等于把一次超时放大成两帧。
                    if target <= now {
                        target = now;
                    }
                    self.next_frame_at = target;
                    // 只睡到「还差一个自旋余量」，剩下的用让出时间片的忙等把边缘对准；
                    // sleep 的分辨率不足以稳定命中 6.9ms 的节拍。
                    const SPIN_MARGIN: Duration = Duration::from_micros(900);
                    if let Some(coarse) = target.checked_sub(SPIN_MARGIN) {
                        let now = Instant::now();
                        if coarse > now {
                            std::thread::sleep(coarse - now);
                        }
                    }
                    while Instant::now() < target {
                        std::hint::spin_loop();
                    }
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                } else {
                    self.next_frame_at = Instant::now();
                }
                if self.fps_log {
                    let ms = work_ms;
                    self.fps_frames += 1;
                    self.fps_worst_ms = self.fps_worst_ms.max(ms);
                    if self.fps_window_start.elapsed().as_secs_f32() >= 1.0 {
                        let (l, r, p) = self.fps_worst_parts;
                        let (rp, rf, rt) = self.fps_worst_render_parts;
                        println!(
                            "📊 {} 帧/秒，帧间隔 {:.1}~{:.1}ms，最慢一帧 {:.1}ms（逻辑 {:.1} 渲染 {:.1}[页面 {:.1} 覆盖层 {:.1} tabBar {:.1}] 上屏 {:.1}）{}",
                            self.fps_frames,
                            if self.frame_gap_min_ms == f32::MAX { 0.0 } else { self.frame_gap_min_ms },
                            self.frame_gap_max_ms,
                            self.fps_worst_ms,
                            l, r, rp, rf, rt, p,
                            if self.is_animating() { "（动画中）" } else { "" }
                        );
                        let g = self.gate_counts;
                        if g != (0, 0, 0, 0, 0) {
                            println!(
                                "   ↳ 整帧归因：需重绘 {} / swiper {} / scroll-view {} / 动画 {}（其中损伤区被否 {}）",
                                g.0, g.1, g.2, g.3, g.4
                            );
                        }
                        self.gate_counts = (0, 0, 0, 0, 0);
                        self.fps_window_start = Instant::now();
                        self.fps_frames = 0;
                        self.fps_worst_ms = 0.0;
                        self.fps_worst_parts = (0.0, 0.0, 0.0);
                        self.fps_worst_render_parts = (0.0, 0.0, 0.0);
                        self.frame_gap_max_ms = 0.0;
                        self.frame_gap_min_ms = f32::MAX;
                    }
                }
                // 后续帧的调度交给 about_to_wait：动画中按刷新率 WaitUntil，空闲则 Wait 休眠。
            }
            _ => {}
        }
    }

    /// 定时唤醒（WaitUntil 到期）时触发下一帧重绘，实现按刷新率的稳定出帧节流。
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::ResumeTimeReached { .. } = cause {
            if let Some(w) = &self.window { w.request_redraw(); }
        }
    }

    /// 事件处理完毕后决定控制流：
    /// - 有动画：按显示器刷新率安排下一帧（WaitUntil），保证 60/120Hz 平滑且不空转。
    /// - 空闲：Wait 休眠，CPU 占用降为 0，直到下一个输入事件。
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.needs_redraw {
            if let Some(w) = &self.window { w.request_redraw(); }
        }
        if self.is_animating() || self.pending_navigation.is_some() {
            let target = self.last_present + self.frame_interval;
            event_loop.set_control_flow(ControlFlow::WaitUntil(target));
        } else if !self.needs_redraw {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}
