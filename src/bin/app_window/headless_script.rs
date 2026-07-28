//! 无头快照的「动作脚本」。
//!
//! 命令行给出的手势与输入按**书写顺序**执行，因此多步交互可以脚本化：
//! `--click 25,520 --type 1212121 --click 313,520` = 点输入框 → 打字 → 点发送。
//!
//! 从前这些动作是 `snapshot_all` 的六个独立参数、按代码里写死的顺序各执行一次，
//! 「先点 A 再点 B」根本表达不出来 —— 而输入框相关的回归恰恰全是多步的。
//! 每个动作都走**与交互窗体完全相同的入口**（`sim_press` / `handle_click` /
//! `handle_text_input`），所以无头验证与真机行为同源。

use std::time::{Duration, Instant};

use super::scroll_bench::DragSpec;

/// 一步动作
#[derive(Debug, Clone)]
pub enum Action {
    /// 按下并保持（验证按压态 / hover-class / 过渡）
    Press(f32, f32),
    /// 一次点击：命中 → 冒泡 → 导航
    Click(f32, f32),
    /// 完整触摸序列：touchstart →（按住 ms，可触发长按）→ touchend/tap
    Touch { x: f32, y: f32, hold_ms: u64 },
    /// 滑动手势；`hold` 为真时停在终点不抬手
    Swipe { from: (f32, f32), to: (f32, f32), steps: u32, hold: bool },
    /// 逐帧计时的拖动（滑动手感测量）
    Drag(DragSpec),
    /// 往当前焦点输入框逐字符输入（等价于真机敲键盘/IME 上屏）
    Type(String),
    /// 特殊键：enter / backspace / escape / left / right / home / end / selectall
    Key(String),
    /// 空转等待若干秒，期间照常出帧
    Wait(f32),
    /// 触控板双指滑动 / 滚轮：光标停在 (x, y)，每步滚 (dx, dy) 逻辑像素。
    /// `dy > 0` = 内容向上走（滚动位置变大），与手指上滑一致。
    Wheel { x: f32, y: f32, dx: f32, dy: f32, steps: u32, precise: bool },
}

/// 解析一个命令行开关。返回 `None` 表示这不是动作类开关。
pub fn parse(flag: &str, value: Option<String>) -> Option<Action> {
    let nums = |v: &str| -> Vec<f32> {
        v.split(',').filter_map(|s| s.trim().parse::<f32>().ok()).collect()
    };
    match flag {
        "--press" | "--click" => {
            let v = nums(&value?);
            let (x, y) = (*v.first()?, *v.get(1)?);
            Some(if flag == "--press" { Action::Press(x, y) } else { Action::Click(x, y) })
        }
        "--touch" => {
            let v = nums(&value?);
            Some(Action::Touch {
                x: *v.first()?,
                y: *v.get(1)?,
                hold_ms: v.get(2).map(|f| *f as u64).unwrap_or(0),
            })
        }
        "--swipe" | "--swipe-hold" => {
            let v = nums(&value?);
            Some(Action::Swipe {
                from: (*v.first()?, *v.get(1)?),
                to: (*v.get(2)?, *v.get(3)?),
                steps: v.get(4).map(|f| *f as u32).unwrap_or(12).max(1),
                hold: flag == "--swipe-hold",
            })
        }
        "--drag" => Some(Action::Drag(super::scroll_bench::parse(&value?)?)),
        "--type" => Some(Action::Type(value?)),
        "--key" => Some(Action::Key(value?.to_ascii_lowercase())),
        "--wait" => Some(Action::Wait(value?.trim().parse().ok()?)),
        "--wheel" | "--wheel-lines" => {
            let v = nums(&value?);
            Some(Action::Wheel {
                x: *v.first()?,
                y: *v.get(1)?,
                dx: v.get(2).copied().unwrap_or(0.0),
                dy: *v.get(3)?,
                steps: v.get(4).map(|f| (*f as u32).max(1)).unwrap_or(1),
                precise: flag == "--wheel",
            })
        }
        _ => None,
    }
}

impl crate::MiniAppWindow {
    /// 依次执行动作脚本
    pub(crate) fn run_actions(&mut self, actions: &[Action], route: &str) {
        for action in actions {
            self.run_action(action, route);
        }
    }

    fn run_action(&mut self, action: &Action, route: &str) {
        // 命中测试依赖上一帧注册的交互元素/事件绑定，先确保出过一帧
        // （真实窗体里也不可能在首帧之前就点击）
        self.render();
        match action {
            Action::Press(x, y) => {
                let ts = Instant::now().elapsed().as_millis() as u64;
                super::events::mouse::handle_mouse_pressed(
                    *x,
                    *y,
                    &mut self.scroll,
                    &mut self.interaction,
                    ts,
                );
                self.needs_redraw = true;
                self.render();
            }
            Action::Click(x, y) => {
                self.handle_click(*x, *y);
                // 点击引发的导航**可能是延后一个微任务的**：框架型产物（uni-app）的事件
                // 代理对冒泡事件走 `nextTick(invoke)`，所以处理函数在下一次 pump 才真正
                // 执行。从前这里「拿不到导航就立刻 break」，等于只泵了一次 —— 点底部导航
                // 在快照工具里永远换不了页（交互窗体反而正常，它每帧都 pump）。
                self.pump_pending_navigation(8);
                super::print_js_output(&self.app);
                self.needs_redraw = true;
                self.render();
            }
            Action::Touch { x, y, hold_ms } => {
                self.sim_press(*x, *y);
                let deadline = Instant::now() + Duration::from_millis(*hold_ms);
                while Instant::now() < deadline {
                    self.pump_one_frame();
                    std::thread::sleep(Duration::from_millis(8));
                }
                self.pump_one_frame();
                self.sim_release(*x, *y);
                self.pump_pending_navigation(8);
                self.settle_scroll_animations(2000);
            }
            Action::Swipe { from, to, steps, hold } => {
                self.sim_press(from.0, from.1);
                for i in 1..=*steps {
                    let t = i as f32 / *steps as f32;
                    self.sim_move(from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
                    self.pump_one_frame();
                    std::thread::sleep(Duration::from_millis(8));
                }
                // `--swipe-hold`：停在终点不抬手，截「手势进行中」的那一帧
                if !*hold {
                    self.sim_release(to.0, to.1);
                    self.pump_pending_navigation(8);
                    self.settle_scroll_animations(2000);
                }
            }
            Action::Drag(spec) => self.run_drag_bench(spec, route),
            Action::Type(text) => {
                if !self.interaction.has_focused_input() {
                    eprintln!("⚠️  --type \"{text}\"：当前没有聚焦的输入框（先 --click 到输入框上）");
                    return;
                }
                // 逐字符走真机那条链路：每个字符都产出一次 InputChange，
                // 于是 `bindinput` 的触发次数与真机一致（微信是每输入一个字符一次）。
                for ch in text.chars() {
                    let buf = ch.to_string();
                    let results = super::events::keyboard::handle_text_input(
                        &buf,
                        false,
                        &mut self.interaction,
                    );
                    self.apply_interaction_results(results);
                }
                self.pump_pending_navigation(8);
                super::print_js_output(&self.app);
                self.needs_redraw = true;
                self.render();
            }
            Action::Key(name) => {
                use mini_render::ui::interaction::KeyInput;
                let key = match name.as_str() {
                    "enter" | "confirm" => KeyInput::Enter,
                    "backspace" => KeyInput::Backspace,
                    "delete" => KeyInput::Delete,
                    "escape" | "esc" => KeyInput::Escape,
                    "left" => KeyInput::Left,
                    "right" => KeyInput::Right,
                    "home" => KeyInput::Home,
                    "end" => KeyInput::End,
                    "selectall" => KeyInput::SelectAll,
                    other => {
                        eprintln!("⚠️  --key {other}：不认识的按键");
                        return;
                    }
                };
                if let Some(r) = self.interaction.handle_key_input(key) {
                    self.apply_interaction_results(vec![r]);
                }
                self.pump_pending_navigation(8);
                super::print_js_output(&self.app);
                self.needs_redraw = true;
                self.render();
            }
            Action::Wait(secs) => {
                let deadline = Instant::now() + Duration::from_secs_f32(*secs);
                while Instant::now() < deadline {
                    self.pump_one_frame();
                    std::thread::sleep(Duration::from_millis(8));
                }
            }
            // 走与交互窗体完全相同的 `MouseWheel` 分支（含覆盖层锁页面、主轴锁定、
            // 嵌套传递），所以触控板手感的回归可以无头复跑
            Action::Wheel { x, y, dx, dy, steps, precise } => {
                use winit::event::MouseScrollDelta;
                self.mouse_pos = (*x, *y);
                let sf = self.scale_factor;
                for _ in 0..*steps {
                    // winit 给的是物理像素、且方向与内容相反（手指上滑 → deltaY 为负）
                    let delta = if *precise {
                        MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(
                            (-*dx * sf as f32) as f64,
                            (-*dy * sf as f32) as f64,
                        ))
                    } else {
                        MouseScrollDelta::LineDelta(-*dx / 20.0, -*dy / 20.0)
                    };
                    let over_fixed = self
                        .renderer
                        .as_ref()
                        .map(|r| r.fixed_layer_hit(*x, *y))
                        .unwrap_or(false);
                    if over_fixed && std::env::var("MINI_SCROLL_LOG").is_ok() {
                        eprintln!("🖱 ({x},{y}) 落在 fixed 覆盖层上 → 页面滚动被锁");
                    }
                    let out = super::handle_mouse_wheel_gated(
                        delta,
                        (*x, *y),
                        &mut self.interaction,
                        &mut self.scroll,
                        sf,
                        over_fixed,
                    );
                    if out.redraw {
                        self.needs_redraw = true;
                    }
                    if out.fixed_dirty {
                        self.fixed_dirty = true;
                    }
                    self.pump_one_frame();
                    std::thread::sleep(Duration::from_millis(8));
                }
                // 触控板抬手（`TouchPhase::Ended`）：越界立即回弹
                self.scroll.end_wheel_gesture();
                for c in self.interaction.scroll_controllers.values_mut() {
                    c.end_wheel_gesture();
                }
                self.fixed_dirty = true;
                self.settle_scroll_animations(2000);
            }
        }
    }

    /// 把一批交互结果交给与交互窗体同一个处理器
    fn apply_interaction_results(
        &mut self,
        results: Vec<mini_render::ui::interaction::InteractionResult>,
    ) {
        for r in results {
            super::interaction_handler::handle_interaction_result(
                &r,
                self.window.as_ref(),
                self.renderer.as_ref(),
                &mut self.app,
                &mut self.clipboard,
                self.scroll.get_position(),
                self.scale_factor,
            );
        }
    }

    /// 泵若干轮，把逻辑层排队的导航请求跑完
    fn pump_pending_navigation(&mut self, rounds: u32) {
        for _ in 0..rounds {
            self.app.update().ok();
            if self.pending_navigation.is_none() {
                self.pending_navigation = super::check_navigation(&mut self.app);
            }
            if self.pending_navigation.is_some() {
                self.process_navigation();
            }
        }
    }
}
