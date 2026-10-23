//! `MiniApp::update` 里「长连接与媒体」这一段：WebSocket 事件、InnerAudioContext 的时钟与事件。
//!
//! 与 `process_network_responses` 同一个模式：后台线程 / 播放时钟产生事件，这里每帧取走、
//! 以 JSON 字面量注入回调（不手拼字符串）。拆出来是因为 app.rs 已过 500 行。

use super::app::MiniApp;

impl MiniApp {
    /// WebSocket 与音频：取事件回给 JS（`__resolveSocket` / `__resolveAudio`）
    pub(super) fn process_io_events(&mut self) {
        crate::audio_ctx::tick();
        let sockets = crate::websocket::take_events();
        let audio = crate::audio_ctx::take_events();
        let raw = crate::net_socket::take_events();
        if sockets.is_empty() && audio.is_empty() && raw.is_empty() {
            return;
        }
        // TCP / UDP（wx.createTCPSocket / createUDPSocket）
        for (id, (ty, json)) in raw {
            let js = format!("__resolveNetSocket({id}, {}, {})", serde_json::Value::String(ty.into()), serde_json::Value::String(json));
            if let Err(e) = self.eval(&js) {
                eprintln!("⚠️  TCP / UDP 回调执行失败 (id={id}): {e}");
            }
        }
        for (id, ev) in sockets {
            let (ty, json) = ev.to_js();
            let js = format!("__resolveSocket({id}, {}, {})", serde_json::Value::String(ty.into()), serde_json::Value::String(json));
            if let Err(e) = self.eval(&js) {
                eprintln!("⚠️  WebSocket 回调执行失败 (id={id}): {e}");
            }
        }
        for (id, ty, json) in audio {
            let js = format!("__resolveAudio({id}, {}, {})", serde_json::Value::String(ty.into()), serde_json::Value::String(json));
            if let Err(e) = self.eval(&js) {
                eprintln!("⚠️  音频回调执行失败 (id={id}): {e}");
            }
        }
        // 回调里多半有 setData：与网络回包同一个重绘标记
        crate::net::mark_dirty();
    }

    /// 有连接开着、有音频在播 / 在加载：宿主据此续帧（与「有请求在飞」同一个判据）
    pub(super) fn has_pending_io(&self) -> bool {
        crate::websocket::has_pending() || crate::audio_ctx::has_pending() || crate::net_socket::has_pending()
    }
}
