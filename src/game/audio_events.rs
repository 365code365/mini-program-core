//! pump 里与音频有关的一步（game-doc/02 §8.1：事件在 rAF 之前派发）：离线渲染补到此刻、收回报，事件交回 JS。

use super::GameEngine;

impl GameEngine {
    pub(super) fn dispatch_audio(&mut self, t_us: u64) {
        let (events, logs) = {
            let mut a = self.audio.borrow_mut();
            a.tick(t_us);
            (a.take_events(), a.take_logs())
        };
        for l in logs {
            self.host_log(&l);
        }
        if events.is_empty() {
            return;
        }
        self.guards.watchdog.arm();
        let mut errors = Vec::new();
        for (id, ty, detail) in events {
            let js = format!("__resolveAudio({id}, {}, {})", serde_json::json!(ty), serde_json::json!(detail.to_string()));
            if let Err(e) = self.app.eval(&js) {
                errors.push(e);
            }
        }
        self.guards.watchdog.disarm();
        for e in errors {
            self.report_error(super::error_kind(&e, "audio"), &e);
        }
        self.after_js();
    }

    /// 诊断：(输出帧数, 延迟 µs, 回调块帧数, 回调次数, 采样率, 是否真设备)
    pub fn audio_stats(&self) -> (u64, u32, u32, u64, u32, bool) {
        self.audio.borrow().stats()
    }
}
