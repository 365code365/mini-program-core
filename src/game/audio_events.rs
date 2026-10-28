//! pump 里与音频有关的一步（game-doc/02 §8.1：事件在 rAF 之前派发）：离线渲染补到此刻、收回报，事件交回 JS。

use super::GameEngine;

impl GameEngine {
    pub(super) fn dispatch_audio(&mut self, t_us: u64) {
        self.audio.borrow_mut().tick(t_us);
        self.flush_audio_events();
    }

    /// 把排着的音频事件交给 JS。进后台时也直接调：后台不再 pump，pause 事件要当场发出去
    pub(super) fn flush_audio_events(&mut self) {
        let (events, logs, web, session) = {
            let mut a = self.audio.borrow_mut();
            (a.take_events(), a.take_logs(), a.take_web_events(), a.take_session_event())
        };
        for l in logs {
            self.host_log(&l);
        }
        // setInnerAudioOption → 宿主落到平台（Android 焦点、iOS AVAudioSession，见 audio/session.rs）
        if let Some(s) = session {
            self.events.push_back(s.to_string());
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

    /// 系统音频打断开始 / 结束（宿主：Android 焦点丢失 / 通话，iOS AVAudioSession.interruptionNotification）。
    /// 先把音频停掉、发完 pause / statechange，再派发 wx.onAudioInterruptionBegin / End（audio/session.rs）
    pub fn audio_interruption(&mut self, begin: bool) {
        if !self.audio.borrow_mut().set_interrupted(begin) {
            return;
        }
        self.flush_audio_events();
        let name = if begin { "audioInterruptionBegin" } else { "audioInterruptionEnd" };
        self.guards.watchdog.arm();
        let r = self.app.eval(&format!("typeof __hostDispatch === 'function' && __hostDispatch('{name}', '{{}}')"));
        self.guards.watchdog.disarm();
        if let Err(e) = r {
            self.report_error(super::error_kind(&e, "audio"), &e);
        }
        self.after_js();
    }

    /// 诊断：音频输出统计（JSON，见 GameAudio::stats_json）
    pub fn audio_stats(&self) -> serde_json::Value {
        self.audio.borrow().stats_json()
    }
}
