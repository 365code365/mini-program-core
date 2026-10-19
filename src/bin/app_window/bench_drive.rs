//! 把 [`super::bench_run`] 接到窗体上：无头逐帧合成、窗口模式的脚本推进。
//!
//! 都是 `impl crate::MiniAppWindow`，入口全部以 `self.bench.is_some()` 为前提 ——
//! 不带 `--bench` 时调用方根本不进来，快照与交互窗体的行为逐字节不变。
//!
//! 驱动方式与 `run_drag_bench` 相同：手指从 (视口中线, 视口 3/4 高) 按下，每帧上移 `dy`，
//! 走 `sim_press` / `sim_move` / `sim_release`（与交互窗体同一条指针链路）。
use super::bench_run::{save_png, BenchDriver, BenchScene, FrameSample, Mode, Phase, ScrollProbe, Stage};
use super::scroll_bench::DragSpec;
use super::*;
use crate::*;

fn ms(d: Duration) -> f32 {
    d.as_secs_f32() * 1000.0
}

/// 拖动的起点（逻辑坐标）：与 `run_drag_bench` 一致
fn drag_origin() -> (f32, f32) {
    (LOGICAL_WIDTH as f32 / 2.0, LOGICAL_HEIGHT as f32 * 0.75)
}

impl crate::MiniAppWindow {
    /// `--bench` 生效：建驱动器。窗口模式还要先切到 `--route`
    /// （交互窗体平时不认 `--route`，基准要能对任意页面开窗测）
    pub(crate) fn bench_setup(
        &mut self,
        out: std::path::PathBuf,
        shot: Option<std::path::PathBuf>,
        hz: f32,
        windowed: bool,
        scene: BenchScene,
    ) -> Result<(), String> {
        if windowed {
            if let Some(route) = scene.route.as_deref() {
                self.bench_enter_route(route)?;
            }
        } else if scene.route.is_none() {
            eprintln!("⚠️  --bench 没给 --route：所有页面的帧会记进同一份 JSON");
        }
        let mode = if windowed { Mode::Windowed } else { Mode::Headless };
        self.bench = Some(BenchDriver::new(out, shot, hz, mode, scene));
        Ok(())
    }

    /// 无头跑完：写 JSON（写失败以退出码 1 结束）
    pub(crate) fn bench_finish_headless(&self) {
        if let Some(b) = self.bench.as_ref().filter(|b| b.mode == Mode::Headless) {
            b.finish_or_exit(&self.backend);
        }
    }

    /// 与 `snapshot_all` 切页的写法一致（销毁上一页、清栈、再进目标页）
    fn bench_enter_route(&mut self, route: &str) -> Result<(), String> {
        let route = route.trim_start_matches('/').to_string();
        if !self.pages.contains_key(&route) {
            return Err(format!("--route {route}：页面未加载"));
        }
        if self.page_stack.last().map(|p| p.path == route).unwrap_or(false) {
            return Ok(());
        }
        self.unload_current_page();
        self.page_stack.clear();
        self.back_shots.clear();
        self.interaction.clear_page_state();
        self.navigate_to(&route, HashMap::new())
    }

    fn page_cursor(&self) -> Option<(u64, u64)> {
        self.canvas.as_ref().map(|c| c.cursor())
    }

    fn scroll_probe(&self) -> ScrollProbe {
        let mut areas: Vec<(String, f32, f32)> = self
            .interaction
            .scroll_controllers
            .iter()
            .map(|(id, c)| (id.clone(), c.get_position(), c.get_max_scroll()))
            .collect();
        areas.sort_by(|a, b| a.0.cmp(&b.0));
        ScrollProbe { page: self.scroll.get_position(), page_max: self.scroll.get_max_scroll(), areas }
    }

    /// 帧末的状态：守卫要看（S2 动画在跑、S5 弹窗弹着、滚动位置）
    fn fill_frame_state(&self, s: &mut FrameSample) {
        s.animating = self.renderer.as_ref().map(|r| r.has_active_animations()).unwrap_or(false);
        s.modal_visible = self.modal.as_ref().map(|m| m.visible).unwrap_or(false);
        s.scroll_y = self.scroll.get_position();
    }

    // ───────────────────────────── 无头 ─────────────────────────────

    /// `--drag` 的基准版：每帧 移动 → `pump_one_frame` → 额外合成，按 `--hz` 走节拍器
    pub(crate) fn bench_drag(&mut self, spec: &DragSpec, route: &str) {
        let Some(mut b) = self.bench.take() else { return };
        let (cx, mut cy) = drag_origin();
        self.sim_press(cx, cy);
        b.begin_drive(self.scroll_probe());
        b.pacer.reset(Instant::now());
        for i in 0..spec.frames {
            cy -= spec.dy;
            let t0 = Instant::now();
            // 输入处理算进逻辑段（与 run_drag_bench 一样从移动之前开始计时）
            self.sim_move(cx, cy);
            self.bench_headless_frame(&mut b, t0, i == 0);
            b.pacer.wait_next(b.interval());
        }
        b.end_drive(self.scroll_probe());
        self.sim_release(cx, cy);
        self.settle_scroll_animations(2000);
        let work: Vec<f32> = b.frames.iter().map(|f| f.work_ms).collect();
        let tag = compositor::diag::backend_tag(&self.backend);
        app_window::scroll_bench::report(&tag, &format!("{route} --bench"), &work, b.interval());
        self.bench = Some(b);
    }

    /// `--frames N` 的基准版：每帧 `pump_one_frame` → 额外合成
    pub(crate) fn bench_frames(&mut self, n: u32) {
        let Some(mut b) = self.bench.take() else { return };
        b.begin_drive(self.scroll_probe());
        b.pacer.reset(Instant::now());
        for i in 0..n {
            let t0 = Instant::now();
            self.bench_headless_frame(&mut b, t0, i == 0);
            b.pacer.wait_next(b.interval());
        }
        b.end_drive(self.scroll_probe());
        self.bench = Some(b);
    }

    fn bench_headless_frame(&mut self, b: &mut BenchDriver, t0: Instant, first: bool) {
        let interval = b.interval_since(t0);
        let cursor = self.page_cursor();
        // 清零：这一帧没重绘的话分段就是 0，不会把上一帧的数字算到这一帧头上
        self.render_parts = (0.0, 0.0, 0.0);
        self.pump_one_frame();
        let t_pumped = t0.elapsed();
        let scale = self.scale_factor;
        let (pw, ph) = ((LOGICAL_WIDTH as f64 * scale) as u32, (LOGICAL_HEIGHT as f64 * scale) as u32);
        // 缓冲尺寸只在第一帧（或缩放变了）分配，分配不进合成计时
        if b.buf.len() != (pw * ph) as usize {
            b.buf = vec![0u32; (pw * ph) as usize];
        }
        let has_tabbar = self.page_stack.last().map(|p| self.is_tabbar_page(&p.path)).unwrap_or(false);
        let tc = Instant::now();
        let presented = match self.headless_layers(scale, has_tabbar) {
            Some(layers) => {
                compositor::cpu::compose(&mut b.buf, &layers);
                true
            }
            None => false,
        };
        let compose = tc.elapsed();
        let work = t0.elapsed();
        let (rp, rf, rt) = self.render_parts;
        let render_ms = rp + rf + rt;
        let mut s = FrameSample {
            phase: Phase::Drive,
            work_ms: ms(work),
            // render_parts 从 render_with_damage 里取数据之后才开始计，
            // 取数据（`__getRenderData()` 往返）因此落在逻辑段，与窗口模式的分法一致
            logic_ms: (ms(t_pumped) - render_ms).max(0.0),
            render_ms,
            render_parts: self.render_parts,
            compose_ms: ms(compose),
            interval_ms: interval,
            rerastered: cursor != self.page_cursor(),
            presented,
            ..FrameSample::default()
        };
        self.fill_frame_state(&mut s);
        b.push(s);
        if first {
            if let Some(path) = &b.shot {
                match save_png(path, &b.buf, pw, ph) {
                    Ok(()) => println!("🖼  首帧 -> {}", path.display()),
                    Err(e) => eprintln!("⚠️  首帧图：{e}"),
                }
            }
        }
    }

    // ───────────────────────────── 窗口 ─────────────────────────────

    /// `RedrawRequested` 开头：定下这一帧的阶段，并执行这一阶段的输入（eval / 拖动一步）
    pub(crate) fn bench_frame_begin(&mut self, begin: Instant) {
        let Some(mut b) = self.bench.take() else { return };
        b.frame_cursor = self.page_cursor();
        b.present_note = None;
        self.render_parts = (0.0, 0.0, 0.0);
        let settle = Duration::from_secs_f32(b.scene.settle.unwrap_or(0.0).max(0.0));
        if b.stage == Stage::Settle(None) {
            b.stage = Stage::Settle(Some(begin + settle));
        }
        if let Stage::Settle(Some(until)) = b.stage {
            if begin >= until {
                b.stage = if b.scene.evals.is_empty() { Stage::Drive(0) } else { Stage::Eval };
            }
        }
        b.current = match b.stage {
            Stage::Settle(_) => Phase::Settle,
            Stage::Eval => {
                for script in &b.scene.evals {
                    match self.app.eval(script) {
                        Ok(_) => print_js_output(&self.app),
                        Err(e) => eprintln!("⚠️  --eval 执行失败: {}", e),
                    }
                }
                // 下一帧起是 drive；这一帧之后的 UI 指令（showModal 等）由本帧照常处理
                b.stage = Stage::Drive(0);
                Phase::Eval
            }
            Stage::Drive(i) => {
                let (cx, cy) = drag_origin();
                if i == 0 {
                    b.begin_drive(self.scroll_probe());
                    if b.scene.drag.is_some() {
                        self.sim_press(cx, cy);
                    }
                }
                if let Some(d) = b.scene.drag {
                    self.sim_move(cx, cy - d.dy * (i + 1) as f32);
                }
                Phase::Drive
            }
            // 跑完就退出事件循环了，到不了这里；万一多来一帧也只是多记一条
            Stage::Done => Phase::Drive,
        };
        self.bench = Some(b);
    }

    /// `present()` 里合成那一步的耗时与上屏结果
    pub(crate) fn bench_note_present(&mut self, compose_ms: f32, presented: bool) {
        if let Some(b) = self.bench.as_mut() {
            b.present_note = Some((compose_ms, presented));
        }
    }

    /// `RedrawRequested` 上屏之后：记这一帧。drive 跑满就写 JSON，返回 `true`（调用方退出事件循环）
    pub(crate) fn bench_frame_end(&mut self, begin: Instant, t_logic: Duration, t_render: Duration, t_presented: Duration) -> bool {
        let Some(mut b) = self.bench.take() else { return false };
        let interval = b.interval_since(begin);
        let (compose_ms, presented) = b.present_note.take().unwrap_or((0.0, false));
        let mut s = FrameSample {
            phase: b.current,
            work_ms: ms(t_presented),
            logic_ms: ms(t_logic),
            render_ms: ms(t_render.saturating_sub(t_logic)),
            render_parts: self.render_parts,
            compose_ms,
            interval_ms: interval,
            rerastered: b.frame_cursor != self.page_cursor(),
            presented,
            ..FrameSample::default()
        };
        self.fill_frame_state(&mut s);
        b.push(s);
        let mut finished = false;
        if let (Phase::Drive, Stage::Drive(i)) = (b.current, b.stage) {
            let done = i + 1;
            if done >= b.scene.drive_frames().max(1) {
                b.end_drive(self.scroll_probe());
                if let Some(d) = b.scene.drag {
                    let (cx, cy) = drag_origin();
                    self.sim_release(cx, cy - d.dy * done as f32);
                }
                b.stage = Stage::Done;
                b.finish_or_exit(&self.backend);
                finished = true;
            } else {
                b.stage = Stage::Drive(done);
            }
        }
        self.bench = Some(b);
        finished
    }

    /// 开窗之后：记下显示器刷新率（R12），节拍改按 `--hz`，缩放取窗口的
    pub(crate) fn bench_on_window(&mut self, display_millihertz: Option<u32>, scale: f64) {
        if let Some(b) = self.bench.as_mut() {
            b.display_hz = display_millihertz.map(|m| m as f64 / 1000.0);
            b.scene.scale = scale;
            self.frame_interval = b.interval();
            println!("📏 --bench 窗口模式：节拍按 --hz {}（每帧 {:.2}ms）", b.hz, b.interval().as_secs_f64() * 1000.0);
        }
    }
}
