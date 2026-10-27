//! 窗口与方向（game-doc/04 G6.3，GT-O）：JS 侧在 game-window.js。
//!
//! - 建实例时发宿主事件 `{"type":"deviceOrientation","value":<game.json 的 deviceOrientation>,"initial":true}`：
//!   宿主据此锁方向（Android：portrait → 竖屏、landscape → 横屏随重力正反、landscapeLeft / Right → 固定一边）。
//!   宿主最好在建实例**之前**就按 game.json 锁好（MiniGameView 这样做），否则第一帧拿到的是竖屏尺寸；
//! - `wx.setDeviceOrientation({value})` 同一个事件（不带 initial），宿主转完之后调 `mr_game_resize`；
//! - `mr_game_resize(w, h, dpr, orientation)`：更新系统信息（windowWidth / Height、screenWidth / Height、deviceOrientation），
//!   派发 `wx.onWindowResize({windowWidth, windowHeight})`；方向值变了再派发 `wx.onDeviceOrientationChange({value})`
//!   （value ∈ portrait / landscape / landscapeReverse，宿主给就用宿主的，没给按宽高推）。
//!   **主画布不跟着改尺寸**：微信里也是游戏自己在 onWindowResize 里改 canvas.width / height；改之前画面由系统缩放铺满。

use super::GameEngine;
use serde_json::json;

impl GameEngine {
    /// 宿主窗口变了（转屏、分屏、折叠屏）。`orientation`：宿主知道的实际方向（landscapeReverse 只有宿主分得出来）
    pub fn resize(&mut self, width: u32, height: u32, dpr: f32, orientation: Option<&str>) {
        let (width, height) = (width.max(1), height.max(1));
        // 引擎内旋转：宿主给的竖屏尺寸对调成游戏看到的横屏
        let (width, height) = if self.rotation.is_some() && width < height { (height, width) } else { (width, height) };
        let orientation = if self.rotation.is_some() { Some("landscape") } else { orientation };
        let dpr = if dpr > 0.1 { dpr } else { self.dpr };
        let value = match orientation {
            Some(o @ ("portrait" | "landscape" | "landscapeReverse")) => o.to_string(),
            _ => if width > height { "landscape".into() } else { "portrait".into() },
        };
        let size_changed = (width, height) != (self.width, self.height) || (dpr - self.dpr).abs() > 1e-6;
        let orient_changed = value != self.orientation;
        if !size_changed && !orient_changed {
            return;
        }
        (self.width, self.height, self.dpr) = (width, height, dpr);
        self.orientation = value.clone();
        self.canvases.borrow_mut().window = (width, height);
        self.install_window_info();
        let mut js = String::new();
        if size_changed {
            js.push_str(&format!("__gameWindowResize({});", json!({ "windowWidth": width, "windowHeight": height })));
        }
        if orient_changed {
            js.push_str(&format!("__gameOrientationChange({});", json!({ "value": value })));
        }
        self.guards.watchdog.arm();
        let r = self.app.eval(&format!("typeof __gameWindowResize === 'function' && (function(){{ {js} }})()"));
        self.guards.watchdog.disarm();
        if let Err(e) = r {
            self.report_error(super::error_kind(&e, "resize"), &e);
        }
        self.after_js();
    }

    /// 系统信息按当前尺寸重算、装进 JS（建实例与 resize 共用）
    pub(super) fn install_window_info(&mut self) {
        let eo = crate::host::EngineOptions { system_info: self.system_info_opts.clone(), ..Default::default() };
        let mut info = crate::host::engine_options::system_info(self.width, self.height, self.dpr, &eo);
        info["deviceOrientation"] = json!(if self.orientation == "portrait" { "portrait" } else { "landscape" });
        info["platform"] = json!(super::target_platform());
        info["SDKVersion"] = json!(super::GAME_SDK_VERSION);
        crate::host::engine_options::install_system_info(&self.app, &info);
    }

    /// 建实例时：按 game.json 告诉宿主锁哪个方向
    pub(super) fn announce_orientation(&mut self) {
        let v = self.config.device_orientation.as_str();
        self.events.push_back(json!({ "type": "deviceOrientation", "value": v, "initial": true }).to_string());
    }
}
