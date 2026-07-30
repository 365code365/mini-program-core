//! 无头运行：逐页整帧快照与拖动帧成本基准（与交互窗体同一条管线）
//!
//! 从 `src/bin/window.rs` 拆出来的一片（`impl crate::MiniAppWindow`）。**纯搬迁**，
//! 一行逻辑没改；判据是 65 张画廊图与逐页整帧快照逐字节不变。
#![allow(clippy::too_many_arguments)]
use super::*;
use crate::*;

impl crate::MiniAppWindow {
    /// `--drag <每帧像素>x<帧数>`：走与交互窗体同一套鼠标事件模拟一次手指拖动，
    /// 逐帧计时。滑动手感只有连续拖动才测得出来（见 app_window::scroll_bench）。
    pub(crate) fn run_drag_bench(&mut self, spec: &app_window::scroll_bench::DragSpec, route: &str) {
        let (cx, mut cy) = (LOGICAL_WIDTH as f32 / 2.0, LOGICAL_HEIGHT as f32 * 0.75);
        self.sim_press(cx, cy);
        let mut frame_ms: Vec<f32> = Vec::with_capacity(spec.frames as usize);
        let mut worst = (0.0f32, (0.0f32, 0.0f32, 0.0f32));
        let mut input_total = 0.0f32;
        for _ in 0..spec.frames {
            cy -= spec.dy;
            let t0 = Instant::now();
            self.sim_move(cx, cy);
            input_total += t0.elapsed().as_secs_f32() * 1000.0;
            self.pump_one_frame();
            let ms = t0.elapsed().as_secs_f32() * 1000.0;
            if ms > worst.0 {
                worst = (ms, self.render_parts);
            }
            frame_ms.push(ms);
            // 按刷新率节流，模拟真实拖动的采样节奏
            let spent = t0.elapsed();
            if let Some(rest) = self.frame_interval.checked_sub(spent) {
                std::thread::sleep(rest);
            }
        }
        self.sim_release(cx, cy);
        self.settle_scroll_animations(2000);
        app_window::scroll_bench::report(route, &frame_ms, self.frame_interval);
        app_window::scroll_bench::report_worst_parts(worst.0, worst.1);
        println!(
            "   ↳ 其中输入处理（命中/手势/触摸事件派发）平均 {:.2}ms/帧",
            input_total / spec.frames.max(1) as f32
        );
        if let Some(s) = mini_render::renderer::draw_profile::summary(8) {
            println!("   ↳ {s}");
        }
    }

    /// 无窗口模式：按**与交互窗体完全相同的管线**把每个页面合成成整帧图片。
    ///
    /// 这是「窗体是否忠实还原小程序」的可验证入口 —— 页面加载、样式合并、
    /// 自定义 tabBar、fixed 覆盖层、Toast/Modal 外壳、像素合成顺序都与真实运行一致，
    /// 因此产出的 PNG 可以直接和编译出的 H5 截图做像素级对比。
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn snapshot_all(&mut self, out_dir: &std::path::Path, scale: f64, time: Option<f32>, settle: Option<f32>, only: Option<&str>, scroll: f32, evals: &[String], frames: u32, actions: &[app_window::headless_script::Action]) -> Result<usize, String> {
        self.setup_canvas(scale);
        let routes: Vec<String> = match only {
            Some(route) => vec![route.trim_start_matches('/').to_string()],
            None => self.app_config.pages.clone(),
        };
        let mut written = 0usize;
        for route in routes {
            if !self.pages.contains_key(&route) {
                eprintln!("⚠️  跳过 {}（页面未加载）", route);
                continue;
            }
            // 构造函数已经加载过首页：重复 navigate 会让 onLoad 跑第二遍，
            // 依赖「首次进入」的逻辑（如只弹一次的新人券）会被吃掉，两端数据就不一致了。
            let already_loaded = self.page_stack.last().map(|p| p.path == route).unwrap_or(false);
            if !already_loaded {
                // 与 switchTab/reLaunch 一致：销毁上一页前派发 onUnload，
                // 否则上一页的定时器会继续跑，几秒后把路由顶掉
                // （启动页的守护定时器会 switchTab 回首页，快照就截错了页）
                self.unload_current_page();
                self.page_stack.clear();
                self.back_shots.clear();
                self.interaction.clear_page_state();
                self.navigate_to(&route, HashMap::new())?;
            }
            self.update_renderers();
            // `--settle`：让**真实时间**流过给定时长，把逻辑层真正跑起来 ——
            // 不等的话 setInterval / setTimeout 一次都不会触发，秒杀倒计时、
            // 延时弹层、轮播自动播放在快照里全停在 0s 的样子（和真机不符）。
            //
            // 和 `--time` 分开是有意的：双端对比时 H5 侧刻意禁用了页面脚本
            // （见 examples/compare.rs 的 inject_capture_override，为了截图确定性），
            // 那条链路只需要把动画时钟拨到位，不能让 JS 状态往前跑。
            if let Some(secs) = settle {
                let deadline = Instant::now() + Duration::from_secs_f32(secs);
                while Instant::now() < deadline {
                    // 跑**完整的一帧**（与交互窗体同一套闸门），不只是 update。
                    //
                    // 关键在于「真的出帧」：远程图片、`wx.request` 这些异步资源是
                    // **绘制期**才发起的（绘制到 `<image src="http…">` 才知道要下载它）。
                    // 从前 settle 只 update 不 render，于是等待这几秒里根本没人发起下载，
                    // 最后一帧才发起、当然来不及 —— 闪屏页的整屏背景图因此永远是占位图，
                    // 而真机上那几秒是一直在出帧的。
                    self.pump_one_frame();
                    evt::update_toast_timeout(&mut self.toast);
                    std::thread::sleep(Duration::from_millis(8)); // ≈120Hz，与真机出帧节奏同量级
                }
            }
            if let Some(secs) = time {
                if let Some(r) = &mut self.renderer { r.set_animation_time(secs); }
                if let Some(r) = &mut self.tabbar_renderer { r.set_animation_time(secs); }
            }
            // 注入页面状态：把「交互之后」的场景（购物车有商品、开关已打开等）
            // 也纳入可截图、可对比的范围，而不是只能测首次进入的初始态。
            // 放在等待之后执行：页面此时已稳定（入场动画结束、延时弹层已弹出），
            // 脚本里「关掉优惠券弹层」这类操作才不会被随后的 setTimeout 又打开。
            // 逐段执行：每段之后都把导航请求跑完，
            // 于是「点商品进详情 → 返回上一页」这类多步交互可以脚本化验证。
            //
            // 注入之前先出一帧：真机上 `setData` 永远发生在已经渲染过的页面上，
            // 于是它走的是「增量失效」路径。不先出这一帧的话，脚本注入等于
            // 首帧数据，永远只能测到整帧重绘那条路 —— 对照验证会变成空转。
            if !evals.is_empty() {
                self.render();
            }
            for script in evals {
                match self.app.eval(script) {
                    Ok(_) => print_js_output(&self.app),
                    Err(e) => eprintln!("⚠️  --eval 执行失败: {}", e),
                }
                for _ in 0..8 {
                    self.app.update().ok();
                    if self.pending_navigation.is_none() {
                        self.pending_navigation = app_window::check_navigation(&mut self.app);
                    }
                    if self.pending_navigation.is_none() {
                        break;
                    }
                    self.process_navigation();
                    print_js_output(&self.app);
                }
            }
            // 真实内容高要渲染一次才知道，而 set_position 会按 max_scroll 夹紧 ——
            // 不先渲染的话 `--scroll` 会被夹到初始上限，深位置截图全都截到同一处。
            if scroll > 0.0 {
                self.render();
            }
            self.scroll.set_position(scroll);
            self.app.update().ok();
            // 先把逻辑层这一批 UI 指令（Toast/Modal/下拉刷新）交给宿主，
            // 再让宿主侧的滚动动画（惯性、边界回弹、下拉刷新的按住/归位）走完 ——
            // 顺序反了的话脚本刚触发的状态在图里还没体现出来。
            {
                let mut pull_req: Option<bool> = None;
                evt::process_ui_events(&mut self.app, &mut self.toast, &mut self.loading, &mut self.modal, &mut pull_req);
                self.apply_pull_down_request(pull_req);
            }
            let anim_deadline = Instant::now() + Duration::from_millis(2000);
            while self.scroll.is_animating() && Instant::now() < anim_deadline {
                self.update_scroll();
                self.app.update().ok();
                std::thread::sleep(Duration::from_millis(8));
            }
            // 注入脚本刚触发的动画（`wx.createAnimation`、按压过渡）需要一点真实时间
            // 才能看出效果，否则截到的永远是第 0 帧。只在有 --eval 时等，
            // 双端对比那条链路不受影响（它靠 --time 固定动画时钟）。
            if !evals.is_empty() {
                let deadline = Instant::now() + Duration::from_millis(300);
                while Instant::now() < deadline {
                    self.app.update().ok();
                    // 必须真的出帧：动画的起点是「渲染器第一次看到它」的时刻，
                    // 只 update 不 render 的话最后截到的永远是第 0 帧。
                    self.render();
                    std::thread::sleep(Duration::from_millis(8));
                }
            }
            // 手势/输入动作：按命令行**书写顺序**依次执行（见 app_window::headless_script）。
            // 每一步都走与交互窗体完全相同的入口，所以「点输入框 → 打字 → 点发送」
            // 这类多步交互在无头模式下也能如实复现。
            self.run_actions(actions, &route);
            // `--frames N`：按刷新率跑 N 个**与交互窗体同一套闸门/损伤区逻辑**的帧再截图。
            // 局部重绘这类只在连续出帧时才暴露的问题（比如某个动画元素被漏出损伤区而静止），
            // 只有这样才测得到 —— 单帧快照永远走整帧重绘，看不出来。
            for _ in 0..frames {
                self.pump_one_frame();
                std::thread::sleep(self.frame_interval);
            }
            { let mut pull_req: Option<bool> = None;
                        evt::process_ui_events(&mut self.app, &mut self.toast, &mut self.loading, &mut self.modal, &mut pull_req);
                        self.apply_pull_down_request(pull_req); }
            // 截图前再走一帧完整闸门：异步资源（远程图片、网络回调）刚到位时
            // 需要一次**整帧**重绘才会出现 —— 只调 render() 的话，这一帧的重绘范围
            // 会被上一次 setData 的增量失效范围收窄（闪屏页倒计时每秒 setData，
            // 图片区域因此永远不在范围内，截出来一直是占位图）。
            self.pump_one_frame();
            self.render();

            let (pw, ph) = (
                (LOGICAL_WIDTH as f64 * scale) as u32,
                (LOGICAL_HEIGHT as f64 * scale) as u32,
            );
            let mut buffer = vec![0u32; (pw * ph) as usize];
            let present_bg = self
                .renderer
                .as_ref()
                .and_then(|r| r.page_style().background)
                .map(|c| ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32)
                .unwrap_or(0xF5F5F5);
            // 导航之后当前页可能已不是入口 route
            let route = self.page_stack.last().map(|p| p.path.clone()).unwrap_or(route);
            let has_tabbar = self.is_tabbar_page(&route);
            if let Some(canvas) = &self.canvas {
                present_to_buffer(
                    &mut buffer, pw, ph, canvas,
                    self.fixed_canvas.as_ref(), self.tabbar_canvas.as_ref(),
                    (self.scroll.get_position() * scale as f32) as i32, has_tabbar,
                    if has_tabbar { (tabbar_height() as f64 * scale) as u32 } else { 0 },
                    self.fixed_rows,
                    present_bg,
                );
            }
            let gap = self.scroll.top_gap();
            if gap > 0.5 || self.pull_refreshing {
                app_window::render::render_pull_indicator(
                    &mut buffer, pw, ph, scale as f32,
                    gap, self.pull_refreshing, self.scroll.pull_progress(), 0.25,
                );
            }
            render_ui_overlay(
                &mut buffer, pw, ph, scale as f32, self.last_frame,
                &self.toast, &self.loading, &self.modal, self.text_renderer.as_deref(),
            );
            if let Some(sheet) = &self.picker_sheet {
                picker_sheet::render(&mut buffer, pw, ph, scale as f32, sheet, self.text_renderer.as_deref());
            }
            // 侧滑返回进行中：与交互窗体同一段合成（无头也要能截到跟手的那一帧）
            if let Some(eb) = &self.edge_back {
                let off = (eb.offset() * scale as f32).round() as u32;
                app_window::edge_back::compose(
                    &mut buffer, pw, ph, off, self.back_shots.last(), eb.progress(),
                );
            }

            let mut rgba = Vec::with_capacity(buffer.len() * 4);
            for px in &buffer {
                rgba.extend_from_slice(&[((px >> 16) & 0xFF) as u8, ((px >> 8) & 0xFF) as u8, (px & 0xFF) as u8, 255]);
            }
            let dir = out_dir.join(&route);
            std::fs::create_dir_all(&dir).map_err(|e| format!("创建 {:?} 失败: {}", dir, e))?;
            let file = dir.join("rust.png");
            image::save_buffer(&file, &rgba, pw, ph, image::ColorType::Rgba8)
                .map_err(|e| format!("写入 {:?} 失败: {}", file, e))?;
            println!("🖼  {} -> {}", route, file.display());
            written += 1;
        }
        Ok(written)
    }
}
