//! `MiniApp` 给小游戏用的两样东西：定时器时钟（跟帧时刻走）与直接调全局函数（rAF 每帧调一次，
//! 不经 `eval` 解析）。拆出来是因为 app.rs 已过 500 行。小程序侧 `timer_clock` 恒为 `None`，行为不变。
use super::app::MiniApp;
use std::time::{Duration, Instant};

impl MiniApp {
    /// 定时器的「此刻」：设了帧时钟用帧时钟，否则墙上时钟（小程序）
    pub(super) fn timer_now(&self) -> Instant {
        self.timer_clock.unwrap_or_else(Instant::now)
    }

    /// 设定定时器时钟（小游戏用；`None` 回到墙上时钟）
    pub fn set_timer_clock(&mut self, at: Option<Instant>) {
        self.timer_clock = at;
    }

    /// 最近一个要到点的定时器还有多少毫秒（没有定时器 None）：宿主据此决定要不要为远期定时器保持出帧
    pub fn next_timer_due_ms(&self) -> Option<u64> {
        let now = self.timer_now();
        self.timers
            .values()
            .map(|s| {
                let due = s.last_trigger + Duration::from_millis(s.delay_ms as u64);
                due.saturating_duration_since(now).as_millis() as u64
            })
            .min()
    }

    /// 直接调一个全局函数（见 `JsRuntime::call_num`）
    pub fn call_num(&self, name: &str, args: &[f64]) -> Result<f64, String> {
        self.runtime.lock().unwrap().call_num(name, args)
    }
}
