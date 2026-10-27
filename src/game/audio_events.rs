//! pump 里与音频有关的一步（game-doc/02 §8.1：事件在 rAF 之前派发）：离线渲染补到此刻、收回报，事件交回 JS。

use super::GameEngine;

impl GameEngine {
    pub(super) fn dispatch_audio(&mut self, t_us: u64) {
        self.audio.borrow_mut().tick(t_us);
        self.flush_audio_events();
    }

    /// 把排着的音频事件交给 JS。进后台时也直接调：后台不再 pump，pause 事件要当场发出去
    pub(super) fn flush_audio_events(&mut self) {
        let (events, logs, web) = {
            let mut a = self.audio.borrow_mut();
            (a.take_events(), a.take_logs(), a.take_web_events())
        };
        for l in logs {
            self.host_log(&l);
        }
        if events.is_empty() && web.is_empty() {
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
        // WebAudio：源节点 ended、decodeAudioData 的结果（game-webaudio.js 的 __webAudioEvent）
        for ev in web {
            if let Err(e) = self.app.eval(&format!("typeof __webAudioEvent === 'function' && __webAudioEvent({ev})")) {
                errors.push(e);
            }
        }
        self.guards.watchdog.disarm();
        for e in errors {
            self.report_error(super::error_kind(&e, "audio"), &e);
        }
        self.after_js();
    }

    /// 诊断：音频输出统计（JSON，见 GameAudio::stats_json）
    pub fn audio_stats(&self) -> serde_json::Value {
        self.audio.borrow().stats_json()
    }
}
