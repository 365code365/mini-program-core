//! `GameEngine` 的输入入口（宿主调）与每帧派发（pump 第 1 步，见 frame.rs）。
//! 语义在 touch.rs / pc_input.rs；这里只把排好的事件补上 `timeStamp`（游戏时间轴）交给 JS（game-input.js）。
//!
//! 命中优先级（02 §7.4：系统 Modal / 带 mask 的提示 > 原生按钮 > 胶囊 > 游戏）要等 G7 的 showModal / showToast
//! 落地才有覆盖层可拦；届时在 [`GameEngine::input_blocked`] 里判，被拦的序列整段不进游戏。

use super::GameEngine;

impl GameEngine {
    /// 一个指针事件（`phase` 0 下 / 1 移 / 2 抬 / 3 取消；坐标是视图内逻辑像素；`t_us` 宿主时刻，与 pump 同一时钟，0 = 此刻）
    pub fn touch(&mut self, phase: i32, id: i32, x: f32, y: f32, force: f32, t_us: u64) {
        if self.input_blocked() {
            return;
        }
        let (x, y) = self.host_to_game(x, y);
        self.touch.push(phase, id, x, y, force, t_us);
    }

    /// 鼠标（`phase` 0 下 / 1 移 / 2 抬；`button` 0 左 1 中 2 右）
    pub fn mouse(&mut self, phase: i32, button: i32, x: f32, y: f32, t_us: u64) {
        let (x, y) = self.host_to_game(x, y);
        self.pc.mouse(phase, button, x, y, t_us);
    }

    pub fn wheel(&mut self, dx: f32, dy: f32, dz: f32, x: f32, y: f32, t_us: u64) {
        let (x, y) = self.host_to_game(x, y);
        self.pc.wheel(dx, dy, dz, x, y, t_us);
    }
    /// 宿主坐标 → 游戏坐标（引擎内旋转时反变换，rotate.rs；宿主竖屏的宽 = 游戏窗口的高）
    fn host_to_game(&self, x: f32, y: f32) -> (f32, f32) {
        match self.rotation {
            Some(r) => r.to_game(x, y, self.height as f32, self.width as f32),
            None => (x, y),
        }
    }

    pub fn key(&mut self, down: bool, key: &str, code: &str, t_us: u64) {
        self.pc.key(down, key, code, t_us);
    }

    /// 有输入在排队：宿主据此在下一个 vsync 来 pump（静止的游戏没有挂着的 rAF 时也要派发）
    pub fn has_pending_input(&self) -> bool {
        self.touch.has_pending() || self.pc.has_pending()
    }

    /// 覆盖层拦截（G7 之前恒为 false）
    fn input_blocked(&self) -> bool {
        false
    }

    /// 进后台 / surface 没了：按着的手指全部取消，立刻派发（后台不再 pump）
    pub(super) fn cancel_touches(&mut self) {
        self.touch.cancel_all(0);
        self.dispatch_input();
    }

    /// pump 第 1 步：排队的输入按到达顺序进 JS
    pub(super) fn dispatch_input(&mut self) {
        if !self.has_pending_input() {
            return;
        }
        let touches = self.touch.drain();
        for l in self.touch.take_logs() {
            self.host_log(&l);
        }
        let pcs = self.pc.drain();
        let mut errors = Vec::new();
        {
            let clock = self.clock.borrow();
            self.guards.watchdog.arm();
            for e in touches {
                let mut v = e.json;
                v["timeStamp"] = clock.host_to_game_ms(e.t_us).into();
                if let Err(err) = self.app.call_str2("__gameInput", e.kind, &v.to_string()) {
                    errors.push(err);
                }
            }
            for e in pcs {
                let mut v = e.json;
                v["timeStamp"] = clock.host_to_game_ms(e.t_us).into();
                // 键盘沿用 host-capabilities.js 的 wx.onKeyDown / onKeyUp 通道
                let f = if e.kind.starts_with("key") { "__hostDispatch" } else { "__gameInput" };
                if let Err(err) = self.app.call_str2(f, e.kind, &v.to_string()) {
                    errors.push(err);
                }
            }
            self.guards.watchdog.disarm();
        }
        for e in errors {
            self.report_error(super::error_kind(&e, "input"), &e);
        }
        self.after_js();
    }
}
